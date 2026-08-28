import { createHash } from 'node:crypto'
import { readFile, mkdir, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const configuredInstaller = process.argv.find(argument => argument.startsWith('--installer='))?.slice('--installer='.length)
const installer = resolve(root, configuredInstaller || 'src-tauri/target/release/bundle/nsis/Long解压_1.1.15_x64-setup.exe')
const sevenZip = join(root, 'src-tauri', 'resources', 'archive-engine', '7z.exe')
const outputRoot = join(root, 'test-results', 'video-runtime-package')
const extractionRoot = join(outputRoot, 'extracted')
const reportPath = join(outputRoot, 'result.json')
const manifest = JSON.parse(await readFile(join(root, 'config', 'media-dependencies.json'), 'utf8'))
const ffmpeg = manifest.dependencies.find(item => item.id === 'ffmpeg')

function run(command, args, label) {
  const result = spawnSync(command, args, { encoding: 'utf8', windowsHide: true })
  if (result.status !== 0) {
    throw new Error(`${label} failed (${result.status}): ${result.stderr || result.stdout}`)
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`
}

async function identity(path) {
  const bytes = await readFile(path)
  return {
    bytes: bytes.length,
    sha256: createHash('sha256').update(bytes).digest('hex'),
  }
}

await stat(installer)
await rm(extractionRoot, { recursive: true, force: true })
await mkdir(extractionRoot, { recursive: true })
const archiveTest = run(sevenZip, ['t', installer], 'NSIS archive test')
run(sevenZip, ['x', '-y', `-o${extractionRoot}`, installer, 'resources\\video-engine\\*'], 'NSIS video runtime extraction')

const runtimeRoot = join(extractionRoot, 'resources', 'video-engine')
const expectedFiles = [
  ...ffmpeg.runtimeCandidate.files.map(item => ({ ...item, relativePath: item.name })),
  ...ffmpeg.runtimeCandidate.documentationFiles.map(item => ({ ...item, relativePath: item.name })),
  ...ffmpeg.runtimeCandidate.licenseFiles.map(item => ({ ...item, relativePath: join('licenses', item.name) })),
]
const actualFiles = []
const differences = []
for (const expected of expectedFiles) {
  const actual = await identity(join(runtimeRoot, expected.relativePath))
  actualFiles.push({ relativePath: expected.relativePath.replaceAll('\\', '/'), ...actual })
  if (actual.bytes !== expected.bytes) {
    differences.push(`${expected.relativePath}: expected ${expected.bytes} bytes, actual ${actual.bytes}`)
  }
  if (actual.sha256 !== expected.sha256) {
    differences.push(`${expected.relativePath}: expected SHA-256 ${expected.sha256}, actual ${actual.sha256}`)
  }
}

const ffmpegExe = join(runtimeRoot, 'ffmpeg.exe')
const ffprobeExe = join(runtimeRoot, 'ffprobe.exe')
const version = run(ffmpegExe, ['-version'], 'packaged ffmpeg version')
const probeVersion = run(ffprobeExe, ['-version'], 'packaged ffprobe version')
const encoders = run(ffmpegExe, ['-hide_banner', '-encoders'], 'packaged ffmpeg encoders')
const encoderHelp = run(ffmpegExe, ['-hide_banner', '-h', 'encoder=h264_mf'], 'packaged h264_mf options')
const filters = run(ffmpegExe, ['-hide_banner', '-filters'], 'packaged ffmpeg filters')
const fixtureProbe = JSON.parse(run(ffprobeExe, [
  '-v', 'error', '-show_streams', '-of', 'json',
  join(root, 'tests', 'fixtures', 'media', 'videos', 'h264-vfr-audio-rotation-subtitles.mp4'),
], 'packaged ffprobe real fixture'))

const expectedCapabilities = {
  version: '9.0.1',
  licensePolicy: 'LGPL build without GPL/nonfree/external H.264 encoders',
  videoEncoder: 'h264_mf',
  audioEncoder: 'aac',
  hardwareEncodingDefault: false,
  filters: ['scale', 'format', 'fps', 'transpose', 'aresample'],
  realFixtureCodecs: ['h264', 'aac', 'mov_text'],
}
const actualCapabilities = {
  version: version.includes('ffmpeg version 9.0.1') && probeVersion.includes('ffprobe version 9.0.1') ? '9.0.1' : 'unexpected',
  licensePolicy: !version.includes('--enable-gpl') && !version.includes('--enable-nonfree') && !encoders.includes('libx264') && !encoders.includes('libx265'),
  videoEncoder: encoders.includes('h264_mf'),
  audioEncoder: encoders.includes(' AAC (Advanced Audio Coding)'),
  hardwareEncodingDefault: encoderHelp.includes('hw_encoding') && encoderHelp.includes('default false'),
  filters: expectedCapabilities.filters.filter(filter => filters.split(/\r?\n/).some(line => line.split(/\s+/).includes(filter))),
  realFixtureCodecs: fixtureProbe.streams.map(stream => stream.codec_name),
}
if (actualCapabilities.version !== expectedCapabilities.version) differences.push('packaged FFmpeg/ffprobe version is not 9.0.1')
if (!actualCapabilities.licensePolicy) differences.push('packaged FFmpeg violates the LGPL-only encoder/configuration boundary')
if (!actualCapabilities.videoEncoder) differences.push('packaged FFmpeg is missing h264_mf')
if (!actualCapabilities.audioEncoder) differences.push('packaged FFmpeg is missing native AAC')
if (!actualCapabilities.hardwareEncodingDefault) differences.push('packaged h264_mf does not default hw_encoding to false')
for (const filter of expectedCapabilities.filters) {
  if (!actualCapabilities.filters.includes(filter)) differences.push(`packaged FFmpeg is missing filter ${filter}`)
}
for (const codec of expectedCapabilities.realFixtureCodecs) {
  if (!actualCapabilities.realFixtureCodecs.includes(codec)) differences.push(`packaged ffprobe did not observe ${codec} in the real fixture`)
}

const installerIdentity = await identity(installer)
const report = {
  schemaVersion: 1,
  measuredAt: new Date().toISOString(),
  installer: { path: installer, ...installerIdentity },
  archiveIntegrity: archiveTest.includes('Everything is Ok'),
  expected: {
    resourceCount: expectedFiles.length,
    runtimePayloadBytes: ffmpeg.installerImpact.runtimePayloadBytes,
    capabilities: expectedCapabilities,
  },
  actual: {
    resourceCount: actualFiles.length,
    runtimePayloadBytes: actualFiles.reduce((total, item) => total + item.bytes, 0),
    files: actualFiles,
    capabilities: actualCapabilities,
  },
  differences,
  passed: differences.length === 0 && archiveTest.includes('Everything is Ok'),
}
await mkdir(outputRoot, { recursive: true })
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`)
if (!report.passed) throw new Error(`packaged video runtime differed from expectations:\n${differences.join('\n')}`)
console.log(`Real packaged video runtime passed (${actualFiles.length} exact resources, ${report.actual.runtimePayloadBytes} bytes; differences=0).`)

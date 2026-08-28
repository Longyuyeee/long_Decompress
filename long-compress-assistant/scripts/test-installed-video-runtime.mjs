import { createHash } from 'node:crypto'
import { cp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { basename, dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const executable = resolve(process.env.TAURI_APP_BINARY || '')
const evidenceRoot = resolve(
  process.env.VIDEO_RUNTIME_EVIDENCE_DIRECTORY
    || join(root, 'test-results', 'installed-video-runtime'),
)
const fixture = join(root, 'tests', 'fixtures', 'media', 'videos', 'h264-vfr-audio-rotation-subtitles.mp4')
const installRoot = dirname(executable)
const runtimeRoot = join(installRoot, 'resources', 'video-engine')
const output = join(evidenceRoot, 'installed-software-transcode.mp4')
const reportPath = join(evidenceRoot, 'installed-preflight.json')
const sandboxRoot = join(evidenceRoot, 'isolated-install-copy')
const differences = []

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function run(command, args, label, expectedStatus = 0) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    windowsHide: true,
    maxBuffer: 32 * 1024 * 1024,
  })
  if (result.error) throw new Error(`${label} failed to launch: ${result.error.message}`)
  if (result.status !== expectedStatus) {
    throw new Error(
      `${label} exited ${result.status}; expected ${expectedStatus}: ${result.stderr || result.stdout}`,
    )
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

async function invokePreflight(app, report, expectedStatus) {
  await rm(report, { force: true })
  run(app, ['--internal-video-engine-preflight-report', report], 'installed production preflight', expectedStatus)
  return JSON.parse(await readFile(report, 'utf8'))
}

assert(process.env.TAURI_APP_BINARY, 'TAURI_APP_BINARY must point to the formally installed application')
await stat(executable)
await stat(join(runtimeRoot, 'ffmpeg.exe'))
await stat(join(runtimeRoot, 'ffprobe.exe'))
await stat(fixture)
await rm(evidenceRoot, { recursive: true, force: true })
await mkdir(evidenceRoot, { recursive: true })

const installedPreflight = await invokePreflight(executable, reportPath, 0)
assert(installedPreflight.passed === true, 'installed production preflight did not pass')
assert(installedPreflight.status?.version === '9.0.1', 'installed preflight version differs')
assert(installedPreflight.status?.videoEncoder === 'h264_mf', 'installed preflight video encoder differs')
assert(installedPreflight.status?.audioEncoder === 'aac', 'installed preflight audio encoder differs')
assert(installedPreflight.status?.hardwareEncoding === false, 'installed preflight enabled hardware encoding')
assert(installedPreflight.status?.mediaFoundationAvailable === true, 'Media Foundation was not admitted')
assert(installedPreflight.status?.files?.length === 8, 'installed preflight did not verify all eight resources')

const ffmpeg = join(runtimeRoot, 'ffmpeg.exe')
const ffprobe = join(runtimeRoot, 'ffprobe.exe')
const progress = run(ffmpeg, [
  '-hide_banner', '-nostats', '-loglevel', 'error', '-y', '-i', fixture,
  '-map', '0:v:0', '-map', '0:a:0?', '-vf', 'scale=480:-2,format=nv12',
  '-c:v', 'h264_mf', '-hw_encoding', '0', '-rate_control', 'quality', '-quality', '70',
  '-c:a', 'aac', '-b:a', '128k', '-movflags', '+faststart',
  '-progress', 'pipe:1', output,
], 'installed Media Foundation software transcode')
assert(progress.includes('progress=end'), 'installed transcode progress did not reach end')

const probe = JSON.parse(run(ffprobe, [
  '-v', 'error',
  '-show_entries', 'format=format_name,duration,size:stream=index,codec_type,codec_name,width,height',
  '-of', 'json', output,
], 'installed ffprobe output validation'))
const video = probe.streams.find(stream => stream.codec_type === 'video')
const audio = probe.streams.find(stream => stream.codec_type === 'audio')
// The frozen fixture carries a 90-degree display matrix. FFmpeg applies that
// orientation before this C-01 capability smoke scale, so 640x360 becomes the
// visible 360x640 surface and scales to the even 480x854 encoder matrix.
assert(video?.codec_name === 'h264' && video.width === 480 && video.height === 854, 'installed output video facts differ')
assert(audio?.codec_name === 'aac', 'installed output AAC stream is missing')
assert(Math.abs(Number(probe.format.duration) - 1.2) <= 0.05, `installed output duration differs: ${probe.format.duration}`)

await mkdir(sandboxRoot, { recursive: true })
const sandboxExecutable = join(sandboxRoot, basename(executable))
await cp(executable, sandboxExecutable)
await cp(runtimeRoot, join(sandboxRoot, 'resources', 'video-engine'), { recursive: true })

const sandboxFfmpeg = join(sandboxRoot, 'resources', 'video-engine', 'ffmpeg.exe')
const originalFfmpeg = await readFile(sandboxFfmpeg)
await rm(sandboxFfmpeg)
const missingReport = await invokePreflight(
  sandboxExecutable,
  join(evidenceRoot, 'missing-resource-preflight.json'),
  2,
)
assert(missingReport.passed === false, 'missing installed resource was accepted')
assert(missingReport.error?.includes('VIDEO_ENGINE_RESOURCE_MISSING'), 'missing resource classification differs')

const replaced = Buffer.from(originalFfmpeg)
replaced[0] ^= 0xff
await writeFile(sandboxFfmpeg, replaced)
const replacedReport = await invokePreflight(
  sandboxExecutable,
  join(evidenceRoot, 'replaced-resource-preflight.json'),
  2,
)
assert(replacedReport.passed === false, 'replaced installed executable was accepted')
assert(replacedReport.error?.includes('VIDEO_ENGINE_RESOURCE_HASH_MISMATCH'), 'replacement classification differs')

const registryEdition = spawnSync(
  'reg.exe',
  ['query', 'HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion', '/v', 'EditionID'],
  { encoding: 'utf8', windowsHide: true },
)
const editionId = registryEdition.status === 0
  ? registryEdition.stdout.match(/EditionID\s+REG_SZ\s+(.+)/)?.[1]?.trim() || 'unknown'
  : 'unknown'

const finalReport = {
  schemaVersion: 1,
  measuredAt: new Date().toISOString(),
  machine: { platform: process.platform, arch: process.arch, windowsEditionId: editionId },
  expected: {
    productionPreflight: true,
    mediaFoundationAvailable: true,
    videoCodec: 'h264',
    audioCodec: 'aac',
    width: 480,
    height: 854,
    durationSeconds: 1.2,
    missingResourceRefused: true,
    replacedResourceRefused: true,
  },
  actual: {
    executable: { path: executable, ...await identity(executable) },
    runtimeRoot,
    productionPreflight: installedPreflight,
    fixture: { path: fixture, ...await identity(fixture) },
    output: { path: output, ...await identity(output), probe },
    progress: progress.trim().split(/\r?\n/),
    missingResource: missingReport,
    replacedResource: replacedReport,
  },
  differences,
  passed: differences.length === 0,
}
await writeFile(join(evidenceRoot, 'result.json'), `${JSON.stringify(finalReport, null, 2)}\n`)
console.log(`Installed video runtime passed (production preflight, real software transcode, missing/replaced refusal; differences=${differences.length}).`)

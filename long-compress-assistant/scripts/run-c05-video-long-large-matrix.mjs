import { createHash } from 'node:crypto'
import { existsSync } from 'node:fs'
import { mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const manifestPath = join(root, 'tests', 'fixtures', 'media', 'c05-video-long-large-matrix.json')
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
const toolRoot = join(root, 'test-results', 'c05-fixture-tool')
const extractedToolRoot = join(toolRoot, 'extracted')
const fixtureFfmpeg = join(extractedToolRoot, 'ffmpeg-9.0.1-full_build', 'bin', 'ffmpeg.exe')
const productFfprobe = join(root, 'src-tauri', 'resources', 'video-engine', 'ffprobe.exe')
const auditRoot = join(root, 'test-results', 'c05-video-long-large-matrix')
const inputRoot = join(auditRoot, 'inputs')
const outputRoot = join(auditRoot, 'outputs')
const runtimeManifestPath = join(auditRoot, 'runtime-manifest.json')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function run(command, args, label, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    windowsHide: true,
    maxBuffer: 64 * 1024 * 1024,
    ...options,
  })
  assert(
    result.status === 0,
    `${label} failed (${result.status}): ${result.error?.message || result.stderr || result.stdout || 'no output'}`,
  )
  return result.stdout || ''
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}

async function ensureFixtureTool() {
  await mkdir(toolRoot, { recursive: true })
  const archives = (await readdir(toolRoot)).filter(name => name.toLowerCase().endsWith('.zip'))
  let archive = null
  for (const name of archives) {
    const candidate = join(toolRoot, name)
    if (await sha256(candidate) === manifest.fixtureTool.archiveSha256) {
      archive = candidate
      break
    }
  }
  if (!archive) {
    archive = join(toolRoot, 'ffmpeg-9.0.1-full_build.zip')
    run('curl.exe', ['--fail', '--location', '--output', archive, manifest.fixtureTool.archiveUrl], 'C-05.2.2 fixture-tool download')
    assert(await sha256(archive) === manifest.fixtureTool.archiveSha256, 'fixture-tool archive SHA-256 differs')
  }
  if (!existsSync(fixtureFfmpeg)) {
    await rm(extractedToolRoot, { recursive: true, force: true })
    await mkdir(extractedToolRoot, { recursive: true })
    run('tar.exe', ['-xf', archive, '-C', extractedToolRoot], 'C-05.2.2 fixture-tool extraction')
  }
  const version = run(fixtureFfmpeg, ['-version'], 'C-05.2.2 fixture-tool identity')
  assert(version.startsWith('ffmpeg version 9.0.1'), 'fixture-tool version differs')
  assert(version.includes('--enable-gpl'), 'fixture tool must remain explicitly classified as GPL test-only tooling')
  assert(manifest.fixtureTool.productIntegrationAllowed === false, 'fixture tool must remain excluded from product integration')
  return {
    archiveBytes: (await stat(archive)).size,
    archiveSha256: await sha256(archive),
    version: version.split(/\r?\n/)[0],
  }
}

function generateInput(item, destination) {
  run(fixtureFfmpeg, [
    '-hide_banner', '-loglevel', 'error',
    '-f', 'lavfi', '-i', `testsrc2=size=${item.width}x${item.height}:rate=${item.frameRate}:duration=${item.durationSeconds}`,
    '-c:v', 'mpeg4', '-q:v', String(item.quality), '-an', '-y', destination,
  ], `generate ${item.id}`)
}

function probe(path) {
  return JSON.parse(run(productFfprobe, [
    '-v', 'error', '-show_streams', '-show_format', '-count_frames', '-of', 'json', path,
  ], `product ffprobe ${path}`))
}

function videoFacts(probeResult) {
  return probeResult.streams.find(stream => stream.codec_type === 'video')
}

assert(manifest.schemaVersion === 1, 'unsupported C-05.2.2 schema')
assert(manifest.expected.executionCases === 2 && manifest.cases.length === 2, 'two product executions are required')
assert(manifest.expected.longDurationMs === 600_000, 'ten-minute requirement drifted')
assert(manifest.expected.largeInputMinBytes === 100 * 1024 * 1024, 'large-file floor must remain 100 MiB')
assert(new Set(manifest.cases.map(item => item.kind)).size === 2, 'long and large cases must remain independent')

const tool = await ensureFixtureTool()
await rm(auditRoot, { recursive: true, force: true })
await mkdir(inputRoot, { recursive: true })
await mkdir(outputRoot, { recursive: true })

const inputs = []
for (const item of manifest.cases) {
  const path = join(inputRoot, item.file)
  generateInput(item, path)
  const bytes = (await stat(path)).size
  const facts = probe(path)
  const video = videoFacts(facts)
  assert(facts.format.format_name.includes('avi'), `${item.id}: input container differs`)
  assert(video?.codec_name === 'mpeg4', `${item.id}: input video codec differs`)
  assert(video?.width === item.width && video?.height === item.height, `${item.id}: input dimensions differ`)
  assert(!facts.streams.some(stream => stream.codec_type === 'audio'), `${item.id}: input must remain audio-free`)
  assert(bytes >= item.minimumInputBytes, `${item.id}: input is below its byte requirement`)
  assert(Math.abs(Number(facts.format.duration) - item.durationSeconds) <= 0.1, `${item.id}: input duration differs`)
  assert(Number(video.nb_read_frames || 0) >= item.minimumDecodedVideoFrames, `${item.id}: input frame scan is incomplete`)
  inputs.push({ id: item.id, kind: item.kind, path, bytes, sha256: await sha256(path), probe: facts })
}

const runtimeManifest = {
  schemaVersion: 1,
  cases: manifest.cases.map(item => ({
    ...item,
    sourcePath: join(inputRoot, item.file),
  })),
}
await writeFile(runtimeManifestPath, JSON.stringify(runtimeManifest, null, 2), 'utf8')

run('cargo', [
  'test', '--lib',
  'commands::compression::cancellation_tests::c05_real_long_duration_and_large_input_matrix',
  '--', '--exact', '--nocapture',
], 'C-05.2.2 product video matrix', {
  cwd: join(root, 'src-tauri'),
  env: {
    ...process.env,
    LONG_C05_VIDEO_LONG_LARGE_MANIFEST: runtimeManifestPath,
    LONG_C05_VIDEO_LONG_LARGE_OUTPUT: outputRoot,
  },
})

const outputs = []
for (const item of manifest.cases) {
  const path = join(outputRoot, `${item.id}.mp4`)
  const facts = probe(path)
  const video = videoFacts(facts)
  assert(facts.format.format_name.includes('mp4'), `${item.id}: output is not MP4`)
  assert(video?.codec_name === 'h264', `${item.id}: output is not H.264`)
  assert(video?.width === item.outputWidth && video?.height === item.outputHeight, `${item.id}: output dimensions differ`)
  assert(!facts.streams.some(stream => stream.codec_type === 'audio'), `${item.id}: output invented audio`)
  assert(Math.abs(Number(facts.format.duration) - item.durationSeconds) <= 2, `${item.id}: output duration differs`)
  assert(Number(video.nb_read_frames || 0) >= item.minimumDecodedVideoFrames, `${item.id}: output frame scan is incomplete`)
  outputs.push({
    id: item.id,
    kind: item.kind,
    path,
    bytes: (await stat(path)).size,
    sha256: await sha256(path),
    probe: facts,
  })
}

const backend = JSON.parse(await readFile(join(outputRoot, 'backend-result.json'), 'utf8'))
assert(backend.length === manifest.expected.executionCases, 'backend result count differs')
const result = {
  schemaVersion: 1,
  fixtureRevision: manifest.fixtureRevision,
  expected: manifest.expected,
  actual: { tool, inputs, outputs, backend },
  differences: [],
}
await writeFile(join(auditRoot, 'result.json'), `${JSON.stringify(result, null, 2)}\n`, 'utf8')
console.log(`C-05.2.2 real video matrix passed (10-minute input; ${inputs.find(item => item.kind === 'large-input').bytes} byte large input; 2 product executions).`)

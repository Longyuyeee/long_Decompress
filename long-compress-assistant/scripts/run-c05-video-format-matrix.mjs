import { createHash } from 'node:crypto'
import { existsSync } from 'node:fs'
import { mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const manifestPath = join(root, 'tests', 'fixtures', 'media', 'c05-video-format-matrix.json')
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
const toolRoot = join(root, 'test-results', 'c05-fixture-tool')
const extractedToolRoot = join(toolRoot, 'extracted')
const fixtureFfmpeg = join(extractedToolRoot, 'ffmpeg-9.0.1-full_build', 'bin', 'ffmpeg.exe')
const productFfprobe = join(root, 'src-tauri', 'resources', 'video-engine', 'ffprobe.exe')
const auditRoot = join(root, 'test-results', 'c05-video-format-matrix')
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
    run('curl.exe', ['--fail', '--location', '--output', archive, manifest.fixtureTool.archiveUrl], 'C-05.2.1 fixture-tool download')
    assert(await sha256(archive) === manifest.fixtureTool.archiveSha256, 'fixture-tool archive SHA-256 differs')
  }
  if (!existsSync(fixtureFfmpeg)) {
    await rm(extractedToolRoot, { recursive: true, force: true })
    await mkdir(extractedToolRoot, { recursive: true })
    run('tar.exe', ['-xf', archive, '-C', extractedToolRoot], 'C-05.2.1 fixture-tool extraction')
  }
  const version = run(fixtureFfmpeg, ['-version'], 'C-05.2.1 fixture-tool identity')
  assert(version.startsWith('ffmpeg version 9.0.1'), 'fixture-tool version differs')
  assert(version.includes('--enable-gpl'), 'fixture tool must remain explicitly classified as GPL test-only tooling')
  assert(manifest.fixtureTool.productIntegrationAllowed === false, 'fixture tool must remain excluded from product integration')
  return { archive, archiveBytes: (await stat(archive)).size, archiveSha256: await sha256(archive), version: version.split(/\r?\n/)[0] }
}

function generationArguments(source, destination) {
  const args = [
    '-hide_banner', '-loglevel', 'error',
    '-f', 'lavfi', '-i', `testsrc2=size=${source.width}x${source.height}:rate=${source.frameRate}:duration=2`,
  ]
  if (source.audioCodec) {
    args.push('-f', 'lavfi', '-i', 'sine=frequency=660:sample_rate=48000:duration=2')
  }
  const codecs = {
    mp4: ['-c:v', 'libx264', '-preset', 'ultrafast', '-pix_fmt', 'yuv420p', '-c:a', 'aac'],
    mov: ['-c:v', 'mpeg4', '-q:v', '5', '-c:a', 'aac'],
    avi: ['-c:v', 'mpeg4', '-q:v', '5', '-an'],
    wmv: ['-c:v', 'wmv2', '-q:v', '5', '-c:a', 'wmav2'],
    webm: ['-c:v', 'libvpx-vp9', '-deadline', 'realtime', '-cpu-used', '8', '-row-mt', '1', '-b:v', '8M', '-c:a', 'libopus'],
  }
  args.push(...codecs[source.format])
  if (source.audioCodec) args.push('-shortest')
  args.push('-y', destination)
  return args
}

function probe(path) {
  return JSON.parse(run(productFfprobe, [
    '-v', 'error', '-show_streams', '-show_format', '-count_frames', '-of', 'json', path,
  ], `product ffprobe ${path}`))
}

assert(manifest.schemaVersion === 1, 'unsupported C-05.2.1 matrix schema')
assert(manifest.expected.inputFormats.join(',') === 'mp4,mov,avi,wmv,webm', 'input format scope drifted')
assert(manifest.expected.inputResolutionTiers.join(',') === '480p,720p,1080p,4k', 'resolution scope drifted')
assert(manifest.expected.presets.join(',') === 'clear,balanced,small', 'preset scope drifted')
assert(manifest.executions.length === manifest.expected.executionCases, 'execution case count drifted')
assert(new Set(manifest.sources.map(item => item.format)).size === 5, 'five distinct input formats are required')
assert(new Set(manifest.executions.map(item => item.preset)).size === 3, 'all three presets are required')

const tool = await ensureFixtureTool()
await rm(auditRoot, { recursive: true, force: true })
await mkdir(inputRoot, { recursive: true })
await mkdir(outputRoot, { recursive: true })

const generatedInputs = []
for (const source of manifest.sources) {
  const path = join(inputRoot, source.file)
  run(fixtureFfmpeg, generationArguments(source, path), `generate ${source.id}`)
  const facts = probe(path)
  const video = facts.streams.find(stream => stream.codec_type === 'video')
  const audio = facts.streams.find(stream => stream.codec_type === 'audio')
  assert(facts.format.format_name.includes(source.containerNeedle), `${source.id}: container differs`)
  assert(video?.codec_name === source.videoCodec, `${source.id}: video codec differs`)
  assert(video?.width === source.width && video?.height === source.height, `${source.id}: dimensions differ`)
  assert((audio?.codec_name ?? null) === source.audioCodec, `${source.id}: audio codec differs`)
  assert(Number(facts.format.duration) >= 1.9, `${source.id}: duration is too short`)
  generatedInputs.push({ id: source.id, path, bytes: (await stat(path)).size, sha256: await sha256(path), probe: facts })
}

const runtimeManifest = {
  schemaVersion: 1,
  cases: manifest.executions.map(execution => {
    const source = manifest.sources.find(item => item.id === execution.sourceId)
    return {
      ...execution,
      sourcePath: join(inputRoot, source.file),
      inputContainerNeedle: source.containerNeedle,
      inputVideoCodec: source.videoCodec,
      inputAudioCodec: source.audioCodec,
    }
  }),
}
await writeFile(runtimeManifestPath, JSON.stringify(runtimeManifest, null, 2), 'utf8')

run('cargo', [
  'test', '--lib',
  'commands::compression::cancellation_tests::c05_real_format_resolution_preset_matrix',
  '--', '--exact', '--nocapture',
], 'C-05.2.1 product video matrix', {
  cwd: join(root, 'src-tauri'),
  env: {
    ...process.env,
    LONG_C05_VIDEO_MATRIX_MANIFEST: runtimeManifestPath,
    LONG_C05_VIDEO_MATRIX_OUTPUT: outputRoot,
  },
})

const outputs = []
for (const execution of manifest.executions) {
  const source = manifest.sources.find(item => item.id === execution.sourceId)
  const path = join(outputRoot, `${execution.id}.mp4`)
  const facts = probe(path)
  const video = facts.streams.find(stream => stream.codec_type === 'video')
  const audio = facts.streams.find(stream => stream.codec_type === 'audio')
  assert(facts.format.format_name.includes('mp4'), `${execution.id}: output is not MP4`)
  assert(video?.codec_name === 'h264', `${execution.id}: output is not H.264`)
  assert(video?.width === execution.outputWidth && video?.height === execution.outputHeight, `${execution.id}: output dimensions differ`)
  assert((audio?.codec_name ?? null) === (source.audioCodec ? 'aac' : null), `${execution.id}: output audio policy differs`)
  assert(Number(video?.nb_read_frames || 0) > 0, `${execution.id}: output has no decodable video frames`)
  if (source.audioCodec) assert(Number(audio?.nb_read_frames || 0) > 0, `${execution.id}: output has no decodable audio frames`)
  assert(Math.abs(Number(facts.format.duration) - 2) <= 0.25, `${execution.id}: output duration differs`)
  outputs.push({
    id: execution.id,
    sourceId: execution.sourceId,
    preset: execution.preset,
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
  actual: { tool, generatedInputs, outputs, backend },
  differences: [],
}
await writeFile(join(auditRoot, 'result.json'), `${JSON.stringify(result, null, 2)}\n`, 'utf8')
console.log(`C-05.2.1 real video matrix passed (${manifest.sources.length} formats; 4 resolution tiers; ${manifest.executions.length} product executions; 3 presets).`)

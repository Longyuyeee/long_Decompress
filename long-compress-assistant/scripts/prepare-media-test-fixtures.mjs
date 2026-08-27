import { createHash } from 'node:crypto'
import { createWriteStream } from 'node:fs'
import { existsSync } from 'node:fs'
import { mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { join, resolve } from 'node:path'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const manifest = JSON.parse(await readFile(join(root, 'tests', 'fixtures', 'media', 'manifest.json'), 'utf8'))
const auditRoot = join(root, 'test-results', 'media-fixture-audit')
const output = join(auditRoot, 'fixtures')
const pythonPackages = join(auditRoot, 'python-packages')
const archive = join(auditRoot, manifest.testTool.fileName)
const toolRoot = join(auditRoot, 'ffmpeg-tool')
const sevenZip = join(root, 'src-tauri', 'resources', 'archive-engine', '7z.exe')
const python = process.env.LONG_MEDIA_FIXTURE_PYTHON || 'python'
const bundledPdfToPpm = join(
  process.env.USERPROFILE || '',
  '.cache', 'codex-runtimes', 'codex-primary-runtime', 'dependencies', 'native',
  'poppler', 'Library', 'bin', 'pdftoppm.exe',
)
const pdfToPpm = process.env.LONG_MEDIA_PDFTOPPM
  || (existsSync(bundledPdfToPpm) ? bundledPdfToPpm : 'pdftoppm')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function run(command, args, label, options = {}) {
  const result = spawnSync(command, args, { encoding: 'utf8', windowsHide: true, ...options })
  assert(
    result.status === 0,
    `${label} failed (${result.status}): ${result.error?.message || result.stderr || result.stdout || 'no process output'}`,
  )
  return `${result.stdout ?? ''}${result.stderr ?? ''}`
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}

async function ensureTestTool() {
  let valid = false
  try {
    valid = (await stat(archive)).size === manifest.testTool.bytes && (await sha256(archive)) === manifest.testTool.sha256
  } catch {}
  if (!valid) {
    const response = await fetch(manifest.testTool.url, {
      redirect: 'follow',
      headers: { Accept: 'application/octet-stream', 'User-Agent': 'Long-Decompress-B00-Fixture-Audit' },
    })
    assert(response.ok && response.body, `FFmpeg test tool download failed: HTTP ${response.status}`)
    await pipeline(Readable.fromWeb(response.body), createWriteStream(archive))
  }
  assert((await stat(archive)).size === manifest.testTool.bytes, 'FFmpeg test-tool size mismatch')
  assert((await sha256(archive)) === manifest.testTool.sha256, 'FFmpeg test-tool SHA-256 mismatch')
  await rm(toolRoot, { recursive: true, force: true })
  await mkdir(toolRoot, { recursive: true })
  run(sevenZip, ['x', '-y', `-o${toolRoot}`, archive], 'FFmpeg test-tool extraction')
  const bin = join(toolRoot, 'ffmpeg-master-latest-win64-gpl', 'bin')
  return { ffmpeg: join(bin, 'ffmpeg.exe'), ffprobe: join(bin, 'ffprobe.exe') }
}

function ensurePythonPackages() {
  const versionProbe = [
    'import importlib.metadata as m, sys',
    "expected={'Pillow':'12.3.0','reportlab':'4.4.9','pypdf':'6.10.0','pdfplumber':'0.11.9','pyHanko':'0.36.2'}",
    'sys.exit(0 if all(m.version(name) == version for name, version in expected.items()) else 1)',
  ].join('; ')
  const probe = spawnSync(python, ['-c', versionProbe], {
    encoding: 'utf8',
    env: { ...process.env, PYTHONPATH: pythonPackages },
    windowsHide: true,
  })
  if (probe.status === 0) return
  run(python, [
    '-m', 'pip', 'install', '--disable-pip-version-check', '--no-input', '--target', pythonPackages,
    '-r', join(root, 'tests', 'fixtures', 'media', 'requirements.txt'),
  ], 'media fixture Python dependency installation')
}

async function generateVideoFixtures(ffmpeg) {
  const source = join(output, 'video-source')
  const videos = join(output, 'videos')
  await mkdir(videos, { recursive: true })
  const concat = join(source, 'frames.txt')
  const frame = (number) => resolve(source, `frame-${number}.png`).replaceAll('\\', '/')
  await writeFile(concat, [
    `file '${frame(1)}'`, 'duration 0.20',
    `file '${frame(2)}'`, 'duration 0.70',
    `file '${frame(3)}'`, 'duration 0.10',
    `file '${frame(3)}'`,
  ].join('\n'), 'utf8')
  const subtitles = join(source, 'fixture.srt')
  await writeFile(subtitles, '1\n00:00:00,000 --> 00:00:01,000\nSynthetic subtitle track\n', 'utf8')
  const h264 = join(videos, 'h264-vfr-audio-rotation-subtitles.mp4')
  run(ffmpeg, [
    '-hide_banner', '-loglevel', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', concat,
    '-f', 'lavfi', '-i', 'sine=frequency=880:sample_rate=48000:duration=1.2', '-i', subtitles,
    '-map', '0:v:0', '-map', '1:a:0', '-map', '2:0', '-fps_mode', 'vfr',
    '-c:v', 'libx264', '-preset', 'ultrafast', '-crf', '28', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-b:a', '96k',
    '-c:s', 'mov_text', '-shortest', h264,
  ], 'H.264 VFR/audio/subtitle fixture generation')
  const rotated = join(videos, 'h264-rotated.mp4')
  run(ffmpeg, ['-hide_banner', '-loglevel', 'error', '-y', '-display_rotation:v:0', '90', '-i', h264, '-map', '0', '-c', 'copy', rotated], 'H.264 rotation display-matrix pass')
  await rm(h264, { force: true })
  const { rename } = await import('node:fs/promises')
  await rename(rotated, h264)

  run(ffmpeg, [
    '-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i', 'testsrc2=size=640x360:rate=24:duration=1',
    '-c:v', 'libx265', '-preset', 'ultrafast', '-x265-params', 'log-level=error', '-tag:v', 'hvc1', '-an', join(videos, 'h265.mp4'),
  ], 'H.265 fixture generation')
}

function inspectVideos(ffprobe) {
  const actual = {}
  for (const expected of manifest.videos) {
    const path = join(output, 'videos', expected.file)
    const streams = JSON.parse(run(ffprobe, ['-v', 'error', '-show_streams', '-of', 'json', path], `${expected.file} stream probe`)).streams
    const packets = JSON.parse(run(ffprobe, ['-v', 'error', '-select_streams', 'v:0', '-show_entries', 'packet=duration_time', '-of', 'json', path], `${expected.file} packet probe`)).packets
    const video = streams.find((stream) => stream.codec_type === 'video')
    const audio = streams.find((stream) => stream.codec_type === 'audio')
    const subtitle = streams.find((stream) => stream.codec_type === 'subtitle')
    const rotation = Number(video?.side_data_list?.find((item) => 'rotation' in item)?.rotation ?? video?.tags?.rotate ?? 0)
    const durations = [...new Set(packets.map((packet) => packet.duration_time).filter(Boolean))]
    actual[expected.file] = {
      videoCodec: video?.codec_name ?? null,
      audioCodec: audio?.codec_name ?? null,
      subtitleCodec: subtitle?.codec_name ?? null,
      rotation: Math.abs(rotation),
      variableFrameRate: durations.length > 1,
      packetDurations: durations,
      width: video?.width,
      height: video?.height,
    }
  }
  return actual
}

function compare(actual) {
  const differences = []
  for (const expected of manifest.images) {
    const item = actual.images[expected.file]
    for (const key of Object.keys(expected).filter((key) => key !== 'file')) {
      if (JSON.stringify(item?.[key]) !== JSON.stringify(expected[key])) differences.push(`image ${expected.file} ${key}: expected ${JSON.stringify(expected[key])}, got ${JSON.stringify(item?.[key])}`)
    }
  }
  for (const expected of manifest.videos) {
    const item = actual.videos[expected.file]
    for (const key of ['videoCodec', 'audioCodec', 'subtitleCodec', 'rotation', 'variableFrameRate']) {
      if (item?.[key] !== expected[key]) differences.push(`video ${expected.file} ${key}: expected ${expected[key]}, got ${item?.[key]}`)
    }
  }
  for (const expected of manifest.pdfs) {
    const item = actual.pdfs[expected.file]
    if (item?.pages !== expected.pages) differences.push(`PDF ${expected.file}: expected ${expected.pages} pages, got ${item?.pages}`)
    if (expected.expectedText !== undefined && !item?.text?.includes(expected.expectedText)) differences.push(`PDF ${expected.file}: expected text was not extracted`)
    if (expected.fields && JSON.stringify(item?.fields) !== JSON.stringify(expected.fields)) differences.push(`PDF ${expected.file}: form fields differ`)
    if (expected.kind === 'scan' && item?.text !== '') differences.push(`PDF ${expected.file}: scan unexpectedly contains PDF text`)
    if (expected.kind === 'transparency' && !item?.hasTransparency) differences.push(`PDF ${expected.file}: transparency graphics state missing`)
    if (expected.kind === 'digitally-signed') {
      if (!item?.hasByteRange || !item?.hasSignatureContents || item?.signatureCount !== 1) differences.push(`PDF ${expected.file}: embedded signature structure missing`)
      for (const key of ['signatureValid', 'signatureIntact', 'signatureTrusted']) {
        if (item?.[key] !== expected[key]) differences.push(`PDF ${expected.file} ${key}: expected ${expected[key]}, got ${item?.[key]}`)
      }
    }
    if (expected.kind === 'encrypted-refusal' && (!item?.encrypted || !item?.unauthorisedRejected || !item?.authorised)) differences.push(`PDF ${expected.file}: encrypted refusal/authorised boundary failed`)
  }
  return differences
}

async function renderPdfs() {
  const renderRoot = join(auditRoot, 'pdf-renders')
  await rm(renderRoot, { recursive: true, force: true })
  await mkdir(renderRoot, { recursive: true })
  for (const expected of manifest.pdfs) {
    const args = ['-f', '1', '-singlefile', '-png']
    if (expected.kind === 'encrypted-refusal') args.push('-upw', expected.password)
    args.push(join(output, 'pdfs', expected.file), join(renderRoot, expected.file.replace('.pdf', '')))
    run(pdfToPpm, args, `${expected.file} Poppler render`)
  }
  return renderRoot
}

await mkdir(auditRoot, { recursive: true })
await rm(output, { recursive: true, force: true })
await mkdir(output, { recursive: true })
const tools = await ensureTestTool()
ensurePythonPackages()
run(python, [join(root, 'scripts', 'generate-media-fixtures.py'), output], 'image/PDF fixture generation', {
  env: { ...process.env, PYTHONPATH: pythonPackages },
})
await generateVideoFixtures(tools.ffmpeg)
const pythonActual = JSON.parse(await readFile(join(output, 'python-actual.json'), 'utf8'))
const actual = { ...pythonActual, videos: inspectVideos(tools.ffprobe) }
const differences = compare(actual)
const renderRoot = await renderPdfs()
const result = {
  fixtureRevision: manifest.fixtureRevision,
  expected: manifest,
  actual,
  differences,
  renderRoot,
  productIntegrationAllowed: manifest.testTool.productIntegrationAllowed,
}
await writeFile(join(auditRoot, 'result.json'), JSON.stringify(result, null, 2), 'utf8')
assert(differences.length === 0, `media fixture differences:\n${differences.join('\n')}`)
console.log(`Real media fixture baseline passed (${manifest.images.length} images, ${manifest.videos.length} videos, ${manifest.pdfs.length} PDFs).`)

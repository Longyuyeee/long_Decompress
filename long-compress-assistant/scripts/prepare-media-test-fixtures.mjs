import { createHash } from 'node:crypto'
import { createWriteStream } from 'node:fs'
import { existsSync } from 'node:fs'
import { mkdir, readFile, rename, rm, stat, writeFile } from 'node:fs/promises'
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
const imagesOnly = process.argv.includes('--images-only')
assertKnownArguments()

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function assertKnownArguments() {
  const unknown = process.argv.slice(2).filter(argument => argument !== '--images-only')
  if (unknown.length > 0) throw new Error(`Unknown media fixture argument: ${unknown.join(', ')}`)
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

const wait = (milliseconds) => new Promise(resolve => setTimeout(resolve, milliseconds))

async function downloadTestTool() {
  const partial = `${archive}.part`
  const maximumAttempts = 4
  const stallTimeoutMs = 60_000

  for (let attempt = 1; attempt <= maximumAttempts; attempt += 1) {
    let offset = 0
    try {
      offset = (await stat(partial)).size
      if (offset > manifest.testTool.bytes) {
        await rm(partial, { force: true })
        offset = 0
      }
    } catch {}

    const controller = new AbortController()
    let stallTimer
    const resetStallTimer = () => {
      clearTimeout(stallTimer)
      stallTimer = setTimeout(
        () => controller.abort(new Error(`FFmpeg test-tool download stalled for ${stallTimeoutMs / 1000} seconds`)),
        stallTimeoutMs,
      )
    }

    try {
      const headers = {
        Accept: 'application/octet-stream',
        'User-Agent': 'Long-Decompress-B00-Fixture-Audit',
      }
      if (offset > 0) headers.Range = `bytes=${offset}-`
      process.stdout.write(`FFmpeg test-tool download attempt ${attempt}/${maximumAttempts} from byte ${offset}.\n`)
      resetStallTimer()
      const response = await fetch(manifest.testTool.url, {
        redirect: 'follow',
        headers,
        signal: controller.signal,
      })
      assert(response.ok && response.body, `FFmpeg test tool download failed: HTTP ${response.status}`)

      if (offset > 0 && response.status !== 206) {
        await rm(partial, { force: true })
        offset = 0
      }
      if (offset > 0) {
        const contentRange = response.headers.get('content-range') || ''
        assert(contentRange.startsWith(`bytes ${offset}-`), `FFmpeg test-tool resume range mismatch: ${contentRange || 'missing'}`)
      }

      let received = offset
      let nextProgress = Math.max(16 * 1024 * 1024, Math.ceil((received + 1) / (16 * 1024 * 1024)) * 16 * 1024 * 1024)
      const body = Readable.fromWeb(response.body)
      body.on('data', chunk => {
        received += chunk.length
        resetStallTimer()
        if (received >= nextProgress || received === manifest.testTool.bytes) {
          process.stdout.write(`FFmpeg test-tool download: ${received}/${manifest.testTool.bytes} bytes.\n`)
          nextProgress += 16 * 1024 * 1024
        }
      })
      await pipeline(body, createWriteStream(partial, { flags: offset > 0 ? 'a' : 'w' }))
      clearTimeout(stallTimer)
      assert(received === manifest.testTool.bytes, `FFmpeg test-tool download incomplete: ${received}/${manifest.testTool.bytes} bytes`)
      assert((await sha256(partial)) === manifest.testTool.sha256, 'FFmpeg test-tool downloaded SHA-256 mismatch')
      await rm(archive, { force: true })
      await rename(partial, archive)
      return
    } catch (error) {
      clearTimeout(stallTimer)
      if (attempt === maximumAttempts) throw error
      process.stderr.write(`FFmpeg test-tool download attempt ${attempt} failed: ${error instanceof Error ? error.message : String(error)}. Retrying.\n`)
      await wait(attempt * 1_000)
    }
  }
}

async function ensureTestTool() {
  let valid = false
  try {
    valid = (await stat(archive)).size === manifest.testTool.bytes && (await sha256(archive)) === manifest.testTool.sha256
  } catch {}
  if (!valid) {
    await downloadTestTool()
  }
  assert((await stat(archive)).size === manifest.testTool.bytes, 'FFmpeg test-tool size mismatch')
  assert((await sha256(archive)) === manifest.testTool.sha256, 'FFmpeg test-tool SHA-256 mismatch')
  await rm(toolRoot, { recursive: true, force: true })
  await mkdir(toolRoot, { recursive: true })
  run(sevenZip, ['x', '-y', `-o${toolRoot}`, archive], 'FFmpeg test-tool extraction')
  const bin = join(toolRoot, 'ffmpeg-master-latest-win64-gpl', 'bin')
  return { ffmpeg: join(bin, 'ffmpeg.exe'), ffprobe: join(bin, 'ffprobe.exe') }
}

function ensurePythonPackages(imageScopeOnly = false) {
  const versionProbe = [
    'import importlib.metadata as m, sys',
    imageScopeOnly
      ? "expected={'Pillow':'12.3.0'}"
      : "expected={'Pillow':'12.3.0','reportlab':'4.4.9','pypdf':'6.10.0','pdfplumber':'0.11.9','pyHanko':'0.36.2'}",
    'sys.exit(0 if all(m.version(name) == version for name, version in expected.items()) else 1)',
  ].join('; ')
  const probe = spawnSync(python, ['-c', versionProbe], {
    encoding: 'utf8',
    env: { ...process.env, PYTHONPATH: pythonPackages },
    windowsHide: true,
  })
  if (probe.status === 0) return
  const requirementArguments = imageScopeOnly
    ? ['Pillow==12.3.0']
    : ['-r', join(root, 'tests', 'fixtures', 'media', 'requirements.txt')]
  run(python, [
    '-m', 'pip', 'install', '--disable-pip-version-check', '--no-input', '--target', pythonPackages,
    ...requirementArguments,
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

function compare(actual, imageScopeOnly = false) {
  const differences = []
  for (const expected of manifest.images) {
    const item = actual.images[expected.file]
    for (const key of Object.keys(expected).filter((key) => key !== 'file')) {
      if (JSON.stringify(item?.[key]) !== JSON.stringify(expected[key])) differences.push(`image ${expected.file} ${key}: expected ${JSON.stringify(expected[key])}, got ${JSON.stringify(item?.[key])}`)
    }
  }
  if (imageScopeOnly) return differences
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
ensurePythonPackages(imagesOnly)
run(python, [join(root, 'scripts', 'generate-media-fixtures.py'), output, ...(imagesOnly ? ['--images-only'] : [])], 'image/PDF fixture generation', {
  env: { ...process.env, PYTHONPATH: pythonPackages },
})
if (imagesOnly) {
  const actual = JSON.parse(await readFile(join(output, 'python-actual.json'), 'utf8'))
  const differences = compare(actual, true)
  const frozen = JSON.parse(await readFile(join(root, 'tests', 'fixtures', 'media', 'image-baseline.json'), 'utf8'))
  for (const expected of frozen.inputs) {
    const path = join(output, 'images', expected.file)
    assert((await stat(path)).size === expected.bytes, `${expected.file}: frozen byte size drifted`)
    assert((await sha256(path)) === expected.sha256, `${expected.file}: frozen SHA-256 drifted`)
  }
  const rejectionPdf = join(output, 'pdfs', 'rejected-input.pdf')
  assert((await stat(rejectionPdf)).size > 0, 'B-02 PDF rejection fixture is empty')
  assert((await readFile(rejectionPdf)).subarray(0, 5).toString('ascii') === '%PDF-', 'B-02 PDF rejection fixture is not a real PDF')
  await writeFile(join(auditRoot, 'image-workspace-result.json'), JSON.stringify({ actual, differences }, null, 2), 'utf8')
  assert(differences.length === 0, `image fixture differences:\n${differences.join('\n')}`)
  console.log(`Real image fixture baseline passed (${manifest.images.length} images and one PDF rejection boundary).`)
} else {
const tools = await ensureTestTool()
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
}

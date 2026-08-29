import { createHash } from 'node:crypto'
import { existsSync } from 'node:fs'
import { copyFile, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const manifest = JSON.parse(await readFile(join(root, 'tests', 'fixtures', 'media', 'manifest.json'), 'utf8'))
const auditRoot = join(root, 'test-results', 'media-fixture-audit')
const output = join(auditRoot, 'fixtures')
const pythonPackages = join(auditRoot, 'python-packages')
const frozenVideoRoot = join(root, 'tests', 'fixtures', 'media', 'videos')
const productFfprobe = join(root, 'src-tauri', 'resources', 'video-engine', 'ffprobe.exe')
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

async function prepareFrozenVideoFixtures() {
  const targetRoot = join(output, 'videos')
  await mkdir(targetRoot, { recursive: true })
  for (const expected of manifest.videos) {
    const source = join(frozenVideoRoot, expected.file)
    const target = join(targetRoot, expected.file)
    assert((await stat(source)).size === expected.bytes, `${expected.file}: frozen source byte size drifted`)
    assert((await sha256(source)) === expected.sha256, `${expected.file}: frozen source SHA-256 drifted`)
    await copyFile(source, target)
    assert((await stat(target)).size === expected.bytes, `${expected.file}: copied byte size drifted`)
    assert((await sha256(target)) === expected.sha256, `${expected.file}: copied SHA-256 drifted`)
  }
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
    if (expected.annotationSubtypes && JSON.stringify(item?.annotationSubtypes) !== JSON.stringify(expected.annotationSubtypes)) differences.push(`PDF ${expected.file}: annotation subtypes differ`)
    if (expected.outlineTitles && JSON.stringify(item?.outlineTitles) !== JSON.stringify(expected.outlineTitles)) differences.push(`PDF ${expected.file}: outline titles differ`)
    if (expected.attachments && JSON.stringify(item?.attachments) !== JSON.stringify(expected.attachments)) differences.push(`PDF ${expected.file}: attachments differ`)
    if (expected.imageCount !== undefined && item?.imageCount !== expected.imageCount) differences.push(`PDF ${expected.file}: expected ${expected.imageCount} images, got ${item?.imageCount}`)
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
await prepareFrozenVideoFixtures()
const pythonActual = JSON.parse(await readFile(join(output, 'python-actual.json'), 'utf8'))
const actual = { ...pythonActual, videos: inspectVideos(productFfprobe) }
const differences = compare(actual)
const renderRoot = await renderPdfs()
const result = {
  fixtureRevision: manifest.fixtureRevision,
  expected: manifest,
  actual,
  differences,
  renderRoot,
  videoFixtureSource: manifest.videoFixtureSource,
}
await writeFile(join(auditRoot, 'result.json'), JSON.stringify(result, null, 2), 'utf8')
assert(differences.length === 0, `media fixture differences:\n${differences.join('\n')}`)
console.log(`Real media fixture baseline passed (${manifest.images.length} images, ${manifest.videos.length} videos, ${manifest.pdfs.length} PDFs).`)
}

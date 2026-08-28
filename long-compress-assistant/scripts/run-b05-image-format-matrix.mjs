import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const manifestPath = join(root, 'tests', 'fixtures', 'media', 'b05-image-format-matrix.json')
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
const fixtureRoot = join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'images')
const auditRoot = join(root, 'test-results', 'b05-image-format-matrix')
const outputRoot = join(auditRoot, 'outputs')
const python = process.env.LONG_MEDIA_FIXTURE_PYTHON || 'python'
const pythonPackages = join(root, 'test-results', 'media-fixture-audit', 'python-packages')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function run(command, args, label, options = {}) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', windowsHide: true, ...options })
  assert(result.status === 0, `${label} failed (${result.status}): ${result.error?.message || result.stderr || result.stdout}`)
  return result.stdout || ''
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}

assert(manifest.schemaVersion === 1, 'unsupported B-05.1 matrix schema')
assert(manifest.expected.publicFormats.join(',') === 'jpeg,png,webp', 'B-05.1 public format scope drifted')
assert(manifest.expected.samplesPerFormat === 3, 'B-05.1 must require three samples per public format')
assert(manifest.cases.length === manifest.expected.totalCases, 'B-05.1 case count drifted')
assert(manifest.expected.animationBoundary === 'gif-rejected-without-output', 'B-05.1 animation boundary drifted')
assert(manifest.cases.some(item => item.width <= 128 && item.height <= 96), 'B-05.1 small-image coverage is missing')
assert(manifest.cases.some(item => item.width * item.height >= 2_000_000), 'B-05.1 large-image coverage is missing')
assert(manifest.cases.some(item => item.hasAlpha), 'B-05.1 alpha coverage is missing')
assert(manifest.cases.some(item => item.metadata), 'B-05.1 metadata coverage is missing')
const counts = Object.fromEntries(manifest.expected.publicFormats.map(format => [format, 0]))
const inputs = []
for (const expected of manifest.cases) {
  assert(expected.format in counts, `${expected.file}: undeclared format ${expected.format}`)
  if (expected.format !== 'png') assert(Number.isFinite(expected.minimumPsnrDb), `${expected.file}: lossy quality floor is missing`)
  counts[expected.format] += 1
  const path = join(fixtureRoot, expected.file)
  const bytes = (await stat(path)).size
  const digest = await sha256(path)
  assert(bytes === expected.bytes, `${expected.file}: expected ${expected.bytes} input bytes, got ${bytes}`)
  assert(digest === expected.sha256, `${expected.file}: frozen SHA-256 drifted`)
  inputs.push({ file: expected.file, format: expected.format, bytes, sha256: digest })
}
for (const [format, count] of Object.entries(counts)) {
  assert(count === manifest.expected.samplesPerFormat, `${format}: expected 3 real samples, got ${count}`)
}

await rm(auditRoot, { recursive: true, force: true })
await mkdir(outputRoot, { recursive: true })
run('cargo', [
  'test', '--lib',
  'services::image_compression_service::tests::b05_public_format_matrix_uses_real_production_compression',
  '--', '--exact', '--nocapture',
], 'B-05.1 production compression matrix', {
  cwd: join(root, 'src-tauri'),
  env: { ...process.env, LONG_B05_IMAGE_MATRIX_OUTPUT: outputRoot },
})
const verification = JSON.parse(run(
  python,
  [join(root, 'scripts', 'verify-b05-image-format-matrix.py'), manifestPath, fixtureRoot, outputRoot],
  'B-05.1 decoded output verification',
  { env: { ...process.env, PYTHONPATH: pythonPackages } },
))
assert(verification.differences.length === manifest.expected.decodedDifferences, `decoded differences: ${verification.differences.join('; ')}`)

const outputs = []
for (const item of verification.actual) {
  const expected = manifest.cases.find(candidate => candidate.file === item.file)
  const extension = expected.format === 'jpeg' ? 'jpg' : expected.format
  const path = join(outputRoot, `${expected.file}.compressed.${extension}`)
  outputs.push({ ...item, sha256: await sha256(path) })
}
const animationOutput = join(outputRoot, 'animated.gif.compressed.webp')
const animationRejectedWithoutOutput = await stat(animationOutput).then(() => false, () => true)
assert(animationRejectedWithoutOutput, 'animated GIF boundary unexpectedly published an output')
const result = {
  schemaVersion: 1,
  fixtureRevision: manifest.fixtureRevision,
  expected: manifest.expected,
  actual: {
    samplesPerFormat: counts,
    totalCases: inputs.length,
    animationBoundary: animationRejectedWithoutOutput ? 'gif-rejected-without-output' : 'unexpected-output',
    decodedDifferences: verification.differences.length,
    inputs,
    outputs,
  },
  differences: verification.differences,
}
await writeFile(join(auditRoot, 'result.json'), JSON.stringify(result, null, 2), 'utf8')
console.log(`B-05.1 real production image matrix passed (${inputs.length} cases; JPEG/PNG/WebP=${Object.values(counts).join('/')}; decoded differences=${verification.differences.length}).`)

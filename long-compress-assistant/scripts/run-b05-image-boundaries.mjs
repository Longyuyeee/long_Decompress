import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const outputRoot = path.join(root, 'test-results', 'b05-image-boundaries')
const overLimitImage = path.join(outputRoot, 'over-100mp-valid.png')
const rustReportPath = path.join(outputRoot, 'rust-actual.json')
const resultPath = path.join(outputRoot, 'result.json')
const python = process.env.LONG_MEDIA_FIXTURE_PYTHON || 'python'
const pythonPackages = path.join(root, 'test-results', 'media-fixture-audit', 'python-packages')

rmSync(outputRoot, { recursive: true, force: true })
mkdirSync(outputRoot, { recursive: true })

function run(command, args, label, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    windowsHide: true,
    maxBuffer: 32 * 1024 * 1024,
    ...options,
  })
  assert.equal(
    result.status,
    0,
    `${label} failed (${result.status}): ${result.error?.message || result.stderr || result.stdout || 'no output'}`,
  )
  if (result.stdout) process.stdout.write(result.stdout)
  if (result.stderr) process.stderr.write(result.stderr)
}

run(
  python,
  [path.join(root, 'scripts', 'generate-b05-image-boundary-fixture.py'), overLimitImage],
  'valid over-100MP PNG generation',
  { env: { ...process.env, PYTHONPATH: pythonPackages } },
)
assert.ok(statSync(overLimitImage).size > 0, 'the valid over-limit PNG must contain real bytes')

run(
  'cargo',
  [
    'test',
    '--lib',
    '--manifest-path', path.join(root, 'src-tauri', 'Cargo.toml'),
    'services::image_compression_service::tests::b05_2_2_real_resource_and_failure_boundaries',
    '--',
    '--exact',
    '--nocapture',
  ],
  'B-05.2.2 production image boundary test',
  {
    env: {
      ...process.env,
      LONG_B05_IMAGE_BOUNDARY_REPORT: rustReportPath,
      LONG_B05_OVER_LIMIT_IMAGE: overLimitImage,
    },
  },
)

const actual = JSON.parse(readFileSync(rustReportPath, 'utf8'))
const expected = {
  resourceBelowLimitPixels: 96_000_000,
  resourceBelowLimitDecoded: true,
  resourceAboveLimitRejected: true,
  resourceAboveLimitOutputExists: false,
  longPathPublished: true,
  longPathSourceUnchanged: true,
  conflictSkip: true,
  conflictRename: true,
  conflictReplaceRejected: true,
  targetRacePreservedExisting: true,
  targetRaceStagingFiles: 0,
  storageFullKind: 'storage-full',
  storageFullOutputExists: false,
  storageFullStagingFiles: 0,
  cancelledAfterEncodingStarted: true,
  cancelledOutputExists: false,
  cancelledStagingFiles: 0,
}
const differences = Object.entries(expected)
  .filter(([key, value]) => JSON.stringify(actual[key]) !== JSON.stringify(value))
  .map(([key, value]) => `${key}: expected ${JSON.stringify(value)}, got ${JSON.stringify(actual[key])}`)
if (!(actual.longPathUtf16 > 260)) {
  differences.push(`longPathUtf16: expected >260, got ${actual.longPathUtf16}`)
}
const overLimitBytes = statSync(overLimitImage).size
const overLimitSha256 = createHash('sha256').update(readFileSync(overLimitImage)).digest('hex')
const result = {
  scope: 'B-05.2.2 real image resource and failure boundaries',
  expected,
  actual: { ...actual, overLimitBytes, overLimitSha256 },
  differences,
}
writeFileSync(resultPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8')
assert.deepEqual(differences, [], `B-05.2.2 expected/actual differences:\n${differences.join('\n')}`)
console.log(`B-05.2.2 real production image boundaries passed (${actual.elapsedMs} ms; differences=0).`)

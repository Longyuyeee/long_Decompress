import { spawnSync } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const outputRoot = join(root, 'test-results', 'd03-pdf-publication')
const files = [
  'text-vector.pdf',
  'scanned-image.pdf',
  'mixed-content.pdf',
  'transparency.pdf',
  'form.pdf',
  'annotation.pdf',
  'outline.pdf',
  'attachment.pdf',
]
const modes = ['lossless-organization', 'compatible-image-optimization']
const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const result = spawnSync(cargo, [
  'test', '--manifest-path', join(root, 'src-tauri', 'Cargo.toml'), '--lib',
  'services::pdf_publish::tests::publishes_real_pdf_matrix_and_rejects_post_validation_failures',
  '--', '--ignored', '--exact', '--nocapture',
], {
  cwd: root,
  encoding: 'utf8',
  windowsHide: true,
  maxBuffer: 64 * 1024 * 1024,
  env: { ...process.env, LONG_MEDIA_FIXTURE_PYTHON: process.env.LONG_MEDIA_FIXTURE_PYTHON || 'python' },
})
if (result.status !== 0) throw new Error(`D-03.3 real publication test failed: ${result.stderr || result.stdout || result.error?.message}`)
const marker = 'D03_PDF_PUBLICATION_RESULT='
const line = result.stdout.split(/\r?\n/u).find(value => value.startsWith(marker))
if (!line) throw new Error('D-03.3 real publication test did not emit structured evidence')
const actual = JSON.parse(line.slice(marker.length))
const reports = new Map(actual.reports.map(report => [`${report.file}:${report.mode}`, report]))
const comparisons = []

for (const file of files) {
  for (const mode of modes) {
    const report = reports.get(`${file}:${mode}`)
    const expected = {
      published: true,
      outputBytesPositive: true,
      outputNoLarger: true,
      verifiedHashMatchesFinal: true,
      sourceHashUnchanged: true,
      independentFactsEqual: true,
      markOfTheWeb: 'not-present',
      stagingCount: 0,
    }
    const observed = {
      published: report?.finalOutputExists ?? false,
      outputBytesPositive: (report?.outputBytes ?? 0) > 0,
      outputNoLarger: (report?.outputBytes ?? Number.MAX_SAFE_INTEGER) <= (report?.inputBytes ?? -1),
      verifiedHashMatchesFinal: report?.outputSha256 === report?.finalSha256,
      sourceHashUnchanged: report?.sourceHashUnchanged ?? false,
      independentFactsEqual: report?.independentFactsEqual ?? false,
      markOfTheWeb: report?.markOfTheWeb ?? null,
      stagingCount: report?.stagingCount ?? null,
    }
    const differences = Object.keys(expected).filter(key => JSON.stringify(expected[key]) !== JSON.stringify(observed[key]))
    comparisons.push({ case: `${file}:${mode}`, expected, actual: observed, observedOutputBytes: report?.outputBytes ?? null, differences })
  }
}

for (const [caseName, expected, observed] of [
  ['source-change', 'PDF_PUBLISH_SOURCE_CHANGED', actual.sourceChangeError],
  ['staging-change', 'PDF_PUBLISH_STAGING_CHANGED', actual.stagingChangeError],
  ['target-race', 'PDF_PUBLISH_TARGET_APPEARED', actual.targetRaceError],
  ['post-validation-cancellation', 'PDF_PUBLISH_CANCELLED', actual.cancelError],
]) {
  comparisons.push({ case: caseName, expected, actual: observed, differences: expected === observed ? [] : ['error'] })
}
for (const [caseName, observed] of [
  ['source-change-output-absent', actual.sourceChangeOutputAbsent],
  ['staging-change-output-absent', actual.stagingChangeOutputAbsent],
  ['target-race-bytes-preserved', actual.targetRaceBytesPreserved],
  ['cancel-output-absent', actual.cancelOutputAbsent],
  ['motw-output-exists', actual.motwOutputExists],
  ['duplicate-lock-rejected', actual.duplicateLockRejected],
  ['lock-released-after-drop', actual.lockReleasedAfterDrop],
]) comparisons.push({ case: caseName, expected: true, actual: observed, differences: observed === true ? [] : ['result'] })
comparisons.push({
  case: 'motw-policy',
  expected: { status: 'applied', finalZone: 3 },
  actual: { status: actual.motwStatus, finalZone: actual.motwFinalZone },
  differences: actual.motwStatus === 'applied' && actual.motwFinalZone === 3 ? [] : ['markOfTheWeb'],
})
comparisons.push({ case: 'staging-cleanup', expected: 0, actual: actual.stagingFilesRemaining, differences: actual.stagingFilesRemaining === 0 ? [] : ['stagingFilesRemaining'] })

const differenceCount = comparisons.reduce((count, comparison) => count + comparison.differences.length, 0)
const evidence = {
  schemaVersion: 1,
  node: 'D-03.3',
  runtime: 'src-tauri/resources/pdf-engine/qpdf.exe',
  independentInspector: 'scripts/inspect-d01-pdf.py (pypdf)',
  testKind: 'real-atomic-pdf-publication-and-failure-rollback',
  expectedVsActual: comparisons,
  differenceCount,
  passed: differenceCount === 0,
}
await mkdir(outputRoot, { recursive: true })
await writeFile(join(outputRoot, 'result.json'), JSON.stringify(evidence, null, 2), 'utf8')
if (differenceCount !== 0) throw new Error(`D-03.3 expected/actual differences remain: ${differenceCount}`)
console.log(`D-03.3 real PDF publication passed (${comparisons.length} comparisons; expected/actual differences: ${differenceCount}).`)

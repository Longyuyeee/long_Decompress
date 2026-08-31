import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const fixtureRoot = join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'pdfs')
const outputRoot = join(root, 'test-results', 'd03-pdf-validation')
const cases = [
  { file: 'text-vector.pdf', pages: 1, formFields: 0, annotations: 0, outlines: 0, attachments: 0 },
  { file: 'scanned-image.pdf', pages: 1, formFields: 0, annotations: 0, outlines: 0, attachments: 0 },
  { file: 'mixed-content.pdf', pages: 1, formFields: 0, annotations: 0, outlines: 0, attachments: 0 },
  { file: 'transparency.pdf', pages: 1, formFields: 0, annotations: 0, outlines: 0, attachments: 0 },
  { file: 'form.pdf', pages: 1, formFields: 2, annotations: 1, outlines: 0, attachments: 0 },
  { file: 'annotation.pdf', pages: 1, formFields: 0, annotations: 1, outlines: 0, attachments: 0 },
  { file: 'outline.pdf', pages: 2, formFields: 0, annotations: 0, outlines: 2, attachments: 0 },
  { file: 'attachment.pdf', pages: 1, formFields: 0, annotations: 0, outlines: 0, attachments: 1 },
]
const modes = ['lossless-organization', 'compatible-image-optimization']
const hash = async path => createHash('sha256').update(await readFile(path)).digest('hex')
const before = new Map(await Promise.all(cases.map(async item => [item.file, await hash(join(fixtureRoot, item.file))])))

const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const result = spawnSync(cargo, [
  'test', '--manifest-path', join(root, 'src-tauri', 'Cargo.toml'), '--lib',
  'services::pdf_output_validation::tests::validates_real_pdf_structure_and_rejects_corruption_races_and_larger_output',
  '--', '--ignored', '--exact', '--nocapture',
], {
  cwd: root,
  encoding: 'utf8',
  windowsHide: true,
  maxBuffer: 64 * 1024 * 1024,
  env: { ...process.env, LONG_MEDIA_FIXTURE_PYTHON: process.env.LONG_MEDIA_FIXTURE_PYTHON || 'python' },
})
if (result.status !== 0) throw new Error(`D-03.2 real validation test failed: ${result.stderr || result.stdout || result.error?.message}`)
const marker = 'D03_PDF_VALIDATION_RESULT='
const line = result.stdout.split(/\r?\n/u).find(value => value.startsWith(marker))
if (!line) throw new Error('D-03.2 real validation test did not emit structured evidence')
const actual = JSON.parse(line.slice(marker.length))
const reports = new Map(actual.reports.map(report => [`${report.file}:${report.mode}`, report]))
const comparisons = []

for (const fixture of cases) {
  for (const mode of modes) {
    const report = reports.get(`${fixture.file}:${mode}`)
    const inputBytes = (await stat(join(fixtureRoot, fixture.file))).size
    const expected = {
      inputBytes,
      pages: fixture.pages,
      formFields: fixture.formFields,
      annotations: fixture.annotations,
      outlines: fixture.outlines,
      attachments: fixture.attachments,
      outputBytesPositive: true,
      outputNoLarger: true,
      outputSha256Bound: true,
      independentFactsEqual: true,
      finalOutputExists: false,
      stagingExistsBeforeDrop: true,
      sourceHashChanged: false,
    }
    const observed = {
      inputBytes: report?.inputBytes ?? null,
      pages: report?.pages ?? null,
      formFields: report?.formFields ?? null,
      annotations: report?.annotations ?? null,
      outlines: report?.outlines ?? null,
      attachments: report?.attachments ?? null,
      outputBytesPositive: (report?.outputBytes ?? 0) > 0,
      outputNoLarger: (report?.outputBytes ?? Number.MAX_SAFE_INTEGER) <= inputBytes,
      outputSha256Bound: /^[0-9a-f]{64}$/u.test(report?.outputSha256 ?? ''),
      independentFactsEqual: report?.independentFactsEqual ?? null,
      finalOutputExists: report?.finalOutputExists ?? null,
      stagingExistsBeforeDrop: report?.stagingExistsBeforeDrop ?? null,
      sourceHashChanged: before.get(fixture.file) !== await hash(join(fixtureRoot, fixture.file)),
    }
    const differences = Object.keys(expected).filter(key => JSON.stringify(expected[key]) !== JSON.stringify(observed[key]))
    comparisons.push({ case: `${fixture.file}:${mode}`, expected, actual: observed, observedOutputBytes: report?.outputBytes ?? null, differences })
  }
}

for (const [caseName, expected, observed] of [
  ['corrupt-candidate', 'PDF_OUTPUT_QPDF_CHECK_FAILED', actual.corruptError],
  ['target-race', 'PDF_OUTPUT_TARGET_APPEARED', actual.targetRaceError],
  ['larger-candidate', 'PDF_OUTPUT_LARGER_THAN_SOURCE', actual.largerError],
  ['validation-cancellation', 'PDF_OUTPUT_CANCELLED', actual.cancelledError],
]) {
  comparisons.push({ case: caseName, expected, actual: observed, differences: expected === observed ? [] : ['error'] })
}
comparisons.push({
  case: 'damaged-input',
  expected: 'PDF_ANALYSIS_*',
  actual: actual.damagedInputError,
  differences: typeof actual.damagedInputError === 'string' && actual.damagedInputError.startsWith('PDF_ANALYSIS_') ? [] : ['error'],
})
comparisons.push({ case: 'target-race-bytes', expected: true, actual: actual.targetRaceBytesPreserved, differences: actual.targetRaceBytesPreserved === true ? [] : ['bytes'] })
comparisons.push({ case: 'no-final-outputs', expected: true, actual: actual.finalOutputsAbsent, differences: actual.finalOutputsAbsent === true ? [] : ['output'] })

const differenceCount = comparisons.reduce((count, comparison) => count + comparison.differences.length, 0)
const evidence = {
  schemaVersion: 1,
  node: 'D-03.2',
  runtime: 'src-tauri/resources/pdf-engine/qpdf.exe',
  independentInspector: 'scripts/inspect-d01-pdf.py (pypdf)',
  testKind: 'real-product-qpdf-candidate-validation-plus-independent-preservation-inspection',
  expectedVsActual: comparisons,
  differenceCount,
  passed: differenceCount === 0,
}
await mkdir(outputRoot, { recursive: true })
await writeFile(join(outputRoot, 'result.json'), JSON.stringify(evidence, null, 2), 'utf8')
if (differenceCount !== 0) throw new Error(`D-03.2 expected/actual differences remain: ${differenceCount}`)
console.log(`D-03.2 real PDF validation passed (${comparisons.length} comparisons; expected/actual differences: ${differenceCount}).`)

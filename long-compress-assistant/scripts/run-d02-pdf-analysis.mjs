import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises'
import { basename, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const manifest = JSON.parse(await readFile(join(root, 'tests', 'fixtures', 'media', 'manifest.json'), 'utf8'))
const outputRoot = join(root, 'test-results', 'd02-pdf-analysis')
const fixtureRoot = join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'pdfs')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}

const sourceHashesBefore = new Map()
for (const fixture of manifest.pdfs) {
  sourceHashesBefore.set(fixture.file, await sha256(join(fixtureRoot, fixture.file)))
}

const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const result = spawnSync(cargo, [
  'test',
  '--manifest-path',
  join(root, 'src-tauri', 'Cargo.toml'),
  '--lib',
  'services::pdf_analysis::tests::probes_real_pdf_fixture_matrix_with_product_runtime',
  '--',
  '--ignored',
  '--exact',
  '--nocapture',
], {
  cwd: root,
  encoding: 'utf8',
  windowsHide: true,
  maxBuffer: 64 * 1024 * 1024,
})
assert(result.status === 0, `real Rust PDF analysis failed (${result.status}): ${result.stderr || result.stdout || result.error?.message || 'no output'}`)

const marker = 'D02_PDF_ANALYSIS_RESULT='
const line = result.stdout.split(/\r?\n/).find(item => item.startsWith(marker))
assert(line, 'real Rust PDF analysis did not emit structured evidence')
const actual = JSON.parse(line.slice(marker.length))
const reports = new Map(actual.reports.map(report => [basename(report.source), report]))
const comparisons = []

for (const fixture of manifest.pdfs) {
  const report = reports.get(fixture.file)
  assert(report, `${fixture.file}: product analysis report is missing`)
  const expected = {
    inputBytes: (await stat(join(fixtureRoot, fixture.file))).size,
    pageCount: fixture.pages,
    encrypted: fixture.kind === 'encrypted-refusal',
    passwordState: fixture.kind === 'encrypted-refusal' ? 'accepted' : 'not-required',
    hasDigitalSignature: fixture.kind === 'digitally-signed',
    signatureFieldNames: fixture.signatureField ? [fixture.signatureField] : [],
    hasFormFields: fixture.kind === 'acroform',
    formFieldNames: [...(fixture.fields ?? [])].sort(),
    hasAttachments: fixture.kind === 'attachment',
    attachmentNames: [...(fixture.attachments ?? [])].sort(),
    sourceMutated: false,
  }
  const observed = {
    ...Object.fromEntries(Object.keys(expected).filter(key => key !== 'sourceMutated').map(key => [key, report[key]])),
    sourceMutated: sourceHashesBefore.get(fixture.file) !== await sha256(join(fixtureRoot, fixture.file)),
  }
  const differences = Object.keys(expected).filter(key => JSON.stringify(expected[key]) !== JSON.stringify(observed[key]))
  comparisons.push({ file: fixture.file, kind: fixture.kind, expected, actual: observed, differences })
}

const lockedExpected = {
  analysisComplete: false,
  pageCount: null,
  encrypted: true,
  passwordState: 'required',
  hasDigitalSignature: null,
  hasFormFields: null,
  hasAttachments: null,
  blockingReasons: ['PDF_ANALYSIS_PASSWORD_REQUIRED'],
}
const lockedActual = Object.fromEntries(Object.keys(lockedExpected).map(key => [key, actual.lockedEncryptedReport[key]]))
const lockedDifferences = Object.keys(lockedExpected).filter(key => JSON.stringify(lockedExpected[key]) !== JSON.stringify(lockedActual[key]))
comparisons.push({ file: 'encrypted.pdf', kind: 'encrypted-without-password', expected: lockedExpected, actual: lockedActual, differences: lockedDifferences })

const securityExpected = {
  wrongPasswordError: 'PDF_ANALYSIS_INVALID_PASSWORD',
  wrongPasswordLeaked: false,
}
const securityActual = actual.securityControls
const securityDifferences = Object.keys(securityExpected).filter(key => JSON.stringify(securityExpected[key]) !== JSON.stringify(securityActual[key]))
comparisons.push({ file: 'encrypted.pdf', kind: 'wrong-password-security', expected: securityExpected, actual: securityActual, differences: securityDifferences })

const differenceCount = comparisons.reduce((count, comparison) => count + comparison.differences.length, 0)
const evidence = {
  schemaVersion: 1,
  node: 'D-02.1',
  fixtureRevision: manifest.fixtureRevision,
  runtime: 'src-tauri/resources/pdf-engine/qpdf.exe',
  testKind: 'real-product-rust-analysis-with-real-qpdf-and-synthetic-real-pdfs',
  expectedVsActual: comparisons,
  differenceCount,
  passed: differenceCount === 0,
}
await mkdir(outputRoot, { recursive: true })
await writeFile(join(outputRoot, 'result.json'), JSON.stringify(evidence, null, 2), 'utf8')
assert(differenceCount === 0, `D-02.1 expected/actual differences remain: ${differenceCount}`)
console.log(`D-02.1 real product PDF analysis passed (${comparisons.length} comparisons; expected/actual differences: ${differenceCount}).`)

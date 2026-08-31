import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const fixtureRoot = join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'pdfs')
const outputRoot = join(root, 'test-results', 'd03-pdf-staging')
const sourceNames = ['text-vector.pdf', 'mixed-content.pdf', 'signed.pdf', 'encrypted.pdf']
const hash = async path => createHash('sha256').update(await readFile(path)).digest('hex')
const before = new Map(await Promise.all(sourceNames.map(async name => [name, await hash(join(fixtureRoot, name))])))

const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const result = spawnSync(cargo, [
  'test', '--manifest-path', join(root, 'src-tauri', 'Cargo.toml'), '--lib',
  'services::pdf_transform::tests::transforms_real_pdf_modes_to_owned_staging_and_blocks_risks',
  '--', '--ignored', '--exact', '--nocapture',
], { cwd: root, encoding: 'utf8', windowsHide: true, maxBuffer: 32 * 1024 * 1024 })
if (result.status !== 0) throw new Error(`D-03.1 real staging test failed: ${result.stderr || result.stdout || result.error?.message}`)
const marker = 'D03_PDF_STAGING_RESULT='
const line = result.stdout.split(/\r?\n/u).find(value => value.startsWith(marker))
if (!line) throw new Error('D-03.1 real staging test did not emit structured evidence')
const actual = JSON.parse(line.slice(marker.length))
const reports = new Map(actual.reports.map(report => [report.file, report]))
const comparisons = []

for (const expected of [
  { file: 'text-vector.pdf', mode: 'lossless-organization', inputPages: 1 },
  { file: 'mixed-content.pdf', mode: 'compatible-image-optimization', inputPages: 1 },
]) {
  const report = reports.get(expected.file)
  const expectedFacts = {
    mode: expected.mode,
    inputBytes: (await stat(join(fixtureRoot, expected.file))).size,
    inputPages: expected.inputPages,
    stagedPages: expected.inputPages,
    stagedBytesPositive: true,
    sourceChanged: false,
    finalOutputExists: false,
    stagingExistsBeforeDrop: true,
    sourceHashChangedAfterTest: false,
  }
  const actualFacts = {
    mode: report?.mode ?? null,
    inputBytes: report?.inputBytes ?? null,
    inputPages: report?.inputPages ?? null,
    stagedPages: report?.stagedPages ?? null,
    stagedBytesPositive: (report?.stagedBytes ?? 0) > 0,
    sourceChanged: report?.sourceChanged ?? null,
    finalOutputExists: report?.finalOutputExists ?? null,
    stagingExistsBeforeDrop: report?.stagingExistsBeforeDrop ?? null,
    sourceHashChangedAfterTest: before.get(expected.file) !== await hash(join(fixtureRoot, expected.file)),
  }
  const differences = Object.keys(expectedFacts).filter(key => JSON.stringify(expectedFacts[key]) !== JSON.stringify(actualFacts[key]))
  comparisons.push({ case: expected.file, expected: expectedFacts, actual: actualFacts, observedStagedBytes: report?.stagedBytes ?? null, differences })
}

for (const [caseName, expected, observed] of [
  ['signed-refusal', 'PDF_TRANSFORM_SIGNED_DOCUMENT_BLOCKED', actual.signedError],
  ['encrypted-refusal', 'PDF_TRANSFORM_ENCRYPTED_DOCUMENT_BLOCKED:', actual.encryptedError?.slice(0, 'PDF_TRANSFORM_ENCRYPTED_DOCUMENT_BLOCKED:'.length)],
  ['cancelled-before-launch', 'PDF_TRANSFORM_CANCELLED', actual.cancelledError],
]) {
  comparisons.push({ case: caseName, expected, actual: observed, differences: expected === observed ? [] : ['error'] })
}

for (const name of sourceNames) {
  if (before.get(name) !== await hash(join(fixtureRoot, name))) {
    comparisons.push({ case: `${name}-source-integrity`, expected: false, actual: true, differences: ['sha256'] })
  }
}

const differenceCount = comparisons.reduce((count, comparison) => count + comparison.differences.length, 0)
const evidence = {
  schemaVersion: 1,
  node: 'D-03.1',
  runtime: 'src-tauri/resources/pdf-engine/qpdf.exe',
  testKind: 'real-product-rust-owned-staging-with-real-qpdf-and-real-pdfs',
  expectedVsActual: comparisons,
  differenceCount,
  passed: differenceCount === 0,
}
await mkdir(outputRoot, { recursive: true })
await writeFile(join(outputRoot, 'result.json'), JSON.stringify(evidence, null, 2), 'utf8')
if (differenceCount !== 0) throw new Error(`D-03.1 expected/actual differences remain: ${differenceCount}`)
console.log(`D-03.1 real PDF staging passed (${comparisons.length} comparisons; expected/actual differences: ${differenceCount}).`)

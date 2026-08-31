import { spawnSync } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { join } from 'node:path'

const root = fileURLToPath(new URL('../', import.meta.url))
const outputRoot = join(root, 'test-results', 'd04-pdf-command')
const files = [
  'text-vector.pdf', 'scanned-image.pdf', 'mixed-content.pdf', 'transparency.pdf',
  'chinese-font.pdf', 'large-pages.pdf', 'large-image.pdf', 'form.pdf',
  'annotation.pdf', 'outline.pdf', 'attachment.pdf',
]
const modes = ['lossless-organization', 'compatible-image-optimization']
const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const result = spawnSync(cargo, [
  'test', '--manifest-path', join(root, 'src-tauri', 'Cargo.toml'), '--lib',
  'commands::pdf_engine::tests::real_product_command_revalidates_and_publishes_with_truthful_stages',
  '--', '--ignored', '--exact', '--nocapture',
], {
  cwd: root,
  encoding: 'utf8',
  env: { ...process.env, RUST_TEST_THREADS: '1' },
})
if (result.status !== 0) {
  throw new Error(`D-04.3 real PDF product command failed: ${result.stderr || result.stdout || result.error?.message}`)
}
const marker = 'D04_PDF_COMMAND_RESULT='
const processOutput = `${result.stdout || ''}\n${result.stderr || ''}`
const line = processOutput.split(/\r?\n/u).find(value => value.includes(marker))
if (!line) throw new Error('D-04.3 real PDF product command did not emit structured evidence')
const actual = JSON.parse(line.slice(line.indexOf(marker) + marker.length))
const reports = new Map(actual.reports.map(report => [`${report.file}:${report.mode}`, report]))
const comparisons = []
for (const file of files) {
  for (const mode of modes) {
    const report = reports.get(`${file}:${mode}`)
    const expected = {
      stages: ['Transforming', 'Validating', 'Publishing'],
      finalOutputExists: true,
      outputBytesPositive: true,
      structuralFactsEqual: true,
      sourceBytesUnchanged: true,
      markOfTheWeb: 'not-present',
    }
    const observed = {
      stages: report?.stages ?? null,
      finalOutputExists: report?.finalOutputExists ?? false,
      outputBytesPositive: (report?.outputBytes ?? 0) > 0,
      structuralFactsEqual: report?.structuralFactsEqual ?? false,
      sourceBytesUnchanged: report?.sourceBytesUnchanged ?? false,
      markOfTheWeb: report?.markOfTheWeb ?? null,
    }
    comparisons.push({
      case: `${file}:${mode}`,
      expected,
      actual: observed,
      inputBytes: report?.inputBytes ?? null,
      outputBytes: report?.outputBytes ?? null,
      pageCount: report?.pageCount ?? null,
      differences: Object.keys(expected).filter(key => JSON.stringify(expected[key]) !== JSON.stringify(observed[key])),
    })
  }
}
for (const [file, code] of [
  ['signed.pdf', 'PDF_TRANSFORM_SIGNED_DOCUMENT_BLOCKED'],
  ['encrypted.pdf', 'PDF_TRANSFORM_ANALYSIS_INCOMPLETE'],
]) {
  const report = actual.blocked.find(item => item.file === file)
  const observed = {
    code: report?.actualError?.startsWith(code) ? code : report?.actualError ?? null,
    outputAbsent: report?.outputAbsent ?? false,
  }
  const expected = { code, outputAbsent: true }
  comparisons.push({
    case: `${file}:stable-refusal`, expected, actual: observed,
    differences: Object.keys(expected).filter(key => expected[key] !== observed[key]),
  })
}

const differenceCount = comparisons.reduce((count, comparison) => count + comparison.differences.length, 0)
const evidence = {
  schemaVersion: 2,
  node: 'D-04.3',
  runtime: 'src-tauri/resources/pdf-engine/qpdf.exe',
  testKind: 'real-product-command-full-content-structure-and-boundary-matrix',
  expectedVsActual: comparisons,
  differenceCount,
  passed: differenceCount === 0,
  source: { command: 'npm run test:pdf-d04-command:real' },
}
await mkdir(outputRoot, { recursive: true })
await writeFile(join(outputRoot, 'result.json'), `${JSON.stringify(evidence, null, 2)}\n`, 'utf8')
if (differenceCount) throw new Error(`D-04.3 real PDF product command differences remain: ${differenceCount}`)
console.log(`D-04.3 real PDF product matrix passed (${comparisons.length} cases; expected/actual differences: 0).`)

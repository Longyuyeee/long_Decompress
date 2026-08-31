import { spawnSync } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const outputRoot = join(root, 'test-results', 'd04-pdf-command')
const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const result = spawnSync(cargo, [
  'test', '--manifest-path', join(root, 'src-tauri', 'Cargo.toml'), '--lib',
  'commands::pdf_engine::tests::real_product_command_revalidates_and_publishes_with_truthful_stages',
  '--', '--ignored', '--exact', '--nocapture',
], {
  cwd: root,
  encoding: 'utf8',
  windowsHide: true,
  maxBuffer: 64 * 1024 * 1024,
})
if (result.status !== 0) {
  throw new Error(`D-04.1 real PDF product command failed: ${result.stderr || result.stdout || result.error?.message}`)
}
const marker = 'D04_PDF_COMMAND_RESULT='
const line = result.stdout.split(/\r?\n/u).find(value => value.startsWith(marker))
if (!line) throw new Error('D-04.1 real PDF product command did not emit structured evidence')
const actual = JSON.parse(line.slice(marker.length))
const expected = {
  stages: ['Transforming', 'Validating', 'Publishing'],
  finalOutputExists: true,
  outputBytesPositive: true,
  pageCount: 1,
  sourceBytesUnchanged: true,
  markOfTheWeb: 'not-present',
}
const observed = {
  stages: actual.stages,
  finalOutputExists: actual.finalOutputExists,
  outputBytesPositive: actual.outputBytes > 0,
  pageCount: actual.pageCount,
  sourceBytesUnchanged: actual.sourceBytesUnchanged,
  markOfTheWeb: actual.markOfTheWeb,
}
const differences = Object.keys(expected).filter(key => JSON.stringify(expected[key]) !== JSON.stringify(observed[key]))
const evidence = {
  schemaVersion: 1,
  node: 'D-04.1',
  runtime: 'src-tauri/resources/pdf-engine/qpdf.exe',
  testKind: 'real-product-command-revalidation-stage-and-publication',
  expected,
  actual: observed,
  inputBytes: actual.inputBytes,
  outputBytes: actual.outputBytes,
  differenceCount: differences.length,
  differences,
  source: { command: 'npm run test:pdf-d04-command:real' },
}
await mkdir(outputRoot, { recursive: true })
await writeFile(join(outputRoot, 'result.json'), `${JSON.stringify(evidence, null, 2)}\n`, 'utf8')
if (differences.length) throw new Error(`D-04.1 real PDF product command differences: ${differences.join(', ')}`)
console.log('D-04.1 real PDF product command passed (6 comparisons; expected/actual differences: 0).')

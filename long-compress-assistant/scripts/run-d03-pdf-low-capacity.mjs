import { spawnSync } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const volumeArgument = process.argv.find(value => value.startsWith('--volume='))
const volume = volumeArgument?.slice('--volume='.length)
if (!volume) throw new Error('D-03.3.1 requires --volume=<isolated-mounted-volume>')

const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const result = spawnSync(cargo, [
  'test', '--release', '--manifest-path', join(root, 'src-tauri', 'Cargo.toml'), '--lib',
  'services::pdf_publish::tests::real_low_capacity_volume_blocks_pdf_transaction_without_artifacts',
  '--', '--ignored', '--exact', '--nocapture',
], {
  cwd: root,
  encoding: 'utf8',
  windowsHide: true,
  maxBuffer: 16 * 1024 * 1024,
  env: { ...process.env, LONG_D03_LOW_CAPACITY_PATH: volume },
})
if (result.status !== 0) throw new Error(`D-03.3.1 real low-capacity test failed: ${result.stderr || result.stdout || result.error?.message}`)
const marker = 'D03_PDF_LOW_CAPACITY_RESULT='
const line = result.stdout.split(/\r?\n/u).find(value => value.startsWith(marker))
if (!line) throw new Error('D-03.3.1 test did not emit structured evidence')
const actual = JSON.parse(line.slice(marker.length))
const expected = {
  fileSystem: 'NTFS',
  volumeSmallerThanReserve: true,
  realWriteProbeBytes: 1024 * 1024,
  error: 'PDF_TRANSFORM_RESOURCE_PREFLIGHT_BLOCKED',
  finalOutputExists: false,
  stagingFileCount: 0,
  sourceHashUnchanged: true,
}
const observed = {
  fileSystem: actual.fileSystem,
  volumeSmallerThanReserve: actual.totalBytes < actual.reserveBytes && actual.availableBytes < actual.reserveBytes,
  realWriteProbeBytes: actual.realWriteProbeBytes,
  error: actual.error,
  finalOutputExists: actual.finalOutputExists,
  stagingFileCount: actual.stagingFiles?.length,
  sourceHashUnchanged: actual.sourceHashUnchanged,
}
const differences = Object.keys(expected).filter(key => JSON.stringify(expected[key]) !== JSON.stringify(observed[key]))
const evidence = {
  schemaVersion: 1,
  node: 'D-03.3.1',
  testKind: 'real-isolated-dynamic-vhd-ntfs-low-capacity-product-transaction',
  expectedVsActual: [{ case: 'controlled-low-capacity-volume', expected, actual: observed, differences }],
  volumeFacts: {
    mountPoint: actual.mountPoint,
    totalBytes: actual.totalBytes,
    availableBytes: actual.availableBytes,
    reserveBytes: actual.reserveBytes,
  },
  differenceCount: differences.length,
  passed: differences.length === 0,
}
const outputRoot = join(root, 'test-results', 'd03-pdf-low-capacity')
await mkdir(outputRoot, { recursive: true })
await writeFile(join(outputRoot, 'result.json'), JSON.stringify(evidence, null, 2), 'utf8')
if (differences.length !== 0) throw new Error(`D-03.3.1 expected/actual differences remain: ${differences.join(', ')}`)
console.log('D-03.3.1 real isolated low-capacity volume passed (1 comparison; expected/actual differences: 0).')

import { createHash } from 'node:crypto'
import { cp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { basename, dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const executable = resolve(process.env.TAURI_APP_BINARY || '')
const evidenceRoot = resolve(process.env.PDF_RUNTIME_EVIDENCE_DIRECTORY || join(root, 'test-results', 'installed-pdf-runtime'))
const installRoot = dirname(executable)
const runtimeRoot = join(installRoot, 'resources', 'pdf-engine')
const sandboxRoot = join(evidenceRoot, 'isolated-install-copy')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function run(command, args, label, expectedStatus = 0) {
  const result = spawnSync(command, args, { encoding: 'utf8', windowsHide: true, maxBuffer: 32 * 1024 * 1024 })
  if (result.error) throw new Error(`${label} failed to launch: ${result.error.message}`)
  if (result.status !== expectedStatus) throw new Error(`${label} exited ${result.status}; expected ${expectedStatus}: ${result.stderr || result.stdout}`)
}

async function invokePreflight(app, report, expectedStatus) {
  await rm(report, { force: true })
  run(app, ['--internal-pdf-engine-preflight-report', report], 'installed PDF production preflight', expectedStatus)
  return JSON.parse(await readFile(report, 'utf8'))
}

async function identity(path) {
  const bytes = await readFile(path)
  return { bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }
}

assert(process.env.TAURI_APP_BINARY, 'TAURI_APP_BINARY must point to the formally installed application')
await stat(executable)
await stat(join(runtimeRoot, 'qpdf.exe'))
await rm(evidenceRoot, { recursive: true, force: true })
await mkdir(evidenceRoot, { recursive: true })

const installed = await invokePreflight(executable, join(evidenceRoot, 'installed-preflight.json'), 0)
assert(installed.passed === true, 'installed PDF production preflight did not pass')
assert(installed.status?.version === '12.4.0', 'installed qpdf version differs')
assert(installed.status?.license === 'Apache-2.0', 'installed qpdf license identity differs')
assert(installed.status?.supportsJsonV2 === true && installed.status?.supportsImageOptimization === true, 'installed qpdf capabilities differ')
assert(installed.status?.cryptoProviders?.includes('openssl') && installed.status?.cryptoProviders?.includes('native'), 'installed qpdf crypto providers differ')
assert(installed.status?.files?.length === 10, 'installed preflight did not verify all ten resources')

await mkdir(sandboxRoot, { recursive: true })
const sandboxExecutable = join(sandboxRoot, basename(executable))
await cp(executable, sandboxExecutable)
await cp(runtimeRoot, join(sandboxRoot, 'resources', 'pdf-engine'), { recursive: true })
const sandboxQpdf = join(sandboxRoot, 'resources', 'pdf-engine', 'qpdf.exe')
const original = await readFile(sandboxQpdf)
await rm(sandboxQpdf)
const missing = await invokePreflight(sandboxExecutable, join(evidenceRoot, 'missing-resource-preflight.json'), 2)
assert(missing.passed === false && missing.error?.includes('PDF_ENGINE_RESOURCE_MISSING'), 'missing installed qpdf was accepted or misclassified')

const replaced = Buffer.from(original)
replaced[0] ^= 0xff
await writeFile(sandboxQpdf, replaced)
const replacement = await invokePreflight(sandboxExecutable, join(evidenceRoot, 'replaced-resource-preflight.json'), 2)
assert(replacement.passed === false && replacement.error?.includes('PDF_ENGINE_RESOURCE_HASH_MISMATCH'), 'replaced installed qpdf was accepted or misclassified')

await writeFile(join(evidenceRoot, 'result.json'), `${JSON.stringify({
  schemaVersion: 1,
  measuredAt: new Date().toISOString(),
  actual: { executable: { path: executable, ...await identity(executable) }, runtimeRoot, productionPreflight: installed, missingResource: missing, replacedResource: replacement },
  passed: true,
}, null, 2)}\n`)
console.log('Installed PDF runtime passed (production preflight and isolated missing/replaced refusal).')

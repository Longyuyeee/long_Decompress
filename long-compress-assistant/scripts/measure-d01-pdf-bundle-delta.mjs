import { createHash } from 'node:crypto'
import { mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const option = name => process.argv.find(argument => argument.startsWith(`--${name}=`))?.slice(name.length + 3)
const requiredPath = name => {
  const value = option(name)
  if (!value) throw new Error(`--${name}=<path> is required`)
  return resolve(root, value)
}
const paths = {
  baselineNsis: requiredPath('baseline-nsis'),
  baselineUpdater: requiredPath('baseline-updater'),
  baselineSignature: requiredPath('baseline-signature'),
  integratedNsis: requiredPath('integrated-nsis'),
  integratedUpdater: requiredPath('integrated-updater'),
  integratedSignature: requiredPath('integrated-signature'),
}
const output = resolve(root, option('output') || 'test-results/d01-pdf-delta/result.json')
const extractionRoot = resolve(root, 'test-results/d01-pdf-delta/extracted')
const sevenZip = resolve(root, 'src-tauri/resources/archive-engine/7z.exe')

function run(command, args, label) {
  const result = spawnSync(command, args, { encoding: 'utf8', windowsHide: true, maxBuffer: 32 * 1024 * 1024 })
  if (result.error || result.status !== 0) throw new Error(`${label} failed (${result.status}): ${result.error?.message || result.stderr || result.stdout}`)
  return `${result.stdout ?? ''}${result.stderr ?? ''}`
}

async function identity(path) {
  const bytes = await readFile(path)
  return { path, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }
}

async function findExecutables(directory) {
  const found = []
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) found.push(...await findExecutables(path))
    else if (entry.isFile() && entry.name.toLowerCase().endsWith('.exe')) found.push(path)
  }
  return found
}

async function inspectBundle(label, nsis, updater, signature) {
  const target = join(extractionRoot, label)
  await rm(target, { recursive: true, force: true })
  await mkdir(target, { recursive: true })
  const archiveTest = run(sevenZip, ['t', updater], `${label} updater archive test`)
  run(sevenZip, ['x', '-y', `-o${target}`, updater], `${label} updater extraction`)
  const executables = await findExecutables(target)
  if (executables.length !== 1) throw new Error(`${label} updater must contain exactly one NSIS executable; found ${executables.length}`)
  const nsisIdentity = await identity(nsis)
  const extractedIdentity = await identity(executables[0])
  if (nsisIdentity.bytes !== extractedIdentity.bytes || nsisIdentity.sha256 !== extractedIdentity.sha256) {
    throw new Error(`${label} updater does not contain the byte-identical NSIS executable`)
  }
  const signatureText = (await readFile(signature, 'utf8')).trim()
  const decoded = Buffer.from(signatureText, 'base64').toString('utf8')
  if (signatureText.length < 300 || !decoded.includes('untrusted comment: signature from tauri secret key')) {
    throw new Error(`${label} updater signature is not a Tauri minisign payload`)
  }
  return {
    nsis: nsisIdentity,
    updater: await identity(updater),
    signature: { ...await identity(signature), characters: signatureText.length },
    updaterArchiveIntegrity: archiveTest.includes('Everything is Ok'),
    updaterContainsByteIdenticalNsis: true,
  }
}

for (const path of Object.values(paths)) await stat(path)
await rm(extractionRoot, { recursive: true, force: true })
const baseline = await inspectBundle('baseline', paths.baselineNsis, paths.baselineUpdater, paths.baselineSignature)
const integrated = await inspectBundle('integrated', paths.integratedNsis, paths.integratedUpdater, paths.integratedSignature)
const nsisBytes = integrated.nsis.bytes - baseline.nsis.bytes
const updaterBytes = integrated.updater.bytes - baseline.updater.bytes
if (nsisBytes <= 0 || updaterBytes <= 0) throw new Error(`PDF runtime bundle deltas must be positive: nsis=${nsisBytes}, updater=${updaterBytes}`)

const report = {
  schemaVersion: 1,
  measuredAt: new Date().toISOString(),
  commit: process.env.GITHUB_SHA || null,
  scope: 'same-commit-same-toolchain-tauri-updater-signed-pdf-resource-delta',
  signingBoundary: 'Tauri updater minisign measurement key; NSIS is not claimed to have Authenticode signing',
  baseline,
  integrated,
  delta: {
    nsisBytes,
    updaterBytes,
    nsisPercent: Number(((nsisBytes / baseline.nsis.bytes) * 100).toFixed(4)),
    updaterPercent: Number(((updaterBytes / baseline.updater.bytes) * 100).toFixed(4)),
  },
  passed: true,
}
await mkdir(dirname(output), { recursive: true })
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`)
console.log(`D-01 signed updater delta passed: NSIS +${nsisBytes} B, updater +${updaterBytes} B.`)

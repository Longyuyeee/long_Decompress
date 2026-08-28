import { createHash } from 'node:crypto'
import { spawn, spawnSync } from 'node:child_process'
import { mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { performance } from 'node:perf_hooks'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const fixtureManifest = JSON.parse(await readFile(join(root, 'tests', 'fixtures', 'media', 'image-baseline.json'), 'utf8'))
const fixtureRoot = join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'images')
const auditRoot = join(root, 'test-results', 'image-baseline')
const outputRoot = join(auditRoot, 'outputs')
const concurrencyRoot = join(auditRoot, 'concurrency')
const toolRoot = join(root, 'tools', 'image-baseline')
const binary = join(toolRoot, 'target', 'release', 'long-image-baseline.exe')
const minimalBinary = join(toolRoot, 'target', 'release', 'long-image-minimal.exe')
const sevenZip = join(root, 'src-tauri', 'resources', 'archive-engine', '7z.exe')
const python = process.env.LONG_MEDIA_FIXTURE_PYTHON || 'python'
const pythonPackages = join(root, 'test-results', 'media-fixture-audit', 'python-packages')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function run(command, args, label, options = {}) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', windowsHide: true, ...options })
  assert(result.status === 0, `${label} failed (${result.status}): ${result.error?.message || result.stderr || result.stdout}`)
  return result.stdout ?? ''
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}

async function verifyInputs() {
  const actual = []
  for (const expected of fixtureManifest.inputs) {
    const path = join(fixtureRoot, expected.file)
    const facts = await stat(path)
    const digest = await sha256(path)
    assert(facts.size === expected.bytes, `${expected.file}: expected ${expected.bytes} bytes, got ${facts.size}`)
    assert(digest === expected.sha256, `${expected.file}: frozen SHA-256 mismatch`)
    actual.push({ file: expected.file, bytes: facts.size, sha256: digest })
  }
  return actual
}

function verifySupplyChain() {
  const tree = run('cargo', ['tree', '-e', 'features'], 'image candidate feature tree', { cwd: toolRoot })
  for (const required of ['libcaesium feature "jpg"', 'libcaesium feature "webp"', 'oxipng feature "parallel"', 'oxipng feature "zopfli"']) {
    assert(tree.includes(required), `required feature missing: ${required}`)
  }
  for (const forbidden of ['libcaesium feature "default"', 'libcaesium feature "gif"', 'libcaesium feature "png"', 'libcaesium feature "tiff"', 'gifski ', 'imagequant ']) {
    assert(!tree.includes(forbidden), `forbidden dependency/feature enabled: ${forbidden}`)
  }
  const metadata = JSON.parse(run('cargo', ['metadata', '--format-version', '1'], 'image candidate metadata', { cwd: toolRoot }))
  const registryPackages = metadata.packages.filter(item => item.source?.startsWith('registry+'))
  const forbiddenLicenses = registryPackages.filter(item => (
    /(?:^|\s(?:OR|AND|WITH)\s)(?:AGPL|GPL)-/i.test(item.license || '')
  ))
  assert(forbiddenLicenses.length === 0, `forbidden registry licenses: ${forbiddenLicenses.map(item => `${item.name}:${item.license}`).join(', ')}`)
  const locked = Object.fromEntries(registryPackages.filter(item => ['libcaesium', 'oxipng'].includes(item.name)).map(item => [item.name, item.version]))
  assert(locked.libcaesium === '0.21.0' && locked.oxipng === '10.2.0', `candidate versions drifted: ${JSON.stringify(locked)}`)
  return { locked, registryPackageCount: registryPackages.length, forbiddenLicenses: [] }
}

function parseProbe(output) {
  const cases = []
  let peakWorkingSetBytes = 0
  for (const line of output.split(/\r?\n/)) {
    const fields = line.split('|')
    if (fields[0] === 'RESULT') {
      cases.push({
        kind: fields[1], status: fields[2], inputBytes: Number(fields[3]),
        outputBytes: Number(fields[4]), elapsedMicros: Number(fields[5]), detail: fields[6] || '',
      })
    } else if (fields[0] === 'PROCESS' && fields[1] === 'peakWorkingSetBytes') {
      peakWorkingSetBytes = Number(fields[2])
    }
  }
  assert(cases.length === 4, `expected four probe cases, got ${cases.length}`)
  assert(cases.filter(item => item.status === 'ok').length === 3, 'JPEG/WebP/PNG candidates must succeed')
  assert(cases.find(item => item.kind === 'gif-boundary')?.status === 'rejected', 'GIF must be explicitly rejected')
  assert(peakWorkingSetBytes > 0, 'Windows peak working-set measurement is missing')
  return { cases, peakWorkingSetBytes }
}

async function runConcurrentProbe(index) {
  const output = join(concurrencyRoot, `worker-${index}`)
  await mkdir(output, { recursive: true })
  return new Promise((resolve, reject) => {
    const child = spawn(binary, [fixtureRoot, output], { windowsHide: true })
    let stdout = ''
    let stderr = ''
    child.stdout.on('data', chunk => { stdout += chunk })
    child.stderr.on('data', chunk => { stderr += chunk })
    child.on('error', reject)
    child.on('close', code => code === 0 ? resolve(parseProbe(stdout)) : reject(new Error(`concurrent worker ${index} failed (${code}): ${stderr || stdout}`)))
  })
}

async function compressedSize(path, name) {
  const archive = join(auditRoot, `${name}.zip`)
  await rm(archive, { force: true })
  run(sevenZip, ['a', '-tzip', '-mx=9', archive, path], `${name} payload compression`)
  return (await stat(archive)).size
}

await rm(auditRoot, { recursive: true, force: true })
await mkdir(outputRoot, { recursive: true })
const inputs = await verifyInputs()
const supplyChain = verifySupplyChain()
run('cargo', ['build', '--release', '--bins', '--locked'], 'image candidate release build', { cwd: toolRoot })

const started = performance.now()
const probeOutput = run(binary, [fixtureRoot, outputRoot], 'image candidate real processing')
const wallMilliseconds = performance.now() - started
const probe = parseProbe(probeOutput)
const verification = JSON.parse(run(
  python,
  [join(root, 'scripts', 'verify-image-baseline.py'), fixtureRoot, outputRoot],
  'decoded output verification',
  { env: { ...process.env, PYTHONPATH: pythonPackages } },
))
assert(verification.differences.length === 0, `decoded output differences: ${verification.differences.join('; ')}`)

await mkdir(concurrencyRoot, { recursive: true })
const concurrentStarted = performance.now()
const concurrent = await Promise.all([0, 1, 2, 3].map(runConcurrentProbe))
const concurrentWallMilliseconds = performance.now() - concurrentStarted

const executableBytes = (await stat(binary)).size
const minimalExecutableBytes = (await stat(minimalBinary)).size
const compressedExecutableBytes = await compressedSize(binary, 'candidate-runtime')
const compressedMinimalBytes = await compressedSize(minimalBinary, 'minimal-runtime')
const outputs = []
for (const name of ['exif-orientation.optimized.jpg', 'photo.optimized.webp', 'transparent.optimized.png']) {
  const path = join(outputRoot, name)
  outputs.push({ file: name, bytes: (await stat(path)).size, sha256: await sha256(path) })
}

const result = {
  schemaVersion: 1,
  fixtureRevision: fixtureManifest.fixtureRevision,
  expected: {
    frozenInputCount: 5,
    successfulFormats: ['jpeg', 'webp', 'png-lossless'],
    rejectedFormats: ['gif'],
    decodedDifferences: 0,
    forbiddenLicenses: 0,
  },
  actual: {
    inputs,
    supplyChain,
    probe,
    verification: verification.actual,
    outputs,
    wallMilliseconds,
    concurrentWorkers: concurrent.length,
    concurrentWallMilliseconds,
    candidatePayload: {
      executableBytes,
      minimalExecutableBytes,
      incrementalExecutableBytes: executableBytes - minimalExecutableBytes,
      compressedExecutableBytes,
      compressedMinimalBytes,
      incrementalCompressedBytes: compressedExecutableBytes - compressedMinimalBytes,
      measurementScope: 'isolated-candidate-payload-not-final-nsis-delta'
    }
  },
  differences: [],
}
await writeFile(join(auditRoot, 'result.json'), JSON.stringify(result, null, 2), 'utf8')
console.log(`Real image baseline passed (JPEG/WebP/PNG + GIF boundary; ${probe.peakWorkingSetBytes} B peak working set).`)

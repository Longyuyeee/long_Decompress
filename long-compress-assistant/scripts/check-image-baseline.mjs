import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

export async function checkImageBaseline() {
  const fixture = JSON.parse(await readFile(join(root, 'tests', 'fixtures', 'media', 'image-baseline.json'), 'utf8'))
  assert(fixture.schemaVersion === 1 && fixture.inputs?.length === 5, 'five frozen B-01 inputs are required')
  const names = new Set(fixture.inputs.map(item => item.file))
  for (const name of ['exif-orientation.jpg', 'photo.webp', 'transparent.png', 'animated.gif', 'ultra-large.png']) {
    assert(names.has(name), `missing frozen B-01 input: ${name}`)
  }
  for (const item of fixture.inputs) {
    assert(Number.isSafeInteger(item.bytes) && item.bytes > 0, `${item.file}: non-empty byte fact is required`)
    assert(/^[a-f0-9]{64}$/.test(item.sha256), `${item.file}: frozen SHA-256 is required`)
  }
  assert(fixture.inputs.find(item => item.file === 'animated.gif')?.expectedAction === 'reject', 'GIF must remain an explicit rejection boundary')
  assert(fixture.inputs.find(item => item.file === 'ultra-large.png')?.pixels === 96_000_000, 'large-pixel resource boundary drifted')

  const cargo = await readFile(join(root, 'tools', 'image-baseline', 'Cargo.toml'), 'utf8')
  assert(cargo.includes('libcaesium = { version = "=0.21.0", default-features = false, features = ["jpg", "webp"] }'), 'libcaesium feature lock drifted')
  assert(cargo.includes('oxipng = { version = "=10.2.0", default-features = false, features = ["parallel", "zopfli"] }'), 'oxipng feature lock drifted')
  const lock = (await readFile(join(root, 'tools', 'image-baseline', 'Cargo.lock'), 'utf8'))
    .replace(/\r\n?/g, '\n')
  assert(lock.includes('name = "libcaesium"\nversion = "0.21.0"'), 'libcaesium lock entry is missing')
  assert(lock.includes('name = "oxipng"\nversion = "10.2.0"'), 'oxipng lock entry is missing')
  assert(!lock.includes('name = "gifski"') && !lock.includes('name = "imagequant"'), 'forbidden GIF/PNG dependencies entered the lockfile')

  const dependencies = JSON.parse(await readFile(join(root, 'config', 'media-dependencies.json'), 'utf8'))
  const imageDependencies = dependencies.dependencies.filter(item => item.workload === 'image')
  assert(imageDependencies.length === 2 && imageDependencies.every(item => item.integrationAllowed === false), 'B-01 candidates must remain outside product runtime')
  assert(imageDependencies.every(item => item.status === 'candidate-build-verified-runtime-blocked'), 'B-01 candidate status is incomplete')
  const payload = dependencies.candidateBaselines?.image
  assert(payload?.scope === 'isolated-static-Rust-candidate-not-product-runtime', 'candidate payload scope is missing')
  assert(payload.incrementalExecutableBytes > 0 && payload.incrementalCompressedBytes > 0, 'candidate payload measurements are missing')
  assert(payload.finalNsisDeltaBytes === null && payload.finalNsisMeasurementStage === 'B-03 product integration', 'final NSIS delta must remain deferred to B-03')
  return { inputs: fixture.inputs.length, incrementalCompressedBytes: payload.incrementalCompressedBytes }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    assert(process.argv.length === 2, 'check-image-baseline.mjs does not accept arguments')
    const result = await checkImageBaseline()
    process.stdout.write(`Image baseline gate passed (${result.inputs} frozen inputs; ${result.incrementalCompressedBytes} B isolated compressed delta).\n`)
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}

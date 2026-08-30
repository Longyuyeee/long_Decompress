import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const contract = JSON.parse(await readFile(join(root, 'config', 'pdf-optimization-contract.json'), 'utf8'))
const dependencies = JSON.parse(await readFile(join(root, 'config', 'media-dependencies.json'), 'utf8'))
const fixtures = JSON.parse(await readFile(join(root, 'tests', 'fixtures', 'media', 'manifest.json'), 'utf8'))
const releaseGates = JSON.parse(await readFile(join(root, 'config', 'media-release-gates.json'), 'utf8'))

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function validate(candidate) {
  assert(candidate.schemaVersion === 1, 'unsupported PDF optimization contract schema')
  assert(candidate.node === 'D-01.2.2' && candidate.baselineNode === 'D-01.1', 'PDF contract node identity drifted')
  assert(/^\d{4}-\d{2}-\d{2}$/.test(candidate.reviewedAt), 'PDF contract review date is missing')
  assert(candidate.engine?.id === 'qpdf' && candidate.engine?.version === '12.4.0', 'qpdf identity drifted')
  assert(candidate.engine?.integrationAllowed === true && candidate.engine?.candidateCacheOnly === false && candidate.engine?.productionPreflightOnly === true, 'D-01.2.1 qpdf runtime admission boundary is invalid')
  assert(candidate.engine?.officialReferences?.length >= 4 && candidate.engine.officialReferences.every(url => url.startsWith('https://')), 'official qpdf references are incomplete')

  const lossless = candidate.modes?.['lossless-organization']
  const image = candidate.modes?.['compatible-image-optimization']
  assert(lossless?.lossy === false && image?.lossy === true, 'PDF mode loss semantics are incorrect')
  for (const argument of ['--object-streams=generate', '--compress-streams=y', '--decode-level=generalized', '--recompress-flate', '--compression-level=9']) {
    assert(lossless.arguments.includes(argument) && image.arguments.includes(argument), `required qpdf argument is missing: ${argument}`)
  }
  for (const argument of ['--optimize-images', '--jpeg-quality=85', '--oi-min-width=128', '--oi-min-height=128', '--oi-min-area=16384']) {
    assert(!lossless.arguments.includes(argument) && image.arguments.includes(argument), `image-only qpdf argument boundary drifted: ${argument}`)
  }
  assert(lossless.forbiddenChanges.includes('visible-page-content'), 'lossless mode must forbid visible-content changes')
  assert(image.allowedChanges.includes('eligible-image-pixels') && image.forbiddenChanges.includes('non-image-page-content'), 'image optimization change boundary is incomplete')
  assert(candidate.documentPolicies?.['digitally-signed']?.startsWith('analysis-only'), 'signed PDF execution must remain blocked')
  assert(candidate.documentPolicies?.encrypted?.startsWith('require-correct-password'), 'encrypted PDF must require a correct password before planning')
  assert(candidate.executionBoundary?.acceptRawArguments === false, 'raw qpdf arguments must remain forbidden')
  assert(candidate.executionBoundary?.publishProductRuntime === true && candidate.executionBoundary?.enableProductUi === false, 'D-01.2.1 must package identity-checked qpdf without exposing a half-built UI')
  assert(candidate.executionBoundary?.ghostscriptAllowed === false, 'Ghostscript must remain outside the redistribution boundary')
  assert(candidate.executionBoundary?.sourceMutationAllowed === false, 'PDF source mutation must remain forbidden')
}

validate(contract)
for (const mutation of [
  copy => { copy.engine.integrationAllowed = false },
  copy => { copy.modes['lossless-organization'].arguments.push('--optimize-images') },
  copy => { copy.executionBoundary.acceptRawArguments = true },
  copy => { copy.documentPolicies['digitally-signed'] = 'eligible' },
]) {
  const copy = structuredClone(contract)
  mutation(copy)
  let rejected = false
  try { validate(copy) } catch { rejected = true }
  assert(rejected, 'negative PDF contract control was unexpectedly accepted')
}

const qpdf = dependencies.dependencies.find(item => item.id === 'qpdf')
assert(qpdf?.version === contract.engine.version, 'qpdf dependency and capability-contract versions differ')
assert(qpdf?.integrationAllowed === true && qpdf?.status === 'runtime-admitted-d01-complete', 'qpdf D-01 runtime admission is incomplete')
assert(dependencies.blockedAlternatives?.some(item => item.id === 'ghostscript' && item.integrationAllowed === false), 'Ghostscript block is missing')

const fixtureKinds = new Set(fixtures.pdfs.map(item => item.kind))
for (const kind of contract.requiredFixtureKinds) assert(fixtureKinds.has(kind), `required D-01 fixture kind is missing: ${kind}`)
assert(fixtures.fixtureRevision === '2026-08-30-d01.1', 'D-01 fixture revision drifted')

for (const required of ['mixed-content', 'annotation-preserve', 'outline-preserve', 'attachment-preserve']) {
  assert(releaseGates.nodes?.D?.requiredRealCases?.includes(required), `PDF release gate is missing ${required}`)
}

console.log(`PDF optimization contract gate passed (${contract.requiredFixtureKinds.length} fixture kinds; qpdf production preflight only).`)

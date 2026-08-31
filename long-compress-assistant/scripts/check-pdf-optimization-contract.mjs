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
  assert(candidate.node === 'D-03.1' && candidate.baselineNode === 'D-01.1', 'PDF contract node identity drifted')
  assert(/^\d{4}-\d{2}-\d{2}$/.test(candidate.reviewedAt), 'PDF contract review date is missing')
  assert(candidate.engine?.id === 'qpdf' && candidate.engine?.version === '12.4.0', 'qpdf identity drifted')
  assert(candidate.engine?.integrationAllowed === true && candidate.engine?.candidateCacheOnly === false && candidate.engine?.productionPreflightOnly === false, 'D-02.1 qpdf read-only analysis boundary is invalid')
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
  assert(candidate.documentPolicies?.encrypted?.startsWith('require-correct-password') && candidate.documentPolicies.encrypted.includes('execution-blocked'), 'encrypted PDF must require a correct password for planning and remain execution-blocked')
  assert(candidate.analysisBoundary?.readOnly === true && candidate.analysisBoundary?.fixedArgumentsOnly === true, 'PDF analysis must remain read-only and fixed-argument only')
  assert(candidate.analysisBoundary?.passwordTransport === 'stdin-via-password-file-dash' && candidate.analysisBoundary?.passwordInArguments === false, 'PDF password transport must stay off the process argument list')
  assert(candidate.analysisBoundary?.timeoutSeconds === 30 && candidate.analysisBoundary?.maximumJsonBytes === 33554432, 'PDF analysis process bounds drifted')
  for (const argument of ['--is-encrypted', '--requires-password', '--password-file=-', '--json=2', '--json-stream-data=none', '--json-key=pages', '--json-key=acroform', '--json-key=attachments', '--json-key=outlines']) {
    assert(candidate.inspection?.arguments?.includes(argument), `required PDF analysis argument is missing: ${argument}`)
  }
  assert(candidate.executionBoundary?.acceptRawArguments === false, 'raw qpdf arguments must remain forbidden')
  assert(candidate.executionBoundary?.publishProductRuntime === true && candidate.executionBoundary?.enableProductUi === true, 'D-02.2 must expose the read-only PDF configuration UI')
  assert(candidate.executionBoundary?.executionEnabled === false && candidate.executionBoundary?.createsTasks === false, 'D-02.2 must not execute PDF transformations or create tasks')
  assert(candidate.executionBoundary?.configurationPersistence === 'page-local-draft-only', 'D-02.2 configuration must remain a page-local draft')
  assert(candidate.executionBoundary?.defaultOutput === 'new-file' && candidate.executionBoundary?.sourceMutationAllowed === false, 'PDF configuration must always propose a new output file')
  assert(candidate.executionBoundary?.lossyModeRequiresExplicitConfirmation === true, 'lossy PDF mode must require explicit confirmation')
  assert(candidate.executionBoundary?.sizeReductionGuaranteed === false, 'PDF UI must not guarantee size reduction')
  assert(candidate.executionBoundary?.signedDocumentCanFreezeConfiguration === false, 'signed PDF configuration must remain blocked')
  assert(candidate.executionBoundary?.stagingTransformEnabled === true && candidate.executionBoundary?.stagingApiExposure === 'internal-library-only', 'D-03.1 must expose only the internal owned-staging transform')
  assert(candidate.executionBoundary?.validationEnabled === false && candidate.executionBoundary?.publicationEnabled === false, 'D-03.1 must not validate or publish candidates yet')
  assert(candidate.executionBoundary?.encryptedExecutionEnabled === false, 'encrypted PDF execution must remain blocked')
  assert(candidate.executionBoundary?.transformTimeoutSeconds === 600 && candidate.executionBoundary?.stagingCleanup === 'owned-drop-guard', 'D-03.1 process and staging bounds drifted')
  assert(candidate.executionBoundary?.capacityPreflight === 'shared-storage-preflight' && candidate.executionBoundary?.sourceIntegrityCheck === 'sha256-before-and-after-transform', 'D-03.1 shared preflight/source integrity boundary drifted')
  assert(candidate.executionBoundary?.ghostscriptAllowed === false, 'Ghostscript must remain outside the redistribution boundary')
  assert(candidate.executionBoundary?.sourceMutationAllowed === false, 'PDF source mutation must remain forbidden')
}

validate(contract)
for (const mutation of [
  copy => { copy.engine.integrationAllowed = false },
  copy => { copy.modes['lossless-organization'].arguments.push('--optimize-images') },
  copy => { copy.executionBoundary.acceptRawArguments = true },
  copy => { copy.analysisBoundary.passwordInArguments = true },
  copy => { copy.documentPolicies['digitally-signed'] = 'eligible' },
  copy => { copy.executionBoundary.executionEnabled = true },
  copy => { copy.executionBoundary.lossyModeRequiresExplicitConfirmation = false },
  copy => { copy.executionBoundary.publicationEnabled = true },
  copy => { copy.executionBoundary.encryptedExecutionEnabled = true },
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

console.log(`PDF optimization contract gate passed (${contract.requiredFixtureKinds.length} fixture kinds; D-03.1 internal owned staging enabled, validation/publication frozen).`)

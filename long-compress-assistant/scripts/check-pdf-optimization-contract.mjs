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
  assert(candidate.node === 'D-04.1' && candidate.baselineNode === 'D-01.1', 'PDF contract node identity drifted')
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
  for (const argument of ['--is-encrypted', '--requires-password', '--password-file=-', '--json=2', '--json-stream-data=none', '--json-key=pages', '--json-key=acroform', '--json-key=attachments', '--json-key=outlines', '--json-key=qpdf', '--show-attachment']) {
    assert(candidate.inspection?.arguments?.includes(argument), `required PDF analysis argument is missing: ${argument}`)
  }
  assert(candidate.executionBoundary?.acceptRawArguments === false, 'raw qpdf arguments must remain forbidden')
  assert(candidate.executionBoundary?.publishProductRuntime === true && candidate.executionBoundary?.enableProductUi === true, 'D-02.2 must expose the read-only PDF configuration UI')
  assert(candidate.executionBoundary?.executionEnabled === true && candidate.executionBoundary?.createsTasks === false, 'D-04.1 must expose only the product command before task/UI orchestration')
  assert(candidate.executionBoundary?.configurationPersistence === 'page-local-draft-only', 'D-02.2 configuration must remain a page-local draft')
  assert(candidate.executionBoundary?.defaultOutput === 'new-file' && candidate.executionBoundary?.sourceMutationAllowed === false, 'PDF configuration must always propose a new output file')
  assert(candidate.executionBoundary?.lossyModeRequiresExplicitConfirmation === true, 'lossy PDF mode must require explicit confirmation')
  assert(candidate.executionBoundary?.sizeReductionGuaranteed === false, 'PDF UI must not guarantee size reduction')
  assert(candidate.executionBoundary?.signedDocumentCanFreezeConfiguration === false, 'signed PDF configuration must remain blocked')
  assert(candidate.executionBoundary?.stagingTransformEnabled === true && candidate.executionBoundary?.stagingApiExposure === 'internal-library-only', 'D-03.1 must expose only the internal owned-staging transform')
  assert(candidate.executionBoundary?.validationEnabled === true && candidate.executionBoundary?.validationApiExposure === 'internal-library-only', 'D-03.2 must enable only internal candidate validation')
  assert(candidate.executionBoundary?.publicationEnabled === true && candidate.executionBoundary?.publicationApiExposure === 'internal-library-only', 'D-03.3 must enable only internal atomic publication')
  assert(candidate.executionBoundary?.productCommand === 'compress_pdf_file', 'D-04.1 product command identity drifted')
  assert(JSON.stringify(candidate.executionBoundary?.productCommandStages) === '["Transforming","Validating","Publishing"]', 'D-04.1 truthful stage contract drifted')
  assert(candidate.executionBoundary?.productCommandRevalidatesEngineAndDocument === true && candidate.executionBoundary?.sharedCancellationRegistry === true, 'D-04.1 must revalidate through the shared cancellation path')
  assert(candidate.executionBoundary?.outputLockScope === 'process-wide-normalized-destination', 'D-03.3 cross-task output lock drifted')
  assert(candidate.executionBoundary?.markOfTheWebPolicy === 'propagate-internet-or-restricted-zone-before-atomic-rename', 'D-03.3 Mark-of-the-Web policy drifted')
  for (const stage of ['normalized-cross-task-output-lock', 'candidate-validation', 'cancellation-recheck', 'source-sha256-recheck', 'candidate-sha256-recheck', 'mark-of-the-web-policy', 'same-directory-atomic-rename', 'published-filesystem-identity']) {
    assert(candidate.executionBoundary?.publicationTransaction?.includes(stage), `D-03.3 publication stage is missing: ${stage}`)
  }
  for (const fact of ['filesystem-bytes', 'sha256', 'savings-ratio', 'mark-of-the-web-status', 'validated-structural-facts']) {
    assert(candidate.executionBoundary?.publishedFacts?.includes(fact), `D-03.3 published fact is missing: ${fact}`)
  }
  assert(candidate.executionBoundary?.controlledLowCapacityVolumeEvidence === true, 'D-03.3.1 controlled low-capacity volume evidence must remain closed')
  assert(candidate.executionBoundary?.encryptedExecutionEnabled === false, 'encrypted PDF execution must remain blocked')
  assert(candidate.executionBoundary?.transformTimeoutSeconds === 600 && candidate.executionBoundary?.stagingCleanup === 'owned-drop-guard', 'D-03.1 process and staging bounds drifted')
  assert(candidate.executionBoundary?.capacityPreflight === 'shared-storage-preflight' && candidate.executionBoundary?.sourceIntegrityCheck === 'sha256-before-and-after-transform', 'D-03.1 shared preflight/source integrity boundary drifted')
  assert(candidate.executionBoundary?.validationTimeoutSeconds === 120 && candidate.executionBoundary?.maximumValidationJsonBytes === 33554432, 'D-03.2 validation process bounds drifted')
  assert(candidate.executionBoundary?.maximumAttachmentBytes === 67108864 && candidate.executionBoundary?.maximumTotalAttachmentBytes === 134217728, 'D-03.2 attachment validation bounds drifted')
  assert(JSON.stringify(candidate.executionBoundary?.qpdfCheckAcceptedExitCodes) === '[0]', 'D-03.2 qpdf check must fail closed on warnings/errors')
  for (const fact of ['candidate-sha256-before-and-after-validation', 'page-count', 'page-media-boxes', 'form-field-identities', 'annotation-page-and-subtype', 'outline-title-and-destination-page', 'attachment-name-bytes-and-sha256']) {
    assert(candidate.executionBoundary?.validatedFacts?.includes(fact), `D-03.2 validation fact is missing: ${fact}`)
  }
  for (const failure of ['damaged-input', 'corrupt-candidate', 'validation-cancellation', 'target-race', 'larger-output-default-refusal']) {
    assert(candidate.executionBoundary?.failureMatrix?.includes(failure), `D-03.2 failure boundary is missing: ${failure}`)
  }
  assert(candidate.executionBoundary?.ghostscriptAllowed === false, 'Ghostscript must remain outside the redistribution boundary')
  assert(candidate.executionBoundary?.sourceMutationAllowed === false, 'PDF source mutation must remain forbidden')
  assert(candidate.executionBoundary?.largerOutputDefault === 'do-not-publish' && candidate.executionBoundary?.largerOutputExplicitRetention === 'validated-product-request-only', 'larger PDF output policy drifted')
}

validate(contract)
for (const mutation of [
  copy => { copy.engine.integrationAllowed = false },
  copy => { copy.modes['lossless-organization'].arguments.push('--optimize-images') },
  copy => { copy.executionBoundary.acceptRawArguments = true },
  copy => { copy.analysisBoundary.passwordInArguments = true },
  copy => { copy.documentPolicies['digitally-signed'] = 'eligible' },
  copy => { copy.executionBoundary.executionEnabled = false },
  copy => { copy.executionBoundary.lossyModeRequiresExplicitConfirmation = false },
  copy => { copy.executionBoundary.encryptedExecutionEnabled = true },
  copy => { copy.executionBoundary.validationEnabled = false },
  copy => { copy.executionBoundary.validatedFacts = copy.executionBoundary.validatedFacts.filter(value => value !== 'attachment-name-bytes-and-sha256') },
  copy => { copy.executionBoundary.publicationEnabled = false },
  copy => { copy.executionBoundary.publicationTransaction = copy.executionBoundary.publicationTransaction.filter(value => value !== 'source-sha256-recheck') },
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
for (const required of ['qpdf-check', 'annotation-policy', 'outline-policy', 'attachment-bytes']) {
  assert(releaseGates.nodes?.D?.requiredValidation?.includes(required), `PDF release validation is missing ${required}`)
}

console.log(`PDF optimization contract gate passed (${contract.requiredFixtureKinds.length} fixture kinds; D-04.1 product command enabled, task/UI orchestration frozen).`)

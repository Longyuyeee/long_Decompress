import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const contractPath = join(root, 'config', 'media-release-gates.json')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

export function validateMediaReleaseGates(contract) {
  assert(contract.schemaVersion === 1, 'unsupported media release gate schema')
  assert(/^\d{4}-\d{2}-\d{2}$/.test(contract.reviewedAt), 'reviewedAt must be an ISO date')
  const policy = contract.policy
  assert(policy?.versionBump === 'complete-user-visible-node-only', 'partial media nodes must not bump versions')
  assert(policy?.requiresInstalledNsis === true, 'installed NSIS evidence is required')
  assert(policy?.requiresPreviousReleaseRestore === true, 'previous release restore is required')
  assert(policy?.requiresPublicUpdateAfterRelease === true, 'public updater evidence is required')
  assert(policy?.unsignedWindowsBuildAllowed === true, 'the project has no commercial signing certificate')
  assert(policy?.unsignedNativeContextMenuPackageAllowed === false, 'unsigned native context-menu identity packages are forbidden')
  assert(policy?.estimatedMetricsAcceptedAsEvidence === false, 'estimated metrics cannot be release evidence')
  assert(policy?.ignoredOrSyntheticSuccessAccepted === false, 'ignored or synthetic successes cannot close a node')
  assert(contract.sharedInstalledChecks?.length >= 10, 'installed lifecycle matrix is incomplete')
  assert(contract.sharedRollbackFaults?.length >= 6, 'rollback fault matrix is incomplete')

  const expectedNodes = ['B', 'C', 'D']
  assert(Object.keys(contract.nodes ?? {}).sort().join(',') === expectedNodes.join(','), 'B/C/D node gates are required')
  for (const node of expectedNodes) {
    const gate = contract.nodes[node]
    assert(gate.publicFormats?.length > 0, `${node}: public format scope is required`)
    assert(gate.requiredRealCases?.length >= 5, `${node}: real installed cases are incomplete`)
    assert(gate.requiredValidation?.includes('filesystem-bytes'), `${node}: final filesystem byte validation is required`)
  }
  assert(contract.nodes.B.publicFormats.join(',') === 'jpeg,webp,png-lossless', 'B format scope drifted from the dependency audit')
  assert(contract.nodes.B.requiredRealCases.includes('three-samples-per-public-format'), 'B-05.1 three-sample format matrix is required')
  assert(contract.nodes.B.requiredRealCases.includes('gif-explicit-preserve-or-reject'), 'GIF boundary evidence is required')
  assert(contract.nodes.D.requiredRealCases.includes('signed-explicit-refusal'), 'signed PDF refusal evidence is required')
  assert(contract.requiredEvidenceFields?.length >= 13, 'release evidence template fields are incomplete')
  return contract
}

async function validateInstalledBaseline(evidencePath) {
  const evidence = JSON.parse(await readFile(evidencePath, 'utf8'))
  assert(evidence.succeeded === true, 'installed baseline did not succeed')
  assert(Array.isArray(evidence.checks) && evidence.checks.length >= 35, 'installed baseline has insufficient checks')
  const failed = evidence.checks.filter(check => check.passed !== true)
  assert(failed.length === 0, `installed baseline contains failed checks: ${failed.map(check => check.name).join(', ')}`)
  assert(evidence.previousVersion && evidence.candidateVersion, 'installed baseline version traceability is missing')
  assert(evidence.restoredInstallLocation, 'installed baseline did not restore the previous installation')
  return { checks: evidence.checks.length, previousVersion: evidence.previousVersion, candidateVersion: evidence.candidateVersion }
}

export async function checkMediaReleaseGates({ installedEvidence = '' } = {}) {
  const contract = validateMediaReleaseGates(JSON.parse(await readFile(contractPath, 'utf8')))
  const baseline = installedEvidence ? await validateInstalledBaseline(installedEvidence) : null
  return { contract, baseline }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    const evidenceFlag = process.argv.indexOf('--installed-evidence')
    const installedEvidence = evidenceFlag >= 0 ? process.argv[evidenceFlag + 1] : ''
    const allowedCount = installedEvidence ? 2 : 0
    assert(process.argv.slice(2).length === allowedCount, 'usage: check-media-release-gates.mjs [--installed-evidence <result.json>]')
    assert(evidenceFlag < 0 || installedEvidence, '--installed-evidence requires a path')
    const result = await checkMediaReleaseGates({ installedEvidence })
    process.stdout.write(`Media release gates passed${result.baseline ? ` with ${result.baseline.checks} real installed checks` : ''}.\n`)
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}

import { readFile, stat } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const contractPath = join(root, 'config', 'media-metric-sources.json')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

export function validateMediaMetricContract(contract) {
  assert(contract.schemaVersion === 1, 'unsupported media metric contract schema')
  assert(/^\d{4}-\d{2}-\d{2}$/.test(contract.reviewedAt), 'reviewedAt must be an ISO date')
  const policy = contract.policy
  assert(policy?.unknownValue === null, 'unknown metrics must be represented as null')
  assert(policy?.estimatedValuesMustBeLabeled === true, 'estimated values must be labeled')
  assert(policy?.estimatedValuesMayEnterHistory === false, 'estimated values must not enter history')
  assert(policy?.finalMetricsRequireValidatedOutput === true, 'final metrics require validated output')
  assert(policy?.qualityScoreAllowed === false, 'synthetic quality scores must remain forbidden')
  assert(policy?.inputBytesSource === 'source-file-filesystem-metadata-before-processing', 'input byte source drifted')
  assert(policy?.finalOutputBytesSource === 'published-file-filesystem-metadata', 'output byte source drifted')

  const workloads = contract.workloads
  assert(Object.keys(workloads ?? {}).sort().join(',') === 'image,pdf,video', 'image/video/PDF contracts are required')
  assert(workloads.image.progress.percent === 'batch-item-count-only', 'image percent must use completed item count')
  assert(workloads.image.progress.eta === null, 'image ETA is not currently provable')
  assert(workloads.video.progress.authoritativeSource === 'ffmpeg-progress-pipe', 'video progress must use FFmpeg progress pipe')
  assert(workloads.video.progress.percent === 'out_time_us/probed-duration-us', 'video percent must be timestamp based')
  assert(workloads.video.progress.eta.includes('after-two-samples'), 'video ETA requires a stable sample window')
  assert(workloads.pdf.progress.percent === null, 'PDF must not expose a simulated percentage')
  assert(workloads.pdf.progress.eta === null, 'PDF must not expose a simulated ETA')
  assert(Array.isArray(contract.stages) && contract.stages.at(-1) === 'completed', 'verified terminal stage is required')
  return contract
}

async function verifyRealFixtureFacts(contract) {
  const auditRoot = join(root, 'test-results', 'media-fixture-audit')
  const result = JSON.parse(await readFile(join(auditRoot, 'result.json'), 'utf8'))
  assert(result.differences?.length === 0, 'real fixture baseline contains unresolved differences')
  assert(result.productIntegrationAllowed === false, 'GPL test tool must remain excluded from the product')
  assert(result.expected.identityPolicy?.acceptanceBasis === 'decoded-and-probed-properties', 'fixture acceptance basis is missing')
  assert(result.expected.identityPolicy?.byteIdentityRequired === false, 'B-00 fixtures must not claim byte identity')
  assert(result.expected.identityPolicy?.performanceBaselineEligible === false, 'unfrozen fixtures must not enter performance baselines')

  const groups = [['images', 'images'], ['videos', 'videos'], ['pdfs', 'pdfs']]
  const measured = []
  for (const [manifestKey, directory] of groups) {
    for (const item of result.expected[manifestKey]) {
      const fixture = join(auditRoot, 'fixtures', directory, item.file)
      const facts = await stat(fixture)
      assert(facts.isFile() && facts.size > 0, `${manifestKey}/${item.file}: real file is missing or empty`)
      measured.push({ workload: manifestKey.slice(0, -1), file: item.file, inputBytes: facts.size })
    }
  }
  const expectedCount = result.expected.images.length + result.expected.videos.length + result.expected.pdfs.length
  assert(measured.length === expectedCount, `expected ${expectedCount} real fixture files, found ${measured.length}`)
  assert(measured.every(item => Number.isSafeInteger(item.inputBytes)), 'filesystem byte facts must be safe integers')
  assert(contract.policy.finalMetricsRequireValidatedOutput, 'real facts cannot bypass output validation')
  return measured
}

export async function checkMediaMetrics({ real = false } = {}) {
  const contract = validateMediaMetricContract(JSON.parse(await readFile(contractPath, 'utf8')))
  const measured = real ? await verifyRealFixtureFacts(contract) : []
  return { contract, measured }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    const real = process.argv.includes('--real')
    const unexpected = process.argv.slice(2).filter(argument => argument !== '--real')
    assert(unexpected.length === 0, `unknown arguments: ${unexpected.join(', ')}`)
    const result = await checkMediaMetrics({ real })
    process.stdout.write(`Media metric contract passed${real ? ` with ${result.measured.length} real filesystem facts` : ''}.\n`)
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}

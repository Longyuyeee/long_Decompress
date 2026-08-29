import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const evidenceRoot = resolve(
  process.env.WINDOWS_N_EVIDENCE_DIRECTORY
    || process.argv[2]
    || join(root, 'test-results', 'windows-n-video-runtime'),
)
const manifest = JSON.parse(await readFile(join(root, 'config', 'media-dependencies.json'), 'utf8'))
const videoEvidence = manifest.candidateBaselines.video
const candidateEvidence = videoEvidence.c05InstalledCandidate
const beforePath = join(evidenceRoot, 'before-media-feature-pack.json')
const afterPath = join(evidenceRoot, 'after-media-feature-pack.json')
const runtimePath = join(evidenceRoot, 'after-feature-pack-runtime', 'result.json')
const producerPath = join(root, videoEvidence.windowsNEvidenceScript)
const verificationPath = join(evidenceRoot, 'verification.json')
const checks = []
await mkdir(evidenceRoot, { recursive: true })

function assert(condition, message) {
  checks.push({ passed: Boolean(condition), failureIfFalse: message })
  if (!condition) throw new Error(message)
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}

async function readJson(path) {
  return JSON.parse((await readFile(path, 'utf8')).replace(/^\uFEFF/, ''))
}

function assertSha256(value, label) {
  assert(/^[a-f0-9]{64}$/.test(value), `${label}: lowercase SHA-256 is required`)
}

function verifyCommon(report, phase, producerSha256) {
  assert(report.schemaVersion === 2, `${phase}: unsupported report schema`)
  assert(report.phase === phase, `${phase}: phase identity differs`)
  assert(report.passed === true && report.failure === null, `${phase}: phase did not pass`)
  assert(report.producerScriptSha256 === producerSha256, `${phase}: evidence producer identity differs`)
  assert(Array.isArray(report.checks) && report.checks.length >= 7, `${phase}: check list is incomplete`)
  assert(report.checks.every(check => check.passed === true), `${phase}: a recorded check failed`)
  assert(report.machine?.isWindowsNEdition === true, `${phase}: machine is not classified as Windows N`)
  assert(/N$/i.test(report.machine?.editionId), `${phase}: EditionID is not a Windows N edition`)
  assert(report.machine?.architecture === 'X64', `${phase}: Windows x64 evidence is required`)
  assertSha256(report.machine?.identitySha256, `${phase}: machine identity`)
  assert(report.expected?.windowsNEdition === true, `${phase}: Windows N expectation is missing`)
  assert(report.expected?.installedVersion === candidateEvidence.version, `${phase}: candidate version differs`)
  assert(report.expected?.executableBytes === candidateEvidence.executableBytes, `${phase}: expected executable size differs`)
  assert(report.expected?.executableSha256 === candidateEvidence.executableSha256, `${phase}: expected executable hash differs`)
  assert(report.actual?.executable?.bytes === candidateEvidence.executableBytes, `${phase}: installed executable size differs`)
  assert(report.actual?.executable?.sha256 === candidateEvidence.executableSha256, `${phase}: installed executable hash differs`)
  const modules = report.actual?.mediaFoundationModules
  assert(Array.isArray(modules) && modules.length === 3, `${phase}: Media Foundation module inventory is incomplete`)
  assert(new Set(modules.map(module => module.name)).size === 3, `${phase}: Media Foundation module inventory contains duplicates`)
  assert(['mfplat.dll', 'mf.dll', 'mfreadwrite.dll'].every(name => modules.some(module => module.name === name)), `${phase}: Media Foundation module identity differs`)
  for (const module of modules.filter(item => item.present)) {
    assert(Number.isSafeInteger(module.bytes) && module.bytes > 0, `${phase}: ${module.name} byte size is invalid`)
    assertSha256(module.sha256, `${phase}: ${module.name}`)
  }
}

let failure = null
let before
let after
let runtime
try {
  const producerSha256 = await sha256(producerPath)
  before = await readJson(beforePath)
  verifyCommon(before, 'MissingMediaFeaturePack', producerSha256)
  after = await readJson(afterPath)
  verifyCommon(after, 'MediaFeaturePackInstalled', producerSha256)
  runtime = await readJson(runtimePath)

  assert(before.machine.identitySha256 === after.machine.identitySha256, 'both phases must run on the same machine')
  assert(after.actual?.beforeReportSha256 === await sha256(beforePath), 'after phase is not chained to the exact before report bytes')
  assert(before.actual?.productionPreflight?.schemaVersion === 1, 'before production preflight schema differs')
  assert(before.actual?.productionPreflight?.passed === false, 'before production preflight was not refused')
  assert(/^VIDEO_ENGINE_MEDIA_FOUNDATION_UNAVAILABLE: (mfplat|mf|mfreadwrite)\.dll: win32=\d+$/.test(before.actual?.productionPreflight?.error), 'before Media Foundation refusal classification differs')
  assert(after.actual?.productionPreflight?.schemaVersion === 1, 'after production preflight schema differs')
  assert(after.actual?.productionPreflight?.passed === true, 'after production preflight did not pass')
  assert(after.actual?.productionPreflight?.status?.mediaFoundationAvailable === true, 'after production preflight did not admit Media Foundation')
  assert(after.actual?.productionPreflight?.status?.version === '9.0.1', 'after FFmpeg version differs')
  assert(after.actual?.productionPreflight?.status?.videoEncoder === 'h264_mf', 'after video encoder differs')
  assert(after.actual?.productionPreflight?.status?.audioEncoder === 'aac', 'after audio encoder differs')
  assert(after.actual?.productionPreflight?.status?.hardwareEncoding === false, 'after preflight enabled hardware encoding')
  assert(after.actual.mediaFoundationModules.every(module => module.present === true), 'Media Feature Pack phase does not contain all required modules')

  assert(runtime.schemaVersion === 1 && runtime.passed === true, 'post-feature-pack installed runtime matrix did not pass')
  assert(Array.isArray(runtime.differences) && runtime.differences.length === 0, 'post-feature-pack installed runtime matrix contains differences')
  assert(/N$/i.test(runtime.machine?.windowsEditionId), 'post-feature-pack runtime matrix is not from Windows N')
  assert(runtime.actual?.executable?.bytes === candidateEvidence.executableBytes, 'runtime matrix executable size differs')
  assert(runtime.actual?.executable?.sha256 === candidateEvidence.executableSha256, 'runtime matrix executable hash differs')
  assert(runtime.actual?.productionPreflight?.passed === true, 'runtime matrix production preflight did not pass')
  assert(runtime.actual?.productionPreflight?.status?.mediaFoundationAvailable === true, 'runtime matrix did not admit Media Foundation')
  assert(runtime.actual?.missingResource?.passed === false && runtime.actual.missingResource.error?.includes('VIDEO_ENGINE_RESOURCE_MISSING'), 'runtime matrix missing-resource refusal differs')
  assert(runtime.actual?.replacedResource?.passed === false && runtime.actual.replacedResource.error?.includes('VIDEO_ENGINE_RESOURCE_HASH_MISMATCH'), 'runtime matrix replaced-resource refusal differs')
  const streams = runtime.actual?.output?.probe?.streams
  const format = runtime.actual?.output?.probe?.format
  const video = streams?.find(stream => stream.codec_type === 'video')
  const audio = streams?.find(stream => stream.codec_type === 'audio')
  assert(video?.codec_name === 'h264' && video.width === 480 && video.height === 854, 'runtime matrix output video facts differ')
  assert(audio?.codec_name === 'aac', 'runtime matrix output AAC stream is missing')
  assert(Math.abs(Number(format?.duration) - 1.2) <= 0.05, 'runtime matrix output duration differs')
  assertSha256(runtime.actual?.output?.sha256, 'runtime matrix output')
} catch (error) {
  failure = error instanceof Error ? error.message : String(error)
}

const verification = {
  schemaVersion: 1,
  verifiedAt: new Date().toISOString(),
  evidenceRoot,
  expectedCandidate: {
    version: candidateEvidence.version,
    executableBytes: candidateEvidence.executableBytes,
    executableSha256: candidateEvidence.executableSha256,
  },
  machineIdentitySha256: before?.machine?.identitySha256 ?? null,
  checks,
  failure,
  passed: failure === null,
}
await writeFile(verificationPath, `${JSON.stringify(verification, null, 2)}\n`)
if (failure) throw new Error(`${failure}. Verification: ${verificationPath}`)
console.log(`Windows N video runtime evidence passed (${checks.length} checks). Verification: ${verificationPath}`)

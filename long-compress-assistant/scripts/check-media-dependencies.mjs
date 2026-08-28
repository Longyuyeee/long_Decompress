import { createHash } from 'node:crypto'
import { createWriteStream } from 'node:fs'
import { access, mkdir, readFile, readdir, rename, rm, stat } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { basename, join, resolve } from 'node:path'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const manifestPath = join(root, 'config', 'media-dependencies.json')
const real = process.argv.includes('--real')
const cache = join(root, 'test-results', 'media-dependency-audit')
const sevenZip = join(root, 'src-tauri', 'resources', 'archive-engine', '7z.exe')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function validateArtifact(artifact, label, allowedHosts) {
  assert(artifact && typeof artifact === 'object', `${label}: artifact is required`)
  assert(typeof artifact.fileName === 'string' && artifact.fileName.length > 3, `${label}: fileName is required`)
  assert(Number.isSafeInteger(artifact.bytes) && artifact.bytes > 0, `${label}: positive byte size is required`)
  assert(/^[a-f0-9]{64}$/.test(artifact.sha256), `${label}: exact lowercase SHA-256 is required`)
  const url = new URL(artifact.url)
  assert(url.protocol === 'https:', `${label}: artifact URL must use HTTPS`)
  assert(allowedHosts.includes(url.hostname), `${label}: unapproved download host ${url.hostname}`)
}

function validateManifest(manifest) {
  assert(manifest.schemaVersion === 1, 'unsupported media dependency manifest schema')
  assert(/^\d{4}-\d{2}-\d{2}$/.test(manifest.reviewedAt), 'reviewedAt must be an ISO date')
  const hosts = manifest.policy?.allowedDownloadHosts
  assert(Array.isArray(hosts) && hosts.length > 0, 'allowed download hosts are required')
  assert(manifest.policy?.releaseRequiresRealVerification === true, 'release must require real dependency verification')
  assert(Number.isSafeInteger(manifest.policy?.securityReviewCadenceDays), 'security review cadence is required')
  assert(Array.isArray(manifest.dependencies) && manifest.dependencies.length >= 4, 'all candidate workloads must be locked')

  const ids = new Set()
  for (const dependency of manifest.dependencies) {
    const label = dependency.id || 'unknown dependency'
    assert(!ids.has(label), `duplicate dependency id: ${label}`)
    ids.add(label)
    assert(['image', 'video', 'pdf'].includes(dependency.workload), `${label}: invalid workload`)
    assert(/^\d+\.\d+\.\d+(?:[-+].+)?$/.test(dependency.version), `${label}: exact semantic version is required`)
    if (dependency.integrationAllowed) {
      assert(dependency.workload === 'image', `${label}: only the B-03 image workload may enter runtime`)
      assert(['libcaesium', 'oxipng', 'image', 'img-parts'].includes(label), `${label}: runtime dependency is not approved`)
      assert(dependency.status === 'runtime-admitted-b03-service', `${label}: runtime admission status is invalid`)
    } else {
      assert(dependency.status.includes('blocked') || dependency.status.includes('candidate'), `${label}: pre-engine status must remain blocked/candidate`)
    }
    assert(dependency.projectUrl?.startsWith('https://'), `${label}: project URL is required`)
    assert(dependency.license?.expression && dependency.license?.url?.startsWith('https://'), `${label}: license identity and URL are required`)
    assert(dependency.license?.noticeRequired === true, `${label}: redistribution notice policy is required`)
    assert(dependency.linkage, `${label}: linkage/redistribution mode is required`)
    assert(dependency.platforms?.includes('windows-x86_64'), `${label}: Windows x64 support decision is required`)
    assert(dependency.features?.allowed?.length > 0, `${label}: allowed feature set is required`)
    assert(dependency.features?.forbidden?.length > 0, `${label}: forbidden feature set is required`)
    assert(!dependency.features.allowed.some(feature => dependency.features.forbidden.includes(feature)), `${label}: allowed and forbidden features overlap`)
    assert(dependency.installerImpact && 'compressedInstallerDeltaBytes' in dependency.installerImpact, `${label}: installer impact field is required`)
    assert(dependency.installerImpact.measurementStage, `${label}: installer measurement stage is required`)
    assert(dependency.securityOwner, `${label}: security update owner is required`)
    validateArtifact(dependency.artifact, label, hosts)
    if (dependency.signature) {
      validateArtifact(dependency.signature, `${label} signature`, hosts)
      assert(/^[a-f0-9]{64}$/.test(dependency.signature.keySha256), `${label}: signing-key SHA-256 is required`)
      assert(/^[A-F0-9]{40}$/.test(dependency.signature.fingerprint), `${label}: signing-key fingerprint is required`)
    }
    if (dependency.upstreamChecksum) validateArtifact(dependency.upstreamChecksum, `${label} checksum`, hosts)
  }
  assert(['libcaesium', 'oxipng', 'image', 'img-parts', 'ffmpeg', 'qpdf'].every((id) => ids.has(id)), 'image, video, and PDF dependencies are incomplete')
  const caesium = manifest.dependencies.find((item) => item.id === 'libcaesium')
  assert(['default', 'gif', 'png'].every((feature) => caesium.features.forbidden.includes(feature)), 'AGPL/GPL libcaesium feature paths must remain forbidden')
  const ffmpeg = manifest.dependencies.find((item) => item.id === 'ffmpeg')
  assert(['--enable-gpl', '--enable-nonfree', 'libx264', 'libx265'].every((feature) => ffmpeg.features.forbidden.includes(feature)), 'FFmpeg GPL/nonfree paths must remain forbidden')
  assert(ffmpeg.integrationAllowed === false, 'C-01.1 FFmpeg candidate must remain outside the product runtime')
  assert(ffmpeg.runtimeCandidate?.reproducibility === 'two-clean-builds-in-different-directories-byte-identical', 'FFmpeg reproducibility evidence is missing')
  assert(ffmpeg.runtimeCandidate?.buildScript === 'scripts/build-ffmpeg-c01-windows.sh', 'FFmpeg build script identity is missing')
  assert(ffmpeg.runtimeCandidate?.softwareEncoder?.name === 'h264_mf' && ffmpeg.runtimeCandidate?.softwareEncoder?.forceHardware === false, 'FFmpeg software encoder policy is invalid')
  assert(ffmpeg.runtimeCandidate?.files?.length === 2, 'FFmpeg and ffprobe runtime identities are required')
  for (const file of ffmpeg.runtimeCandidate.files) {
    assert(['ffmpeg.exe', 'ffprobe.exe'].includes(file.name), `unexpected FFmpeg runtime file: ${file.name}`)
    assert(Number.isSafeInteger(file.bytes) && file.bytes > 0, `${file.name}: positive byte size is required`)
    assert(/^[a-f0-9]{64}$/.test(file.sha256), `${file.name}: exact lowercase SHA-256 is required`)
  }
  assert(ffmpeg.runtimeCandidate?.licenseFiles?.length === 2, 'FFmpeg LGPL license payload is incomplete')
  assert(ffmpeg.installerImpact.runtimePayloadBytes === 24515448, 'FFmpeg isolated runtime payload measurement changed')
  assert(ffmpeg.installerImpact.compressedInstallerDeltaBytes === null, 'FFmpeg must not claim an NSIS delta before product integration')
  assert(manifest.blockedAlternatives?.some((item) => item.id === 'ghostscript' && item.integrationAllowed === false), 'Ghostscript must remain explicitly blocked')
}

async function sha256(path) {
  const bytes = await readFile(path)
  return createHash('sha256').update(bytes).digest('hex')
}

async function download(artifact) {
  const target = join(cache, artifact.fileName)
  const partial = `${target}.part`
  try {
    const current = await stat(target)
    if (current.size === artifact.bytes && (await sha256(target)) === artifact.sha256) return target
  } catch {}
  await rm(target, { force: true })
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    await rm(partial, { force: true })
    try {
      const response = await fetch(artifact.url, {
        redirect: 'follow',
        signal: AbortSignal.timeout(60_000),
        headers: {
          Accept: 'application/octet-stream',
          'User-Agent': 'Long-Decompress-Media-Dependency-Audit/1.1.14',
        },
      })
      assert(response.ok && response.body, `${artifact.fileName}: download failed with HTTP ${response.status}`)
      await pipeline(Readable.fromWeb(response.body), createWriteStream(partial))
      await rename(partial, target)
      return target
    } catch (error) {
      await rm(partial, { force: true })
      if (process.platform === 'win32') {
        try {
          run('curl.exe', [
            '--silent', '--show-error', '--location', '--fail', '--max-time', '120',
            '--user-agent', 'Long-Decompress-Media-Dependency-Audit/1.1.14',
            '--output', partial, artifact.url,
          ], `${artifact.fileName} curl fallback`)
          await rename(partial, target)
          return target
        } catch {
          await rm(partial, { force: true })
        }
      }
      if (attempt === 3) throw error
      process.stderr.write(`${artifact.fileName}: download attempt ${attempt}/3 failed; retrying.\n`)
    }
  }
  throw new Error(`${artifact.fileName}: download attempts exhausted`)
}

async function verifyDownloaded(artifact, label) {
  const path = await download(artifact)
  const actual = await stat(path)
  assert(actual.size === artifact.bytes, `${label}: expected ${artifact.bytes} bytes, got ${actual.size}`)
  const digest = await sha256(path)
  assert(digest === artifact.sha256, `${label}: SHA-256 mismatch (${digest})`)
  return path
}

function run(command, args, label) {
  const result = spawnSync(command, args, { encoding: 'utf8', windowsHide: true })
  assert(result.status === 0, `${label} failed (${result.status}): ${result.stderr || result.stdout}`)
  return `${result.stdout ?? ''}${result.stderr ?? ''}`
}

function toWslPath(path) {
  const match = /^([A-Za-z]):\\(.*)$/.exec(resolve(path))
  assert(match, `cannot translate path to WSL: ${path}`)
  return `/mnt/${match[1].toLowerCase()}/${match[2].replaceAll('\\', '/')}`
}

function shellQuote(value) {
  return `'${value.replaceAll("'", "'\\''")}'`
}

function verifyFfmpegSignatureWithWsl(dependency, sourcePath, signaturePath, keyPath) {
  const home = `/tmp/long-decompress-ffmpeg-gpg-${process.pid}`
  const command = [
    'set -eu',
    `rm -rf ${shellQuote(home)}`,
    `mkdir -m 700 ${shellQuote(home)}`,
    `gpg --homedir ${shellQuote(home)} --batch --import ${shellQuote(toWslPath(keyPath))}`,
    `gpg --homedir ${shellQuote(home)} --batch --with-colons --fingerprint ffmpeg-devel@ffmpeg.org`,
    `gpg --homedir ${shellQuote(home)} --batch --verify ${shellQuote(toWslPath(signaturePath))} ${shellQuote(toWslPath(sourcePath))}`,
    `rm -rf ${shellQuote(home)}`,
  ].join('; ')
  const output = run('wsl.exe', ['sh', '-lc', command], 'FFmpeg detached-signature verification')
  assert(output.includes(dependency.signature.fingerprint), 'FFmpeg signing-key fingerprint mismatch')
  assert(output.includes('Good signature'), 'FFmpeg detached signature was not accepted')
}

async function verifyReal(manifest) {
  assert(process.platform === 'win32', 'real media dependency verification currently requires Windows x64')
  await mkdir(cache, { recursive: true })
  await access(sevenZip)
  for (const dependency of manifest.dependencies) {
    const artifactPath = await verifyDownloaded(dependency.artifact, dependency.id)
    const listing = run(sevenZip, ['l', '-slt', artifactPath], `${dependency.id} archive listing`)
    if (['libcaesium', 'oxipng', 'image', 'img-parts'].includes(dependency.id)) {
      const crateRoot = join(cache, `real-${dependency.id}`)
      await rm(crateRoot, { recursive: true, force: true })
      await mkdir(crateRoot, { recursive: true })
      run(sevenZip, ['x', '-y', `-o${crateRoot}`, artifactPath], `${dependency.id} gzip extraction`)
      const tarName = (await readdir(crateRoot))[0]
      assert(tarName, `${dependency.id}: crate did not contain a tar stream`)
      const crateListing = run(sevenZip, ['l', '-slt', join(crateRoot, tarName)], `${dependency.id} tar listing`)
      assert(crateListing.includes('Cargo.toml') && /LICENSE/i.test(crateListing), `${dependency.id}: crate metadata/license missing`)
    }
    if (dependency.id === 'ffmpeg') {
      const signaturePath = await verifyDownloaded(dependency.signature, 'ffmpeg signature')
      const keyPath = await verifyDownloaded({
        fileName: basename(dependency.signature.keyUrl),
        url: dependency.signature.keyUrl,
        bytes: dependency.signature.keyBytes,
        sha256: dependency.signature.keySha256,
      }, 'ffmpeg signing key')
      assert((await stat(signaturePath)).size > 0, 'ffmpeg detached signature is empty')
      assert(listing.includes('ffmpeg-9.0.1.tar'), 'FFmpeg xz archive does not contain the expected tar stream')
      verifyFfmpegSignatureWithWsl(dependency, artifactPath, signaturePath, keyPath)
    }
    if (dependency.id === 'qpdf') {
      const checksumPath = await verifyDownloaded(dependency.upstreamChecksum, 'qpdf upstream checksum')
      const checksum = await readFile(checksumPath, 'utf8')
      assert(checksum.includes(`${dependency.artifact.sha256}  ${dependency.artifact.fileName}`), 'qpdf upstream checksum does not authorize the locked artifact')
      const extractRoot = join(cache, 'real-qpdf')
      await rm(extractRoot, { recursive: true, force: true })
      await mkdir(extractRoot, { recursive: true })
      run(sevenZip, ['x', '-y', `-o${extractRoot}`, artifactPath], 'qpdf extraction')
      const bin = join(extractRoot, `qpdf-${dependency.version}-mingw64`, 'bin')
      let subsetBytes = 0
      for (const file of dependency.runtimeSubset.files) subsetBytes += (await stat(join(bin, file))).size
      assert(subsetBytes === dependency.runtimeSubset.bytes, `qpdf runtime subset changed (${subsetBytes} bytes)`)
      const qpdf = join(bin, 'qpdf.exe')
      assert(run(qpdf, ['--version'], 'qpdf version').includes(`qpdf version ${dependency.version}`), 'qpdf executable version mismatch')
      assert(run(qpdf, ['--show-crypto'], 'qpdf crypto').toLowerCase().includes('openssl'), 'qpdf expected OpenSSL crypto provider is missing')
    }
  }
}

async function assertIntegrationBoundary(manifest) {
  const roots = [join(root, 'src-tauri', 'Cargo.toml'), join(root, 'src-tauri', 'src'), join(root, 'src'), join(root, 'src-tauri', 'resources')]
  const blockedNeedles = manifest.dependencies.filter(item => !item.integrationAllowed).flatMap((item) => [item.id, item.artifact.fileName])
  async function inspect(path) {
    const info = await stat(path)
    if (info.isDirectory()) {
      for (const entry of await readdir(path)) await inspect(join(path, entry))
      return
    }
    if (info.size > 4 * 1024 * 1024) return
    const source = await readFile(path).catch(() => null)
    if (!source) return
    const text = source.toString('utf8').toLowerCase()
    const match = blockedNeedles.find((needle) => text.includes(needle.toLowerCase()))
    assert(!match, `media engine appeared before approval: ${match} in ${resolve(path)}`)
  }
  for (const path of roots) await inspect(path)

  const cargo = await readFile(join(root, 'src-tauri', 'Cargo.toml'), 'utf8')
  const exact = [
    'libcaesium = { version = "=0.21.0", default-features = false, features = ["jpg", "webp"] }',
    'oxipng = { version = "=10.2.0", default-features = false, features = ["parallel", "zopfli"] }',
    'image = { version = "=0.25.10", default-features = false, features = ["jpeg", "png", "webp"] }',
    'img-parts = { version = "=0.4.0", default-features = false, features = ["std"] }',
  ]
  for (const declaration of exact) assert(cargo.includes(declaration), `approved image runtime declaration drifted: ${declaration}`)
  const lock = (await readFile(join(root, 'src-tauri', 'Cargo.lock'), 'utf8')).replace(/\r\n?/g, '\n')
  for (const [name, version] of [['libcaesium', '0.21.0'], ['oxipng', '10.2.0'], ['image', '0.25.10'], ['img-parts', '0.4.0']]) {
    assert(lock.includes(`name = "${name}"\nversion = "${version}"`), `${name}: approved product lock entry is missing`)
  }
  assert(!lock.includes('name = "gifski"') && !lock.includes('name = "imagequant"'), 'forbidden GIF/PNG dependency entered the product lockfile')
}

const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
validateManifest(manifest)

// Negative controls prove the gate rejects missing identity, license, HTTPS, unapproved runtime admission, and forbidden features.
for (const mutation of [
  (copy) => { delete copy.dependencies[0].artifact.sha256 },
  (copy) => { delete copy.dependencies[1].license.expression },
  (copy) => { copy.dependencies.find(item => item.id === 'ffmpeg').artifact.url = 'http://ffmpeg.org/unsafe' },
  (copy) => { copy.dependencies.find(item => item.id === 'qpdf').integrationAllowed = true },
  (copy) => { copy.dependencies.find(item => item.id === 'libcaesium').features.allowed.push('gif') },
]) {
  const copy = structuredClone(manifest)
  mutation(copy)
  let rejected = false
  try { validateManifest(copy) } catch { rejected = true }
  assert(rejected, 'negative dependency-gate control was unexpectedly accepted')
}

await assertIntegrationBoundary(manifest)
if (real) await verifyReal(manifest)
console.log(`Media dependency gate passed (${manifest.dependencies.length} locked dependencies; real=${real}).`)

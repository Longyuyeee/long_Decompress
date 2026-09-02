import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), '..')

const readJson = relativePath =>
  JSON.parse(readFileSync(join(projectRoot, relativePath), 'utf8'))

const readText = relativePath =>
  readFileSync(join(projectRoot, relativePath), 'utf8')

const readCargoManifestVersion = (relativePath, packageName) => {
  const manifest = readText(relativePath)
  const packageBlock =
    manifest.match(/^\[package\]\r?\n([\s\S]*?)(?=^\[|(?![\s\S]))/m)?.[1] ?? ''
  const name = packageBlock.match(/^name\s*=\s*"([^"]+)"/m)?.[1]
  const version = packageBlock.match(/^version\s*=\s*"([^"]+)"/m)?.[1]
  if (name !== packageName || !version) {
    throw new Error(`Unable to read ${packageName} version from ${relativePath}.`)
  }
  return version
}

const readCargoLockVersion = (relativePath, packageName) => {
  const lockfile = readText(relativePath)
  const escapedName = packageName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const match = lockfile.match(
    new RegExp(`\\[\\[package\\]\\]\\s+name = "${escapedName}"\\s+version = "([^"]+)"`, 'm'),
  )
  if (!match) throw new Error(`Unable to read ${packageName} version from ${relativePath}.`)
  return match[1]
}

const parseArguments = argv => {
  const options = { expected: '', tag: '' }
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index]
    if (argument === '--expected' || argument === '--tag') {
      const value = argv[index + 1]
      if (!value || value.startsWith('--')) throw new Error(`${argument} requires a value.`)
      options[argument.slice(2)] = value
      index += 1
      continue
    }
    throw new Error(`Unknown argument: ${argument}`)
  }
  return options
}

export const collectReleaseIdentity = () => {
  const packageJson = readJson('package.json')
  const packageLock = readJson('package-lock.json')
  const tauriConfig = readJson('src-tauri/tauri.conf.json')

  return {
    packageJson: packageJson.version,
    packageLock: packageLock.version,
    packageLockRoot: packageLock.packages?.['']?.version,
    tauriConfig: tauriConfig.package?.version,
    tauriCargo: readCargoManifestVersion('src-tauri/Cargo.toml', 'long-compress-assistant'),
    tauriCargoLock: readCargoLockVersion('src-tauri/Cargo.lock', 'long-compress-assistant'),
    shellCargo: readCargoManifestVersion(
      'src-tauri/shell-extension/Cargo.toml',
      'long-compress-shell-extension',
    ),
    shellCargoLock: readCargoLockVersion(
      'src-tauri/shell-extension/Cargo.lock',
      'long-compress-shell-extension',
    ),
  }
}

export const verifyReleaseIdentity = ({ expected = '', tag = '' } = {}) => {
  const tauriConfig = readJson('src-tauri/tauri.conf.json')
  const mainWindow = tauriConfig.tauri?.windows?.[0]
  if (mainWindow?.decorations !== false) {
    throw new Error('The main window must disable native decorations and use the in-app title bar.')
  }
  const protocol = tauriConfig.tauri?.allowlist?.protocol
  if (protocol?.asset !== true || !Array.isArray(protocol.assetScope) || protocol.assetScope.length !== 0) {
    throw new Error('Image asset protocol must be enabled with an empty default scope.')
  }
  const imageSources = tauriConfig.tauri?.security?.csp
    ?.match(/(?:^|;)\s*img-src\s+([^;]+)/)?.[1]
    ?.trim()
    ?.split(/\s+/) ?? []
  for (const source of ['asset:', 'https://asset.localhost']) {
    if (!imageSources.includes(source)) {
      throw new Error(`Tauri image CSP is missing the required local asset source: ${source}`)
    }
  }

  const identity = collectReleaseIdentity()
  const entries = Object.entries(identity)
  const versions = new Set(entries.map(([, version]) => version))
  if (versions.size !== 1 || entries.some(([, version]) => !version)) {
    throw new Error(
      `Release version sources disagree: ${entries
        .map(([source, version]) => `${source}=${version ?? '<missing>'}`)
        .join(', ')}`,
    )
  }

  const [version] = versions
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$/.test(version)) {
    throw new Error(`Application version is not a supported semantic version: ${version}`)
  }
  if (expected && version !== expected) {
    throw new Error(`Application version ${version} does not match expected version ${expected}.`)
  }
  if (tag && tag !== `v${version}`) {
    throw new Error(`Tag ${tag} does not match application version v${version}.`)
  }

  const versionSuffix = version.replace(/[^0-9A-Za-z]/g, '_')
  const resourceDirectory = join(projectRoot, 'src-tauri', 'resources')
  const expectedShellDll = `long_compress_shell_extension_${versionSuffix}.dll`
  if (!existsSync(join(resourceDirectory, expectedShellDll))) {
    throw new Error(`Versioned shell extension is missing: src-tauri/resources/${expectedShellDll}`)
  }
  const stagedShellDlls = readdirSync(resourceDirectory).filter(name =>
    /^long_compress_shell_extension_.*\.dll$/i.test(name),
  )
  if (stagedShellDlls.length !== 1 || stagedShellDlls[0] !== expectedShellDll) {
    throw new Error(
      `Shell-extension resources must contain only ${expectedShellDll}; found: ${
        stagedShellDlls.join(', ') || '<none>'
      }`,
    )
  }

  return { version, expectedShellDll, sources: identity }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    await import('./check-media-dependencies.mjs')
    const { checkMediaMetrics } = await import('./check-media-metrics.mjs')
    await checkMediaMetrics()
    const { checkMediaReleaseGates } = await import('./check-media-release-gates.mjs')
    await checkMediaReleaseGates()
    const { checkImageBaseline } = await import('./check-image-baseline.mjs')
    await checkImageBaseline()
    const result = verifyReleaseIdentity(parseArguments(process.argv.slice(2)))
    process.stdout.write(
      `Release identity verified: v${result.version} (${result.expectedShellDll})\n`,
    )
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}

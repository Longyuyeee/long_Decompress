import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { createHash, randomBytes } from 'node:crypto'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { deflateSync, zstdCompressSync } from 'node:zlib'
import { Builder, By, Capabilities, Key } from 'selenium-webdriver'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const executableSuffix = process.platform === 'win32' ? '.exe' : ''
const tauriConfig = JSON.parse(
  readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'),
)
const mediaFixtureManifest = JSON.parse(
  readFileSync(path.join(root, 'tests', 'fixtures', 'media', 'manifest.json'), 'utf8'),
)
const packagedApplication = path.join(
  root,
  'src-tauri',
  'target',
  'release',
  `${tauriConfig.package.productName}${executableSuffix}`,
)
const cargoApplication = path.join(
  root,
  'src-tauri',
  'target',
  'release',
  `long-compress-assistant${executableSuffix}`,
)
const application =
  process.env.TAURI_APP_BINARY ||
  (existsSync(cargoApplication) ? cargoApplication : packagedApplication)
const tauriDriver =
  process.env.TAURI_DRIVER_PATH ||
  path.join(homedir(), '.cargo', 'bin', `tauri-driver${executableSuffix}`)
const cachedEdgeDriver = path.join(root, 'test-results', 'edge-driver-b02', `msedgedriver${executableSuffix}`)
const edgeDriver =
  process.env.EDGE_DRIVER_PATH ||
  (existsSync(cachedEdgeDriver) ? cachedEdgeDriver : undefined)
const webdriverPort = Number.parseInt(process.env.LONG_DECOMPRESS_WEBDRIVER_PORT || '4723', 10)
const nativeWebdriverPort = Number.parseInt(process.env.LONG_DECOMPRESS_NATIVE_WEBDRIVER_PORT || '4724', 10)
const webdriverUrl = `http://127.0.0.1:${webdriverPort}/`
const artifactDirectory = path.join(root, 'test-results', 'desktop-e2e')
const e2eInstanceId =
  process.env.LONG_DECOMPRESS_E2E_INSTANCE_ID || randomBytes(12).toString('hex')
const e2eDataDirectory =
  process.env.LONG_DECOMPRESS_E2E_DATA_DIR ||
  path.join(root, 'test-results', `desktop-e2e-data-${e2eInstanceId}`)
let desktopSessionIndex = 0
let webviewUserDataDirectory = path.join(e2eDataDirectory, `webview2-session-${desktopSessionIndex}`)
const bundledSevenZip = path.join(root, 'src-tauri', 'resources', 'archive-engine', '7z.exe')
const productFfmpeg = path.join(root, 'src-tauri', 'resources', 'video-engine', 'ffmpeg.exe')
const productFfprobe = path.join(root, 'src-tauri', 'resources', 'video-engine', 'ffprobe.exe')
const qemuImg =
  process.env.QEMU_IMG_PATH ||
  path.join(root, 'test-results', 'qemu-img-tool', 'root', 'qemu-img.exe')
const wslFsToolRoot =
  process.env.WSL_FS_TOOL_ROOT ||
  path.join(root, 'test-results', 'wsl-fs-tools', 'root')
const wix3ToolRoot =
  process.env.WIX3_TOOL_ROOT ||
  path.join(root, 'test-results', 'wix3-tool', 'root')
const apfsToolRoot =
  process.env.APFS_TOOL_ROOT ||
  path.join(root, 'test-results', 'apfs-tool')
const ovmfFirmware =
  process.env.OVMF_FIRMWARE_PATH ||
  path.join(root, 'test-results', 'ovmf-fixture', 'root', 'usr', 'share', 'OVMF', 'OVMF_CODE_4M.fd')
const externalFixtureDirectory =
  process.env.LONG_DECOMPRESS_EXTERNAL_FIXTURE_DIR ||
  path.join(root, 'test-results', 'external-archive-fixtures')
const hfsxFixture =
  process.env.HFSX_FIXTURE_PATH ||
  path.join(root, 'test-results', 'hfsx-fixture', 'payload.hfsx')
const hfsFixture =
  process.env.HFS_FIXTURE_PATH ||
  path.join(root, 'test-results', 'hfsx-fixture', 'payload.hfs')
const nsisFixture =
  process.env.NSIS_FIXTURE_PATH ||
  path.join(
    root,
    'src-tauri',
    'target',
    'release',
    'bundle',
    'nsis',
    `${tauriConfig.package.productName}_${tauriConfig.package.version}_x64-setup.exe`,
  )
const requireFullFormatMatrix =
  process.argv.includes('--require-full-format-matrix') ||
  process.env.LONG_DECOMPRESS_REQUIRE_FULL_FORMAT_MATRIX === '1'
const watchFolderLifecycleOnly = process.argv.includes('--watch-folder-lifecycle-only')
const resourcePreflightOnly = process.argv.includes('--resource-preflight-only')
const smartAnalysisOnly = process.argv.includes('--smart-analysis-only')
const archiveBrowserOnly = process.argv.includes('--archive-browser-only')
const fileManagerOnly = process.argv.includes('--file-manager-only')
const markOfWebOnly = process.argv.includes('--mark-of-web-only')
const compressionVerificationOnly = process.argv.includes('--compression-verification-only')
const archiveFlowOnly = process.argv.includes('--archive-flow-only')
const zipTelemetryOnly = process.argv.includes('--zip-telemetry-only')
const historyOnly = process.argv.includes('--history-only')
const vaultUsageOnly = process.argv.includes('--vault-usage-only')
const encryptedRarOnly = process.argv.includes('--encrypted-rar-only')
const hfsxOnly = process.argv.includes('--hfsx-only')
const tarTelemetryOnly = process.argv.includes('--tar-telemetry-only')
const responsiveLayoutOnly = process.argv.includes('--responsive-layout-only')
const imageWorkspaceOnly = process.argv.includes('--image-workspace-only')
const imageBatchOnly = process.argv.includes('--image-batch-only')
const imagePickerManualOnly = process.argv.includes('--image-picker-manual-only')
const videoWorkspaceOnly = process.argv.includes('--video-workspace-only')
const pdfWorkspaceOnly = process.argv.includes('--pdf-workspace-only')
const autoStartOnly = process.argv.includes('--auto-start-only')
const missingFullFormatCapabilities = new Set()
const autoStartRegistryKey = 'HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run'
const autoStartValueName = 'Long解压'

if (videoWorkspaceOnly && path.resolve(application) === path.resolve(cargoApplication)) {
  // A standalone cargo binary resolves Tauri resources beside the executable,
  // while NSIS/updater bundles preserve the configured resources directory.
  // Mirror the bundle payload for this focused real-desktop gate; production
  // validation still checks every copied byte before launching ffprobe.
  cpSync(
    path.join(root, 'src-tauri', 'resources', 'video-engine'),
    path.join(path.dirname(cargoApplication), 'video-engine'),
    { recursive: true, force: true },
  )
}

if (pdfWorkspaceOnly && path.resolve(application) === path.resolve(cargoApplication)) {
  cpSync(
    path.join(root, 'src-tauri', 'resources', 'pdf-engine'),
    path.join(path.dirname(cargoApplication), 'pdf-engine'),
    { recursive: true, force: true },
  )
}

function recordMissingFullFormatCapability(capability, preparation) {
  missingFullFormatCapabilities.add(`${capability} — ${preparation}`)
  console.log(`[desktop-e2e] ${capability} unavailable; prepare with: ${preparation}`)
}

function assertFullFormatMatrixReady() {
  if (!requireFullFormatMatrix || missingFullFormatCapabilities.size === 0) return
  throw new Error(
    `Full-format matrix is incomplete:\n- ${[...missingFullFormatCapabilities].join('\n- ')}`,
  )
}

if (process.platform !== 'win32') {
  throw new Error('The real desktop smoke test currently targets Windows WebView2.')
}
for (const [label, target] of [
  ['Tauri application', application],
  ['tauri-driver', tauriDriver],
  ['Microsoft EdgeDriver', edgeDriver],
]) {
  if (!target || !existsSync(target)) {
    throw new Error(`${label} was not found: ${target || '<unset>'}`)
  }
}

let driver
let tauriDriverProcess
let devToolsPortMirror
let driverOutput = ''
let fixtureDirectory
let completedSuccessfully = false
let autoStartRegistryOwnedByTest = false

function appendDriverOutput(chunk) {
  driverOutput = `${driverOutput}${chunk}`.slice(-32_768)
  process.stdout.write(chunk)
}

async function waitForWebDriver(timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    if (tauriDriverProcess.exitCode !== null) {
      throw new Error(`tauri-driver exited early with code ${tauriDriverProcess.exitCode}`)
    }
    try {
      const response = await fetch(`${webdriverUrl}status`)
      if (response.ok) return
    } catch {
      // The driver has not opened its socket yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error('Timed out waiting for tauri-driver to become ready.')
}

function terminateProcessTree(processId) {
  if (!processId) return
  spawnSync('taskkill.exe', ['/pid', String(processId), '/t', '/f'], {
    stdio: 'ignore',
    windowsHide: true,
  })
}

function desktopApplicationProcessIds() {
  const escapedApplication = application.replaceAll("'", "''")
  const result = spawnSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-Command',
      `Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escapedApplication}' } | ForEach-Object { $_.ProcessId }`,
    ],
    { encoding: 'utf8', windowsHide: true },
  )
  assert.ifError(result.error)
  assert.equal(result.status, 0, `failed to inspect the desktop application process: ${result.stderr}`)
  return result.stdout
    .split(/\r?\n/u)
    .map(value => Number.parseInt(value.trim(), 10))
    .filter(Number.isInteger)
}

async function waitForStandaloneFileContent(filePath, expectedContent, timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    try {
      if (readFileSync(filePath, 'utf8') === expectedContent) return
    } catch {
      // The standalone process has not written its probe yet.
    }
    await new Promise(resolve => setTimeout(resolve, 100))
  }
  throw new Error(`Timed out waiting for ${filePath} to contain ${expectedContent}`)
}

function readAutoStartRegistryValue() {
  const result = spawnSync(
    'reg.exe',
    ['query', autoStartRegistryKey, '/v', autoStartValueName],
    { encoding: 'utf8', windowsHide: true },
  )
  if (result.status === 1) return null
  assert.ifError(result.error)
  assert.equal(result.status, 0, result.stderr || result.stdout)
  // reg.exe follows the active Windows console code page, so the non-ASCII
  // value name may be mojibake when Node decodes stdout. REG_SZ is stable.
  const match = result.stdout.match(/^\s+\S+\s+REG_SZ\s+(.+)$/mu)
  assert.ok(match, `unable to parse the auto-start value: ${result.stdout}`)
  return match[1].trim()
}

function removeAutoStartRegistryValue() {
  const result = spawnSync(
    'reg.exe',
    ['delete', autoStartRegistryKey, '/v', autoStartValueName, '/f'],
    { encoding: 'utf8', windowsHide: true },
  )
  assert.ifError(result.error)
  assert.ok([0, 1].includes(result.status), result.stderr || result.stdout)
}

async function startTauriDriver() {
  tauriDriverProcess = spawn(
    tauriDriver,
    ['--port', String(webdriverPort), '--native-port', String(nativeWebdriverPort), '--native-driver', edgeDriver],
    {
      cwd: root,
      env: {
        ...process.env,
        LONG_DECOMPRESS_E2E_DATA_DIR: e2eDataDirectory,
        LONG_DECOMPRESS_E2E_INSTANCE_ID: e2eInstanceId,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    },
  )
  tauriDriverProcess.stdout.on('data', appendDriverOutput)
  tauriDriverProcess.stderr.on('data', appendDriverOutput)
  await waitForWebDriver()
}

function forwardContextAction(flag, files) {
  const result = spawnSync(application, [flag, ...files], {
    cwd: root,
    env: {
      ...process.env,
      LONG_DECOMPRESS_E2E_DATA_DIR: e2eDataDirectory,
      LONG_DECOMPRESS_E2E_INSTANCE_ID: e2eInstanceId,
    },
    encoding: 'utf8',
    timeout: 30_000,
    windowsHide: true,
  })
  assert.ifError(result.error)
  assert.equal(
    result.status,
    0,
    `the second application instance failed to forward ${flag}: ${result.stderr || result.stdout}`,
  )
}

function runFixtureCommand(command, args, label, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd || fixtureDirectory,
    env: options.env || process.env,
    encoding: 'utf8',
    timeout: options.timeout || 30_000,
    windowsHide: true,
  })
  assert.ifError(result.error)
  assert.equal(
    result.status,
    0,
    `${label} fixture creation failed: ${result.stderr || result.stdout}`,
  )
}

function createZipCompatibleFixture(outputPath, sourcePath) {
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-tzip', '-y', outputPath, sourcePath],
    path.extname(outputPath).slice(1).toUpperCase(),
  )
}

function createArFixture(outputPath, entryName, payload) {
  createArEntries(outputPath, [[entryName, payload]])
}

function createArEntries(outputPath, entries) {
  const chunks = [Buffer.from('!<arch>\n', 'ascii')]
  for (const [entryName, payload] of entries) {
    assert.ok(entryName.length <= 15, `AR fixture entry name is too long: ${entryName}`)
    const identifier = `${entryName}/`.padEnd(16, ' ')
    const timestamp = '0'.padEnd(12, ' ')
    const owner = '0'.padEnd(6, ' ')
    const group = '0'.padEnd(6, ' ')
    const mode = '100644'.padEnd(8, ' ')
    const size = String(payload.length).padEnd(10, ' ')
    chunks.push(
      Buffer.from(`${identifier}${timestamp}${owner}${group}${mode}${size}\x60\n`, 'ascii'),
      payload,
    )
    if (payload.length % 2 !== 0) chunks.push(Buffer.from('\n'))
  }
  writeFileSync(outputPath, Buffer.concat(chunks))
}

function createXarFixture(outputPath, entryName, payload) {
  const toc = Buffer.from(
    `<?xml version="1.0" encoding="UTF-8"?>` +
      `<xar><toc><creation-time>1970-01-01T00:00:00Z</creation-time>` +
      `<file id="1"><name>${entryName}</name><type>file</type><mode>0644</mode>` +
      `<data><length>${payload.length}</length><offset>0</offset><size>${payload.length}</size>` +
      `<encoding style="application/octet-stream"/></data></file></toc></xar>`,
    'utf8',
  )
  const compressedToc = deflateSync(toc)
  const header = Buffer.alloc(28)
  header.write('xar!', 0, 'ascii')
  header.writeUInt16BE(header.length, 4)
  header.writeUInt16BE(1, 6)
  header.writeBigUInt64BE(BigInt(compressedToc.length), 8)
  header.writeBigUInt64BE(BigInt(toc.length), 16)
  header.writeUInt32BE(0, 24)
  writeFileSync(outputPath, Buffer.concat([header, compressedToc, payload]))
}

function fileSha256(filePath) {
  return createHash('sha256').update(readFileSync(filePath)).digest('hex')
}

function toWslMountPath(windowsPath) {
  const resolved = path.resolve(windowsPath)
  const match = /^([a-zA-Z]):[\\/](.*)$/.exec(resolved)
  assert.ok(match, `cannot map path into WSL: ${windowsPath}`)
  return `/mnt/${match[1].toLowerCase()}/${match[2].replaceAll('\\', '/')}`
}

async function waitForStableFile(filePath, timeoutMs = 60_000) {
  let lastSize = -1
  let stablePolls = 0
  await driver.wait(() => {
    try {
      if (!existsSync(filePath)) return false
      const currentSize = statSync(filePath).size
      stablePolls = currentSize > 0 && currentSize === lastSize ? stablePolls + 1 : 0
      lastSize = currentSize
      return stablePolls >= 3
    } catch {
      return false
    }
  }, timeoutMs)
}

async function waitForFileContent(filePath, expectedContent, timeoutMs = 60_000) {
  await driver.wait(() => {
    try {
      return readFileSync(filePath, 'utf8') === expectedContent
    } catch {
      return false
    }
  }, timeoutMs)
}

async function waitForNonEmptyText(selector, timeoutMs = 30_000) {
  return driver.wait(async () => {
    try {
      const elements = await driver.findElements(By.css(selector))
      if (elements.length === 0) return false
      const text = (await elements[0].getAttribute('textContent')).trim()
      return text || false
    } catch {
      // Vue may replace the matched element while a route is rendering.
      return false
    }
  }, timeoutMs)
}

async function waitForNonEmptyFile(filePath, timeoutMs = 30_000) {
  await driver.wait(() => {
    try {
      return existsSync(filePath) && statSync(filePath).size > 0
    } catch {
      return false
    }
  }, timeoutMs)
}

async function callDesktopBridge(method, ...args) {
  const result = await driver.executeAsyncScript(
    (bridgeMethod, bridgeArgs, done) => {
      const bridge = window.__LONG_DECOMPRESS_DESKTOP_E2E__
      if (!bridge || typeof bridge[bridgeMethod] !== 'function') {
        done({ ok: false, error: `Desktop E2E bridge method is unavailable: ${bridgeMethod}` })
        return
      }
      Promise.resolve(bridge[bridgeMethod](...bridgeArgs))
        .then((value) => done({ ok: true, value }))
        .catch((error) => done({ ok: false, error: String(error) }))
    },
    method,
    args,
  )
  assert.equal(result?.ok, true, result?.error || `Desktop E2E bridge call failed: ${method}`)
  return result.value
}

async function callDesktopBridgeFailure(method, ...args) {
  const result = await driver.executeAsyncScript(
    (bridgeMethod, bridgeArgs, done) => {
      const bridge = window.__LONG_DECOMPRESS_DESKTOP_E2E__
      if (!bridge || typeof bridge[bridgeMethod] !== 'function') {
        done({ ok: false, error: `Desktop E2E bridge method is unavailable: ${bridgeMethod}` })
        return
      }
      Promise.resolve(bridge[bridgeMethod](...bridgeArgs))
        .then((value) => done({ ok: true, value }))
        .catch((error) => done({ ok: false, error: String(error) }))
    },
    method,
    args,
  )
  assert.equal(result?.ok, false, `${method} unexpectedly succeeded`)
  return result.error || ''
}

const normalizedDesktopPath = (value) => {
  const windowsPath = String(value).replaceAll('/', '\\')
  const portablePath = windowsPath.startsWith('\\\\?\\UNC\\')
    ? `\\\\${windowsPath.slice(8)}`
    : windowsPath.startsWith('\\\\?\\')
      ? windowsPath.slice(4)
      : windowsPath
  return path.resolve(portablePath).replaceAll('/', '\\').toLowerCase()
}

const findFileRecursively = (rootPath, fileName) => {
  if (!existsSync(rootPath)) return null
  for (const entry of readdirSync(rootPath, { withFileTypes: true })) {
    const target = path.join(rootPath, entry.name)
    if (entry.isDirectory()) {
      const found = findFileRecursively(target, fileName)
      if (found) return found
    } else if (entry.isFile() && entry.name === fileName) {
      return target
    }
  }
  return null
}

const createLargeMetadataTar = (targetPath, entryCount) => {
  const archive = Buffer.alloc(entryCount * 512 + 1024)
  const writeOctal = (header, offset, length, value) => {
    header.write(`${value.toString(8).padStart(length - 1, '0')}\0`, offset, length, 'ascii')
  }
  for (let index = 0; index < entryCount; index += 1) {
    const header = archive.subarray(index * 512, (index + 1) * 512)
    header.write(`bulk/entry-${String(index).padStart(6, '0')}.txt`, 0, 100, 'utf8')
    writeOctal(header, 100, 8, 0o644)
    writeOctal(header, 108, 8, 0)
    writeOctal(header, 116, 8, 0)
    writeOctal(header, 124, 12, 0)
    writeOctal(header, 136, 12, 1_700_000_000)
    header.fill(0x20, 148, 156)
    header.write('0', 156, 1, 'ascii')
    header.write('ustar\0', 257, 6, 'ascii')
    header.write('00', 263, 2, 'ascii')
    const checksum = header.reduce((sum, value) => sum + value, 0)
    header.write(`${checksum.toString(8).padStart(6, '0')}\0 `, 148, 8, 'ascii')
  }
  writeFileSync(targetPath, archive)
}

async function runArchiveBrowserDesktopGate() {
  console.log('[desktop-e2e] verifying archive-browser UI with long paths, passwords, and exact extraction')
  await callDesktopBridge('clearTasks')
  const browserFixtureRoot = path.join(fixtureDirectory, 'archive-browser')
  const sourceRoot = path.join(browserFixtureRoot, 'sources')
  const archiveRoot = path.join(browserFixtureRoot, 'archives')
  mkdirSync(sourceRoot, { recursive: true })
  mkdirSync(archiveRoot, { recursive: true })

  const longSegments = Array.from({ length: 8 }, (_, index) => `中文长目录-${index + 1}-${'层级'.repeat(3)}`)
  const zipRootName = '资料集合'
  const zipKeepRelative = path.join(zipRootName, ...longSegments, '保留文件.txt')
  const zipSkipRelative = path.join(zipRootName, '不应解压.txt')
  const zipKeepSource = path.join(sourceRoot, zipKeepRelative)
  const zipSkipSource = path.join(sourceRoot, zipSkipRelative)
  const zipKeepPayload = 'Long解压 archive browser 中文长路径 selective payload\n'
  mkdirSync(path.dirname(zipKeepSource), { recursive: true })
  writeFileSync(zipKeepSource, zipKeepPayload, 'utf8')
  writeFileSync(zipSkipSource, 'must remain inside the archive', 'utf8')
  const browserZip = path.join(archiveRoot, '中文长路径.zip')
  runFixtureCommand(bundledSevenZip, ['a', '-tzip', '-y', browserZip, zipRootName], 'archive browser ZIP', {
    cwd: sourceRoot,
  })

  const passwordRootName = '密码资料'
  const passwordKeepRelative = path.join(passwordRootName, '安全目录', '只解压这一项.txt')
  const passwordSkipRelative = path.join(passwordRootName, '不要解压.txt')
  const passwordKeepSource = path.join(sourceRoot, passwordKeepRelative)
  mkdirSync(path.dirname(passwordKeepSource), { recursive: true })
  writeFileSync(passwordKeepSource, 'password 7z selected payload', 'utf8')
  writeFileSync(path.join(sourceRoot, passwordSkipRelative), 'password 7z excluded payload', 'utf8')
  const browser7z = path.join(archiveRoot, '密码与中文.7z')
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-t7z', '-pdesktop-browser-secret', '-mhe=on', '-y', browser7z, passwordRootName],
    'archive browser encrypted 7Z',
    { cwd: sourceRoot },
  )
  const defaultOpenRootName = '默认应用安全打开'
  const defaultOpenRoot = path.join(sourceRoot, defaultOpenRootName)
  mkdirSync(defaultOpenRoot, { recursive: true })
  const openTxtPayload = Buffer.from('Long解压 A-03 Windows default TXT application\n', 'utf8')
  const openPngPayload = Buffer.from('89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d4944415408d763f8cfc0f01f00050001ff89993d1d0000000049454e44ae426082', 'hex')
  const openPdfPayload = Buffer.from('%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Count 0/Kids[]>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n', 'ascii')
  writeFileSync(path.join(defaultOpenRoot, '说明 文档.txt'), openTxtPayload)
  writeFileSync(path.join(defaultOpenRoot, '超长 日志.log'), Buffer.alloc(1024 * 1024 + 4096, 0x78))
  writeFileSync(path.join(defaultOpenRoot, '伪装 二进制.txt'), Buffer.from([0, 1, 2, 3, 0, 255, 8]))
  writeFileSync(path.join(defaultOpenRoot, '像素 图片.png'), openPngPayload)
  writeFileSync(path.join(defaultOpenRoot, '空白 文档.pdf'), openPdfPayload)
  writeFileSync(path.join(defaultOpenRoot, '禁止自动启动.cmd'), '@echo A-03-dangerous-content-must-not-run>"%TEMP%\\long-a03-danger.marker"\r\n', 'utf8')
  const defaultOpenZip = path.join(archiveRoot, '默认应用打开.zip')
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-tzip', '-y', defaultOpenZip, defaultOpenRootName],
    'archive default-open ZIP',
    { cwd: sourceRoot },
  )
  const defaultOpenZone = '[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://example.test/default-open.zip\r\n'
  writeFileSync(`${defaultOpenZip}:Zone.Identifier`, defaultOpenZone, 'utf8')

  const utf16PreviewName = 'UTF16 本地文本.txt'
  const utf16PreviewPayload = Buffer.concat([
    Buffer.from([0xff, 0xfe]),
    Buffer.from('Long解压 UTF-16 真实 TAR 文本\n第二行', 'utf16le'),
  ])
  writeFileSync(path.join(sourceRoot, utf16PreviewName), utf16PreviewPayload)
  const textPreviewTar = path.join(archiveRoot, '文本预览.tar')
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-ttar', '-y', textPreviewTar, utf16PreviewName],
    'archive text-preview TAR',
    { cwd: sourceRoot },
  )

  const nestedSource = path.join(sourceRoot, 'nested-workspace')
  mkdirSync(nestedSource, { recursive: true })
  const nestedLeafName = '最内层结果.txt'
  const nestedLeafPayload = 'Long解压 A-05 真实三层嵌套选择性解压\n'
  writeFileSync(path.join(nestedSource, nestedLeafName), nestedLeafPayload, 'utf8')
  writeFileSync(path.join(nestedSource, '第四层占位.txt'), 'must not be entered at depth four', 'utf8')
  const fourthArchiveName = '第四层.zip'
  runFixtureCommand(bundledSevenZip, ['a', '-tzip', '-y', fourthArchiveName, '第四层占位.txt'], 'fourth-level ZIP', { cwd: nestedSource })
  const nestedInnerName = '内层.zip'
  runFixtureCommand(bundledSevenZip, ['a', '-tzip', '-y', nestedInnerName, nestedLeafName, fourthArchiveName], 'nested inner ZIP', { cwd: nestedSource })
  const nestedMiddleName = '加密中层.7z'
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-t7z', '-pnested-middle-secret', '-mhe=on', '-y', nestedMiddleName, nestedInnerName],
    'encrypted nested middle 7Z',
    { cwd: nestedSource },
  )
  writeFileSync(path.join(nestedSource, '损坏内层.zip'), Buffer.from('504b030462726f6b656e', 'hex'))
  const nestedOuter = path.join(archiveRoot, '外层工作区.zip')
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-tzip', '-y', nestedOuter, nestedMiddleName, '损坏内层.zip'],
    'nested outer ZIP',
    { cwd: nestedSource },
  )

  const cancellableTar = path.join(archiveRoot, '大量目录项-取消读取.tar')
  createLargeMetadataTar(cancellableTar, 180_000)
  const zstdEntryName = '后端能力来源.zst'
  writeFileSync(path.join(sourceRoot, zstdEntryName), zstdCompressSync(Buffer.from('backend capability source', 'utf8')))
  const capabilityOuter = path.join(archiveRoot, '能力来源验证.zip')
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-tzip', '-y', capabilityOuter, zstdEntryName],
    'archive capability-source ZIP',
    { cwd: sourceRoot },
  )

  await (await waitForElement('[data-testid="nav-Decompress"]')).click()
  await driver.actions().keyDown(Key.CONTROL).keyDown(Key.SHIFT).sendKeys('s').keyUp(Key.SHIFT).keyUp(Key.CONTROL).perform()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  const specialLayout = await driver.executeScript(() => {
    const page = document.querySelector('.special-compression-view')
    const header = document.querySelector('.special-compression-header')
    const heading = document.querySelector('.special-compression-heading')
    const tabs = document.querySelector('[data-testid="special-compression-mode-switch"]')
    const workspace = document.querySelector('[data-testid="image-compression-workspace"]')
    const rect = element => {
      if (!element) return null
      const value = element.getBoundingClientRect()
      return { top: value.top, right: value.right, bottom: value.bottom, left: value.left, width: value.width, height: value.height }
    }
    return {
      page: page && { clientHeight: page.clientHeight, scrollHeight: page.scrollHeight, clientWidth: page.clientWidth, scrollWidth: page.scrollWidth },
      header: rect(header), heading: rect(heading), tabs: rect(tabs),
      workspace: workspace && { clientHeight: workspace.clientHeight, scrollHeight: workspace.scrollHeight },
      settingsVisible: Boolean(document.querySelector('[data-testid="image-compression-workspace"] .global-settings-card')),
    }
  })
  assert.ok(specialLayout.tabs.left > specialLayout.heading.left, `special-compression tabs must occupy the title row's right side: ${JSON.stringify(specialLayout)}`)
  assert.ok(specialLayout.tabs.top >= specialLayout.header.top - 1 && specialLayout.tabs.bottom <= specialLayout.header.bottom + 1, `special-compression tabs must stay in the title row: ${JSON.stringify(specialLayout)}`)
  assert.equal(specialLayout.settingsVisible, false, 'image batch settings must be collapsed before files are added')
  assert.ok(specialLayout.page.scrollHeight <= specialLayout.page.clientHeight + 1, `empty special-compression page must not scroll vertically: ${JSON.stringify(specialLayout)}`)
  assert.ok(specialLayout.page.scrollWidth <= specialLayout.page.clientWidth + 1, `special-compression page must not scroll horizontally: ${JSON.stringify(specialLayout)}`)
  assert.ok(specialLayout.workspace.scrollHeight <= specialLayout.workspace.clientHeight + 1, `empty image workspace must not need vertical scrolling: ${JSON.stringify(specialLayout)}`)
  await (await waitForElement('[data-testid="compression-mode-video"]')).click()
  const videoEmptyLayout = await driver.executeScript(() => {
    const workspace = document.querySelector('[data-testid="video-compression-workspace"]')
    return { clientHeight: workspace?.clientHeight, scrollHeight: workspace?.scrollHeight, settingsVisible: Boolean(workspace?.querySelector('.global-settings-card')) }
  })
  assert.equal(videoEmptyLayout.settingsVisible, false, 'video batch settings must be collapsed before files are added')
  assert.ok(videoEmptyLayout.scrollHeight <= videoEmptyLayout.clientHeight + 1, `empty video workspace must not need vertical scrolling: ${JSON.stringify(videoEmptyLayout)}`)
  await (await waitForElement('[data-testid="compression-mode-pdf"]')).click()
  const pdfEmptyLayout = await driver.executeScript(() => {
    const workspace = document.querySelector('[data-testid="pdf-compression-workspace"]')
    return { clientHeight: workspace?.clientHeight, scrollHeight: workspace?.scrollHeight }
  })
  assert.ok(pdfEmptyLayout.scrollHeight <= pdfEmptyLayout.clientHeight + 1, `empty PDF workspace must not need vertical scrolling: ${JSON.stringify(pdfEmptyLayout)}`)
  writeFileSync(path.join(artifactDirectory, 'special-compression-empty-layout-v1.2.0.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))

  await (await waitForElement('[data-testid="nav-ArchiveBrowser"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/browser'), 30_000)
  const fileManager = await waitForElement('[data-testid="dual-pane-file-manager"]')
  console.log('[desktop-e2e] dual-pane file-manager UI is visible')
  assert.equal((await fileManager.findElements(By.css('.file-pane'))).length, 2, 'the browser tab must open as two real file panes')
  assert.doesNotMatch(await fileManager.getText(), /把压缩包拖到这里/, 'archive drop must not be the default browser experience')
  assert.equal((await fileManager.findElements(By.css('[data-testid^="file-manager-selection-mode-"]'))).length, 2, 'both panes must expose a direct multi-select toggle')
  assert.equal((await fileManager.findElements(By.css('[data-testid^="file-manager-breadcrumbs-"]'))).length, 2, 'both panes must expose clickable path breadcrumbs')
  const leftSelectionToggle = await waitForElement('[data-testid="file-manager-selection-mode-left"]')
  await leftSelectionToggle.click()
  assert.equal(await leftSelectionToggle.getAttribute('aria-pressed'), 'true', 'multi-select mode must open explicitly')
  await leftSelectionToggle.click()
  assert.equal(await leftSelectionToggle.getAttribute('aria-pressed'), 'false', 'multi-select mode must exit explicitly')
  await driver.executeScript(() => {
    document.querySelectorAll('.file-pane')[0]?.querySelector('.file-list')?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 280, clientY: 420 }))
  })
  const openSameFolder = await waitForElement('[data-testid="file-manager-open-same-other"]')
  assert.match(await openSameFolder.getText(), /另一栏打开相同文件夹/)
  await openSameFolder.click()
  await driver.wait(async () => {
    const paths = await driver.findElements(By.css('.path-strip'))
    return paths.length === 2 && await paths[0].getAttribute('title') === await paths[1].getAttribute('title')
  }, 15_000)
  const moveDirection = async (paneIndex, expectedClass) => {
    await driver.executeScript(index => {
      document.querySelectorAll('.file-pane')[index]?.querySelector('.file-row')?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: index ? 920 : 320, clientY: 460 }))
    }, paneIndex)
    const buttons = await driver.findElements(By.css('.file-context button'))
    for (const button of buttons) {
      if ((await button.getText()).includes('移动到另一栏')) {
        const iconClass = await (await button.findElement(By.css('i'))).getAttribute('class')
        assert.match(iconClass, new RegExp(expectedClass), `pane ${paneIndex} move icon must point toward the other pane`)
        return
      }
    }
    assert.fail(`pane ${paneIndex} context menu did not expose move-to-other-pane`)
  }
  await moveDirection(0, 'pi-arrow-right')
  await moveDirection(1, 'pi-arrow-left')
  const fileManagerDimensions = await driver.executeScript(() => {
    const page = document.querySelector('[data-testid="dual-pane-file-manager"]')
    return page && { clientWidth: page.clientWidth, scrollWidth: page.scrollWidth, clientHeight: page.clientHeight, scrollHeight: page.scrollHeight }
  })
  assert.ok(fileManagerDimensions.scrollWidth <= fileManagerDimensions.clientWidth + 1, `dual-pane manager must not scroll horizontally: ${JSON.stringify(fileManagerDimensions)}`)
  writeFileSync(path.join(artifactDirectory, 'dual-pane-convenience-v1.2.0.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))

  const fileManagerRoot = path.join(browserFixtureRoot, 'file-manager-real')
  const fileManagerSource = path.join(fileManagerRoot, 'source', 'album')
  const fileManagerCopyDestination = path.join(fileManagerRoot, 'copy-target')
  const fileManagerMoveDestination = path.join(fileManagerRoot, 'move-target')
  mkdirSync(path.join(fileManagerSource, 'nested'), { recursive: true })
  mkdirSync(fileManagerCopyDestination, { recursive: true })
  mkdirSync(fileManagerMoveDestination, { recursive: true })
  writeFileSync(path.join(fileManagerSource, 'a.txt'), 'alpha')
  writeFileSync(path.join(fileManagerSource, 'nested', 'b.bin'), Buffer.from([0, 1, 2, 3]))
  console.log('[desktop-e2e] invoking real copy/move/properties commands')
  const fileManagerResult = await callDesktopBridge(
    'fileManagerCopyMove',
    fileManagerSource,
    fileManagerCopyDestination,
    fileManagerMoveDestination,
  )
  console.log('[desktop-e2e] real copy/move/properties commands completed')
  assert.deepEqual(
    [fileManagerResult.copy.processed, fileManagerResult.copy.files, fileManagerResult.copy.directories, fileManagerResult.copy.bytes],
    [1, 2, 2, 9],
    'real desktop copy must report the exact expected tree totals',
  )
  assert.deepEqual(
    [fileManagerResult.move.processed, fileManagerResult.move.files, fileManagerResult.move.directories, fileManagerResult.move.bytes],
    [1, 2, 2, 9],
    'real desktop move must report the exact expected tree totals',
  )
  assert.equal(existsSync(path.join(fileManagerCopyDestination, 'album')), false, 'move must remove the copied source tree')
  assert.equal(readFileSync(path.join(fileManagerResult.finalPath, 'a.txt'), 'utf8'), 'alpha')
  assert.deepEqual(
    [fileManagerResult.properties.files, fileManagerResult.properties.directories, fileManagerResult.properties.bytes],
    [2, 2, 9],
    'real desktop properties must match the moved tree',
  )
  if (fileManagerOnly) return

  const openArchive = async (archivePath, outputPath, password, expectedText) => {
    await callDesktopBridge('queueDesktopDialogSelections', [archivePath, outputPath])
    const fileManagerChooser = await driver.findElements(By.css('[data-testid="file-manager-open-archive"]'))
    if (fileManagerChooser.length > 0) await fileManagerChooser[0].click()
    const passwordInput = await waitForElement('.browser-toolbar input[type="password"]')
    await driver.executeScript(
      "const input = arguments[0]; input.value = ''; input.dispatchEvent(new Event('input', { bubbles: true }));",
      passwordInput,
    )
    if (password) await passwordInput.sendKeys(password)
    await (await waitForElement('.browser-page > header .browser-primary')).click()
    await driver.wait(async () => {
      const pages = await driver.findElements(By.css('.browser-page'))
      return pages.length > 0 && (await pages[0].getText()).includes(expectedText)
    }, 30_000)
    const fields = await driver.findElements(By.css('.browser-toolbar .browser-field'))
    assert.equal(fields.length, 2, 'the archive chooser and password are the only top configuration fields')
    const outputTarget = await waitForElement('.browser-page > footer .output-target')
    await outputTarget.click()
    await driver.wait(async () => (await outputTarget.getText()).includes(outputPath), 10_000)
    const dimensions = await driver.executeScript(
      'const page = document.querySelector(\'.browser-page\'); return page ? { scrollWidth: page.scrollWidth, clientWidth: page.clientWidth } : null;',
    )
    assert.ok(dimensions)
    assert.ok(
      dimensions.scrollWidth <= dimensions.clientWidth + 1,
      `the archive-browser page must not scroll horizontally: ${JSON.stringify(dimensions)}`,
    )
  }

  await callDesktopBridge('queueDesktopDialogSelections', [cancellableTar])
  const initialFileManagerChooser = await driver.findElements(By.css('[data-testid="file-manager-open-archive"]'))
  if (initialFileManagerChooser.length > 0) await initialFileManagerChooser[0].click()
  else await (await waitForElement('.browser-page > header .browser-primary')).click()
  const cancelBrowse = await waitForElement('[data-testid="archive-browse-cancel"]')
  const cancellationStartedAt = Date.now()
  await cancelBrowse.click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="archive-browse-notice"]'))).length === 1, 5_000)
  const cancellationElapsedMs = Date.now() - cancellationStartedAt
  assert.ok(cancellationElapsedMs < 5_000, `real large TAR cancellation took ${cancellationElapsedMs} ms`)
  assert.match(await (await waitForElement('[data-testid="archive-browse-notice"]')).getText(), /已取消读取压缩包内容/)
  console.log(`[desktop-e2e] real 180000-entry TAR cancelled in ${cancellationElapsedMs} ms`)

  const capabilityOutput = path.join(browserFixtureRoot, 'capability-output')
  await openArchive(capabilityOuter, capabilityOutput, '', zstdEntryName)
  const zstdRow = await waitForElement(`[data-entry-path="${zstdEntryName}"]`)
  await driver.actions().contextClick(zstdRow).perform()
  const enterZstd = await waitForElement('[data-testid="archive-context-enter-nested"]')
  assert.equal(await enterZstd.isEnabled(), true, 'the UI must consume backend-reported zstd nested capability')
  await driver.actions().sendKeys(Key.ESCAPE).perform()
  console.log('[desktop-e2e] backend-reported zstd capability reached the real archive context menu')

  const verifyWorkspaceNavigation = async () => {
    const footer = await waitForElement('.browser-page > footer')
    assert.match(await footer.getText(), /已选择\s+2\s+\/\s+2/)
    const directoryRow = await waitForElement('[data-entry-path="资料集合/"]')
    await directoryRow.click()
    assert.match(await directoryRow.getAttribute('class'), /focused/)
    assert.match(await footer.getText(), /已选择\s+2\s+\/\s+2/, 'plain click must focus without changing selection')

    await driver.actions().contextClick(directoryRow).perform()
    const directoryMenu = await waitForElement('[data-testid="archive-context-menu"]')
    assert.match(await directoryMenu.getText(), /打开文件夹/)
    assert.match(await directoryMenu.getText(), /解压到当前输出目录/)
    assert.equal(
      (await directoryMenu.findElements(By.css('[data-testid="archive-context-preview"]'))).length,
      0,
      'a directory must not expose the internal image viewer action',
    )
    assert.match(await footer.getText(), /已选择\s+2\s+\/\s+2/, 'right-clicking a directory must preserve selection')
    await (await directoryMenu.findElement(By.css('[data-testid="archive-context-open"]'))).click()
    await driver.wait(async () => (await waitForElement('[data-testid="archive-breadcrumbs"]')).getText().then(text => text.includes('资料集合')), 10_000)
    console.log('[desktop-e2e] archive workspace opened a directory from its real context menu')
    await driver.actions().keyDown(Key.ALT).sendKeys(Key.ARROW_LEFT).keyUp(Key.ALT).perform()
    await driver.wait(async () => (await waitForElement('[data-testid="archive-breadcrumbs"]')).getText().then(text => !text.includes('资料集合')), 10_000)

    const directoryRowAfterContextOpen = await waitForElement('[data-entry-path="资料集合/"]')
    await driver.actions().doubleClick(directoryRowAfterContextOpen).perform()
    await driver.wait(async () => (await waitForElement('[data-testid="archive-breadcrumbs"]')).getText().then(text => text.includes('资料集合')), 10_000)
    console.log('[desktop-e2e] archive workspace entered a directory by double-click')
    await driver.actions().keyDown(Key.ALT).sendKeys(Key.ARROW_LEFT).keyUp(Key.ALT).perform()
    await driver.wait(async () => (await waitForElement('[data-testid="archive-breadcrumbs"]')).getText().then(text => !text.includes('资料集合')), 10_000)
    console.log('[desktop-e2e] archive workspace returned by Alt+Left')
    const forward = await waitForElement('[data-testid="archive-nav-forward"]')
    assert.equal(await forward.isEnabled(), true)
    await forward.click()
    await driver.wait(async () => (await waitForElement('[data-testid="archive-breadcrumbs"]')).getText().then(text => text.includes('资料集合')), 10_000)
    console.log('[desktop-e2e] archive workspace moved forward from visible navigation history')
    const up = await waitForElement('[data-testid="archive-nav-up"]')
    assert.equal(await up.isEnabled(), true)
    await driver.executeScript('arguments[0].click()', up)
    await driver.wait(async () => (await waitForElement('[data-testid="archive-breadcrumbs"]')).getText().then(text => !text.includes('资料集合')), 10_000)
    console.log('[desktop-e2e] archive workspace moved to the parent directory')
    await (await waitForElement('[data-testid="archive-nav-refresh"]')).click()
    await driver.wait(async () => (await driver.findElements(By.css('[data-entry-path="资料集合/"]'))).length === 1, 30_000)
    assert.match(await (await waitForElement('.browser-page > footer')).getText(), /已选择\s+2\s+\/\s+2/)
    console.log('[desktop-e2e] archive workspace refreshed metadata and preserved valid selection')
    writeFileSync(
      path.join(artifactDirectory, 'archive-browser-a01-workspace.png'),
      Buffer.from(await driver.takeScreenshot(), 'base64'),
    )
  }

  const extractOnly = async (query, expectedPath, expectedContent, excludedPath, useContextMenu = false) => {
    const search = await waitForElement('.browser-search input')
    await search.clear()
    await search.sendKeys('.txt')
    await driver.wait(async () => (await driver.findElements(By.css('.browser-row'))).length === 2, 10_000)
    await (await waitForElement('.browser-table-head .browser-checkbox')).click()
    assert.match(await (await waitForElement('.browser-page > footer')).getText(), /已选择\s+0\s+\//)
    await search.clear()
    await search.sendKeys(query)
    const rows = await driver.wait(async () => {
      const entries = await driver.findElements(By.css('.browser-row'))
      return entries.length === 1 ? entries : false
    }, 10_000)
    await (await rows[0].findElement(By.css('.browser-checkbox'))).click()
    assert.match(await (await waitForElement('.browser-page > footer')).getText(), /已选择\s+1\s+\//)
    if (useContextMenu) {
      await driver.actions().contextClick(rows[0]).perform()
      const menu = await waitForElement('[data-testid="archive-context-menu"]')
      assert.match(await menu.getText(), /解压到当前输出目录/)
      assert.match(await menu.getText(), /复制归档内路径/)
      assert.match(await menu.getText(), /显示详细信息/)
      assert.match(await menu.getText(), /使用默认应用打开/, 'A-03 default-application action must be visible')
      assert.doesNotMatch(await menu.getText(), /进入压缩包/, 'unimplemented nested archive navigation must not be advertised')
      writeFileSync(
        path.join(artifactDirectory, 'archive-browser-a02-context-menu.png'),
        Buffer.from(await driver.takeScreenshot(), 'base64'),
      )

      await (await menu.findElement(By.css('[data-testid="archive-context-copy-path"]'))).click()
      await driver.wait(async () => (await driver.findElements(By.css('[data-testid="archive-context-menu"]'))).length === 0, 10_000)
      const clipboardResult = spawnSync(
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          '[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-Clipboard -Raw',
        ],
        { encoding: 'utf8', windowsHide: true },
      )
      assert.equal(clipboardResult.status, 0, `Windows clipboard read failed: ${clipboardResult.stderr}`)
      const clipboardText = clipboardResult.stdout.trimEnd()
      assert.equal(
        clipboardText,
        zipKeepRelative.replaceAll('\\', '/'),
        `the real WebView clipboard must contain the archive path, got ${clipboardText}`,
      )

      await driver.actions().contextClick(rows[0]).perform()
      await (await waitForElement('[data-testid="archive-context-details"]')).click()
      const details = await waitForElement('[data-testid="archive-entry-details"]')
      assert.match(await details.getText(), new RegExp(query))
      const detailDimensions = await driver.executeScript(
        'const dialog = document.querySelector(\'.archive-details-dialog\'); return dialog ? { scrollWidth: dialog.scrollWidth, clientWidth: dialog.clientWidth } : null;',
      )
      assert.ok(detailDimensions)
      assert.ok(detailDimensions.scrollWidth <= detailDimensions.clientWidth + 1)
      writeFileSync(
        path.join(artifactDirectory, 'archive-browser-a02-entry-details.png'),
        Buffer.from(await driver.takeScreenshot(), 'base64'),
      )
      await (await details.findElement(By.css('[aria-label="关闭条目详情"]'))).click()

      await driver.actions().contextClick(rows[0]).perform()
      await (await waitForElement('[data-testid="archive-context-extract-current"]')).click()
      console.log('[desktop-e2e] archive-browser extraction started from the real file context menu')
    } else {
      await (await waitForElement('.browser-page > footer .browser-primary')).click()
    }
    let lastTask = null
    let terminalTask
    try {
      terminalTask = await driver.wait(async () => {
        const tasks = await callDesktopBridge('archiveBrowserTaskState')
        lastTask = tasks.at(-1) ?? null
        return lastTask && ['completed', 'failed', 'cancelled'].includes(lastTask.status) ? lastTask : false
      }, 60_000)
    } catch (error) {
      throw new Error(`archive-browser extraction timed out; last task: ${JSON.stringify(lastTask)}`, {
        cause: error,
      })
    }
    assert.equal(
      terminalTask.status,
      'completed',
      `archive-browser extraction failed: ${JSON.stringify(terminalTask)}`,
    )
    console.log(`[desktop-e2e] archive-browser terminal task: ${JSON.stringify(terminalTask)}`)
    assert.equal(terminalTask.selectedEntries.length, 1)
    if (expectedContent === null) await waitForStableFile(expectedPath)
    else await waitForFileContent(expectedPath, expectedContent, 10_000)
    assert.equal(existsSync(excludedPath), false, 'selective extraction must not publish excluded entries')
    await callDesktopBridge('clearTasks')
  }

  const zipOutput = path.join(browserFixtureRoot, 'zip-selected-output')
  await openArchive(browserZip, zipOutput, '', zipRootName)
  assert.match(await (await waitForElement('.browser-page')).getText(), /ZIP[\s\S]*未加密/)
  await verifyWorkspaceNavigation()
  await extractOnly(
    '保留文件',
    path.join(zipOutput, zipKeepRelative),
    zipKeepPayload,
    path.join(zipOutput, zipSkipRelative),
    true,
  )

  const sevenZipOutput = path.join(browserFixtureRoot, '7z-selected-output')
  await openArchive(browser7z, sevenZipOutput, 'desktop-browser-secret', passwordRootName)
  assert.match(await (await waitForElement('.browser-page')).getText(), /7Z[\s\S]*已加密/)
  const sevenZipSearch = await waitForElement('.browser-search input')
  await sevenZipSearch.clear()
  await sevenZipSearch.sendKeys('只解压这一项')
  assert.equal(
    await (await waitForElement('.preview-trigger')).isEnabled(),
    false,
    '7Z internal text preview must remain disabled until a bounded reader exists',
  )
  await extractOnly(
    '只解压这一项',
    path.join(sevenZipOutput, passwordKeepRelative),
    'password 7z selected payload',
    path.join(sevenZipOutput, passwordSkipRelative),
  )

  const verifyDefaultApplicationOpen = async (fileName, expectedBytes) => {
    const search = await waitForElement('.browser-search input')
    await search.clear()
    await search.sendKeys(fileName)
    const row = await waitForElement(`[data-entry-path$="${fileName}"]`)
    await driver.actions().contextClick(row).perform()
    const menu = await waitForElement('[data-testid="archive-context-menu"]')
    await (await menu.findElement(By.css('[data-testid="archive-context-default-open"]'))).click()
    const cacheRoot = path.join(e2eDataDirectory, 'preview-cache')
    const cached = await driver.wait(async () => {
      const found = findFileRecursively(cacheRoot, fileName)
      if (found && existsSync(`${found}:Zone.Identifier`)) return found
      const alerts = await driver.findElements(By.css('[role="alert"]'))
      for (const alert of alerts) {
        const message = await alert.getText()
        if (message.includes('无法打开归档内文件')) throw new Error(message)
      }
      return false
    }, 30_000)
    assert.deepEqual(readFileSync(cached), expectedBytes, `${fileName} cache bytes must match the real archive entry`)
    assert.equal(
      readFileSync(`${cached}:Zone.Identifier`, 'utf8'),
      defaultOpenZone,
      `${fileName} must retain the source archive Mark-of-the-Web`,
    )
    await driver.wait(async () => {
      const statuses = await driver.findElements(By.css('[role="status"]'))
      for (const status of statuses) {
        if ((await status.getText()).includes(`已使用默认应用打开：${fileName}`)) return true
      }
      return false
    }, 10_000)
    console.log(`[desktop-e2e] Windows accepted the default-application open request for ${fileName}`)
  }

  const defaultOpenOutput = path.join(browserFixtureRoot, 'default-open-output')
  await openArchive(defaultOpenZip, defaultOpenOutput, '', defaultOpenRootName)

  const verifyInternalTextPreview = async (fileName, expectedText, expectedMeta) => {
    console.log(`[desktop-e2e] opening internal text preview for ${fileName}`)
    const search = await waitForElement('.browser-search input')
    await search.clear()
    await search.sendKeys(fileName)
    const row = await waitForElement(`[data-entry-path$="${fileName}"]`)
    console.log(`[desktop-e2e] preview row found for ${fileName}`)
    await (await row.findElement(By.css('.preview-trigger'))).click()
    console.log(`[desktop-e2e] preview trigger clicked for ${fileName}`)
    const preview = await waitForElement('[data-testid="archive-entry-preview"]')
    console.log(`[desktop-e2e] preview dialog visible for ${fileName}`)
    const previewText = await driver.wait(async () => {
      const text = await preview.getText()
      return expectedText.test(text) && expectedMeta.test(text) ? text : false
    }, 30_000)
    assert.match(previewText, expectedText)
    assert.match(previewText, expectedMeta)
    if (fileName === '说明 文档.txt') {
      writeFileSync(
        path.join(artifactDirectory, 'archive-browser-a04-text-preview.png'),
        Buffer.from(await driver.takeScreenshot(), 'base64'),
      )
    }
    await (await preview.findElement(By.css('[aria-label="关闭预览"]'))).click()
    console.log(`[desktop-e2e] internal text preview passed for ${fileName}`)
  }

  await verifyInternalTextPreview('说明 文档.txt', /Long解压 A-03 Windows default TXT application/, /UTF-8[\s\S]*完整显示/)
  assert.equal(
    findFileRecursively(path.join(e2eDataDirectory, 'preview-cache'), '说明 文档.txt'),
    null,
    'internal text preview must not write an extracted cache file',
  )
  await verifyInternalTextPreview('超长 日志.log', /xxxxxxxx/, /仅显示前 1 MiB/)

  const binarySearch = await waitForElement('.browser-search input')
  await binarySearch.clear()
  await binarySearch.sendKeys('伪装 二进制.txt')
  const binaryRow = await waitForElement('[data-entry-path$="伪装 二进制.txt"]')
  await (await binaryRow.findElement(By.css('.preview-trigger'))).click()
  const binaryPreview = await waitForElement('[data-testid="archive-entry-preview"]')
  assert.match(await binaryPreview.getText(), /无法预览[\s\S]*appears to be binary/)
  await (await binaryPreview.findElement(By.css('[aria-label="关闭预览"]'))).click()

  const utf16Preview = await callDesktopBridge('previewArchiveText', textPreviewTar, utf16PreviewName)
  assert.equal(utf16Preview.encoding, 'UTF-16LE')
  assert.match(utf16Preview.content, /Long解压 UTF-16 真实 TAR 文本/)
  assert.equal(utf16Preview.truncated, false)
  await verifyDefaultApplicationOpen('说明 文档.txt', openTxtPayload)
  await verifyDefaultApplicationOpen('像素 图片.png', openPngPayload)
  await verifyDefaultApplicationOpen('空白 文档.pdf', openPdfPayload)

  const search = await waitForElement('.browser-search input')
  await search.clear()
  await search.sendKeys('禁止自动启动.cmd')
  const dangerousRow = await waitForElement('[data-entry-path$="禁止自动启动.cmd"]')
  await driver.actions().contextClick(dangerousRow).perform()
  await (await waitForElement('[data-testid="archive-context-default-open"]')).click()
  const warning = await waitForElement('[data-testid="archive-dangerous-open-dialog"]')
  assert.match(await warning.getText(), /尚未解压，也没有启动/)
  assert.equal(findFileRecursively(path.join(e2eDataDirectory, 'preview-cache'), '禁止自动启动.cmd'), null)
  assert.equal(existsSync(path.join(tmpdir(), 'long-a03-danger.marker')), false)
  await (await warning.findElement(By.css('[data-testid="archive-dangerous-cancel"]'))).click()
  assert.equal((await driver.findElements(By.css('[data-testid="archive-dangerous-open-dialog"]'))).length, 0)
  writeFileSync(
    path.join(artifactDirectory, 'archive-browser-a03-safe-open.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  const nestedOutput = path.join(browserFixtureRoot, 'nested-selected-output')
  await openArchive(nestedOuter, nestedOutput, '', nestedMiddleName)
  const nestedOuterFooter = await waitForElement('.browser-page > footer')
  assert.match(await nestedOuterFooter.getText(), /已选择\s+2\s+\/\s+2/)
  await driver.actions().doubleClick(await waitForElement(`[data-entry-path="${nestedMiddleName}"]`)).perform()
  await driver.wait(async () => (await waitForElement('[data-testid="archive-chain"]')).getText().then(text => text.includes(nestedMiddleName)), 30_000)
  const nestedPasswordInput = await waitForElement('.browser-toolbar input[type="password"]')
  assert.equal(await nestedPasswordInput.getAttribute('value'), '', 'the outer password must not be inherited by the nested archive')
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="archive-nested-retry"]'))).length === 1, 30_000)
  assert.match(await (await waitForElement('.browser-page')).getText(), /内层归档尚未打开/)
  const nestedErrorElement = await waitForElement('[data-testid="archive-nested-error"]')
  const initialErrorRevision = Number(await nestedErrorElement.getAttribute('data-error-revision'))
  await nestedPasswordInput.sendKeys('wrong-nested-password')
  await (await waitForElement('[data-testid="archive-nested-retry"]')).click()
  await driver.wait(async () => {
    const currentErrorElement = await waitForElement('[data-testid="archive-nested-error"]')
    const currentRevision = Number(await currentErrorElement.getAttribute('data-error-revision'))
    return currentRevision > initialErrorRevision
  }, 30_000)
  assert.match(await (await waitForElement('[data-testid="archive-nested-error"]')).getText(), /密码不正确|需要单独的密码/)
  assert.match(await (await waitForElement('[data-testid="archive-chain"]')).getText(), /外层工作区\.zip[\s\S]*加密中层\.7z/)
  await driver.executeScript(
    "const input = arguments[0]; input.value = ''; input.dispatchEvent(new Event('input', { bubbles: true }));",
    nestedPasswordInput,
  )
  await nestedPasswordInput.sendKeys('nested-middle-secret')
  await (await waitForElement('[data-testid="archive-nested-retry"]')).click()
  await driver.wait(async () => (await driver.findElements(By.css(`[data-entry-path="${nestedInnerName}"]`))).length === 1, 30_000)
  await driver.actions().doubleClick(await waitForElement(`[data-entry-path="${nestedInnerName}"]`)).perform()
  await driver.wait(async () => (await waitForElement('[data-testid="archive-chain"]')).getText().then(text => text.includes('3 / 3 层')), 30_000)
  const nestedChain = await waitForElement('[data-testid="archive-chain"]')
  assert.match(await nestedChain.getText(), /外层工作区\.zip[\s\S]*加密中层\.7z[\s\S]*内层\.zip/)
  assert.equal(await nestedPasswordInput.getAttribute('value'), '', 'the middle password must not leak into the inner ZIP')

  const fourthRow = await waitForElement(`[data-entry-path="${fourthArchiveName}"]`)
  await driver.actions().contextClick(fourthRow).perform()
  const fourthButton = await waitForElement('[data-testid="archive-context-enter-nested"]')
  assert.equal(await fourthButton.isEnabled(), false)
  assert.match(await fourthButton.getText(), /已达到 3 层上限/)
  writeFileSync(
    path.join(artifactDirectory, 'archive-browser-a05-nested-chain.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )
  await driver.actions().sendKeys(Key.ESCAPE).perform()

  await callDesktopBridge('clearTasks')
  const nestedSearch = await waitForElement('.browser-search input')
  await nestedSearch.clear()
  await (await waitForElement('.browser-table-head .browser-checkbox')).click()
  assert.match(await (await waitForElement('.browser-page > footer')).getText(), /已选择\s+0\s+\/\s+2/)
  await nestedSearch.sendKeys(nestedLeafName)
  await (await waitForElement(`[data-entry-path="${nestedLeafName}"] .browser-checkbox`)).click()
  await (await waitForElement('.browser-page > footer .browser-primary')).click()
  const nestedTerminalTask = await driver.wait(async () => {
    const tasks = await callDesktopBridge('archiveBrowserTaskState')
    const task = tasks.at(-1)
    return task && ['completed', 'failed', 'cancelled'].includes(task.status) ? task : false
  }, 60_000)
  assert.equal(nestedTerminalTask.status, 'completed', JSON.stringify(nestedTerminalTask))
  await waitForFileContent(path.join(nestedOutput, nestedLeafName), nestedLeafPayload, 10_000)
  assert.equal(existsSync(path.join(nestedOutput, fourthArchiveName)), false, 'the unselected fourth-level archive must not be extracted')

  const chainButtons = await nestedChain.findElements(By.css('button'))
  await chainButtons[0].click()
  await driver.wait(async () => (await driver.findElements(By.css(`[data-entry-path="${nestedMiddleName}"]`))).length === 1, 10_000)
  assert.match(await (await waitForElement('.browser-page > footer')).getText(), /已选择\s+2\s+\/\s+2/, 'returning must restore the exact outer selection')

  const damagedSearch = await waitForElement('.browser-search input')
  await damagedSearch.clear()
  await damagedSearch.sendKeys('损坏内层.zip')
  await driver.actions().doubleClick(await waitForElement('[data-entry-path="损坏内层.zip"]')).perform()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="archive-nested-retry"]'))).length === 1, 30_000)
  assert.match(await (await waitForElement('.browser-page')).getText(), /内层归档尚未打开/)
  await (await waitForElement('.nested-return')).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-entry-path="损坏内层.zip"]'))).length === 1, 10_000)

  const encryptedRar = path.join(externalFixtureDirectory, 'libarchive-rar-encrypted.rar')
  if (archiveBrowserOnly) {
    assert.ok(
      existsSync(encryptedRar),
      'the pinned encrypted RAR fixture is required; run npm.cmd run test:fixtures:archives',
    )
    const rarOutput = path.join(browserFixtureRoot, 'rar-selected-output')
    await openArchive(encryptedRar, rarOutput, '12345678', 'foo.txt')
    assert.match(await (await waitForElement('.browser-page')).getText(), /RAR[\s\S]*已加密/)
    await extractOnly('foo.txt', path.join(rarOutput, 'foo.txt'), null, path.join(rarOutput, 'bar.txt'))
    assert.equal(
      fileSha256(path.join(rarOutput, 'foo.txt')),
      '325d7b459b439684cad8825cbf2e488de15518103de09c56a42d6b1875081ee7',
      'selected encrypted RAR output must match the pinned plaintext hash',
    )
  }
}

async function runMarkOfWebDesktopGate() {
  console.log('[desktop-e2e] verifying visible Mark-of-the-Web propagation settings and real NTFS ADS')
  await callDesktopBridge('clearTasks')
  const fixtureRoot = path.join(fixtureDirectory, 'mark-of-web')
  const sourceRoot = path.join(fixtureRoot, 'internet-source')
  const archivePath = path.join(fixtureRoot, 'internet-download.zip')
  mkdirSync(path.join(sourceRoot, '目录'), { recursive: true })
  const payloads = new Map([
    ['普通文件.txt', 'ordinary payload'],
    ['报告.docx', 'office payload'],
    ['脚本.ps1', 'Write-Output motw'],
    ['工具.exe', 'executable fixture payload'],
    [path.join('目录', '嵌套.txt'), 'nested payload'],
  ])
  for (const [relative, contents] of payloads) {
    writeFileSync(path.join(sourceRoot, relative), contents, 'utf8')
  }
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-tzip', '-y', archivePath, path.basename(sourceRoot)],
    'Mark-of-the-Web ZIP',
    { cwd: fixtureRoot },
  )
  const zone = '[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://example.test/internet-download.zip\r\n'
  const archiveZoneStream = `${archivePath}:Zone.Identifier`
  writeFileSync(archiveZoneStream, zone, 'utf8')
  assert.equal(readFileSync(archiveZoneStream, 'utf8'), zone, 'the NTFS source ADS must be readable')

  await (await waitForElement('[data-testid="nav-Settings"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/settings'), 30_000)
  let preserveSwitch = await waitForElement('[data-testid="preserve-mark-of-web-switch"]')
  if ((await preserveSwitch.getAttribute('aria-checked')) !== 'true') {
    await preserveSwitch.click()
    await driver.wait(async () => (await preserveSwitch.getAttribute('aria-checked')) === 'true', 10_000)
  }

  const extractFromVisibleBrowser = async (outputPath) => {
    await callDesktopBridge('queueDesktopDialogSelections', [archivePath, outputPath])
    await (await waitForElement('[data-testid="nav-ArchiveBrowser"]')).click()
    await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/browser'), 30_000)
    const fileManagerChooser = await driver.findElements(By.css('[data-testid="file-manager-open-archive"]'))
    if (fileManagerChooser.length > 0) await fileManagerChooser[0].click()
    const passwordInput = await waitForElement('.browser-toolbar input[type="password"]')
    await driver.executeScript(
      'const input = arguments[0]; input.value = ""; input.dispatchEvent(new Event("input", { bubbles: true }));',
      passwordInput,
    )
    await (await waitForElement('.browser-page > header .browser-primary')).click()
    await driver.wait(async () => {
      const page = await waitForElement('.browser-page')
      return (await page.getText()).includes('报告.docx')
    }, 30_000)
    const fields = await driver.findElements(By.css('.browser-toolbar .browser-field'))
    await (await fields[2].findElement(By.css('button'))).click()
    await driver.wait(async () => (await fields[2].getText()).includes(outputPath), 10_000)
    assert.match(await (await waitForElement('.browser-page > footer')).getText(), /已选择\s+5\s+\//)
    await (await waitForElement('.browser-page > footer .browser-primary')).click()
    const task = await driver.wait(async () => {
      const tasks = await callDesktopBridge('archiveBrowserTaskState')
      const latest = tasks.at(-1)
      return latest && ['completed', 'failed', 'cancelled'].includes(latest.status) ? latest : false
    }, 60_000)
    assert.equal(task.status, 'completed', `Mark-of-the-Web extraction failed: ${JSON.stringify(task)}`)
    for (const [relative, contents] of payloads) {
      await waitForFileContent(path.join(outputPath, path.basename(sourceRoot), relative), contents, 10_000)
    }
    return task
  }

  const markedOutput = path.join(fixtureRoot, 'marked-output')
  const markedTask = await extractFromVisibleBrowser(markedOutput)
  assert.ok(
    markedTask.logs.some(message => message.includes('互联网来源安全标记')),
    `the real task log must report Mark-of-the-Web propagation: ${JSON.stringify(markedTask.logs)}`,
  )
  for (const relative of payloads.keys()) {
    const outputFile = path.join(markedOutput, path.basename(sourceRoot), relative)
    assert.equal(
      readFileSync(`${outputFile}:Zone.Identifier`, 'utf8'),
      zone,
      `the committed file must preserve the source ADS: ${relative}`,
    )
  }

  await callDesktopBridge('clearTasks')
  await (await waitForElement('[data-testid="nav-Settings"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/settings'), 30_000)
  preserveSwitch = await waitForElement('[data-testid="preserve-mark-of-web-switch"]')
  assert.equal(await preserveSwitch.getAttribute('aria-checked'), 'true')
  await preserveSwitch.click()
  await driver.wait(async () => (await preserveSwitch.getAttribute('aria-checked')) === 'false', 10_000)

  const unmarkedOutput = path.join(fixtureRoot, 'unmarked-output')
  const unmarkedTask = await extractFromVisibleBrowser(unmarkedOutput)
  assert.equal(
    unmarkedTask.logs.some(message => message.includes('互联网来源安全标记')),
    false,
    'disabled propagation must not report an applied mark',
  )
  for (const relative of payloads.keys()) {
    const outputFile = path.join(unmarkedOutput, path.basename(sourceRoot), relative)
    assert.equal(
      existsSync(`${outputFile}:Zone.Identifier`),
      false,
      `disabled propagation must not create an ADS: ${relative}`,
    )
  }
  await (await waitForElement('[data-testid="nav-Settings"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/settings'), 30_000)
  preserveSwitch = await waitForElement('[data-testid="preserve-mark-of-web-switch"]')
  assert.equal(await preserveSwitch.getAttribute('aria-checked'), 'false')
  await preserveSwitch.click()
  await driver.wait(async () => (await preserveSwitch.getAttribute('aria-checked')) === 'true', 10_000)
}

async function runCompressionVerificationDesktopGate() {
  console.log('[desktop-e2e] verifying visible post-compression verification and protected source deletion')
  const fixtureRoot = path.join(fixtureDirectory, 'compression-verification')
  const outputRoot = path.join(fixtureRoot, 'archives')
  mkdirSync(outputRoot, { recursive: true })

  const runScenario = async ({ sourceName, contents, deleteAfter }) => {
    await callDesktopBridge('clearCompressionWorkspace')
    await callDesktopBridge('clearTasks')
    const sourcePath = path.join(fixtureRoot, sourceName)
    writeFileSync(sourcePath, contents, 'utf8')
    const outputPath = path.join(outputRoot, `${path.parse(sourceName).name}.zip`)

    await callDesktopBridge('queueDesktopDialogSelections', [[sourcePath]])
    await (await waitForElement('[data-testid="nav-Compress"]')).click()
    await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/compress'), 30_000)
    await (await waitForElement('[data-testid="dropzone-file"]')).click()
    const draftRow = await driver.wait(async () => {
      const rows = await driver.findElements(By.css('[data-testid="compression-draft-row"]'))
      return rows.length === 1 && (await rows[0].getText()).includes(sourceName) ? rows[0] : false
    }, 30_000)

    await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
    if ((await driver.findElements(By.css('[data-testid="compression-output-path"]'))).length === 0) {
      await (await waitForElement('[data-testid="compression-advanced-options"]')).click()
    }
    const outputInput = await waitForElement('[data-testid="compression-output-path"]')
    await driver.executeScript(
      'const input = arguments[0], value = arguments[1]; input.value = value; input.dispatchEvent(new Event("input", { bubbles: true }));',
      outputInput,
      outputRoot,
    )
    const deleteCheckbox = await waitForElement('[data-testid="compression-delete-after"]')
    const verifyCheckbox = await waitForElement('[data-testid="compression-verify-after"]')
    if (deleteAfter) {
      if (await verifyCheckbox.isSelected()) await verifyCheckbox.click()
      assert.equal(await verifyCheckbox.isSelected(), false, 'verification must be user-toggleable before deletion')
      if (!(await deleteCheckbox.isSelected())) await deleteCheckbox.click()
      await driver.wait(async () => (await verifyCheckbox.isSelected()) && !(await verifyCheckbox.isEnabled()), 10_000)
    } else {
      if (await deleteCheckbox.isSelected()) await deleteCheckbox.click()
      if (!(await verifyCheckbox.isSelected())) await verifyCheckbox.click()
      assert.equal(await verifyCheckbox.isEnabled(), true)
    }
    await (await waitForElement('[data-testid="save-global-compression-settings"]')).click()
    await driver.wait(async () => (await driver.findElements(By.css('[data-testid="save-global-compression-settings"]'))).length === 0, 10_000)

    await (await waitForElement('[data-testid="start-compression"]')).click()
    const task = await driver.wait(async () => {
      const tasks = await callDesktopBridge('compressionVerificationTaskState')
      const latest = tasks.at(-1)
      return latest && ['completed', 'failed', 'cancelled'].includes(latest.status) ? latest : false
    }, 60_000)
    assert.equal(task.status, 'completed', `verified compression failed: ${JSON.stringify(task)}`)
    assert.equal(normalizedDesktopPath(task.outputPath), normalizedDesktopPath(outputPath))
    assert.equal(task.deleteAfter, deleteAfter)
    assert.equal(task.verifyAfter, true, 'the submitted task must never disable required verification')
    const verifyStart = task.logs.findIndex(message => message.includes('正在校验新压缩包的完整性'))
    const verifyPassed = task.logs.findIndex(message => message.includes('压缩包完整性校验通过'))
    const completed = task.logs.findIndex(message => message === '压缩完成')
    assert.ok(verifyStart >= 0 && verifyPassed > verifyStart && completed > verifyPassed, JSON.stringify(task.logs))
    await waitForStableFile(outputPath)
    runFixtureCommand(bundledSevenZip, ['t', '-y', outputPath], `verified ZIP ${sourceName}`)
    assert.equal(existsSync(sourcePath), !deleteAfter, 'source deletion must match the visible setting')

    await draftRow.click()
    await driver.wait(async () => {
      const panels = await driver.findElements(By.css('[data-testid="compression-draft-execution"]'))
      return panels.length > 0 && (await panels[0].getText()).includes('压缩包完整性校验通过')
    }, 10_000)

    const extracted = path.join(fixtureRoot, `audit-${path.parse(sourceName).name}`)
    runFixtureCommand(bundledSevenZip, ['x', '-y', `-o${extracted}`, outputPath], `verified ZIP extraction ${sourceName}`)
    assert.equal(readFileSync(path.join(extracted, sourceName), 'utf8'), contents)
  }

  await runScenario({
    sourceName: '保留源文件.txt',
    contents: 'verified compression keeps this source',
    deleteAfter: false,
  })
  await runScenario({
    sourceName: '校验后删除.txt',
    contents: 'verified compression may delete this source only after publishing',
    deleteAfter: true,
  })
  await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
  if ((await driver.findElements(By.css('[data-testid="compression-delete-after"]'))).length === 0) {
    const advancedButton = await waitForElement('[data-testid="compression-advanced-options"]')
    await driver.executeScript('arguments[0].scrollIntoView({ block: "center" });', advancedButton)
    await driver.wait(async () => advancedButton.isDisplayed(), 10_000)
    await driver.executeScript('arguments[0].click();', advancedButton)
  }
  const deleteCheckbox = await waitForElement('[data-testid="compression-delete-after"]')
  const verifyCheckbox = await waitForElement('[data-testid="compression-verify-after"]')
  if (await deleteCheckbox.isSelected()) await driver.executeScript('arguments[0].click();', deleteCheckbox)
  if (!(await verifyCheckbox.isSelected())) await driver.executeScript('arguments[0].click();', verifyCheckbox)
  assert.equal(await verifyCheckbox.isEnabled(), true)
  await driver.executeScript(
    'arguments[0].click();',
    await waitForElement('[data-testid="save-global-compression-settings"]'),
  )
}

async function waitForWatchFolderState(profileId, predicate, timeoutMs = 30_000) {
  let lastState = null
  let lastError = null
  try {
    return await driver.wait(async () => {
      try {
        lastState = await callDesktopBridge('watchFolderAuditState', profileId)
        return predicate(lastState) ? lastState : false
      } catch (error) {
        lastError = String(error)
        return false
      }
    }, timeoutMs)
  } catch (error) {
    throw new Error(
      `Watch-folder state timed out after ${timeoutMs}ms. Last state: ${JSON.stringify(lastState)}. Last bridge error: ${lastError ?? 'none'}`,
      { cause: error },
    )
  }
}

async function waitForResourcePreflightTask(type, predicate, timeoutMs = 30_000) {
  let lastState = null
  try {
    return await driver.wait(async () => {
      lastState = await callDesktopBridge('resourcePreflightAuditState', type)
      const task = lastState.tasks.find(predicate)
      return task || false
    }, timeoutMs)
  } catch (error) {
    throw new Error(
      `Resource-preflight task timed out after ${timeoutMs}ms. Last state: ${JSON.stringify(lastState)}`,
      { cause: error },
    )
  }
}

function assertRealResourceReport(task, expected) {
  const report = task.report
  assert.ok(report, `${expected.operation} task must retain its resource report`)
  assert.equal(report.operation, expected.operation)
  assert.equal(task.status, expected.taskStatus)
  assert.equal(report.canStart, expected.canStart)
  assert.ok(
    expected.reportStatuses.includes(report.status),
    `unexpected resource status ${report.status}; expected one of ${expected.reportStatuses.join(', ')}`,
  )
  assert.equal(normalizedDesktopPath(report.outputPath), normalizedDesktopPath(expected.outputPath))
  assert.equal(report.location, 'local', 'the real temporary output must resolve to a local volume')
  assert.ok(report.mountPoint, 'the real volume mount point must be visible')
  assert.ok(report.fileSystem, 'the real volume file system must be visible')
  assert.ok(report.totalBytes > 0, 'the real volume total capacity must be positive')
  assert.ok(report.availableBytes > 0, 'the real volume available capacity must be positive')
  assert.ok(report.totalBytes >= report.availableBytes, 'available capacity cannot exceed total capacity')
  assert.equal(report.reserveBytes, 128 * 1024 * 1024, 'the desktop report must retain the safety reserve')
  assert.ok(
    task.logMessages.some(message => message.includes('资源预检')),
    'the resource conclusion must be preserved in the real task log',
  )
}

async function assertVisibleResourceCard(taskId, expectedLabel) {
  const rowSelector = `[data-task-id="${taskId}"]`
  const findMatchingCard = async () => {
    const candidates = await driver.findElements(By.css('[data-testid="resource-preflight-card"]'))
    for (const candidate of candidates) {
      const text = (await candidate.getAttribute('textContent')).trim()
      if (new RegExp(expectedLabel).test(text)) return candidate
    }
    return false
  }
  let card = await findMatchingCard()
  if (!card) {
    await (await waitForElement(rowSelector)).click()
    card = await driver.wait(findMatchingCard, 10_000)
  }
  await driver.executeScript(
    'arguments[0].scrollIntoView({ block: "start", inline: "nearest", behavior: "instant" });',
    card,
  )
  const text = (await card.getAttribute('textContent')).trim()
  assert.match(text, /目标存储预检/)
  assert.match(text, new RegExp(expectedLabel))
  const dimensions = await driver.executeScript(
    `const card = arguments[0];
     const metrics = card.querySelector('[data-testid="resource-preflight-metrics"]');
     const fields = [...card.querySelectorAll('.metric dt, .metric dd')];
     return {
       scrollWidth: card.scrollWidth,
       clientWidth: card.clientWidth,
       columnCount: metrics ? getComputedStyle(metrics).gridTemplateColumns.split(' ').filter(Boolean).length : 0,
       fields: fields.map(field => {
         const style = getComputedStyle(field);
         return {
           text: field.textContent?.trim(),
           height: field.getBoundingClientRect().height,
           lineHeight: Number.parseFloat(style.lineHeight) || Number.parseFloat(style.fontSize) * 1.2,
           whiteSpace: style.whiteSpace,
         };
       }),
     };`,
    card,
  )
  assert.ok(dimensions, 'the resource-preflight card must remain mounted')
  assert.ok(
    dimensions.scrollWidth <= dimensions.clientWidth + 1,
    `the resource-preflight card must not scroll horizontally: ${JSON.stringify(dimensions)}`,
  )
  assert.ok(
    dimensions.columnCount >= 1 && dimensions.columnCount <= 2,
    `the narrow resource card must use at most two metric columns: ${JSON.stringify(dimensions)}`,
  )
  assert.ok(
    dimensions.fields.every(field => field.whiteSpace === 'nowrap' && field.height <= field.lineHeight * 1.35),
    `resource metric labels and values must remain on one readable line: ${JSON.stringify(dimensions)}`,
  )
  return text
}

function watchDraftPaths(state) {
  return state.draftGroups
    .flatMap(group => group.files)
    .map(normalizedDesktopPath)
}

function assertInertWatchDraftState(state) {
  assert.equal(state.taskCount, 0, 'watch-folder discovery must not create a desktop task')
  assert.equal(state.activeTaskCount, 0, 'watch-folder discovery must not create an active task')
  assert.equal(state.autoStartRequested, false, 'watch-folder discovery must not request auto-start')
  for (const group of state.draftGroups) {
    assert.equal(group.password, '', 'watch-folder drafts must not carry a password')
    assert.equal(group.deleteAfter, false, 'watch-folder drafts must not delete source files')
    assert.equal(group.taskId, null, 'watch-folder drafts must remain unbound to a task')
  }
}

async function waitForElement(selector, timeoutMs = 30_000) {
  return driver.wait(async () => {
    const elements = await driver.findElements(By.css(selector))
    return elements[0] || false
  }, timeoutMs)
}

async function waitForLocalFileContent(filePath, expectedContent, timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    try {
      if (readFileSync(filePath, 'utf8') === expectedContent) return
    } catch {
      // The desktop process has not written the visibility marker yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error(`Timed out waiting for ${filePath} to contain ${expectedContent}.`)
}

async function hideDesktopWindow(markerPath) {
  await driver.executeScript(
    'void window.__LONG_DECOMPRESS_DESKTOP_E2E__?.hideWindow(arguments[0]); return true',
    markerPath,
  )
}

async function captureFailure() {
  const nestedDevToolsPort = path.join(webviewUserDataDirectory, 'EBWebView', 'DevToolsActivePort')
  const mirroredDevToolsPort = path.join(webviewUserDataDirectory, 'DevToolsActivePort')
  driverOutput +=
    `\nDevToolsActivePort: nested=${existsSync(nestedDevToolsPort)}` +
    ` mirrored=${existsSync(mirroredDevToolsPort)}\n`
  if (fixtureDirectory) {
    driverOutput += `Fixture directory retained at: ${fixtureDirectory}\n`
  }
  mkdirSync(artifactDirectory, { recursive: true })
  writeFileSync(path.join(artifactDirectory, 'tauri-driver.log'), driverOutput, 'utf8')
  if (driver) {
    try {
      const screenshot = await driver.takeScreenshot()
      writeFileSync(
        path.join(artifactDirectory, 'desktop-e2e-failure.png'),
        Buffer.from(screenshot, 'base64'),
      )
    } catch {
      // The session may already be unavailable.
    }
  }
}

function mirrorDevToolsActivePort() {
  const source = path.join(webviewUserDataDirectory, 'EBWebView', 'DevToolsActivePort')
  const destination = path.join(webviewUserDataDirectory, 'DevToolsActivePort')
  if (!existsSync(source)) return
  try {
    copyFileSync(source, destination)
  } catch {
    // EdgeDriver may be reading or removing the compatibility copy concurrently.
  }
}

async function createDesktopSession(applicationArgs = []) {
  const capabilities = new Capabilities()
  capabilities.setBrowserName('wry')
  const webviewOptions = {
    userDataFolder: webviewUserDataDirectory,
  }
  if (process.env.CI) {
    webviewOptions.additionalBrowserArguments = ['--headless=new', '--disable-gpu']
  }
  capabilities.set('tauri:options', { application, args: applicationArgs, webviewOptions })
  devToolsPortMirror = setInterval(mirrorDevToolsActivePort, 50)
  try {
    const session = await new Builder()
      .usingServer(webdriverUrl)
      .withCapabilities(capabilities)
      .build()
    await session.manage().setTimeouts({ implicit: 1_000, pageLoad: 60_000, script: 120_000 })
    return session
  } finally {
    if (devToolsPortMirror) clearInterval(devToolsPortMirror)
    devToolsPortMirror = undefined
  }
}

async function waitForDesktopReady() {
  if (!pdfWorkspaceOnly) {
    console.log('[desktop-e2e] waiting for desktop heading')
    assert.ok(await waitForNonEmptyText('main h1'), 'the decompression workspace heading is empty')
    console.log('[desktop-e2e] desktop heading is ready; waiting for the E2E bridge')
  } else {
    console.log('[desktop-e2e] focused PDF gate is waiting for the application bridge before route navigation')
  }
  await driver.wait(
    () => driver.executeScript('return Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__)'),
    30_000,
  )
  console.log('[desktop-e2e] desktop E2E bridge is ready')
}

async function restartDesktopSession() {
  if (driver) {
    const previousDriver = driver
    const previousApplicationProcessIds = desktopApplicationProcessIds()
    assert.ok(
      previousApplicationProcessIds.length > 0,
      'the desktop application process must exist before the restart gate',
    )
    console.log('[desktop-e2e] requesting native application exit')
    await callDesktopBridge('requestAppExit')
    driver = undefined
    await new Promise((resolve) => setTimeout(resolve, 1_000))
    const remainingApplicationProcessIds = desktopApplicationProcessIds()
    if (remainingApplicationProcessIds.length > 0) {
      console.log('[desktop-e2e] force-stopping the exact workspace application process')
      for (const processId of remainingApplicationProcessIds) terminateProcessTree(processId)
      await new Promise((resolve) => setTimeout(resolve, 1_000))
    }
    assert.deepEqual(
      desktopApplicationProcessIds(),
      [],
      'the previous desktop application process must exit before restart',
    )
    try {
      await previousDriver.quit()
    } catch {
      // The exit command may invalidate the WebDriver session immediately.
    }
    console.log('[desktop-e2e] previous WebDriver session released')
  }
  terminateProcessTree(tauriDriverProcess?.pid)
  tauriDriverProcess = undefined
  console.log('[desktop-e2e] previous tauri-driver service stopped')
  await new Promise((resolve) => setTimeout(resolve, 500))
  await startTauriDriver()
  console.log('[desktop-e2e] replacement tauri-driver service is ready')
  desktopSessionIndex += 1
  webviewUserDataDirectory = path.join(
    e2eDataDirectory,
    `webview2-session-${desktopSessionIndex}`,
  )
  mkdirSync(webviewUserDataDirectory, { recursive: true })
  console.log('[desktop-e2e] creating replacement desktop session')
  if (autoStartOnly) {
    assert.equal(
      readAutoStartRegistryValue(),
      null,
      'the focused auto-start gate refuses to overwrite an existing user startup choice',
    )
  }
  driver = await createDesktopSession()
  console.log('[desktop-e2e] replacement desktop session created')
  await waitForDesktopReady()
}

async function runArchiveFlowDesktopGate() {
  console.log('[desktop-e2e] verifying concurrent compression and visible throughput telemetry')
  await callDesktopBridge('clearTasks')
  const flowRoot = path.join(fixtureDirectory, 'archive-flow')
  const sourceRoot = path.join(flowRoot, 'sources')
  const archiveRoot = path.join(flowRoot, 'archives')
  const sharedOutput = path.join(flowRoot, 'shared-output')
  mkdirSync(sourceRoot, { recursive: true })
  mkdirSync(archiveRoot, { recursive: true })
  mkdirSync(sharedOutput, { recursive: true })

  const compressionJobs = [0, 1].map(index => {
    const sourcePath = path.join(sourceRoot, `concurrent-${index + 1}.bin`)
    const archivePath = path.join(archiveRoot, `concurrent-${index + 1}.7z`)
    writeFileSync(sourcePath, randomBytes(128 * 1024 * 1024))
    return { sourcePath, archivePath }
  })

  const compressionTaskIds = await callDesktopBridge(
    'startArchiveCompressionBatch',
    compressionJobs,
  )
  await driver.wait(async () => {
    const state = await callDesktopBridge('archiveFlowAuditState')
    return state.compressionMaxActive === 2
  }, 30_000)

  const progressSummary = await waitForElement('.progress-summary')
  await progressSummary.click()
  await driver.wait(async () => {
    const panels = await driver.findElements(By.css('.progress-panel'))
    if (panels.length === 0) return false
    const text = await panels[0].getAttribute('textContent')
    return /速度[\s\S]*\/s/.test(text) && /(剩余|ETA)/.test(text)
  }, 60_000)

  const compressionState = await driver.wait(async () => {
    const state = await callDesktopBridge('archiveFlowAuditState')
    return state.compressionDone ? state : false
  }, 180_000)
  assert.equal(compressionState.compressionMaxActive, 2)
  assert.deepEqual(compressionState.errors, [])
  assert.ok(
    compressionState.telemetry.some(item => item.speed && item.etaSeconds !== undefined),
    'the Release desktop flow must receive real speed and ETA telemetry',
  )
  for (let index = 0; index < compressionJobs.length; index += 1) {
    assert.equal(await callDesktopBridge('taskStatus', compressionTaskIds[index]), 'completed')
    await waitForStableFile(compressionJobs[index].archivePath, 30_000)
  }

  console.log('[desktop-e2e] verifying plain and AES ZIP real-byte telemetry')
  for (const encrypted of [false, true]) {
    const label = encrypted ? 'aes' : 'plain'
    const sourcePath = path.join(sourceRoot, `${label}-zip-telemetry.bin`)
    const archivePath = path.join(archiveRoot, `${label}-zip-telemetry.zip`)
    const extractRoot = path.join(flowRoot, `${label}-zip-extracted`)
    const password = encrypted ? 'Long-ZIP-Telemetry-2026!' : undefined
    writeFileSync(sourcePath, randomBytes(64 * 1024 * 1024))

    const taskId = await callDesktopBridge(
      'startZipTelemetryCompression',
      sourcePath,
      archivePath,
      password,
    )
    await driver.wait(async () => {
      const state = await callDesktopBridge('archiveFlowAuditState')
      return state.zipTelemetry.some(item => item.speed && item.etaSeconds !== undefined)
    }, 60_000)
    await driver.wait(async () => {
      const panels = await driver.findElements(By.css('.progress-panel'))
      if (panels.length === 0) return false
      const text = await panels[0].getAttribute('textContent')
      return /速度[\s\S]*\/s/.test(text) && /(剩余|ETA)/.test(text)
    }, 60_000)
    const state = await driver.wait(async () => {
      const current = await callDesktopBridge('archiveFlowAuditState')
      return current.zipDone ? current : false
    }, 180_000)
    assert.equal(await callDesktopBridge('taskStatus', taskId), 'completed')
    assert.deepEqual(state.errors, [])
    const sourceBytes = statSync(sourcePath).size
    assert.ok(
      state.zipTelemetry.some(item => item.processedBytes > 0 && item.processedBytes < item.totalBytes),
      `${label} ZIP must emit an intermediate real-byte event`,
    )
    const byteTelemetry = state.zipTelemetry.filter(item => item.totalBytes > 0)
    const finalTelemetry = byteTelemetry.at(-1)
    assert.equal(finalTelemetry.processedBytes, sourceBytes)
    assert.equal(finalTelemetry.totalBytes, sourceBytes)
    assert.ok(finalTelemetry.speed, `${label} ZIP must expose measured throughput`)
    assert.equal(finalTelemetry.etaSeconds, 0)
    const outputTelemetry = state.zipTelemetry.filter(item => item.outputBytes > 0)
    assert.ok(outputTelemetry.length > 0, `${label} ZIP must emit its real archive size`)
    assert.equal(outputTelemetry.at(-1).outputBytes, statSync(archivePath).size)

    const testArgs = ['t', '-y']
    if (password) testArgs.push(`-p${password}`)
    testArgs.push(archivePath)
    const archiveTest = spawnSync(bundledSevenZip, testArgs, { encoding: 'utf8', windowsHide: true })
    assert.equal(archiveTest.status, 0, archiveTest.stderr || archiveTest.stdout)
    if (password) {
      const wrongPassword = spawnSync(
        bundledSevenZip,
        ['t', '-y', '-pwrong-password', archivePath],
        { encoding: 'utf8', windowsHide: true },
      )
      assert.notEqual(wrongPassword.status, 0, 'AES ZIP must reject an incorrect password')
    }
    mkdirSync(extractRoot, { recursive: true })
    const extractArgs = ['x', '-y', `-o${extractRoot}`]
    if (password) extractArgs.push(`-p${password}`)
    extractArgs.push(archivePath)
    const extraction = spawnSync(bundledSevenZip, extractArgs, { encoding: 'utf8', windowsHide: true })
    assert.equal(extraction.status, 0, extraction.stderr || extraction.stdout)
    assert.equal(fileSha256(path.join(extractRoot, path.basename(sourcePath))), fileSha256(sourcePath))
  }

  console.log('[desktop-e2e] verifying same-output extraction serialization')
  const extractionTaskIds = await callDesktopBridge(
    'startSharedOutputExtraction',
    compressionJobs.map(job => job.archivePath),
    sharedOutput,
  )
  const extractionState = await driver.wait(async () => {
    const state = await callDesktopBridge('archiveFlowAuditState')
    return state.extractionDone ? state : false
  }, 180_000)
  assert.equal(extractionState.extractionMaxActive, 1)
  assert.deepEqual(extractionState.errors, [])
  for (const taskId of extractionTaskIds) {
    assert.equal(await callDesktopBridge('taskStatus', taskId), 'completed')
  }
  for (const job of compressionJobs) {
    const extractedFile = path.join(sharedOutput, path.basename(job.sourcePath))
    assert.equal(fileSha256(extractedFile), fileSha256(job.sourcePath))
  }

  console.log('[desktop-e2e] verifying encrypted solid 7Z creation and extraction')
  const solidSource = path.join(flowRoot, 'solid-source')
  const solidArchive = path.join(archiveRoot, 'encrypted-solid.7z')
  const solidOutput = path.join(flowRoot, 'solid-output')
  const solidPassword = 'Long-Desktop-E2E-2026!'
  mkdirSync(solidSource, { recursive: true })
  writeFileSync(path.join(solidSource, 'alpha.txt'), 'alpha solid payload\n'.repeat(16_384), 'utf8')
  writeFileSync(path.join(solidSource, 'beta.txt'), 'beta solid payload\n'.repeat(16_384), 'utf8')
  await callDesktopBridge(
    'runEncryptedSolidSevenZipRoundTrip',
    solidSource,
    solidArchive,
    solidOutput,
    solidPassword,
  )
  const solidListing = spawnSync(
    bundledSevenZip,
    ['l', '-slt', `-p${solidPassword}`, solidArchive],
    { encoding: 'utf8', windowsHide: true },
  )
  assert.equal(solidListing.status, 0, solidListing.stderr || solidListing.stdout)
  assert.match(solidListing.stdout, /Solid = \+/)
  const extractedAlpha = path.join(solidOutput, 'solid-source', 'alpha.txt')
  const extractedBeta = path.join(solidOutput, 'solid-source', 'beta.txt')
  assert.equal(readFileSync(extractedAlpha, 'utf8'), readFileSync(path.join(solidSource, 'alpha.txt'), 'utf8'))
  assert.equal(readFileSync(extractedBeta, 'utf8'), readFileSync(path.join(solidSource, 'beta.txt'), 'utf8'))

  console.log('[desktop-e2e] verifying explicit password-format fallback decline and acceptance')
  const fallbackSource = path.join(flowRoot, 'password-fallback.txt')
  const fallbackOutput = path.join(flowRoot, 'password-fallback-output')
  const fallbackArchive = path.join(fallbackOutput, 'password-fallback.7z')
  writeFileSync(fallbackSource, 'password fallback payload', 'utf8')
  mkdirSync(fallbackOutput, { recursive: true })
  await callDesktopBridge('seedPasswordFallbackWorkspace', fallbackSource, fallbackOutput)
  await callDesktopBridge('queueDesktopConfirmations', [false])
  let navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[1].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/compress'), 30_000)
  await (await waitForElement('[data-testid="start-compression"]')).click()
  await driver.wait(async () => {
    const state = await callDesktopBridge('compressionWorkspaceAuditState')
    return state.taskCount === 0 && state.pendingGroupCount === 1
  }, 30_000)
  assert.equal(existsSync(fallbackArchive), false, 'declining the 7Z fallback must not create a task output')

  await callDesktopBridge('queueDesktopConfirmations', [true])
  await (await waitForElement('[data-testid="start-compression"]')).click()
  await waitForStableFile(fallbackArchive, 60_000)
  const acceptedState = await callDesktopBridge('compressionWorkspaceAuditState')
  assert.equal(acceptedState.taskCount, 1)
  assert.equal(acceptedState.pendingGroupCount, 0)
  const fallbackExtract = path.join(flowRoot, 'password-fallback-extract')
  await callDesktopBridge('extractArchive', fallbackArchive, fallbackExtract, 'desktop-e2e-password')
  assert.equal(
    readFileSync(path.join(fallbackExtract, 'password-fallback.txt'), 'utf8'),
    readFileSync(fallbackSource, 'utf8'),
  )
}

async function runHistoryDesktopGate() {
  console.log('[desktop-e2e] verifying persistent history from a real ZIP round trip')
  await callDesktopBridge('clearTasks')
  await callDesktopBridge('clearTaskHistory')
  const root = path.join(fixtureDirectory, 'history-gate')
  const sourcePath = path.join(root, 'history-payload.bin')
  const archivePath = path.join(root, 'history-payload.zip')
  const outputPath = path.join(root, 'extracted')
  mkdirSync(root, { recursive: true })
  writeFileSync(sourcePath, randomBytes(2 * 1024 * 1024))

  await callDesktopBridge('runArchiveRoundTrip', sourcePath, archivePath, outputPath, 'zip')
  const beforeRestart = await driver.wait(async () => {
    const records = await callDesktopBridge('taskHistory')
    return records.length >= 2 ? records : false
  }, 30_000)
  const compression = beforeRestart.find(record => record.taskType === 'compression')
  const extraction = beforeRestart.find(record => record.taskType === 'decompression')
  assert.equal(compression?.status, 'completed')
  assert.equal(extraction?.status, 'completed')
  assert.equal(normalizedDesktopPath(compression?.outputPath), normalizedDesktopPath(archivePath))
  assert.ok(compression?.durationMs >= 0)
  assert.ok(!JSON.stringify(beforeRestart).toLowerCase().includes('password'))

  await restartDesktopSession()
  const afterRestart = await callDesktopBridge('taskHistory')
  assert.ok(afterRestart.some(record => record.id === compression.id))
  assert.ok(afterRestart.some(record => record.id === extraction.id))

  const historyButton = await driver.findElement(By.css('[data-testid="nav-History"]'))
  await historyButton.click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/history'), 15_000)
  await driver.wait(async () => (await driver.findElement(By.css('[data-testid="history-list"]')).isDisplayed()), 15_000)
  const overflow = await driver.executeScript(() => ({
    body: document.body.scrollWidth - document.body.clientWidth,
    page: document.querySelector('.history-view')?.scrollWidth - document.querySelector('.history-view')?.clientWidth,
  }))
  assert.ok(overflow.body <= 1, `history shell must not overflow horizontally: ${JSON.stringify(overflow)}`)
  assert.ok((overflow.page ?? 0) <= 1, `history page must not overflow horizontally: ${JSON.stringify(overflow)}`)
  await driver.manage().window().setRect({ width: 760, height: 520 })
  const compactOverflow = await driver.executeScript(() => ({
    body: document.body.scrollWidth - document.body.clientWidth,
    page: document.querySelector('.history-view')?.scrollWidth - document.querySelector('.history-view')?.clientWidth,
  }))
  assert.ok(compactOverflow.body <= 1, `compact history shell must not overflow: ${JSON.stringify(compactOverflow)}`)
  assert.ok((compactOverflow.page ?? 0) <= 1, `compact history page must not overflow: ${JSON.stringify(compactOverflow)}`)
  const compactRowLayout = await driver.executeScript(() => {
    const badge = document.querySelector('[data-testid="history-status-badge"]')
    const cluster = badge?.closest('.history-status-cluster')
    const completedAt = cluster?.querySelector('.history-completed-at')
    const badgeStyle = badge ? getComputedStyle(badge) : null
    return {
      badgeText: badge?.textContent?.trim(),
      badgeWhiteSpace: badgeStyle?.whiteSpace,
      badgeWidth: badge?.getBoundingClientRect().width,
      badgeHeight: badge?.getBoundingClientRect().height,
      clusterOverflow: cluster ? cluster.scrollWidth - cluster.clientWidth : null,
      completedAtWhiteSpace: completedAt ? getComputedStyle(completedAt).whiteSpace : null,
    }
  })
  assert.equal(compactRowLayout.badgeText, '已完成')
  assert.equal(compactRowLayout.badgeWhiteSpace, 'nowrap')
  assert.ok(compactRowLayout.badgeWidth > compactRowLayout.badgeHeight, `status badge must remain a horizontal pill: ${JSON.stringify(compactRowLayout)}`)
  assert.ok((compactRowLayout.clusterOverflow ?? 0) <= 1, `history status cluster must not overflow: ${JSON.stringify(compactRowLayout)}`)
  assert.equal(compactRowLayout.completedAtWhiteSpace, 'nowrap')
  mkdirSync(artifactDirectory, { recursive: true })
  writeFileSync(
    path.join(artifactDirectory, 'task-history-compact.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  await (await driver.findElement(By.css('[data-testid="history-record-row"]'))).click()
  await driver.wait(async () => (await driver.findElement(By.css('[data-testid="history-detail"]')).isDisplayed()), 10_000)
  const detailAudit = await driver.executeScript(() => {
    const detail = document.querySelector('[data-testid="history-detail"]')
    const backgroundColor = detail ? getComputedStyle(detail).backgroundColor : ''
    const rgba = backgroundColor.match(/rgba?\(([^)]+)\)/)?.[1]?.split(',').map(value => value.trim()) || []
    const alpha = rgba.length === 4 ? Number(rgba[3]) : 1
    return {
      backgroundColor,
      alpha,
      overflow: detail ? detail.scrollWidth - detail.clientWidth : null,
    }
  })
  assert.equal(detailAudit.alpha, 1, `history detail must use an opaque surface: ${JSON.stringify(detailAudit)}`)
  assert.ok(detailAudit.backgroundColor && detailAudit.backgroundColor !== 'transparent')
  assert.ok((detailAudit.overflow ?? 0) <= 1, `history detail must not overflow horizontally: ${JSON.stringify(detailAudit)}`)
  writeFileSync(
    path.join(artifactDirectory, 'task-history-detail-compact.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )
}

async function runHfsxDesktopGate() {
  console.log('[desktop-e2e] verifying a non-empty HFSX image through the real application backend')
  assert.ok(existsSync(hfsxFixture), `HFSX fixture is missing; run npm.cmd run test:fixtures:hfsx: ${hfsxFixture}`)
  const outputPath = path.join(fixtureDirectory, 'hfsx-extracted')
  await callDesktopBridge('clearTasks')
  await callDesktopBridge('extractArchive', hfsxFixture, outputPath)

  const extractedPayload = path.join(outputPath, 'Firefox', 'known-payload.txt')
  assert.ok(existsSync(extractedPayload), `HFSX payload was not extracted: ${extractedPayload}`)
  assert.deepEqual(
    readFileSync(extractedPayload),
    Buffer.from('Long Decompress HFSX real payload\n', 'utf8'),
  )
}

async function runVaultUsageDesktopGate() {
  console.log('[desktop-e2e] verifying real vault password usage appears in the current local-day trend')
  const root = path.join(fixtureDirectory, 'vault-usage-gate')
  const sourceName = 'vault-usage-payload.txt'
  const sourcePath = path.join(root, sourceName)
  const archivePath = path.join(root, 'vault-usage.7z')
  const outputPath = path.join(root, 'extracted')
  const password = 'Long-Vault-Usage-2026!'
  const payload = `vault usage ${new Date().toISOString()}\n`
  mkdirSync(root, { recursive: true })
  writeFileSync(sourcePath, payload, 'utf8')
  const entryId = await callDesktopBridge('seedVaultPassword', 'Desktop E2E 当天趋势', password)

  const installationKeyPath = path.join(e2eDataDirectory, 'installation.key')
  const entryPath = path.join(e2eDataDirectory, 'passwords', `${entryId}.json`)
  const installationKey = readFileSync(installationKeyPath, 'utf8')
  const storedEntryText = readFileSync(entryPath, 'utf8')
  const storedEntry = JSON.parse(storedEntryText)
  assert.match(
    installationKey,
    /^long-dpapi:v1:/,
    'the real Windows installation key must be wrapped by current-user DPAPI',
  )
  assert.equal(
    installationKey.includes(password),
    false,
    'the installation-key file must not contain the archive password',
  )
  assert.equal(
    storedEntryText.includes(password),
    false,
    'the real password entry JSON must not contain the archive password in plaintext',
  )
  assert.match(storedEntry.password, /^long-vault:v2:/)
  assert.equal(storedEntry.encryption_version, 2)
  assert.equal(storedEntry.encryption_algorithm, 'AES256GCM+WindowsDPAPI')

  const packed = spawnSync(
    bundledSevenZip,
    ['a', '-t7z', `-p${password}`, '-mhe=on', '-y', archivePath, sourceName],
    { cwd: root, encoding: 'utf8', windowsHide: true },
  )
  assert.equal(packed.status, 0, packed.stderr || packed.stdout)

  await restartDesktopSession()
  await callDesktopBridge('extractArchive', archivePath, outputPath)
  assert.equal(readFileSync(path.join(outputPath, sourceName), 'utf8'), payload)

  await (await driver.findElement(By.css('[data-testid="nav-Vault"]'))).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/vault'), 15_000)
  await (await waitForElement('[data-testid="vault-analytics-trigger"]')).click()
  await waitForElement('[data-testid="vault-analytics-modal"]')
  await (await waitForElement('[data-testid="vault-range-7d"]')).click()
  const audit = await driver.wait(async () => {
    const result = await driver.executeScript(() => {
      const counts = [...document.querySelectorAll('[data-testid="vault-usage-day-count"]')]
      const labels = counts.map(node => node.parentElement?.querySelector('span')?.textContent?.trim())
      return {
        lastCount: counts.at(-1)?.textContent?.trim(),
        lastLabel: labels.at(-1),
        total: document.querySelector('[data-testid="vault-range-usage-total"]')?.textContent?.trim(),
      }
    })
    return result.lastCount === '1' ? result : false
  }, 15_000)
  const now = new Date()
  const expectedLabel = `${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
  assert.equal(audit.lastLabel, expectedLabel)
  assert.match(audit.total || '', /1\s*次/)

  const panoramaSemantics = await driver.executeScript(() => {
    const modal = document.querySelector('[data-testid="vault-analytics-modal"]')
    const text = modal?.textContent || ''
    return {
      hasArchiveTitle: text.includes('解压密码库数据全景'),
      hasHitTiers: text.includes('调用层级分布'),
      hasArchiveCoverage: text.includes('归档线索覆盖'),
      hasTraditionalStrength: text.includes('密码强度分布'),
      hasRiskRadar: text.includes('风险雷达'),
    }
  })
  assert.deepEqual(panoramaSemantics, {
    hasArchiveTitle: true,
    hasHitTiers: true,
    hasArchiveCoverage: true,
    hasTraditionalStrength: false,
    hasRiskRadar: false,
  })

  await driver.executeScript(() => {
    document.querySelector('[data-testid="vault-range-7d"]')?.scrollIntoView({ block: 'center' })
  })
  mkdirSync(artifactDirectory, { recursive: true })
  writeFileSync(
    path.join(artifactDirectory, 'vault-archive-panorama.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  await driver.executeScript(() => {
    document.querySelector('[data-testid="vault-analytics-modal"] button[aria-label="关闭"]')?.click()
  })
  await driver.wait(
    async () => (await driver.findElements(By.css('[data-testid="vault-analytics-modal"]'))).length === 0,
    10_000,
  )
  await (await waitForElement('[data-testid="vault-entry-usage"]')).click()
  await waitForElement('[data-testid="vault-entry-profile"]')
  const entrySemantics = await driver.executeScript(() => {
    const modal = document.querySelector('[data-testid="vault-analytics-modal"]')
    const text = modal?.textContent || ''
    return {
      hasEntryName: text.includes('Desktop E2E 当天趋势'),
      hasEntryTimeline: text.includes('单条解压密码使用趋势'),
      hasArchiveContext: text.includes('归档适用信息'),
      hasOverallHitTiers: text.includes('调用层级分布'),
      hasPasswordLength: text.includes('正文长度'),
    }
  })
  assert.deepEqual(entrySemantics, {
    hasEntryName: true,
    hasEntryTimeline: true,
    hasArchiveContext: true,
    hasOverallHitTiers: false,
    hasPasswordLength: false,
  })
  writeFileSync(
    path.join(artifactDirectory, 'vault-entry-unlock-profile.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )
}

async function runEncryptedRarDesktopGate() {
  console.log('[desktop-e2e] preparing pinned upstream encrypted RAR fixture')
  runFixtureCommand(
    process.execPath,
    [path.join(root, 'scripts', 'fetch-archive-test-fixtures.mjs')],
    'encrypted RAR fixture',
  )
  const encryptedRar = path.join(
    externalFixtureDirectory,
    'libarchive-rar-encrypted.rar',
  )
  assert.ok(existsSync(encryptedRar), 'the pinned encrypted RAR fixture must exist')

  console.log('[desktop-e2e] verifying encrypted RAR wrong-password rejection')
  const wrongPasswordStartedAt = Date.now()
  const wrongPasswordOutput = path.join(fixtureDirectory, 'rar-encrypted-wrong-password')
  const wrongPasswordError = await callDesktopBridgeFailure(
    'extractArchive',
    encryptedRar,
    wrongPasswordOutput,
    'wrong-password',
  )
  assert.match(
    wrongPasswordError,
    /password|encrypted|decrypt|checksum|crc|密码|解密/i,
    `encrypted RAR must report a password-related failure: ${wrongPasswordError}`,
  )
  assert.equal(
    existsSync(path.join(wrongPasswordOutput, 'foo.txt')),
    false,
    'wrong RAR password must not publish decrypted output',
  )
  assert.ok(
    Date.now() - wrongPasswordStartedAt < 60_000,
    'wrong RAR password rejection must complete within 60 seconds',
  )
  await callDesktopBridge('clearTasks')

  console.log('[desktop-e2e] verifying encrypted RAR correct-password extraction')
  const correctPasswordOutput = path.join(fixtureDirectory, 'rar-encrypted-correct-password')
  await callDesktopBridge('extractArchive', encryptedRar, correctPasswordOutput, '12345678')
  assert.equal(
    fileSha256(path.join(correctPasswordOutput, 'foo.txt')),
    '325d7b459b439684cad8825cbf2e488de15518103de09c56a42d6b1875081ee7',
    'encrypted RAR foo.txt must match the pinned plaintext',
  )
  assert.equal(
    fileSha256(path.join(correctPasswordOutput, 'bar.txt')),
    '7113d093a90b4a5cbac15a3bc8e85efbac50556c2a1f58f70a283cb2c373f1d5',
    'encrypted RAR bar.txt must match the pinned plaintext',
  )
  await callDesktopBridge('clearTasks')
}

async function runResourcePreflightLayoutDesktopGate() {
  console.log('[desktop-e2e] verifying shared resource-preflight layout in real compression and decompression details')
  const gateRoot = path.join(fixtureDirectory, 'resource-preflight-layout')
  const sourcePath = path.join(gateRoot, 'resource-layout-payload.txt')
  const archivePath = path.join(gateRoot, 'resource-layout-payload.zip')
  const extractedPath = path.join(gateRoot, 'resource-layout-payload', 'resource-layout-payload.txt')
  const payload = `resource preflight layout ${new Date().toISOString()}\n`
  mkdirSync(gateRoot, { recursive: true })
  writeFileSync(sourcePath, payload, 'utf8')
  await callDesktopBridge('clearTasks')
  await callDesktopBridge('clearCompressionWorkspace')
  await driver.manage().window().setRect({ width: 980, height: 720 })

  forwardContextAction('--quick-pack', [sourcePath])
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/compress'), 30_000)
  await waitForStableFile(archivePath)
  const compressionTask = await waitForResourcePreflightTask(
    'compression',
    task => task.status === 'completed' && normalizedDesktopPath(task.outputPath) === normalizedDesktopPath(archivePath),
  )
  assertRealResourceReport(compressionTask, {
    operation: 'compression',
    taskStatus: 'completed',
    canStart: true,
    reportStatuses: ['ready', 'warning'],
    outputPath: archivePath,
  })
  await assertVisibleResourceCard(
    compressionTask.id,
    compressionTask.report.status === 'ready' ? '已通过' : '需留意',
  )
  mkdirSync(artifactDirectory, { recursive: true })
  writeFileSync(
    path.join(artifactDirectory, 'resource-preflight-compression.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  forwardContextAction('--quick-extract', [archivePath])
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/decompress'), 30_000)
  await waitForFileContent(extractedPath, payload)
  const decompressionTask = await waitForResourcePreflightTask(
    'decompression',
    task => task.status === 'completed' && normalizedDesktopPath(task.outputPath) === normalizedDesktopPath(path.dirname(archivePath)),
  )
  assertRealResourceReport(decompressionTask, {
    operation: 'decompression',
    taskStatus: 'completed',
    canStart: true,
    reportStatuses: ['ready', 'warning'],
    outputPath: path.dirname(archivePath),
  })
  await assertVisibleResourceCard(
    decompressionTask.id,
    decompressionTask.report.status === 'ready' ? '已通过' : '需留意',
  )
  writeFileSync(
    path.join(artifactDirectory, 'resource-preflight-decompression.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )
}

async function assertBoundedTaskDetailLayout(type) {
  const selectors = type === 'compression'
    ? {
        details: '[data-testid="compression-draft-details"]',
        config: '[data-testid="compression-draft-config"]',
        execution: '[data-testid="compression-draft-execution"]',
        log: '[data-testid="compression-log-viewport"]',
      }
    : {
        details: '[data-testid="decompression-task-details"]',
        config: '[data-testid="decompression-config-panel"]',
        execution: '[data-testid="decompression-execution-panel"]',
        log: '[data-testid="decompression-log-viewport"]',
      }

  const result = await driver.executeScript((target) => {
    const details = document.querySelector(target.details)
    const config = document.querySelector(target.config)
    const execution = document.querySelector(target.execution)
    const log = document.querySelector(target.log)
    const resource = details?.querySelector('[data-testid="resource-preflight-card"]')
    const metrics = resource?.querySelector('[data-testid="resource-preflight-metrics"]')
    if (!details || !config || !execution || !log || !resource || !metrics) return null
    config.style.scrollBehavior = 'auto'
    config.scrollTop = config.scrollHeight
    const detailRect = details.getBoundingClientRect()
    const configRect = config.getBoundingClientRect()
    const executionRect = execution.getBoundingClientRect()
    const resourceRect = resource.getBoundingClientRect()
    const visibleResourceHeight = Math.max(
      0,
      Math.min(resourceRect.bottom, configRect.bottom) - Math.max(resourceRect.top, configRect.top),
    )
    return {
      detailWidth: details.clientWidth,
      detailHeight: details.clientHeight,
      detailHorizontalOverflow: details.scrollWidth - details.clientWidth,
      configWidth: config.clientWidth,
      configHeight: config.clientHeight,
      configHorizontalOverflow: config.scrollWidth - config.clientWidth,
      executionWidth: execution.clientWidth,
      executionHeight: execution.clientHeight,
      executionHorizontalOverflow: execution.scrollWidth - execution.clientWidth,
      columnTopDelta: Math.abs(configRect.top - executionRect.top),
      columnHeightDelta: Math.abs(configRect.height - executionRect.height),
      isSideBySide: executionRect.left >= configRect.right - 2,
      logClientHeight: log.clientHeight,
      logScrollHeight: log.scrollHeight,
      logHorizontalOverflow: log.scrollWidth - log.clientWidth,
      logOverflowY: getComputedStyle(log).overflowY,
      resourceWidth: resource.clientWidth,
      resourceHorizontalOverflow: resource.scrollWidth - resource.clientWidth,
      resourceVisibleHeight: visibleResourceHeight,
      resourceHeight: resourceRect.height,
      metricColumns: getComputedStyle(metrics).gridTemplateColumns.split(' ').filter(Boolean).length,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
      detailRectWidth: detailRect.width,
    }
  }, selectors)

  assert.ok(result, `${type} detail layout was not rendered`)
  assert.equal(result.isSideBySide, true, `${type} detail columns must remain side by side`)
  assert.ok(result.configWidth >= 250, `${type} config panel is too narrow: ${JSON.stringify(result)}`)
  assert.ok(result.executionWidth >= 250, `${type} execution panel is too narrow: ${JSON.stringify(result)}`)
  assert.ok(result.configHeight >= 340, `${type} config viewport is too short: ${JSON.stringify(result)}`)
  assert.ok(result.detailHeight <= 520, `${type} details grew without a vertical bound: ${JSON.stringify(result)}`)
  assert.ok(result.columnTopDelta <= 2, `${type} columns are not aligned: ${JSON.stringify(result)}`)
  assert.ok(result.columnHeightDelta <= 2, `${type} columns do not share one viewport height: ${JSON.stringify(result)}`)
  assert.ok(result.logScrollHeight > result.logClientHeight, `${type} log fixture must require vertical scrolling`)
  assert.equal(result.logOverflowY, 'auto', `${type} log must own the vertical scrollbar`)
  assert.ok(result.resourceVisibleHeight >= Math.min(120, result.resourceHeight * 0.7), `${type} resource card is clipped: ${JSON.stringify(result)}`)
  assert.ok(
    result.metricColumns >= 1 && result.metricColumns <= 2,
    `${type} resource metrics must use a readable one- or two-column layout`,
  )
  for (const [label, overflow] of Object.entries({
    details: result.detailHorizontalOverflow,
    config: result.configHorizontalOverflow,
    execution: result.executionHorizontalOverflow,
    log: result.logHorizontalOverflow,
    resource: result.resourceHorizontalOverflow,
  })) {
    assert.ok(overflow <= 1, `${type} ${label} has horizontal overflow: ${JSON.stringify(result)}`)
  }
  return result
}

async function runResponsiveTaskDetailDesktopGate() {
  console.log('[desktop-e2e] verifying bounded side-by-side task details at real window sizes')
  mkdirSync(artifactDirectory, { recursive: true })

  for (const type of ['decompression', 'compression']) {
    await callDesktopBridge('seedResponsiveWorkspace', type)
    const targetHash = `#/${type === 'compression' ? 'compress' : 'decompress'}`
    await driver.executeScript(hash => { window.location.hash = hash }, targetHash)
    await driver.wait(async () => (await driver.getCurrentUrl()).includes(type === 'compression' ? '#/compress' : '#/decompress'), 10_000)
    if (type === 'decompression') {
      await (await waitForElement('[data-task-id="responsive-decompression"]')).click()
      await waitForElement('[data-testid="decompression-task-details"]')
    } else {
      await waitForElement('[data-testid="compression-draft-details"]')
    }

    for (const size of [{ width: 920, height: 620 }, { width: 760, height: 520 }]) {
      await driver.manage().window().setRect(size)
      await new Promise(resolve => setTimeout(resolve, 250))
      await assertBoundedTaskDetailLayout(type)
      writeFileSync(
        path.join(artifactDirectory, `responsive-${type}-${size.width}x${size.height}.png`),
        Buffer.from(await driver.takeScreenshot(), 'base64'),
      )
    }
  }
}

async function runPdfWorkspaceDesktopGate() {
  console.log('[desktop-e2e] verifying real D-04.3 PDF cancellation, failure isolation, restart history and default-reader UI')
  await driver.manage().window().setRect({ width: 1600, height: 1000 })
  const pdfRoot = path.join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'pdfs')
  const batchRoot = path.join(fixtureDirectory, 'pdf-d04-3')
  mkdirSync(batchRoot, { recursive: true })
  for (const name of ['form.pdf', 'text-vector.pdf', 'mixed-content.pdf', 'large-image.pdf']) {
    copyFileSync(path.join(pdfRoot, name), path.join(batchRoot, name))
  }
  const fixturePaths = {
    'form.pdf': path.join(batchRoot, 'form.pdf'),
    'text-vector.pdf': path.join(batchRoot, 'text-vector.pdf'),
    'mixed-content.pdf': path.join(batchRoot, 'mixed-content.pdf'),
    'large-image.pdf': path.join(batchRoot, 'large-image.pdf'),
    'signed.pdf': path.join(pdfRoot, 'signed.pdf'),
    'encrypted.pdf': path.join(pdfRoot, 'encrypted.pdf'),
  }
  const fixtures = Object.fromEntries(Object.entries(fixturePaths).map(([name, fixturePath]) => {
    assert.equal(existsSync(fixturePath), true, `missing real PDF fixture: ${fixturePath}`)
    return [name, { path: fixturePath, bytes: statSync(fixturePath).size, sha256: fileSha256(fixturePath) }]
  }))
  await callDesktopBridge('clearTasks')
  await callDesktopBridge('clearTaskHistory')

  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  await (await waitForElement('[data-testid="compression-mode-pdf"]')).click()
  let workspace = await waitForElement('[data-testid="pdf-compression-workspace"]')
  let picker = await waitForElement('[data-testid="pdf-compression-workspace"] [data-testid="dropzone-file"]')
  await callDesktopBridge('queueDesktopDialogSelections', [[fixtures['large-image.pdf'].path]])
  await picker.click()
  await driver.wait(async () => (await workspace.getText()).includes('large-image.pdf') && (await workspace.getText()).includes('可配置'), 30_000)
  await (await waitForElement('[data-testid="pdf-mode-image"]')).click()
  await (await waitForElement('[data-testid="pdf-risk-confirmation"]')).click()
  await (await waitForElement('[data-testid="pdf-allow-larger-output"]')).click()
  await (await waitForElement('[data-testid="pdf-freeze-configuration"]')).click()
  await (await waitForElement('[data-testid="pdf-start-batch"]')).click()
  const cancelBatch = await waitForElement('[data-testid="pdf-cancel-batch"]', 30_000)
  await cancelBatch.click()
  const cancelledHistory = await driver.wait(async () => {
    const records = await callDesktopBridge('taskHistory')
    return records.find(record => record.workloadKind === 'pdf' && record.status === 'cancelled') || false
  }, 30_000)
  assert.equal(existsSync(cancelledHistory.outputPath), false, 'cancelled PDF must not publish a final output')
  assert.equal(fileSha256(fixtures['large-image.pdf'].path), fixtures['large-image.pdf'].sha256, 'cancelled PDF source must remain unchanged')

  await callDesktopBridge('reset')
  await (await waitForElement('[data-testid="nav-History"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/history'), 15_000)
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  await (await waitForElement('[data-testid="compression-mode-pdf"]')).click()
  workspace = await waitForElement('[data-testid="pdf-compression-workspace"]')
  picker = await waitForElement('[data-testid="pdf-compression-workspace"] [data-testid="dropzone-file"]')

  await callDesktopBridge('queueDesktopDialogSelections', [[fixtures['form.pdf'].path]])
  await picker.click()
  await driver.wait(async () => (await workspace.getText()).includes('form.pdf') && (await workspace.getText()).includes('表单字段'), 30_000)
  const formText = await workspace.getText()
  const formExpected = {
    pageCount: 1,
    inputBytes: fixtures['form.pdf'].bytes,
    formFieldCount: 2,
    defaultOutputSuffix: 'form.organized.pdf',
    sourceMutation: false,
  }
  const formActual = {
    pageCount: /页数\s*1/u.test(formText) ? 1 : null,
    inputBytes: fixtures['form.pdf'].bytes,
    formFieldCount: /表单字段\s*2/u.test(formText) ? 2 : null,
    defaultOutputSuffix: formText.includes('form.organized.pdf') ? 'form.organized.pdf' : null,
    sourceMutation: fileSha256(fixtures['form.pdf'].path) !== fixtures['form.pdf'].sha256,
  }

  const imageMode = await waitForElement('[data-testid="pdf-mode-image"]')
  await imageMode.click()
  const freeze = await waitForElement('[data-testid="pdf-freeze-configuration"]')
  assert.notEqual(await freeze.getAttribute('disabled'), null, 'lossy mode must stay disabled before explicit confirmation')
  await (await waitForElement('.risk-confirmation')).click()
  assert.equal(await freeze.getAttribute('disabled'), null, 'explicit lossy confirmation must enable local draft freezing')
  const allowLarger = await waitForElement('[data-testid="pdf-allow-larger-output"]')
  assert.equal(await allowLarger.isSelected(), false, 'larger PDF retention must default to disabled')
  await allowLarger.click()
  assert.equal(await allowLarger.isSelected(), true, 'larger PDF retention requires an explicit user action')
  const enabledFreeze = await waitForElement('[data-testid="pdf-freeze-configuration"]')
  await driver.executeScript('arguments[0].scrollIntoView({ block: "center" })', enabledFreeze)
  await enabledFreeze.click()
  await driver.wait(async () => (await workspace.getText()).includes('配置已锁定'), 10_000)
  assert.match(await workspace.getText(), /form\.optimized\.pdf/u)

  await callDesktopBridge('queueDesktopDialogSelections', [[fixtures['text-vector.pdf'].path, fixtures['mixed-content.pdf'].path]])
  await picker.click()
  await driver.wait(async () => {
    const cards = await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))
    return cards.length === 3 && (await Promise.all(cards.map(card => card.getText()))).every(text => !text.includes('分析中'))
  }, 30_000)
  const safeCards = await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))
  for (const card of safeCards.slice(1, 3)) {
    const freezeButton = await card.findElement(By.css('[data-testid="pdf-freeze-configuration"]'))
    assert.equal(await freezeButton.getAttribute('disabled'), null, 'safe PDF must allow configuration locking')
    await freezeButton.click()
  }
  await driver.wait(async () => (await workspace.getText()).match(/配置已锁定/gu)?.length === 3, 10_000)

  await callDesktopBridge('queueDesktopDialogSelections', [[fixtures['signed.pdf'].path]])
  await picker.click()
  await driver.wait(async () => (await workspace.getText()).includes('signed.pdf') && (await workspace.getText()).includes('当前仅可分析'), 30_000)
  const signedCard = (await driver.findElements(By.css('[data-testid="pdf-draft-card"]')))[3]
  const signedFreeze = await signedCard.findElement(By.css('[data-testid="pdf-freeze-configuration"]'))
  assert.notEqual(await signedFreeze.getAttribute('disabled'), null, 'signed PDF must not freeze an execution configuration')

  await callDesktopBridge('queueDesktopDialogSelections', [[fixtures['encrypted.pdf'].path]])
  await picker.click()
  await driver.wait(async () => (await workspace.getText()).includes('encrypted.pdf') && (await workspace.getText()).includes('需要正确密码'), 30_000)
  let passwordInput = await waitForElement('[data-testid="pdf-password-input"]')
  await passwordInput.sendKeys('wrong-password')
  await (await waitForElement('[data-testid="pdf-password-analyze"]')).click()
  const wrongPasswordError = await driver.wait(async () => {
    const alerts = await driver.findElements(By.css('[data-testid="pdf-draft-card"] [role="alert"]'))
    const text = alerts.length ? await alerts.at(-1).getText() : ''
    return text.includes('PDF_ANALYSIS_INVALID_PASSWORD') ? text : false
  }, 30_000)
  passwordInput = await waitForElement('[data-testid="pdf-password-input"]')
  assert.equal(await passwordInput.getAttribute('value'), '', 'password field must clear after a failed attempt')
  await passwordInput.sendKeys('fixture-user')
  await (await waitForElement('[data-testid="pdf-password-analyze"]')).click()
  await driver.wait(async () => (await workspace.getText()).includes('密码已验证'), 30_000)
  assert.equal((await driver.findElements(By.css('[data-testid="pdf-password-input"]'))).length, 0, 'accepted password must not remain in the DOM')
  const encryptedCard = (await driver.findElements(By.css('[data-testid="pdf-draft-card"]')))[4]
  const encryptedFreeze = await encryptedCard.findElement(By.css('[data-testid="pdf-freeze-configuration"]'))
  assert.notEqual(await encryptedFreeze.getAttribute('disabled'), null, 'encrypted PDF must remain analysis-only after password verification')
  assert.match(await encryptedCard.getText(), /PDF_ENCRYPTED_EXECUTION_UNSUPPORTED/u)

  mkdirSync(artifactDirectory, { recursive: true })
  const layouts = []
  for (const size of [{ width: 1100, height: 720 }, { width: 760, height: 560 }]) {
    await driver.manage().window().setRect(size)
    await new Promise(resolve => setTimeout(resolve, 250))
    const layout = await driver.executeScript(() => {
      const main = document.querySelector('main')
      const workspace = document.querySelector('[data-testid="pdf-compression-workspace"]')
      if (!main || !workspace) return null
      workspace.scrollLeft = 0
      const boundary = workspace.getBoundingClientRect()
      const offenders = [...workspace.querySelectorAll('*')].map(element => {
        const rect = element.getBoundingClientRect()
        return { tag: element.tagName, className: String(element.className), rightOverflow: Math.round(rect.right - boundary.right), leftOverflow: Math.round(boundary.left - rect.left), width: Math.round(rect.width) }
      }).filter(item => item.rightOverflow > 1 || item.leftOverflow > 1).sort((left, right) => Math.max(right.rightOverflow, right.leftOverflow) - Math.max(left.rightOverflow, left.leftOverflow)).slice(0, 8)
      return {
        mainOverflow: main.scrollWidth - main.clientWidth,
        workspaceOverflow: workspace.scrollWidth - workspace.clientWidth,
        workspaceClientWidth: workspace.clientWidth,
        workspaceScrollWidth: workspace.scrollWidth,
        offenders,
      }
    })
    assert.ok(layout, 'PDF workspace layout must be visible')
    assert.ok(layout.mainOverflow <= 1 && layout.workspaceOverflow <= 1, `PDF workspace must not overflow horizontally: ${JSON.stringify(layout)}`)
    layouts.push({ ...size, ...layout })
    writeFileSync(path.join(artifactDirectory, `pdf-workspace-${size.width}x${size.height}.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }

  const startBatch = await waitForElement('[data-testid="pdf-start-batch"]')
  assert.equal(await startBatch.getAttribute('disabled'), null, 'three frozen safe PDFs must enable batch execution')
  rmSync(fixtures['form.pdf'].path)
  await startBatch.click()
  await driver.wait(async () => (await driver.findElement(By.css('body')).getText()).includes('PDF 处理结束：2 个完成，1 个失败，0 个取消'), 120_000)
  const batchHistory = await driver.wait(async () => {
    const records = (await callDesktopBridge('taskHistory')).filter(record => record.workloadKind === 'pdf')
    return records.length === 4 ? records : false
  }, 30_000)
  const failed = batchHistory.find(record => record.status === 'failed')
  const completed = batchHistory.filter(record => record.status === 'completed')
  assert.ok(failed, 'deleted first source must create one failed PDF history row')
  assert.equal(failed.metrics, null, 'failed PDF history must not invent measured metrics')
  assert.equal(existsSync(failed.outputPath), false, 'failed PDF must not publish a final output')
  assert.equal(completed.length, 2, 'a failed first item must not stop the remaining PDF batch')
  for (const record of completed) {
    assert.equal(record.workloadKind, 'pdf')
    assert.equal(record.metrics?.media?.pageCount, 1)
    assert.ok(record.metrics?.inputBytes > 0)
    assert.ok(record.metrics?.outputBytes > 0)
    assert.equal(record.metrics.outputBytes, statSync(record.outputPath).size)
    assert.equal(existsSync(record.outputPath), true, 'published PDF output must exist before history completion')
  }
  assert.equal(batchHistory.filter(record => record.status === 'cancelled').length, 1)
  const defaultOpen = (await driver.findElements(By.css('[data-testid="pdf-open-default-app"]')))[0]
  await defaultOpen.click()
  await driver.wait(async () => (await driver.findElement(By.css('body')).getText()).includes('已将 PDF 交给系统默认阅读器'), 30_000)

  for (const [name, fixture] of Object.entries(fixtures).filter(([name]) => name !== 'form.pdf')) {
    assert.equal(fileSha256(fixture.path), fixture.sha256, `${name} source bytes must remain unchanged`)
  }
  const persistedBeforeRestart = batchHistory.map(record => ({
    id: record.id,
    status: record.status,
    sourcePaths: record.sourcePaths.map(normalizedDesktopPath),
    outputPath: normalizedDesktopPath(record.outputPath),
    metrics: record.metrics,
  })).sort((left, right) => left.id.localeCompare(right.id))
  await restartDesktopSession()
  const persistedAfterRestart = await driver.wait(async () => {
    const records = (await callDesktopBridge('taskHistory')).filter(record => record.workloadKind === 'pdf')
      .map(record => ({
        id: record.id,
        status: record.status,
        sourcePaths: record.sourcePaths.map(normalizedDesktopPath),
        outputPath: normalizedDesktopPath(record.outputPath),
        metrics: record.metrics,
      })).sort((left, right) => left.id.localeCompare(right.id))
    return records.length === 4 ? records : false
  }, 30_000)
  assert.deepEqual(persistedAfterRestart, persistedBeforeRestart, 'PDF completed/failed/cancelled history must survive a complete restart')
  const differences = Object.keys(formExpected).filter(key => JSON.stringify(formExpected[key]) !== JSON.stringify(formActual[key]))
  const wrongPasswordDifferences = wrongPasswordError.includes('PDF_ANALYSIS_INVALID_PASSWORD') ? [] : ['error']
  const differenceCount = differences.length + wrongPasswordDifferences.length
  const evidence = {
    schemaVersion: 1,
    node: 'D-04.3',
    testKind: 'real-windows-tauri-ui-cancellation-failure-isolation-restart-history-and-default-reader',
    expectedVsActual: [
      { case: 'form-facts-and-default-output', expected: formExpected, actual: formActual, differences },
      { case: 'wrong-password', expected: 'PDF_ANALYSIS_INVALID_PASSWORD', actual: wrongPasswordError, differences: wrongPasswordDifferences },
      { case: 'source-integrity', expected: 'all-unchanged', actual: 'all-unchanged', differences: [] },
      { case: 'cancelled-large-image', expected: { status: 'cancelled', outputAbsent: true }, actual: { status: cancelledHistory.status, outputAbsent: !existsSync(cancelledHistory.outputPath) }, differences: [] },
      { case: 'batch-failure-isolation', expected: { completed: 2, failed: 1 }, actual: { completed: completed.length, failed: failed ? 1 : 0 }, differences: [] },
      { case: 'restart-history', expected: 4, actual: persistedAfterRestart.length, differences: [] },
    ],
    layouts,
    differenceCount,
    passed: differenceCount === 0,
  }
  writeFileSync(path.join(artifactDirectory, 'pdf-workspace-result.json'), JSON.stringify(evidence, null, 2), 'utf8')
  assert.equal(evidence.differenceCount, 0, `D-04.3 expected/actual differences remain: ${evidence.differenceCount}`)
}

async function runImageWorkspaceDesktopGate() {
  console.log('[desktop-e2e] verifying the real B-04.5 image execution, results, history and queue isolation')
  // Focused runs inherit the OS-scaled default window, which can be smaller
  // than the functional result viewport. Establish a deterministic working
  // size before execution; the dedicated loop below still audits both compact
  // product sizes for horizontal overflow.
  await driver.manage().window().setRect({ width: 1600, height: 1000 })
  const mediaRoot = path.join(root, 'test-results', 'media-fixture-audit', 'fixtures')
  const acceptedImageNames = ['exif-orientation.jpg', 'photo.webp', 'transparent.png']
  const imageFixtures = acceptedImageNames.map(name => {
    const expected = mediaFixtureManifest.images.find(item => item.file === name)
    assert.ok(expected, `missing image fixture contract: ${name}`)
    return {
      name,
      width: expected.displayWidth,
      height: expected.displayHeight,
      path: path.join(mediaRoot, 'images', name),
    }
  })
  const rejectedFixtures = [
    { name: 'animated.gif', path: path.join(mediaRoot, 'images', 'animated.gif') },
    { name: 'rejected-input.pdf', path: path.join(mediaRoot, 'pdfs', 'rejected-input.pdf') },
  ]
  for (const fixture of [...imageFixtures, ...rejectedFixtures]) {
    assert.equal(existsSync(fixture.path), true, `missing real media fixture: ${fixture.path}`)
    assert.ok(statSync(fixture.path).size > 0, `real media fixture is empty: ${fixture.path}`)
  }

  const archiveSeed = path.join(fixtureDirectory, 'archive-queue-must-survive.txt')
  writeFileSync(archiveSeed, 'B-02 queue isolation', 'utf8')
  await (await waitForElement('[data-testid="nav-Compress"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/compress'), 30_000)
  await callDesktopBridge('seedCompressionAnalysisWorkspace', [{
    name: path.basename(archiveSeed), path: archiveSeed, size: statSync(archiveSeed).size, isDirectory: false,
  }])

  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  await (await waitForElement('[data-testid="compression-mode-image"]')).click()
  await waitForElement('[data-testid="image-compression-workspace"]')
  const batchSettings = await waitForElement('[data-testid="image-compression-workspace"] .secondary-action')
  await batchSettings.click()
  const seed = await callDesktopBridge('seedImageCompressionWorkspace', [
    ...imageFixtures.map(fixture => ({ name: fixture.name, path: fixture.path, size: statSync(fixture.path).size, isDirectory: false })),
    ...rejectedFixtures.map(fixture => ({ name: fixture.name, path: fixture.path, size: statSync(fixture.path).size, isDirectory: false })),
  ])
  assert.equal(seed.accepted, 3, 'JPG, WebP and PNG must enter the image workspace')
  assert.equal(seed.rejected.length, 2, 'GIF and PDF must be rejected before task creation')
  assert.match(seed.rejected.find(item => item.name === 'animated.gif')?.reason || '', /GIF/)
  assert.match(seed.rejected.find(item => item.name === 'rejected-input.pdf')?.reason || '', /JPG|PNG|WebP/)

  const state = await driver.wait(async () => {
    const current = await callDesktopBridge('imageCompressionAuditState')
    return current.length === 3 && current.every(item => item.status === 'ready') ? current : false
  }, 30_000)
  for (const expected of imageFixtures) {
    const actual = state.find(item => item.name === expected.name)
    assert.deepEqual(
      actual && { width: actual.width, height: actual.height, inputSize: actual.inputSize },
      { width: expected.width, height: expected.height, inputSize: statSync(expected.path).size },
      `${expected.name} must use orientation-applied display dimensions and filesystem bytes`,
    )
  }

  const startButton = await waitForElement('[data-testid="image-compression-workspace"] .primary-action')
  assert.equal(await startButton.getAttribute('disabled'), null, 'verified image drafts must enable real execution')
  const imageOutputDirectory = path.join(fixtureDirectory, 'image-results')
  mkdirSync(imageOutputDirectory, { recursive: true })
  await callDesktopBridge('configureImageCompressionWorkspace', imageOutputDirectory)
  await startButton.click()
  const resultState = await driver.wait(async () => {
    const current = await callDesktopBridge('imageCompressionResultAuditState')
    return current.length === 3 && current.every(item => item.taskStatus === 'completed' && item.hasResultPreview)
      ? current
      : false
  }, 60_000)
  for (const expected of imageFixtures) {
    const actual = resultState.find(item => item.name === expected.name)
    assert.ok(actual, `missing real image result: ${expected.name}`)
    assert.equal(actual.inputBytes, statSync(expected.path).size, `${expected.name} input bytes must come from the real source`)
    assert.ok(actual.outputPath && existsSync(actual.outputPath), `${expected.name} must publish a real output file`)
    assert.equal(actual.outputBytes, statSync(actual.outputPath).size, `${expected.name} output bytes must match the real file`)
    assert.ok(actual.outputWidth > 0 && actual.outputHeight > 0, `${expected.name} must expose verified output dimensions`)
    assert.match(actual.outputFormat || '', /jpeg|png|webp/)
  }
  const imageHistory = (await callDesktopBridge('taskHistory')).filter(record =>
    record.workloadKind === 'image' && record.outputPath?.includes(imageOutputDirectory),
  )
  assert.equal(imageHistory.length, 3, 'every real published image must persist one unified history row')
  assert.ok(imageHistory.every(record => record.status === 'completed' && record.metrics), 'real image history must contain completed measured results')
  const imageWorkspaceLayout = await driver.executeScript(() => {
    const page = document.querySelector('.special-compression-view')
    const workspace = document.querySelector('[data-testid="image-compression-workspace"]')
    const boundary = document.querySelector('[data-testid="image-compression-workspace"] .truth-boundary')
    const list = document.querySelector('[data-testid="image-compression-workspace"] .image-list-shell')
    const facts = (element) => element ? {
      clientHeight: element.clientHeight,
      scrollHeight: element.scrollHeight,
      top: element.getBoundingClientRect().top,
      bottom: element.getBoundingClientRect().bottom,
      display: getComputedStyle(element).display,
      overflow: getComputedStyle(element).overflow,
    } : null
    return { page: facts(page), workspace: facts(workspace), boundary: facts(boundary), list: facts(list) }
  })
  console.log(`[desktop-e2e] image workspace layout: ${JSON.stringify(imageWorkspaceLayout)}`)
  const workspaceText = await (await waitForElement('[data-testid="image-compression-workspace"]')).getText()
  assert.doesNotMatch(workspaceText, /B-02|B-03|尚未生成结果文件/)
  assert.match(workspaceText, /实际字节差/)
  const resultPreview = await waitForElement('[data-testid="image-compression-workspace"] .result-ready img')
  assert.equal(await resultPreview.isDisplayed(), true, 'the verified output must render as the result preview')
  await waitForElement('.image-details')

  mkdirSync(artifactDirectory, { recursive: true })
  for (const size of [{ width: 1100, height: 720 }, { width: 760, height: 560 }]) {
    await driver.manage().window().setRect(size)
    await new Promise(resolve => setTimeout(resolve, 250))
    const layout = await driver.executeScript(() => {
      const main = document.querySelector('main')
      const workspace = document.querySelector('[data-testid="image-compression-workspace"]')
      const list = document.querySelector('.image-list')
      const details = document.querySelector('.image-details')
      const config = document.querySelector('.item-config')
      const comparison = document.querySelector('.comparison-panel')
      if (!main || !workspace || !list || !details || !config || !comparison) return null
      const configRect = config.getBoundingClientRect()
      const comparisonRect = comparison.getBoundingClientRect()
      return {
        mainOverflow: main.scrollWidth - main.clientWidth,
        workspaceOverflow: workspace.scrollWidth - workspace.clientWidth,
        listOverflow: list.scrollWidth - list.clientWidth,
        detailsOverflow: details.scrollWidth - details.clientWidth,
        sideBySide: comparisonRect.left >= configRect.right - 2,
        configWidth: configRect.width,
        comparisonWidth: comparisonRect.width,
      }
    })
    assert.ok(layout, 'image details must be visible')
    assert.equal(layout.sideBySide, true, `image details must remain side by side: ${JSON.stringify(layout)}`)
    assert.ok(layout.configWidth >= 190 && layout.comparisonWidth >= 190, `image detail columns are unreadable: ${JSON.stringify(layout)}`)
    for (const [label, overflow] of Object.entries(layout).filter(([key]) => key.endsWith('Overflow'))) {
      assert.ok(overflow <= 1, `image ${label} must not scroll horizontally: ${JSON.stringify(layout)}`)
    }
    writeFileSync(
      path.join(artifactDirectory, `image-workspace-${size.width}x${size.height}.png`),
      Buffer.from(await driver.takeScreenshot(), 'base64'),
    )
  }

  await (await waitForElement('[data-testid="nav-Compress"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).endsWith('#/compress'), 30_000)
  await waitForElement('[data-testid="compression-center"]')
  assert.match(await (await waitForElement('main')).getText(), /Desktop E2E 智能分析/)
  assert.doesNotMatch(await (await waitForElement('main')).getText(), /photo\.webp/)
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  await (await waitForElement('[data-testid="compression-mode-image"]')).click()
  await waitForElement('[data-testid="image-compression-workspace"]')
  const restoredImageNames = (await callDesktopBridge('imageCompressionAuditState'))
    .map(item => item.name)
    .sort()
  assert.deepEqual(
    restoredImageNames,
    acceptedImageNames.toSorted(),
    'switching workload modes must preserve the isolated image queue',
  )

  console.log('[desktop-e2e] verifying the visible image-picker handler path')
  await callDesktopBridge('reset')
  await driver.wait(async () => (await callDesktopBridge('imageCompressionAuditState')).length === 0, 10_000)
  await driver.manage().window().setRect({ width: 1100, height: 720 })
  await (await waitForElement('[data-testid="image-compression-workspace"] .secondary-action')).click()
  const nativePickerDropzone = await waitForElement('[data-testid="dropzone-file"]')
  await driver.wait(async () => nativePickerDropzone.isDisplayed(), 10_000)
  const pickerJpeg = path.join(mediaRoot, 'images', 'exif-orientation.jpg')
  const pickerGif = path.join(mediaRoot, 'images', 'animated.gif')
  await callDesktopBridge('queueDesktopDialogSelections', [[pickerJpeg, pickerGif]])
  await nativePickerDropzone.click()
  const pickerState = await driver.wait(async () => {
    const current = await callDesktopBridge('imageCompressionAuditState')
    return current.length === 1 && current[0].status === 'ready' ? current : false
  }, 30_000)
  assert.deepEqual(
    pickerState[0],
    {
      name: 'exif-orientation.jpg',
      status: 'ready',
      width: 360,
      height: 640,
      inputSize: statSync(pickerJpeg).size,
    },
    'the visible picker handler must preserve the real JPEG path, bytes and decoded dimensions',
  )
  const rejectionText = await driver.wait(async () => {
    const alerts = await driver.findElements(By.css('[role="alert"]'))
    for (const alert of alerts) {
      const text = await alert.getText()
      if (/animated\.gif.*GIF/.test(text)) return text
    }
    return false
  }, 10_000)
  assert.match(rejectionText, /animated\.gif.*GIF/)
  assert.deepEqual(
    (await callDesktopBridge('imageCompressionAuditState')).map(item => item.name),
    ['exif-orientation.jpg'],
    'the rejected GIF must not enter the image queue',
  )
  assert.equal(
    await driver.executeScript('return document.hasFocus()'),
    true,
    'the picker handler must leave the WebView focused',
  )
  writeFileSync(
    path.join(artifactDirectory, 'image-workspace-picker-handler.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )
}

function videoFfmpegProcessIds(sourcePath) {
  const sourceName = path.basename(sourcePath).replaceAll("'", "''")
  const result = spawnSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-Command',
      `Get-CimInstance Win32_Process | Where-Object { $_.Name -ieq 'ffmpeg.exe' -and $_.CommandLine -like '*${sourceName}*' } | ForEach-Object { $_.ProcessId }`,
    ],
    { encoding: 'utf8', windowsHide: true },
  )
  assert.ifError(result.error)
  assert.equal(result.status, 0, `failed to inspect product FFmpeg processes: ${result.stderr}`)
  return result.stdout
    .split(/\r?\n/u)
    .map(value => Number.parseInt(value.trim(), 10))
    .filter(Number.isInteger)
}

async function runVideoWorkspaceDesktopGate() {
  console.log('[desktop-e2e] verifying real C-05.1/C-05.3 video execution, cancellation, restart history and default playback')
  await driver.manage().window().setRect({ width: 1600, height: 1000 })
  const frozenVideo = path.join(root, 'tests', 'fixtures', 'media', 'videos', 'h264-vfr-audio-rotation-subtitles.mp4')
  const multiAudioVideo = path.join(fixtureDirectory, 'multi-audio-30s.mp4')
  const videoOutputDirectory = path.join(fixtureDirectory, 'video-results')
  const cancellationVideo = path.join(root, 'test-results', 'c05-video-long-large-matrix', 'inputs', 'avi-100mib-1080p.avi')
  const cancellationOutputDirectory = path.join(fixtureDirectory, 'video-cancel-results')
  assert.equal(existsSync(frozenVideo), true, `missing frozen video fixture: ${frozenVideo}`)
  assert.equal(existsSync(cancellationVideo), true, `missing C-05.3 cancellation fixture: ${cancellationVideo}`)
  mkdirSync(videoOutputDirectory, { recursive: true })
  mkdirSync(cancellationOutputDirectory, { recursive: true })
  const generated = spawnSync(productFfmpeg, [
    '-hide_banner', '-loglevel', 'error', '-stream_loop', '29', '-i', frozenVideo,
    '-map', '0:v:0', '-map', '0:a:0', '-map', '0:a:0', '-map', '0:s:0', '-c', 'copy', '-t', '30',
    '-metadata:s:a:0', 'language=eng', '-metadata:s:a:1', 'language=zho', '-y', multiAudioVideo,
  ], { encoding: 'utf8', windowsHide: true })
  assert.ifError(generated.error)
  assert.equal(generated.status, 0, generated.stderr || generated.stdout)
  assert.ok(statSync(multiAudioVideo).size > 0, 'real multi-audio fixture must be non-empty')

  await callDesktopBridge('clearTaskHistory')
  await callDesktopBridge('reset')
  const cancellationSourceBytes = statSync(cancellationVideo).size
  const cancellationSourceSha256 = fileSha256(cancellationVideo)
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  await (await waitForElement('[data-testid="compression-mode-video"]')).click()
  let workspace = await waitForElement('[data-testid="video-compression-workspace"]')
  let picker = await waitForElement('[data-testid="video-compression-workspace"] [data-testid="dropzone-file"]')
  await callDesktopBridge('queueDesktopDialogSelections', [[cancellationVideo]])
  await picker.click()
  await driver.wait(async () => {
    const cards = await driver.findElements(By.css('[data-testid="video-draft-card"]'))
    return cards.length === 1 && await cards[0].getAttribute('data-status') === 'ready'
  }, 30_000)
  await callDesktopBridge('queueDesktopDialogSelections', [cancellationOutputDirectory])
  await (await waitForElement('[data-testid="video-compression-workspace"] .output-directory button')).click()
  await driver.wait(async () => (await workspace.getText()).includes(cancellationOutputDirectory), 15_000)
  await (await waitForElement('[data-testid="video-compression-workspace"] .primary-action')).click()
  await driver.wait(async () => {
    const cards = await driver.findElements(By.css('[data-testid="video-draft-card"]'))
    return cards.length === 1 && await cards[0].getAttribute('data-status') === 'compressing'
  }, 30_000)
  const observedFfmpegProcessIds = await driver.wait(() => {
    const processIds = videoFfmpegProcessIds(cancellationVideo)
    return processIds.length > 0 ? processIds : false
  }, 30_000)
  await (await waitForElement('[data-testid="video-compression-workspace"] .danger-action')).click()
  await driver.wait(async () => {
    const cards = await driver.findElements(By.css('[data-testid="video-draft-card"]'))
    return cards.length === 1 && await cards[0].getAttribute('data-status') === 'cancelled'
  }, 30_000)
  await driver.wait(() => videoFfmpegProcessIds(cancellationVideo).length === 0, 30_000)
  const cancelledHistory = await driver.wait(async () => {
    const record = (await callDesktopBridge('taskHistory')).find(candidate =>
      candidate.workloadKind === 'video'
      && candidate.status === 'cancelled'
      && candidate.sourcePaths.some(source => normalizedDesktopPath(source) === normalizedDesktopPath(cancellationVideo)))
    return record || false
  }, 30_000)
  assert.equal(existsSync(cancelledHistory.outputPath), false, 'cancelled video must not publish a final output')
  assert.equal(
    readdirSync(cancellationOutputDirectory).some(name => name.includes('.video-encode-')),
    false,
    'cancelled video must not leave staging output',
  )
  assert.equal(statSync(cancellationVideo).size, cancellationSourceBytes, 'cancellation must not resize the source')
  assert.equal(fileSha256(cancellationVideo), cancellationSourceSha256, 'cancellation must not mutate the source')
  const cancellationAudit = {
    sourcePath: cancellationVideo,
    sourceBytes: cancellationSourceBytes,
    sourceSha256: cancellationSourceSha256,
    observedFfmpegProcessIds,
    historyTaskId: cancelledHistory.id,
    historyStatus: cancelledHistory.status,
    unpublishedOutputPath: cancelledHistory.outputPath,
    processExited: true,
    stagingCleaned: true,
  }
  console.log('[desktop-e2e] real product FFmpeg cancellation cleaned process, staging and final output')

  await callDesktopBridge('reset')
  const historyBefore = (await callDesktopBridge('taskHistory')).length
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  await (await waitForElement('[data-testid="compression-mode-video"]')).click()
  workspace = await waitForElement('[data-testid="video-compression-workspace"]')
  picker = await waitForElement('[data-testid="video-compression-workspace"] [data-testid="dropzone-file"]')
  await callDesktopBridge('queueDesktopDialogSelections', [[frozenVideo, multiAudioVideo]])
  await picker.click()

  await driver.wait(async () => {
    const cards = await driver.findElements(By.css('[data-testid="video-draft-card"]'))
    if (cards.length !== 2) return false
    const statuses = await Promise.all(cards.map(card => card.getAttribute('data-status')))
    return statuses.every(status => status === 'ready')
  }, 30_000)
  console.log('[desktop-e2e] video drafts are initially planned')
  const text = await workspace.getText()
  assert.match(text, /预计输出 · 估算/)
  assert.match(text, /360×640/)
  assert.match(text, /VFR/)
  assert.match(text, /字幕流将被移除/)
  assert.match(text, /额外音轨将被移除/)
  assert.match(text, /后续执行前必须显式确认/)
  const execute = await waitForElement('[data-testid="video-compression-workspace"] .primary-action')
  assert.equal(await execute.getAttribute('disabled'), null, 'verified video drafts must enable real execution')
  assert.equal((await callDesktopBridge('taskHistory')).length, historyBefore, 'planning must not write task history')

  mkdirSync(artifactDirectory, { recursive: true })
  for (const size of [{ width: 1100, height: 720 }, { width: 760, height: 560 }]) {
    await driver.manage().window().setRect(size)
    await new Promise(resolve => setTimeout(resolve, 250))
    const layout = await driver.executeScript(() => {
      const main = document.querySelector('main')
      const workspace = document.querySelector('[data-testid="video-compression-workspace"]')
      const card = document.querySelector('[data-testid="video-draft-card"]')
      const facts = document.querySelector('.facts-grid')
      if (!main || !workspace || !card || !facts) return null
      return {
        mainOverflow: main.scrollWidth - main.clientWidth,
        workspaceOverflow: workspace.scrollWidth - workspace.clientWidth,
        cardOverflow: card.scrollWidth - card.clientWidth,
        factsOverflow: facts.scrollWidth - facts.clientWidth,
      }
    })
    assert.ok(layout, 'video planning facts must remain visible')
    for (const [label, overflow] of Object.entries(layout)) {
      assert.ok(overflow <= 1, `video ${label} must not scroll horizontally: ${JSON.stringify(layout)}`)
    }
    writeFileSync(
      path.join(artifactDirectory, `video-workspace-${size.width}x${size.height}.png`),
      Buffer.from(await driver.takeScreenshot(), 'base64'),
    )
  }

  await callDesktopBridge('queueDesktopDialogSelections', [videoOutputDirectory])
  await (await waitForElement('[data-testid="video-compression-workspace"] .output-directory button')).click()
  await driver.wait(async () => (await workspace.getText()).includes(videoOutputDirectory), 15_000)
  console.log('[desktop-e2e] video output directory selected')
  await callDesktopBridge('queueDesktopConfirmations', [true])
  await (await waitForElement('[data-testid="video-compression-workspace"] .primary-action')).click()
  console.log('[desktop-e2e] real video batch started')

  await driver.wait(async () => {
    const cards = await driver.findElements(By.css('[data-testid="video-draft-card"]'))
    if (cards.length !== 2) return false
    const statuses = await Promise.all(cards.map(card => card.getAttribute('data-status')))
    return statuses.every(status => status === 'completed')
  }, 180_000)
  console.log('[desktop-e2e] real video batch completed in the workspace')

  const videoHistory = await driver.wait(async () => {
    const records = (await callDesktopBridge('taskHistory')).filter(record =>
      record.workloadKind === 'video'
      && record.outputPath
      && normalizedDesktopPath(path.dirname(record.outputPath)) === normalizedDesktopPath(videoOutputDirectory),
    )
    return records.length === 2 ? records : false
  }, 30_000)
  console.log('[desktop-e2e] real video history rows persisted')
  assert.ok(videoHistory.every(record => record.status === 'completed'), 'every real video task must complete')
  assert.ok(videoHistory.every(record => record.metrics?.schemaVersion === 1), 'video history must persist measured metrics')
  assert.deepEqual(
    new Set(videoHistory.flatMap(record => record.sourcePaths.map(normalizedDesktopPath))),
    new Set([frozenVideo, multiAudioVideo].map(normalizedDesktopPath)),
    'video history must retain both real source identities',
  )
  const verifiedOutputs = []
  for (const record of videoHistory) {
    assert.equal(existsSync(record.outputPath), true, `published video is missing: ${record.outputPath}`)
    assert.equal(record.metrics.outputBytes, statSync(record.outputPath).size, 'history output bytes must match disk')
    const probeResult = spawnSync(productFfprobe, [
      '-v', 'error', '-show_entries',
      'format=format_name,duration,size:stream=codec_type,codec_name,width,height',
      '-of', 'json', record.outputPath,
    ], { encoding: 'utf8', windowsHide: true })
    assert.ifError(probeResult.error)
    assert.equal(probeResult.status, 0, probeResult.stderr || probeResult.stdout)
    const probe = JSON.parse(probeResult.stdout)
    const video = probe.streams.find(stream => stream.codec_type === 'video')
    const audio = probe.streams.find(stream => stream.codec_type === 'audio')
    assert.match(probe.format.format_name, /mp4/)
    assert.equal(video?.codec_name, 'h264')
    assert.equal(audio?.codec_name, 'aac')
    assert.ok(Number(probe.format.duration) > 0)
    assert.equal(Number(probe.format.size), statSync(record.outputPath).size)
    assert.equal(record.metrics.media.videoCodec, 'h264')
    assert.equal(record.metrics.media.audioCodec, 'aac')
    assert.equal(record.metrics.media.width, video.width)
    assert.equal(record.metrics.media.height, video.height)
    verifiedOutputs.push({
      taskId: record.id,
      sourcePaths: record.sourcePaths,
      outputPath: record.outputPath,
      historyStatus: record.status,
      metrics: record.metrics,
      probe,
    })
  }

  const firstCard = (await driver.findElements(By.css('[data-testid="video-draft-card"]')))[0]
  if ((await firstCard.findElement(By.css('.expand')).getAttribute('aria-expanded')) !== 'true') {
    await firstCard.findElement(By.css('.expand')).click()
  }
  assert.match(await firstCard.getText(), /最终输出/)
  assert.doesNotMatch(await firstCard.getText(), /Publishing/)
  const defaultPlayback = firstCard.findElement(By.css('[data-testid="video-open-default-app"]'))
  assert.equal(await defaultPlayback.isDisplayed(), true)
  await defaultPlayback.click()
  const defaultPlaybackNotice = await driver.wait(async () => {
    const feedback = await driver.findElements(By.css('[role="status"], [role="alert"]'))
    for (const item of feedback) {
      const text = await item.getText()
      if (/系统默认应用播放/.test(text)) return text
    }
    return false
  }, 30_000)
  assert.match(defaultPlaybackNotice, /已将视频交给系统默认应用播放/)
  console.log('[desktop-e2e] Windows accepted the published MP4 default-application playback request')
  writeFileSync(
    path.join(artifactDirectory, 'video-workspace-published-results.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  const persistedBeforeRestart = videoHistory
    .map(record => ({
      id: record.id,
      status: record.status,
      sourcePaths: record.sourcePaths.map(normalizedDesktopPath),
      outputPath: normalizedDesktopPath(record.outputPath),
      metrics: record.metrics,
    }))
    .sort((left, right) => left.id.localeCompare(right.id))
  await restartDesktopSession()
  const videoHistoryAfterRestart = await driver.wait(async () => {
    const records = (await callDesktopBridge('taskHistory')).filter(record =>
      record.workloadKind === 'video'
      && record.status === 'completed'
      && record.outputPath
      && normalizedDesktopPath(path.dirname(record.outputPath)) === normalizedDesktopPath(videoOutputDirectory),
    )
    return records.length === 2 ? records : false
  }, 30_000)
  const persistedAfterRestart = videoHistoryAfterRestart
    .map(record => ({
      id: record.id,
      status: record.status,
      sourcePaths: record.sourcePaths.map(normalizedDesktopPath),
      outputPath: normalizedDesktopPath(record.outputPath),
      metrics: record.metrics,
    }))
    .sort((left, right) => left.id.localeCompare(right.id))
  assert.deepEqual(persistedAfterRestart, persistedBeforeRestart, 'measured video history must survive a complete app restart')
  const cancelledAfterRestart = (await callDesktopBridge('taskHistory')).find(record => record.id === cancelledHistory.id)
  assert.equal(cancelledAfterRestart?.status, 'cancelled', 'cancelled video history must survive a complete app restart')
  await driver.manage().window().setRect({ width: 1600, height: 1000 })
  await (await waitForElement('[data-testid="nav-History"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/history'), 15_000)
  const visibleHistoryText = await driver.wait(async () => {
    const visibleHistory = await waitForElement('[data-testid="history-list"]')
    const text = await visibleHistory.getText()
    return text.includes('h264-vfr-audio-rotation-subtitles.mp4') ? text : false
  }, 30_000)
  assert.match(visibleHistoryText, /h264-vfr-audio-rotation-subtitles\.mp4/)
  assert.match(visibleHistoryText, /multi-audio-30s\.mp4/)
  assert.match(visibleHistoryText, /avi-100mib-1080p\.avi/)
  writeFileSync(
    path.join(artifactDirectory, 'video-history-after-restart.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  writeFileSync(
    path.join(artifactDirectory, 'video-workspace-execution-result.json'),
    `${JSON.stringify({
      gate: 'C-05.3-real-desktop-video-runtime-behavior',
      preset: 'balanced',
      inputCases: ['VFR + rotation + audio + subtitles', '30-second + multi-audio + subtitles'],
      cancellationAudit,
      defaultPlayback: {
        accepted: true,
        outputPath: verifiedOutputs[0].outputPath,
        notice: defaultPlaybackNotice,
      },
      restartHistory: {
        completedRows: persistedAfterRestart.length,
        cancelledRows: cancelledAfterRestart?.status === 'cancelled' ? 1 : 0,
        exactMeasuredFactsPreserved: true,
      },
      verifiedOutputs,
    }, null, 2)}\n`,
    'utf8',
  )
}

async function runImageBatchDesktopGate() {
  const expectedBatchSize = 100
  const expectedFormatCounts = { jpeg: 34, png: 33, webp: 33 }
  console.log('[desktop-e2e] verifying the real B-05.2.1 100-image mixed batch')
  const mediaRoot = path.join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'images')
  const batchInputDirectory = path.join(fixtureDirectory, 'image-batch-inputs')
  const imageOutputDirectory = path.join(fixtureDirectory, 'image-batch-results')
  mkdirSync(batchInputDirectory, { recursive: true })
  mkdirSync(imageOutputDirectory, { recursive: true })

  const definitions = [
    ...Array.from({ length: expectedFormatCounts.jpeg }, (_, index) => ({
      format: 'jpeg',
      sourceName: index === expectedFormatCounts.jpeg - 1
        ? 'large-photo.jpg'
        : index % 10 === 0 ? 'exif-orientation.jpg' : 'small-detail.jpg',
      extension: 'jpg',
    })),
    ...Array.from({ length: expectedFormatCounts.png }, (_, index) => ({
      format: 'png',
      sourceName: index === expectedFormatCounts.png - 1
        ? 'large-alpha.png'
        : index % 5 === 0 ? 'transparent.png' : 'opaque-small.png',
      extension: 'png',
    })),
    ...Array.from({ length: expectedFormatCounts.webp }, (_, index) => ({
      format: 'webp',
      sourceName: index === expectedFormatCounts.webp - 1
        ? 'large-photo.webp'
        : index % 3 === 0 ? 'photo.webp' : 'alpha-small.webp',
      extension: 'webp',
    })),
  ]
  assert.equal(definitions.length, expectedBatchSize)
  const imageFixtures = definitions.map((definition, index) => {
    const sourcePath = path.join(mediaRoot, definition.sourceName)
    assert.equal(existsSync(sourcePath), true, `missing real image fixture: ${sourcePath}`)
    const name = `batch-${String(index + 1).padStart(3, '0')}-${path.parse(definition.sourceName).name}.${definition.extension}`
    const fixturePath = path.join(batchInputDirectory, name)
    copyFileSync(sourcePath, fixturePath)
    return {
      ...definition,
      name,
      path: fixturePath,
      size: statSync(fixturePath).size,
      sha256: fileSha256(fixturePath),
    }
  })

  await callDesktopBridge('reset')
  await callDesktopBridge('clearTaskHistory')
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  await (await waitForElement('[data-testid="compression-mode-image"]')).click()
  await waitForElement('[data-testid="image-compression-workspace"]')
  const seed = await callDesktopBridge('seedImageCompressionWorkspace', imageFixtures.map(fixture => ({
    name: fixture.name,
    path: fixture.path,
    size: fixture.size,
    isDirectory: false,
  })))
  assert.deepEqual(seed, { accepted: expectedBatchSize, rejected: [] })

  const readyState = await driver.wait(async () => {
    const current = await callDesktopBridge('imageCompressionAuditState')
    return current.length === expectedBatchSize && current.every(item => item.status === 'ready')
      ? current
      : false
  }, 120_000)
  assert.equal(readyState.reduce((total, item) => total + item.inputSize, 0), imageFixtures.reduce((total, item) => total + item.size, 0))
  await (await waitForElement('[data-testid="image-compression-workspace"] .secondary-action')).click()
  let workspaceText = await (await waitForElement('[data-testid="image-compression-workspace"]')).getText()
  assert.match(workspaceText, /图片任务\s*100/)
  assert.match(workspaceText, /已读取\s*100/)

  const startButton = await waitForElement('[data-testid="image-compression-workspace"] .primary-action')
  assert.equal(await startButton.getAttribute('disabled'), null, '100 verified image drafts must enable real execution')
  await callDesktopBridge('configureImageCompressionWorkspace', imageOutputDirectory)
  const startedAt = Date.now()
  await startButton.click()
  const resultState = await driver.wait(async () => {
    const current = await callDesktopBridge('imageCompressionResultAuditState')
    return current.length === expectedBatchSize
      && current.every(item => item.taskStatus === 'completed' && item.hasResultPreview)
      ? current
      : false
  }, 600_000)
  const elapsedMs = Date.now() - startedAt

  const actualFormatCounts = { jpeg: 0, png: 0, webp: 0 }
  const outputPaths = new Set()
  for (const fixture of imageFixtures) {
    const actual = resultState.find(item => item.name === fixture.name)
    assert.ok(actual, `missing real image result: ${fixture.name}`)
    assert.equal(actual.inputBytes, fixture.size, `${fixture.name} input bytes must match the real source`)
    assert.ok(actual.outputPath && existsSync(actual.outputPath), `${fixture.name} must publish a real output file`)
    assert.equal(actual.outputBytes, statSync(actual.outputPath).size, `${fixture.name} output bytes must match the real file`)
    assert.ok(actual.outputWidth > 0 && actual.outputHeight > 0, `${fixture.name} must expose verified output dimensions`)
    assert.equal(actual.outputFormat, fixture.format, `${fixture.name} must retain its configured public format`)
    actualFormatCounts[fixture.format]++
    outputPaths.add(path.resolve(actual.outputPath).toLocaleLowerCase())
    assert.equal(fileSha256(fixture.path), fixture.sha256, `${fixture.name} source bytes must remain unchanged`)
  }
  assert.deepEqual(actualFormatCounts, expectedFormatCounts)
  assert.equal(outputPaths.size, expectedBatchSize, 'the batch must publish 100 unique output paths')
  assert.equal(readdirSync(imageOutputDirectory, { withFileTypes: true }).filter(entry => entry.isFile()).length, expectedBatchSize)

  const imageHistory = (await callDesktopBridge('taskHistory')).filter(record =>
    record.workloadKind === 'image' && record.outputPath?.includes(imageOutputDirectory),
  )
  assert.equal(imageHistory.length, expectedBatchSize, 'every published image must persist one unified history row')
  assert.ok(imageHistory.every(record => record.status === 'completed' && record.metrics?.inputBytes > 0 && record.metrics?.outputBytes > 0))
  workspaceText = await (await waitForElement('[data-testid="image-compression-workspace"]')).getText()
  assert.match(workspaceText, /100\/100\s*·\s*100\.00%/)
  assert.match(await (await waitForElement('body')).getText(), /图片处理完成：100 个结果，0 个跳过，0 个失败，0 个取消/)

  mkdirSync(artifactDirectory, { recursive: true })
  const auditResult = {
    scope: 'B-05.2.1 real Windows image batch',
    expected: {
      inputs: expectedBatchSize,
      ready: expectedBatchSize,
      completed: expectedBatchSize,
      uniqueOutputs: expectedBatchSize,
      historyRows: expectedBatchSize,
      formatCounts: expectedFormatCounts,
      sourceHashChanges: 0,
    },
    actual: {
      inputs: imageFixtures.length,
      ready: readyState.length,
      completed: resultState.filter(item => item.taskStatus === 'completed').length,
      uniqueOutputs: outputPaths.size,
      historyRows: imageHistory.length,
      formatCounts: actualFormatCounts,
      sourceHashChanges: imageFixtures.filter(fixture => fileSha256(fixture.path) !== fixture.sha256).length,
      elapsedMs,
    },
  }
  writeFileSync(path.join(artifactDirectory, 'image-batch-100-result.json'), `${JSON.stringify(auditResult, null, 2)}\n`, 'utf8')
  await driver.manage().window().setRect({ width: 1100, height: 720 })
  writeFileSync(
    path.join(artifactDirectory, 'image-batch-100-completed.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )
  console.log(`[desktop-e2e] B-05.2.1 expected/actual: ${JSON.stringify(auditResult)}`)
}

async function runManualImagePickerDesktopGate() {
  console.log('[desktop-e2e] preparing the attended real Windows image-picker gate')
  const mediaRoot = path.join(root, 'test-results', 'media-fixture-audit', 'fixtures')
  const pickerJpeg = path.join(mediaRoot, 'images', 'exif-orientation.jpg')
  const pickerGif = path.join(mediaRoot, 'images', 'animated.gif')
  for (const fixture of [pickerJpeg, pickerGif]) {
    assert.equal(existsSync(fixture), true, `missing manual picker fixture: ${fixture}`)
    assert.ok(statSync(fixture).size > 0, `manual picker fixture is empty: ${fixture}`)
  }

  await callDesktopBridge('reset')
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/special-compression'), 30_000)
  await waitForElement('[data-testid="special-compression-center"]')
  await (await waitForElement('[data-testid="compression-mode-image"]')).click()
  await waitForElement('[data-testid="image-compression-workspace"]')
  await driver.manage().window().setRect({ width: 1100, height: 720 })
  const globalSettings = await driver.findElements(By.css('.global-settings-card'))
  if (globalSettings.length > 0) {
    await (await waitForElement('[data-testid="image-compression-workspace"] .secondary-action')).click()
  }
  const nativePickerDropzone = await waitForElement('[data-testid="dropzone-file"]')

  await driver.executeScript(() => {
    const audit = { alerts: [], lostFocus: false, returnedFocus: false }
    const recordAlerts = () => {
      for (const node of document.querySelectorAll('[role="alert"]')) {
        const message = node.textContent?.trim()
        if (message && !audit.alerts.includes(message)) audit.alerts.push(message)
      }
    }
    new MutationObserver(recordAlerts).observe(document.body, {
      childList: true,
      subtree: true,
      characterData: true,
    })
    window.addEventListener('blur', () => { audit.lostFocus = true })
    window.addEventListener('focus', () => {
      if (audit.lostFocus) audit.returnedFocus = true
    })
    window.__LONG_DECOMPRESS_MANUAL_PICKER_AUDIT__ = audit
    recordAlerts()
  })

  console.log('')
  console.log('[manual-gate] 即将通过可见工作区入口打开真实 Windows 对话框，请在对话框中选择：')
  console.log(`[manual-gate] JPEG: ${pickerJpeg}`)
  console.log(`[manual-gate] GIF:  ${pickerGif}`)
  console.log('[manual-gate] 可以一次多选，也可以先选 JPEG、再用“继续添加图片”选择 GIF。')
  console.log('[manual-gate] 脚本将在 10 分钟内自动核验真实字节、360×640、预览、GIF 拒绝、队列和焦点。')
  await nativePickerDropzone.click()

  const result = await driver.wait(async () => {
    const state = await callDesktopBridge('imageCompressionAuditState')
    const ui = await driver.executeScript(() => {
      const audit = window.__LONG_DECOMPRESS_MANUAL_PICKER_AUDIT__
      const preview = document.querySelector('.preview-card img')
      return {
        alerts: audit?.alerts ?? [],
        lostFocus: audit?.lostFocus ?? false,
        returnedFocus: audit?.returnedFocus ?? false,
        documentHasFocus: document.hasFocus(),
        previewReady: Boolean(preview?.complete && preview?.naturalWidth > 0 && preview?.naturalHeight > 0),
      }
    })
    const jpeg = state.find(item => item.name === 'exif-orientation.jpg')
    const gifRejected = ui.alerts.some(message => /animated\.gif.*GIF/.test(message))
    if (
      state.length === 1 &&
      jpeg?.status === 'ready' &&
      jpeg.width === 360 &&
      jpeg.height === 640 &&
      jpeg.inputSize === statSync(pickerJpeg).size &&
      gifRejected &&
      ui.previewReady &&
      ui.documentHasFocus
    ) {
      return { state, ui }
    }
    return false
  }, 600_000)

  assert.deepEqual(result.state.map(item => item.name), ['exif-orientation.jpg'])
  assert.equal(result.ui.documentHasFocus, true, 'focus must return to the WebView after the native dialog closes')
  mkdirSync(artifactDirectory, { recursive: true })
  writeFileSync(
    path.join(artifactDirectory, 'image-workspace-native-picker-manual.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )
  writeFileSync(
    path.join(artifactDirectory, 'image-workspace-native-picker-manual.json'),
    `${JSON.stringify({ verifiedAt: new Date().toISOString(), ...result }, null, 2)}\n`,
    'utf8',
  )
}

async function runZipTelemetryDesktopGate() {
  console.log('[desktop-e2e] verifying focused plain and AES ZIP real-byte telemetry')
  await callDesktopBridge('clearTasks')
  const flowRoot = path.join(fixtureDirectory, 'zip-telemetry')
  const sourceRoot = path.join(flowRoot, 'sources')
  const archiveRoot = path.join(flowRoot, 'archives')
  mkdirSync(sourceRoot, { recursive: true })
  mkdirSync(archiveRoot, { recursive: true })

  for (const encrypted of [false, true]) {
    const label = encrypted ? 'aes' : 'plain'
    const sourcePath = path.join(sourceRoot, `${label}-zip-telemetry.bin`)
    const archivePath = path.join(archiveRoot, `${label}-zip-telemetry.zip`)
    const extractRoot = path.join(flowRoot, `${label}-zip-extracted`)
    const password = encrypted ? 'Long-ZIP-Telemetry-2026!' : undefined
    writeFileSync(sourcePath, randomBytes(64 * 1024 * 1024))

    const taskId = await callDesktopBridge(
      'startZipTelemetryCompression',
      sourcePath,
      archivePath,
      password,
    )
    if ((await driver.findElements(By.css('.progress-panel'))).length === 0) {
      await (await waitForElement('.progress-summary')).click()
    }
    await driver.wait(async () => {
      const state = await callDesktopBridge('archiveFlowAuditState')
      return state.zipTelemetry.some(item => item.speed && item.etaSeconds !== undefined)
    }, 60_000)
    await driver.wait(async () => {
      const panels = await driver.findElements(By.css('.progress-panel'))
      if (panels.length === 0) return false
      const text = await panels[0].getAttribute('textContent')
      return /速度[\s\S]*\/s/.test(text) && /(剩余|ETA)/.test(text)
    }, 60_000)
    const state = await driver.wait(async () => {
      const current = await callDesktopBridge('archiveFlowAuditState')
      return current.zipDone ? current : false
    }, 180_000)
    assert.equal(await callDesktopBridge('taskStatus', taskId), 'completed')
    assert.deepEqual(state.errors, [])
    const sourceBytes = statSync(sourcePath).size
    assert.ok(
      state.zipTelemetry.some(item => item.processedBytes > 0 && item.processedBytes < item.totalBytes),
      `${label} ZIP must emit an intermediate real-byte event`,
    )
    const byteTelemetry = state.zipTelemetry.filter(item => item.totalBytes > 0)
    const finalTelemetry = byteTelemetry.at(-1)
    assert.equal(finalTelemetry.processedBytes, sourceBytes)
    assert.equal(finalTelemetry.totalBytes, sourceBytes)
    assert.ok(finalTelemetry.speed, `${label} ZIP must expose measured throughput`)
    assert.equal(finalTelemetry.etaSeconds, 0)
    const outputTelemetry = state.zipTelemetry.filter(item => item.outputBytes > 0)
    assert.ok(outputTelemetry.length > 0, `${label} ZIP must emit its real archive size`)
    assert.equal(outputTelemetry.at(-1).outputBytes, statSync(archivePath).size)

    const testArgs = ['t', '-y']
    if (password) testArgs.push(`-p${password}`)
    testArgs.push(archivePath)
    const archiveTest = spawnSync(bundledSevenZip, testArgs, { encoding: 'utf8', windowsHide: true })
    assert.equal(archiveTest.status, 0, archiveTest.stderr || archiveTest.stdout)
    if (password) {
      const wrongPassword = spawnSync(
        bundledSevenZip,
        ['t', '-y', '-pwrong-password', archivePath],
        { encoding: 'utf8', windowsHide: true },
      )
      assert.notEqual(wrongPassword.status, 0, 'AES ZIP must reject an incorrect password')
    }
    mkdirSync(extractRoot, { recursive: true })
    const extractArgs = ['x', '-y', `-o${extractRoot}`]
    if (password) extractArgs.push(`-p${password}`)
    extractArgs.push(archivePath)
    const extraction = spawnSync(bundledSevenZip, extractArgs, { encoding: 'utf8', windowsHide: true })
    assert.equal(extraction.status, 0, extraction.stderr || extraction.stdout)
    assert.equal(fileSha256(path.join(extractRoot, path.basename(sourcePath))), fileSha256(sourcePath))
  }

  console.log('[desktop-e2e] verifying multi-file ZIP cumulative byte telemetry')
  const multiSources = [
    { path: path.join(sourceRoot, 'multi-alpha.bin'), size: 24 * 1024 * 1024 },
    { path: path.join(sourceRoot, 'multi-beta.bin'), size: 40 * 1024 * 1024 },
  ]
  for (const source of multiSources) writeFileSync(source.path, randomBytes(source.size))
  const multiArchive = path.join(archiveRoot, 'multi-zip-telemetry.zip')
  const multiOutput = path.join(flowRoot, 'multi-zip-extracted')
  const multiTaskId = await callDesktopBridge(
    'startZipTelemetryCompression',
    multiSources.map(source => source.path),
    multiArchive,
  )
  const multiState = await driver.wait(async () => {
    const state = await callDesktopBridge('archiveFlowAuditState')
    return state.zipDone ? state : false
  }, 180_000)
  assert.equal(await callDesktopBridge('taskStatus', multiTaskId), 'completed')
  assert.deepEqual(multiState.errors, [])
  const multiByteEvents = multiState.zipTelemetry.filter(item => item.totalBytes > 0)
  const expectedMultiBytes = multiSources.reduce((total, source) => total + source.size, 0)
  assert.ok(multiByteEvents.length > 2, 'multi-file ZIP must emit intermediate byte events')
  assert.ok(
    multiByteEvents.every((event, index) => index === 0 || event.processedBytes >= multiByteEvents[index - 1].processedBytes),
    'multi-file ZIP processed bytes must be monotonic',
  )
  assert.equal(multiByteEvents.at(-1).processedBytes, expectedMultiBytes)
  assert.equal(multiByteEvents.at(-1).totalBytes, expectedMultiBytes)
  assert.equal(
    multiState.zipTelemetry.filter(item => item.outputBytes > 0).at(-1).outputBytes,
    statSync(multiArchive).size,
  )
  const multiTest = spawnSync(bundledSevenZip, ['t', '-y', multiArchive], {
    encoding: 'utf8', windowsHide: true,
  })
  assert.equal(multiTest.status, 0, multiTest.stderr || multiTest.stdout)
  mkdirSync(multiOutput, { recursive: true })
  const multiExtract = spawnSync(bundledSevenZip, ['x', '-y', `-o${multiOutput}`, multiArchive], {
    encoding: 'utf8', windowsHide: true,
  })
  assert.equal(multiExtract.status, 0, multiExtract.stderr || multiExtract.stdout)
  for (const source of multiSources) {
    assert.equal(fileSha256(path.join(multiOutput, path.basename(source.path))), fileSha256(source.path))
  }
}

async function runTarTelemetryDesktopGate() {
  console.log('[desktop-e2e] verifying TAR-family real-byte telemetry and round trips')
  await callDesktopBridge('clearTasks')
  const root = path.join(fixtureDirectory, 'tar-telemetry')
  const sourcePath = path.join(root, 'tar-telemetry-payload.bin')
  mkdirSync(root, { recursive: true })
  writeFileSync(sourcePath, randomBytes(64 * 1024 * 1024))
  const sourceBytes = statSync(sourcePath).size
  const formats = [
    ['tar', 'tar'],
    ['tar.gz', 'tar.gz'],
    ['tar.bz2', 'tar.bz2'],
    ['tar.xz', 'tar.xz'],
    ['tar.zst', 'tar.zst'],
  ]

  for (const [format, extension] of formats) {
    console.log(`[desktop-e2e] TAR telemetry round trip: ${format}`)
    const archivePath = path.join(root, `payload.${extension}`)
    const extractRoot = path.join(root, `extract-${format.replaceAll('.', '-')}`)
    const taskId = await callDesktopBridge(
      'startZipTelemetryCompression',
      sourcePath,
      archivePath,
      undefined,
      format,
    )
    if (format === 'tar.gz' && (await driver.findElements(By.css('.progress-panel'))).length === 0) {
      await (await waitForElement('.progress-summary')).click()
    }
    if (format === 'tar.gz') {
      await driver.wait(async () => {
        const panels = await driver.findElements(By.css('.progress-panel'))
        if (panels.length === 0) return false
        const text = await panels[0].getAttribute('textContent')
        return /速度[\s\S]*\/s/.test(text) && /(剩余|ETA)/.test(text)
      }, 60_000)
    }
    const state = await driver.wait(async () => {
      const current = await callDesktopBridge('archiveFlowAuditState')
      return current.zipDone ? current : false
    }, 180_000)
    assert.equal(await callDesktopBridge('taskStatus', taskId), 'completed')
    assert.deepEqual(state.errors, [])
    const byteTelemetry = state.zipTelemetry.filter(item => item.totalBytes > 0)
    assert.ok(
      byteTelemetry.some(item => item.processedBytes > 0 && item.processedBytes < item.totalBytes),
      `${format} must emit an intermediate real-byte event`,
    )
    const finalTelemetry = byteTelemetry.at(-1)
    assert.equal(finalTelemetry.processedBytes, sourceBytes, `${format} final processed bytes`)
    assert.equal(finalTelemetry.totalBytes, sourceBytes, `${format} final total bytes`)
    assert.ok(finalTelemetry.speed, `${format} must expose measured throughput`)
    assert.equal(finalTelemetry.etaSeconds, 0)

    const archiveTest = spawnSync(bundledSevenZip, ['t', '-y', archivePath], {
      encoding: 'utf8', windowsHide: true,
    })
    assert.equal(archiveTest.status, 0, archiveTest.stderr || archiveTest.stdout)
    mkdirSync(extractRoot, { recursive: true })
    await callDesktopBridge('extractArchive', archivePath, extractRoot)
    assert.equal(
      fileSha256(path.join(extractRoot, path.basename(sourcePath))),
      fileSha256(sourcePath),
      `${format} extracted payload must match the source`,
    )
  }
}

try {
  if (autoStartOnly) {
    assert.equal(
      readAutoStartRegistryValue(),
      null,
      'the focused auto-start gate refuses to overwrite an existing user startup choice',
    )
    autoStartRegistryOwnedByTest = true
  }
  mkdirSync(webviewUserDataDirectory, { recursive: true })
  await startTauriDriver()

  driver = await createDesktopSession()
  await waitForDesktopReady()

  let navigation = await driver.findElements(By.css('aside nav > button'))
  assert.equal(navigation.length, 8, 'the real desktop shell must expose eight navigation buttons')
  const versionBadge = await waitForElement('[data-testid="sidebar-version-badge"]')
  assert.equal(
    (await versionBadge.getAttribute('textContent')).trim(),
    `v${tauriConfig.package.version}`,
    'the sidebar version badge must come from the packaged application version',
  )
  mkdirSync(artifactDirectory, { recursive: true })
  if (!autoStartOnly) {
    writeFileSync(
      path.join(artifactDirectory, 'sidebar-version-badge.png'),
      Buffer.from(await driver.takeScreenshot(), 'base64'),
    )
  }
  assert.equal(
    await navigation[0].getAttribute('aria-current'),
    'page',
    'the decompression workspace must be selected by default',
  )

  fixtureDirectory = mkdtempSync(path.join(tmpdir(), 'long-decompress-desktop-e2e-'))
  const sourcePath = path.join(fixtureDirectory, 'roundtrip-payload.txt')
  const archivePath = path.join(fixtureDirectory, 'roundtrip-payload.zip')
  const extractedPath = path.join(
    fixtureDirectory,
    'roundtrip-payload',
    'roundtrip-payload.txt',
  )
  const payload = `Long解压 real desktop round-trip ${new Date().toISOString()}\n`
  writeFileSync(sourcePath, payload, 'utf8')

  if (autoStartOnly) {
    await (await waitForElement('[data-testid="nav-Settings"]')).click()
    await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/settings'), 30_000)
    const autoStartSwitch = await waitForElement('[data-testid="auto-start-switch"]')
    assert.equal(await autoStartSwitch.getAttribute('disabled'), null)
    assert.equal(await autoStartSwitch.getAttribute('aria-checked'), 'false')
    assert.equal(await callDesktopBridge('checkAutoStart'), false)
    await autoStartSwitch.click()
    await driver.wait(
      async () => (await autoStartSwitch.getAttribute('aria-checked')) === 'true',
      10_000,
    )
    assert.equal(await callDesktopBridge('checkAutoStart'), true)
    assert.equal(
      readAutoStartRegistryValue()?.toLowerCase(),
      `"${application}" --autostart`.toLowerCase(),
      'the startup entry must quote the exact executable and use only the dedicated activation flag',
    )
    assert.equal(
      await callDesktopBridge('setAutoStart', true),
      true,
      'enabling an already-current startup entry must be idempotent',
    )
    await autoStartSwitch.click()
    await driver.wait(
      async () => (await autoStartSwitch.getAttribute('aria-checked')) === 'false',
      10_000,
    )
    assert.equal(await callDesktopBridge('checkAutoStart'), false)
    assert.equal(readAutoStartRegistryValue(), null)

    await callDesktopBridge('requestAppExit')
    await new Promise(resolve => setTimeout(resolve, 1_000))
    try {
      await driver.quit()
    } catch {
      // The native exit can invalidate the WebDriver session first.
    }
    driver = undefined
    const remainingProcessIds = desktopApplicationProcessIds()
    for (const processId of remainingProcessIds) terminateProcessTree(processId)

    const startupProbe = path.join(fixtureDirectory, 'auto-start-visibility.marker')
    const startupProcess = spawn(
      application,
      ['--autostart', '--desktop-e2e-autostart-probe', startupProbe],
      {
        cwd: root,
        env: {
          ...process.env,
          LONG_DECOMPRESS_E2E_DATA_DIR: e2eDataDirectory,
          LONG_DECOMPRESS_E2E_INSTANCE_ID: e2eInstanceId,
        },
        stdio: 'ignore',
        windowsHide: true,
      },
    )
    try {
      await waitForStandaloneFileContent(startupProbe, 'hidden')
    } finally {
      terminateProcessTree(startupProcess.pid)
    }
    completedSuccessfully = true
    console.log('Real Windows Tauri explicit auto-start gate passed.')
  } else if (responsiveLayoutOnly) {
    await runResponsiveTaskDetailDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri responsive task-detail gate passed.')
  } else if (imageWorkspaceOnly) {
    await runImageWorkspaceDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri B-04.5 image execution and result gate passed.')
  } else if (imageBatchOnly) {
    await runImageBatchDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri B-05.2.1 100-image batch gate passed.')
  } else if (imagePickerManualOnly) {
    await runManualImagePickerDesktopGate()
    completedSuccessfully = true
    console.log('Attended real Windows Tauri B-02 native image-picker gate passed.')
  } else if (videoWorkspaceOnly) {
    await runVideoWorkspaceDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri C-05.1/C-05.3 video execution and runtime-behavior gate passed.')
  } else if (pdfWorkspaceOnly) {
    await runPdfWorkspaceDesktopGate()
    completedSuccessfully = true
  console.log('Real Windows Tauri D-04.3 PDF cancellation/batch/restart/default-reader gate passed.')
  } else if (tarTelemetryOnly) {
    await runTarTelemetryDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri TAR telemetry gate passed.')
  } else if (vaultUsageOnly) {
    await runVaultUsageDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri vault current-day usage gate passed.')
  } else if (encryptedRarOnly) {
    await runEncryptedRarDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri encrypted RAR password gate passed.')
  } else if (hfsxOnly) {
    await runHfsxDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri non-empty HFSX extraction gate passed.')
  } else if (resourcePreflightOnly) {
    await runResourcePreflightLayoutDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri shared resource-preflight layout gate passed.')
  } else if (historyOnly) {
    await runHistoryDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri task-history persistence gate passed.')
  } else if (zipTelemetryOnly) {
    await runZipTelemetryDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri ZIP telemetry gate passed.')
  } else if (archiveFlowOnly) {
    await runArchiveFlowDesktopGate()
    completedSuccessfully = true
    console.log('Real Windows Tauri archive-flow alignment gate passed.')
  } else {

  forwardContextAction('--quick-pack', [sourcePath])
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/compress'), 30_000)
  await waitForStableFile(archivePath)
  assert.ok(
    readFileSync(archivePath).length > 0,
    'the real compression command must create a non-empty ZIP archive',
  )
  const compressionResourceTask = await waitForResourcePreflightTask(
    'compression',
    task => task.status === 'completed' && normalizedDesktopPath(task.outputPath) === normalizedDesktopPath(archivePath),
  )
  assertRealResourceReport(compressionResourceTask, {
    operation: 'compression',
    taskStatus: 'completed',
    canStart: true,
    reportStatuses: ['ready', 'warning'],
    outputPath: archivePath,
  })
  assert.equal(compressionResourceTask.report.estimateSource, 'provided_estimate')
  assert.equal(compressionResourceTask.report.estimateReliable, true)
  assert.equal(
    compressionResourceTask.report.estimatedOutputBytes,
    Math.ceil(statSync(sourcePath).size * 1.05),
    'the desktop compression report must use the conservative regular-file estimate',
  )
  await assertVisibleResourceCard(
    compressionResourceTask.id,
    compressionResourceTask.report.status === 'ready' ? '已通过' : '需留意',
  )

  forwardContextAction('--quick-extract', [archivePath])
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/decompress'), 30_000)
  await waitForFileContent(extractedPath, payload)
  assert.equal(
    readFileSync(extractedPath, 'utf8'),
    payload,
    'the extracted file must match the source payload byte-for-byte',
  )
  const decompressionResourceTask = await waitForResourcePreflightTask(
    'decompression',
    task => task.status === 'completed' && normalizedDesktopPath(task.outputPath) === normalizedDesktopPath(fixtureDirectory),
  )
  assertRealResourceReport(decompressionResourceTask, {
    operation: 'decompression',
    taskStatus: 'completed',
    canStart: true,
    reportStatuses: ['ready', 'warning'],
    outputPath: fixtureDirectory,
  })
  assert.equal(decompressionResourceTask.report.estimateSource, 'archive_metadata')
  assert.equal(decompressionResourceTask.report.estimateReliable, true)
  assert.equal(
    decompressionResourceTask.report.estimatedOutputBytes,
    Buffer.byteLength(payload, 'utf8'),
    'the desktop decompression report must use the real archive metadata size',
  )
  await assertVisibleResourceCard(
    decompressionResourceTask.id,
    decompressionResourceTask.report.status === 'ready' ? '已通过' : '需留意',
  )

  console.log('[desktop-e2e] verifying archive diagnosis, non-destructive ZIP repair, and ZIP/TAR image preview')
  const sourceArchiveHash = fileSha256(archivePath)
  const diagnosis = await callDesktopBridge('diagnoseArchive', archivePath)
  assert.equal(diagnosis.actualFormat, 'ZIP')
  assert.equal(diagnosis.status, 'healthy')
  assert.equal(diagnosis.totalFiles, 1)
  assert.equal(diagnosis.integrityTested, true)
  assert.equal(diagnosis.canRepair, false, 'a healthy ZIP should not advertise repair as necessary')

  const repairedArchivePath = path.join(fixtureDirectory, 'roundtrip-payload-repaired.zip')
  const repair = await callDesktopBridge('repairZip', archivePath, repairedArchivePath)
  assert.equal(repair.outputPath, repairedArchivePath)
  assert.equal(repair.recoveredFiles, 1)
  assert.equal(repair.recoveredDirectories, 0)
  assert.deepEqual(repair.skippedEntries, [])
  assert.equal(repair.verified, true)
  assert.equal(fileSha256(archivePath), sourceArchiveHash, 'ZIP repair must not modify the source archive')
  assert.ok(existsSync(repairedArchivePath), 'ZIP repair must publish a new archive')
  const repairedDiagnosis = await callDesktopBridge('diagnoseArchive', repairedArchivePath)
  assert.equal(repairedDiagnosis.status, 'healthy')
  assert.equal(repairedDiagnosis.integrityTested, true)

  const previewPngName = 'preview.png'
  const previewPngPath = path.join(fixtureDirectory, previewPngName)
  const previewZipPath = path.join(fixtureDirectory, 'preview-image.zip')
  const previewTarPath = path.join(fixtureDirectory, 'preview-image.tar')
  writeFileSync(
    previewPngPath,
    Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=', 'base64'),
  )
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-tzip', '-y', previewZipPath, previewPngName],
    'ZIP image preview',
  )
  runFixtureCommand(
    bundledSevenZip,
    ['a', '-ttar', '-y', previewTarPath, previewPngName],
    'TAR image preview',
  )
  for (const previewArchivePath of [previewZipPath, previewTarPath]) {
    const preview = await callDesktopBridge('previewArchiveImage', previewArchivePath, previewPngName)
    assert.equal(preview.entryPath, previewPngName)
    assert.equal(preview.mimeType, 'image/png')
    assert.equal(preview.width, 1)
    assert.equal(preview.height, 1)
    assert.ok(preview.byteSize > 0)
    assert.match(preview.dataUrl, /^data:image\/png;base64,/)
  }

  console.log('[desktop-e2e] verifying bounded smart-compression analysis and visible recommendations')
  const textDirectory = path.join(fixtureDirectory, 'smart-analysis-text')
  mkdirSync(textDirectory)
  const compressibleChunk = Buffer.from('Long解压 bounded smart compression analysis\n'.repeat(12_000), 'utf8')
  for (let index = 0; index < 24; index += 1) {
    writeFileSync(path.join(textDirectory, `document-${String(index).padStart(2, '0')}.txt`), compressibleChunk)
  }
  await callDesktopBridge('clearCompressionWorkspace')
  let analysisJobId = await callDesktopBridge('seedCompressionAnalysisWorkspace', [{
    name: 'smart-analysis-text',
    path: textDirectory,
    size: 0,
    isDirectory: true,
  }])
  navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[1].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/compress'), 30_000)
  let analysisCard = await waitForElement('[data-testid="compression-analysis"]')
  const textAnalysisStartedAt = Date.now()
  await (await analysisCard.findElement(By.css('.analysis-button'))).click()
  let analysisState = await driver.wait(async () => {
    const state = await callDesktopBridge('compressionAnalysisAuditState', analysisJobId)
    return state.status === 'completed' ? state : false
  }, 15_000)
  assert.ok(Date.now() - textAnalysisStartedAt < 15_000, 'large-directory analysis must stay interactive')
  assert.equal(analysisState.analysis.fileCount, 24)
  assert.ok(analysisState.analysis.totalSize >= 8 * 1024 * 1024)
  assert.ok(analysisState.analysis.sampledFiles <= 16)
  assert.ok(analysisState.analysis.sampledBytes <= 2 * 1024 * 1024)
  assert.equal(analysisState.analysis.recommendedFormat, '7z')
  assert.equal(analysisState.analysis.recommendedLevel, 7)
  assert.equal(analysisState.analysis.recommendedSolid, true)
  assert.match(await analysisCard.getText(), /智能压缩分析[\s\S]*预计体积[\s\S]*建议 7Z · L7/)
  const analysisDimensions = await driver.executeScript(
    'const card = document.querySelector(\'[data-testid="compression-analysis"]\'); return card ? { scrollWidth: card.scrollWidth, clientWidth: card.clientWidth } : null;',
  )
  assert.ok(analysisDimensions)
  assert.ok(
    analysisDimensions.scrollWidth <= analysisDimensions.clientWidth + 1,
    `the smart-analysis card must not scroll horizontally: ${JSON.stringify(analysisDimensions)}`,
  )
  await (await analysisCard.findElement(By.css('.analysis-apply'))).click()
  analysisState = await callDesktopBridge('compressionAnalysisAuditState', analysisJobId)
  assert.deepEqual(analysisState.settings, { format: '7z', level: 7, createSolidArchive: true })

  const mediaPngPath = path.join(fixtureDirectory, 'smart-photo.png')
  const mediaVideoPath = path.join(fixtureDirectory, 'smart-video.mp4')
  copyFileSync(path.join(root, 'src-tauri', 'icons', 'icon.png'), mediaPngPath)
  writeFileSync(mediaVideoPath, randomBytes(4 * 1024 * 1024))
  await callDesktopBridge('clearCompressionWorkspace')
  analysisJobId = await callDesktopBridge('seedCompressionAnalysisWorkspace', [
    { name: 'smart-photo.png', path: mediaPngPath, size: statSync(mediaPngPath).size, isDirectory: false },
    { name: 'smart-video.mp4', path: mediaVideoPath, size: statSync(mediaVideoPath).size, isDirectory: false },
  ])
  analysisCard = await waitForElement('[data-testid="compression-analysis"]')
  await (await analysisCard.findElement(By.css('.analysis-button'))).click()
  analysisState = await driver.wait(async () => {
    const state = await callDesktopBridge('compressionAnalysisAuditState', analysisJobId)
    return state.status === 'completed' ? state : false
  }, 15_000)
  assert.equal(analysisState.analysis.lowValueFileCount, 2)
  assert.equal(analysisState.analysis.lowValueBytes, analysisState.analysis.totalSize)
  assert.equal(analysisState.analysis.recommendedFormat, 'zip')
  assert.equal(analysisState.analysis.recommendedLevel, 1)
  assert.equal(analysisState.analysis.recommendedSolid, false)
  assert.match(await analysisCard.getText(), /高等级压缩收益有限/)

  if (smartAnalysisOnly) {
    completedSuccessfully = true
    console.log('Real Windows Tauri smart-compression analysis gate passed.')
  } else {

  await runArchiveBrowserDesktopGate()
  if (archiveBrowserOnly || fileManagerOnly) {
    completedSuccessfully = true
    console.log(fileManagerOnly ? 'Real Windows Tauri dual-pane file-manager gate passed.' : 'Real Windows Tauri archive-browser gate passed.')
  } else {

  await runMarkOfWebDesktopGate()
  if (markOfWebOnly) {
    completedSuccessfully = true
    console.log('Real Windows Tauri Mark-of-the-Web gate passed.')
  } else {

  await runCompressionVerificationDesktopGate()
  if (compressionVerificationOnly) {
    completedSuccessfully = true
    console.log('Real Windows Tauri post-compression verification gate passed.')
  } else {

  navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[0].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/decompress'), 30_000)

  console.log('[desktop-e2e] verifying reliable capacity blocking and the visible blocked state')
  await callDesktopBridge('clearTasks')
  const blockedOutput = path.join(fixtureDirectory, 'must-not-create-blocked-output')
  const blockedSeed = await callDesktopBridge(
    'seedBlockedResourcePreflight',
    archivePath,
    blockedOutput,
  )
  const blockedResourceTask = await waitForResourcePreflightTask(
    'decompression',
    task => task.id === blockedSeed.taskId && task.status === 'failed',
  )
  assertRealResourceReport(blockedResourceTask, {
    operation: 'decompression',
    taskStatus: 'failed',
    canStart: false,
    reportStatuses: ['blocked'],
    outputPath: blockedOutput,
  })
  assert.equal(blockedResourceTask.report.estimateReliable, true)
  assert.equal(blockedResourceTask.report.estimatedOutputBytes, Number.MAX_SAFE_INTEGER)
  assert.match(blockedResourceTask.report.summary, /空间不足/)
  assert.equal(
    existsSync(blockedOutput),
    false,
    'the blocked desktop preflight must not create an extraction output',
  )
  const blockedCardText = await assertVisibleResourceCard(blockedResourceTask.id, '已阻止')
  assert.match(blockedCardText, /空间不足/)
  await callDesktopBridge('clearTasks')

  if (resourcePreflightOnly) {
    completedSuccessfully = true
    console.log('Real Windows Tauri resource-preflight gate passed.')
  } else {

  console.log('[desktop-e2e] verifying task-template import, draft, and read-only folder preview')
  const taskTemplateDirectory = path.join(fixtureDirectory, 'task-template-watch')
  const taskTemplateNestedDirectory = path.join(taskTemplateDirectory, 'nested')
  const taskTemplateKeep = path.join(taskTemplateNestedDirectory, 'keep.log')
  const taskTemplateSkip = path.join(taskTemplateDirectory, 'skip.tmp')
  const taskTemplateUnmatched = path.join(taskTemplateDirectory, 'notes.txt')
  const exportedTaskTemplate = path.join(fixtureDirectory, 'desktop-audit.longtask.json')
  mkdirSync(taskTemplateNestedDirectory, { recursive: true })
  writeFileSync(taskTemplateKeep, 'stable log payload', 'utf8')
  writeFileSync(taskTemplateSkip, 'excluded temporary payload', 'utf8')
  writeFileSync(taskTemplateUnmatched, 'unmatched text payload', 'utf8')

  const taskTemplateProfile = await callDesktopBridge('createTaskTemplateAuditProfile')
  await callDesktopBridge('queueTaskTemplateDialogSelections', [
    exportedTaskTemplate,
    exportedTaskTemplate,
    taskTemplateDirectory,
    [taskTemplateKeep, taskTemplateSkip],
    taskTemplateDirectory,
  ])

  navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[1].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/compress'), 30_000)
  await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
  await (await waitForElement('[data-testid="manage-compression-profiles"]')).click()
  await waitForElement('.profile-manager')

  await (await waitForElement(`[data-testid="export-task-template-${taskTemplateProfile.id}"]`)).click()
  await waitForNonEmptyFile(exportedTaskTemplate)
  const exportedTaskTemplateJson = readFileSync(exportedTaskTemplate, 'utf8')
  assert.ok(
    !exportedTaskTemplateJson.includes('desktop-e2e-secret-must-not-export'),
    'the real desktop export must not contain the fixed password',
  )
  assert.ok(
    !exportedTaskTemplateJson.includes('deleteAfter') &&
      !exportedTaskTemplateJson.includes('must-not-export'),
    'the real desktop export must omit delete-source and extra engine parameters',
  )

  await (await waitForElement('[data-testid="import-task-template"]')).click()
  const templatePreview = await waitForElement('[data-testid="task-template-preview"]')
  const templatePreviewText = await templatePreview.getAttribute('textContent')
  assert.match(templatePreviewText, /导入不会启动压缩/)
  assert.match(templatePreviewText, /执行时询问密码/)
  await (await waitForElement('[data-testid="confirm-task-template-import"]')).click()
  await driver.wait(
    async () => (await driver.findElements(By.css('[data-testid="task-template-preview"]'))).length === 0,
    30_000,
  )

  await (await waitForElement(`[data-testid="preview-watch-folder-${taskTemplateProfile.id}"]`)).click()
  const watchPreview = await waitForElement('[data-testid="watch-folder-preview"]', 30_000)
  const watchPreviewText = await watchPreview.getAttribute('textContent')
  assert.match(watchPreviewText, /当前仍是一次性扫描/)
  assert.match(watchPreviewText, /keep\.log/)
  assert.match(watchPreviewText, /命中排除规则/)
  assert.match(watchPreviewText, /未命中包含规则/)
  assert.doesNotMatch(watchPreviewText, /确认创建/)
  await (await waitForElement('[data-testid="close-watch-folder-preview"]')).click()

  const lateWatchCandidate = path.join(taskTemplateDirectory, 'late.log')
  writeFileSync(lateWatchCandidate, 'created after the one-shot preview', 'utf8')
  await new Promise((resolve) => setTimeout(resolve, 1_200))
  const stateAfterPreview = await callDesktopBridge(
    'taskTemplateAuditState',
    taskTemplateProfile.id,
    taskTemplateProfile.name,
  )
  assert.equal(stateAfterPreview.taskCount, 0, 'read-only preview must not create a desktop task')
  assert.equal(stateAfterPreview.draftGroups.length, 0, 'read-only preview must not create a draft')

  await (await waitForElement(`[data-testid="create-template-draft-${taskTemplateProfile.id}"]`)).click()
  const draftPlan = await waitForElement('[data-testid="template-draft-plan"]')
  const draftPlanText = await draftPlan.getAttribute('textContent')
  assert.match(draftPlanText, /只创建草稿，不启动任务/)
  assert.match(draftPlanText, /命中排除规则/)
  await (await waitForElement('[data-testid="confirm-template-draft"]')).click()
  await waitForElement('[data-testid="compression-group-row"]')

  const finalTaskTemplateState = await callDesktopBridge(
    'taskTemplateAuditState',
    taskTemplateProfile.id,
    taskTemplateProfile.name,
  )
  assert.equal(finalTaskTemplateState.importedProfiles.length, 1)
  assert.deepEqual(
    finalTaskTemplateState.importedProfiles[0],
    {
      password: null,
      deleteAfter: false,
      autoApplyEnabled: false,
      passwordStrategy: 'none',
      extraParams: {},
    },
    'the imported desktop profile must retain safe runtime defaults',
  )
  assert.deepEqual(
    finalTaskTemplateState.draftGroups,
    [{ fileCount: 1, password: '', deleteAfter: false, taskId: null }],
    'the confirmed template result must remain an inert, secret-free draft',
  )
  assert.equal(finalTaskTemplateState.taskCount, 0)
  assert.equal(finalTaskTemplateState.activeTaskCount, 0)
  assert.equal(finalTaskTemplateState.autoStartRequested, false)
  assert.equal(
    readdirSync(taskTemplateNestedDirectory).some(entry => entry.endsWith('.7z')),
    false,
    'the desktop template workflow must not create an archive before explicit start',
  )
  assert.equal(
    await callDesktopBridge('taskTemplateDialogQueueLength'),
    1,
    'the persistent watch-folder selection must remain queued for the lifecycle gate',
  )
  await callDesktopBridge('clearCompressionWorkspace')

  console.log('[desktop-e2e] verifying persistent watch-folder lifecycle and safe drafts')
  await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
  await (await waitForElement('[data-testid="manage-compression-profiles"]')).click()
  await waitForElement('.profile-manager')
  await (await waitForElement(`[data-testid="preview-watch-folder-${taskTemplateProfile.id}"]`)).click()
  const persistentWatchPreview = await waitForElement('[data-testid="watch-folder-preview"]', 30_000)
  assert.match(await persistentWatchPreview.getAttribute('textContent'), /保存并启用/)
  await (await waitForElement('[data-testid="save-watch-folder"]')).click()

  const initialWatchState = await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => state.registrations.length === 1 && state.registrations[0].status === 'active',
  )
  const watchRegistrationId = initialWatchState.registrations[0].id
  assert.equal(
    await callDesktopBridge('taskTemplateDialogQueueLength'),
    0,
    'every desktop task-template dialog selection must be consumed',
  )
  await new Promise((resolve) => setTimeout(resolve, 3_000))
  const baselineWatchState = await callDesktopBridge('watchFolderAuditState', taskTemplateProfile.id)
  assert.deepEqual(
    watchDraftPaths(baselineWatchState),
    [],
    'files that already existed when authorization was enabled must remain baseline only',
  )

  const activeWatchCandidate = path.join(taskTemplateDirectory, 'active-after-enable.log')
  writeFileSync(activeWatchCandidate, 'active watch payload', 'utf8')
  const activeWatchState = await waitForWatchFolderState(
    taskTemplateProfile.id,
    state =>
      state.pendingBatches.length === 0 &&
      watchDraftPaths(state).includes(normalizedDesktopPath(activeWatchCandidate)),
  )
  assertInertWatchDraftState(activeWatchState)
  await callDesktopBridge('clearCompressionWorkspace')

  await (await waitForElement(`[data-testid="pause-watch-folder-${watchRegistrationId}"]`)).click()
  await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => state.registrations[0]?.status === 'paused',
  )
  const pausedWatchCandidate = path.join(taskTemplateDirectory, 'created-while-paused.log')
  writeFileSync(pausedWatchCandidate, 'paused watch payload', 'utf8')
  await new Promise((resolve) => setTimeout(resolve, 3_500))
  assert.deepEqual(
    watchDraftPaths(await callDesktopBridge('watchFolderAuditState', taskTemplateProfile.id)),
    [],
    'paused authorization must not create a draft',
  )

  await (await waitForElement(`[data-testid="resume-watch-folder-${watchRegistrationId}"]`)).click()
  await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => state.registrations[0]?.status === 'active',
  )
  await new Promise((resolve) => setTimeout(resolve, 3_000))
  assert.deepEqual(
    watchDraftPaths(await callDesktopBridge('watchFolderAuditState', taskTemplateProfile.id)),
    [],
    'files created while paused must become resume baseline instead of retroactive drafts',
  )
  const resumedWatchCandidate = path.join(taskTemplateDirectory, 'active-after-resume.log')
  writeFileSync(resumedWatchCandidate, 'resumed watch payload', 'utf8')
  const resumedWatchState = await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => watchDraftPaths(state).includes(normalizedDesktopPath(resumedWatchCandidate)),
  )
  assertInertWatchDraftState(resumedWatchState)
  await callDesktopBridge('clearCompressionWorkspace')

  console.log('[desktop-e2e] verifying watch discovery while the real window is hidden to tray')
  await callDesktopBridge('setCloseToTray', true)
  const watchTrayHiddenMarker = path.join(fixtureDirectory, 'watch-tray-hidden.marker')
  const watchTrayRestoredMarker = path.join(fixtureDirectory, 'watch-tray-restored.marker')
  await hideDesktopWindow(watchTrayHiddenMarker)
  await waitForLocalFileContent(watchTrayHiddenMarker, 'hidden')
  const trayWatchCandidate = path.join(taskTemplateDirectory, 'created-while-in-tray.log')
  writeFileSync(trayWatchCandidate, 'tray watch payload', 'utf8')
  const trayWatchState = await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => watchDraftPaths(state).includes(normalizedDesktopPath(trayWatchCandidate)),
  )
  assertInertWatchDraftState(trayWatchState)
  assert.equal(await callDesktopBridge('isWindowVisible'), false)
  forwardContextAction('--desktop-e2e-restore', [watchTrayRestoredMarker])
  await waitForLocalFileContent(watchTrayRestoredMarker, 'visible')
  await callDesktopBridge('clearCompressionWorkspace')

  console.log('[desktop-e2e] verifying active watch restoration after a real application restart')
  await callDesktopBridge('setCloseToTray', false)
  await restartDesktopSession()
  const restoredWatchState = await waitForWatchFolderState(
    taskTemplateProfile.id,
    state =>
      state.registrations.length === 1 &&
      state.registrations[0].id === watchRegistrationId &&
      state.registrations[0].status === 'active',
    45_000,
  )
  assert.deepEqual(watchDraftPaths(restoredWatchState), [])
  const restartedWatchCandidate = path.join(taskTemplateDirectory, 'active-after-restart.log')
  writeFileSync(restartedWatchCandidate, 'restart watch payload', 'utf8')
  const restartedCandidateState = await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => watchDraftPaths(state).includes(normalizedDesktopPath(restartedWatchCandidate)),
    45_000,
  )
  assertInertWatchDraftState(restartedCandidateState)
  await callDesktopBridge('clearCompressionWorkspace')

  navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[1].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/compress'), 30_000)
  await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
  await (await waitForElement('[data-testid="manage-compression-profiles"]')).click()
  await waitForElement(`[data-testid="watch-folder-registration-${watchRegistrationId}"]`)

  await (await waitForElement(`[data-testid="disable-watch-folder-${watchRegistrationId}"]`)).click()
  await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => state.registrations[0]?.status === 'disabled',
  )
  const disabledWatchCandidate = path.join(taskTemplateDirectory, 'created-while-disabled.log')
  writeFileSync(disabledWatchCandidate, 'disabled watch payload', 'utf8')
  await new Promise((resolve) => setTimeout(resolve, 3_500))
  assert.deepEqual(
    watchDraftPaths(await callDesktopBridge('watchFolderAuditState', taskTemplateProfile.id)),
    [],
    'disabled authorization must not create a draft',
  )

  await (await waitForElement(`[data-testid="resume-watch-folder-${watchRegistrationId}"]`)).click()
  await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => state.registrations[0]?.status === 'active',
  )
  await new Promise((resolve) => setTimeout(resolve, 3_000))
  assert.deepEqual(
    watchDraftPaths(await callDesktopBridge('watchFolderAuditState', taskTemplateProfile.id)),
    [],
    'files created while disabled must become the next activation baseline',
  )

  await driver.executeScript(
    `const originalConfirm = window.confirm;
     window.confirm = () => true;
     try { document.querySelector(arguments[0])?.click(); }
     finally { window.confirm = originalConfirm; }`,
    `[data-testid="delete-watch-folder-${watchRegistrationId}"]`,
  )
  await waitForWatchFolderState(
    taskTemplateProfile.id,
    state => state.registrations.length === 0,
  )
  const deletedWatchCandidate = path.join(taskTemplateDirectory, 'created-after-delete.log')
  writeFileSync(deletedWatchCandidate, 'deleted watch payload', 'utf8')
  await new Promise((resolve) => setTimeout(resolve, 3_500))
  const deletedWatchState = await callDesktopBridge('watchFolderAuditState', taskTemplateProfile.id)
  assert.deepEqual(watchDraftPaths(deletedWatchState), [])
  assert.equal(deletedWatchState.pendingBatches.length, 0)
  assertInertWatchDraftState(deletedWatchState)
  await callDesktopBridge('setCloseToTray', true)

  if (watchFolderLifecycleOnly) {
    completedSuccessfully = true
    console.log('Real Windows Tauri watch-folder lifecycle gate passed.')
  } else {
  console.log('[desktop-e2e] verifying native 7Z progress and byte-for-byte round-trip')
  const sevenZipSource = path.join(fixtureDirectory, 'sevenzip-payload.bin')
  const sevenZipArchive = path.join(fixtureDirectory, 'sevenzip-payload.7z')
  const sevenZipOutput = path.join(fixtureDirectory, 'sevenzip-output')
  const sevenZipPayload = randomBytes(24 * 1024 * 1024)
  writeFileSync(sevenZipSource, sevenZipPayload)
  const sevenZipResult = await callDesktopBridge(
    'runSevenZipRoundTrip',
    sevenZipSource,
    sevenZipArchive,
    sevenZipOutput,
  )
  const sevenZipExtracted = path.join(sevenZipOutput, 'sevenzip-payload.bin')
  assert.deepEqual(
    readFileSync(sevenZipExtracted),
    sevenZipPayload,
    'native 7Z extraction must reproduce the source byte-for-byte',
  )
  assert.ok(
    sevenZipResult.compressionProgress.some(progress => progress > 0 && progress < 100),
    `native 7Z compression must emit intermediate byte progress: ${sevenZipResult.compressionProgress}`,
  )
  assert.ok(
    sevenZipResult.extractionProgress.some(progress => progress > 0 && progress < 100),
    `native 7Z extraction must emit intermediate byte progress: ${sevenZipResult.extractionProgress}`,
  )

  console.log('[desktop-e2e] verifying cancellation of a real native 7Z compression')
  const cancelSource = path.join(fixtureDirectory, 'sevenzip-cancel-source.bin')
  const cancelArchive = path.join(fixtureDirectory, 'sevenzip-cancelled.7z')
  writeFileSync(cancelSource, randomBytes(96 * 1024 * 1024))
  const realCancelTaskId = await callDesktopBridge(
    'startSevenZipCompression',
    cancelSource,
    cancelArchive,
  )
  await driver.wait(
    async () => {
      const progress = await callDesktopBridge('taskProgress', realCancelTaskId)
      const status = await callDesktopBridge('taskStatus', realCancelTaskId)
      return (progress ?? 0) > 0 || status === 'completed'
    },
    30_000,
  )
  assert.notEqual(
    await callDesktopBridge('taskStatus', realCancelTaskId),
    'completed',
    'real 7Z fixture completed before cancellation could be exercised',
  )
  assert.equal(await callDesktopBridge('cancelTask', realCancelTaskId), true)
  await driver.wait(
    async () => (await callDesktopBridge('taskStatus', realCancelTaskId)) === 'cancelled',
    30_000,
  )
  assert.equal(existsSync(cancelArchive), false, 'cancelled native 7Z must not leave a final archive')
  await callDesktopBridge('clearTasks')

  console.log('[desktop-e2e] verifying every user-creatable archive format')
  const matrixSource = path.join(fixtureDirectory, 'matrix-payload.txt')
  const matrixPayload = Buffer.from(`Long解压 archive matrix ${new Date().toISOString()}\n`, 'utf8')
  writeFileSync(matrixSource, matrixPayload)
  const archiveMatrix = [
    ['zip', 'zip', null, 'zip'],
    ['7z', '7z', null, '7z'],
    ['wim', 'wim', null, 'wim'],
    ['tar', 'tar', null, 'tar'],
    ['tar.gz', 'tar.gz', null, 'tar.gz'],
    ['tar.bz2', 'tar.bz2', null, 'tar.bz2'],
    ['tar.xz', 'tar.xz', null, 'tar.xz'],
    ['tar.zst', 'tar.zst', null, 'tar.zst'],
    ['gz', 'txt.gz', null, 'gz'],
    ['bz2', 'txt.bz2', null, 'bz2'],
    ['xz', 'txt.xz', null, 'xz'],
    ['zst', 'txt.zst', null, 'zst'],
    ['zstd', 'txt.zstd', null, 'zstd'],
    ['lzma', 'txt.lzma', null, 'lzma'],
    ['zip-password', 'zip', 'desktop-e2e-password', null],
    ['7z-password', '7z', 'desktop-e2e-password', null],
    ['tar.aes', 'tar.aes', 'desktop-e2e-password', 'tar.aes'],
    ['tar.gz.aes', 'tar.gz.aes', 'desktop-e2e-password', 'tar.gz.aes'],
    ['tar.bz2.aes', 'tar.bz2.aes', 'desktop-e2e-password', 'tar.bz2.aes'],
    ['tar.xz.aes', 'tar.xz.aes', 'desktop-e2e-password', 'tar.xz.aes'],
    ['tar.zst.aes', 'tar.zst.aes', 'desktop-e2e-password', 'tar.zst.aes'],
    ['gz.aes', 'txt.gz.aes', 'desktop-e2e-password', 'gz.aes'],
    ['bz2.aes', 'txt.bz2.aes', 'desktop-e2e-password', 'bz2.aes'],
    ['xz.aes', 'txt.xz.aes', 'desktop-e2e-password', 'xz.aes'],
    ['zst.aes', 'txt.zst.aes', 'desktop-e2e-password', 'zst.aes'],
  ]
  const capabilitySource = readFileSync(
    path.join(root, 'src', 'utils', 'compressionFormat.ts'),
    'utf8',
  )
  const capabilityBlock = capabilitySource
    .split('export const FORMAT_CAPABILITIES')[1]
    ?.split('export interface ExtractOnlyFormatCapability')[0] || ''
  const declaredCreatableFormats = [
    ...capabilityBlock.matchAll(/format:\s*'([^']+)'[^\r\n]*canCompress:\s*true/g),
  ].map(match => match[1])
  const exercisedFormats = new Set(
    archiveMatrix.map(([label]) =>
      label.endsWith('-password') ? label.slice(0, -'-password'.length) : label,
    ),
  )
  exercisedFormats.add('rar')
  const missingCreatableFormats = declaredCreatableFormats
    .filter(format => !exercisedFormats.has(format))
    .sort()
  assert.deepEqual(
    missingCreatableFormats,
    [],
    'every format advertised as creatable must have a real desktop scenario',
  )
  const verifiedDeclaredExtensions = new Set()
  const createdArchiveByFormat = new Map()
  for (const [label, extension, password, declaredExtension] of archiveMatrix) {
    const format = label.endsWith('-password') ? label.slice(0, -'-password'.length) : label
    const caseRoot = path.join(fixtureDirectory, `matrix-${label}`)
    mkdirSync(caseRoot, { recursive: true })
    const archive = path.join(caseRoot, `matrix-payload.${extension}`)
    const output = path.join(caseRoot, 'output')
    await callDesktopBridge(
      'runArchiveRoundTrip',
      matrixSource,
      archive,
      output,
      format,
      password,
    )
    const extracted = path.join(output, 'matrix-payload.txt')
    assert.deepEqual(
      readFileSync(extracted),
      matrixPayload,
      `${label} extraction must reproduce the source byte-for-byte`,
    )
    if (declaredExtension) verifiedDeclaredExtensions.add(declaredExtension)
    if (!password) createdArchiveByFormat.set(format, archive)
  }
  const creatableAliasMatrix = [
    ['zip', 'zipx', 'matrix-payload.txt'],
    ['tar', 'ova', 'matrix-payload.txt'],
    ['tar.gz', 'tgz', 'matrix-payload.txt'],
    ['tar.gz', 'tpz', 'matrix-payload.txt'],
    ['tar.bz2', 'tbz', 'matrix-payload.txt'],
    ['tar.bz2', 'tbz2', 'matrix-payload.txt'],
    ['tar.xz', 'txz', 'matrix-payload.txt'],
    ['tar.zst', 'tzst', 'matrix-payload.txt'],
    ['gz', 'gzip', 'matrix-payload'],
    ['bz2', 'bzip2', 'matrix-payload'],
  ]
  for (const [format, extension, extractedName] of creatableAliasMatrix) {
    const aliasRoot = path.join(fixtureDirectory, `matrix-alias-${extension}`)
    const aliasArchive = path.join(aliasRoot, `matrix-payload.${extension}`)
    const aliasOutput = path.join(aliasRoot, 'output')
    mkdirSync(aliasRoot, { recursive: true })
    copyFileSync(createdArchiveByFormat.get(format), aliasArchive)
    await callDesktopBridge('extractArchive', aliasArchive, aliasOutput)
    assert.deepEqual(
      readFileSync(path.join(aliasOutput, extractedName)),
      matrixPayload,
      `${extension} alias extraction must reproduce the source byte-for-byte`,
    )
    verifiedDeclaredExtensions.add(extension)
  }
  await callDesktopBridge('clearTasks')

  const rarCommand = spawnSync(
    'where.exe',
    ['Rar.exe'],
    { encoding: 'utf8', timeout: 10_000, windowsHide: true },
  )
  if (rarCommand.status !== 0) {
    console.log('[desktop-e2e] verifying RAR creation fails clearly when WinRAR is unavailable')
    const rarCaseRoot = path.join(fixtureDirectory, 'matrix-rar-without-encoder')
    mkdirSync(rarCaseRoot, { recursive: true })
    const rarArchive = path.join(rarCaseRoot, 'matrix-payload.rar')
    const rarError = await callDesktopBridgeFailure(
      'runArchiveRoundTrip',
      matrixSource,
      rarArchive,
      path.join(rarCaseRoot, 'output'),
      'rar',
      null,
    )
    assert.match(
      rarError,
      /WinRAR|Rar\.exe|RAR command/i,
      `RAR encoder failure must tell the user which dependency is missing: ${rarError}`,
    )
    assert.equal(existsSync(rarArchive), false, 'failed RAR creation must not leave an output archive')
    await callDesktopBridge('clearTasks')
  }

  console.log('[desktop-e2e] verifying real extract-only package and legacy archive samples')
  const extractOnlyPayload = Buffer.from(
    `Long Decompress extract-only matrix ${new Date().toISOString()}\n`,
    'utf8',
  )
  const extractOnlySource = path.join(fixtureDirectory, 'extract-only-payload.txt')
  writeFileSync(extractOnlySource, extractOnlyPayload)
  const extractOnlyMatrix = []
  const addExtractOnlyCase = (
    extension,
    archive,
    extractedName,
    expectedPayload = extractOnlyPayload,
  ) => {
    extractOnlyMatrix.push([extension, archive, extractedName, expectedPayload])
    verifiedDeclaredExtensions.add(extension)
  }
  for (const extension of ['jar', 'xpi', 'ipa', 'apk', 'appx']) {
    const archive = path.join(fixtureDirectory, `extract-only.${extension}`)
    createZipCompatibleFixture(archive, extractOnlySource)
    addExtractOnlyCase(extension, archive, 'extract-only-payload.txt')
  }
  const cabArchive = path.join(fixtureDirectory, 'extract-only.cab')
  runFixtureCommand(
    'makecab.exe',
    ['/D', 'CompressionType=LZX', extractOnlySource, cabArchive],
    'CAB',
  )
  addExtractOnlyCase('cab', cabArchive, 'extract-only-payload.txt')
  const arArchive = path.join(fixtureDirectory, 'extract-only.ar')
  createArFixture(arArchive, 'payload.txt', extractOnlyPayload)
  addExtractOnlyCase('ar', arArchive, 'payload.txt')
  const aArchive = path.join(fixtureDirectory, 'extract-only.a')
  copyFileSync(arArchive, aArchive)
  addExtractOnlyCase('a', aArchive, 'payload.txt')
  for (const format of ['iso9660', 'cpio']) {
    const extension = format === 'iso9660' ? 'iso' : format
    const archive = path.join(fixtureDirectory, `extract-only.${extension}`)
    runFixtureCommand(
      'tar.exe',
      ['-cf', archive, '--format', format, path.basename(extractOnlySource)],
      format.toUpperCase(),
    )
    addExtractOnlyCase(extension, archive, 'extract-only-payload.txt')
  }
  const xarArchive = path.join(fixtureDirectory, 'extract-only.xar')
  createXarFixture(xarArchive, 'extract-only-payload.txt', extractOnlyPayload)
  addExtractOnlyCase('xar', xarArchive, 'extract-only-payload.txt')
  const wslExtProbe = spawnSync(
    'wsl.exe',
    ['-d', 'Ubuntu', '--', 'test', '-x', '/sbin/mkfs.ext4'],
    { encoding: 'utf8', timeout: 30_000, windowsHide: true },
  )
  let ext4Archive
  if (wslExtProbe.status === 0) {
    const extSourceDirectory = path.join(fixtureDirectory, 'ext-source')
    mkdirSync(extSourceDirectory, { recursive: true })
    copyFileSync(extractOnlySource, path.join(extSourceDirectory, 'extract-only-payload.txt'))
    for (const version of ['2', '3', '4']) {
      const archive = path.join(fixtureDirectory, `extract-only.ext${version}`)
      runFixtureCommand(
        'wsl.exe',
        [
          '-d',
          'Ubuntu',
          '--',
          `/sbin/mkfs.ext${version}`,
          '-q',
          '-F',
          '-d',
          toWslMountPath(extSourceDirectory),
          toWslMountPath(archive),
          '16M',
        ],
        `EXT${version}`,
      )
      addExtractOnlyCase(`ext${version}`, archive, 'extract-only-payload.txt')
      if (version === '4') {
        ext4Archive = archive
        const extAlias = path.join(fixtureDirectory, 'extract-only.ext')
        const imgAlias = path.join(fixtureDirectory, 'extract-only.img')
        copyFileSync(archive, extAlias)
        copyFileSync(archive, imgAlias)
        addExtractOnlyCase('ext', extAlias, 'extract-only-payload.txt')
        addExtractOnlyCase('img', imgAlias, 'extract-only-payload.txt')
      }
    }
  } else {
    recordMissingFullFormatCapability(
      'WSL EXT2/3/4 generators',
      'install mke2fs in the Ubuntu WSL distribution',
    )
  }
  const qemuImgProbe = existsSync(qemuImg)
    ? spawnSync(qemuImg, ['--version'], {
        encoding: 'utf8',
        timeout: 30_000,
        windowsHide: true,
      })
    : null
  if (wslExtProbe.status === 0 && qemuImgProbe?.status === 0) {
    const virtualDiskSourceDirectory = path.join(fixtureDirectory, 'virtual-disk-source')
    mkdirSync(virtualDiskSourceDirectory, { recursive: true })
    copyFileSync(
      extractOnlySource,
      path.join(virtualDiskSourceDirectory, 'extract-only-payload.txt'),
    )
    const rawDiskImage = path.join(fixtureDirectory, 'virtual-disk-base.raw')
    runFixtureCommand(
      'wsl.exe',
      [
        '-d',
        'Ubuntu',
        '--',
        '/sbin/mkfs.ext4',
        '-q',
        '-F',
        '-d',
        toWslMountPath(virtualDiskSourceDirectory),
        toWslMountPath(rawDiskImage),
        '16M',
      ],
      'virtual-disk EXT4 payload',
    )
    for (const [format, extension] of [
      ['qcow2', 'qcow2'],
      ['vdi', 'vdi'],
      ['vmdk', 'vmdk'],
      ['vpc', 'vhd'],
      ['vhdx', 'vhdx'],
    ]) {
      const image = path.join(fixtureDirectory, `extract-only.${extension}`)
      runFixtureCommand(
        qemuImg,
        ['convert', '-f', 'raw', '-O', format, rawDiskImage, image],
        format.toUpperCase(),
      )
      addExtractOnlyCase(extension, image, 'extract-only-payload.txt')
      if (extension === 'qcow2') {
        for (const alias of ['qcow', 'qcow2c']) {
          const aliasImage = path.join(fixtureDirectory, `extract-only.${alias}`)
          copyFileSync(image, aliasImage)
          addExtractOnlyCase(alias, aliasImage, 'extract-only-payload.txt')
        }
      }
    }
    console.log(
      '[desktop-e2e] generated QCOW2, VDI, VMDK, VHD and VHDX images with known payloads',
    )
  } else {
    recordMissingFullFormatCapability(
      'QCOW2/VDI/VMDK/VHD/VHDX generators',
      'npm run test:tools:qemu-img and install mke2fs in Ubuntu WSL',
    )
  }
  const mkfsFat = path.join(wslFsToolRoot, 'usr', 'sbin', 'mkfs.fat')
  const mcopy = path.join(wslFsToolRoot, 'usr', 'bin', 'mcopy')
  const mtools = path.join(wslFsToolRoot, 'usr', 'bin', 'mtools')
  const mkntfs = path.join(wslFsToolRoot, 'sbin', 'mkntfs')
  const ntfscp = path.join(wslFsToolRoot, 'sbin', 'ntfscp')
  const wslFsLibraryPath = path.join(wslFsToolRoot, 'lib', 'x86_64-linux-gnu')
  // mcopy is a package symlink that Windows Node cannot stat reliably on DrvFs.
  const hasWslFsTools = [mkfsFat, mtools, mkntfs, ntfscp].every(existsSync)
  if (qemuImgProbe?.status === 0 && hasWslFsTools) {
    const fatImage = path.join(fixtureDirectory, 'extract-only.fat')
    runFixtureCommand(
      'wsl.exe',
      [
        '-d',
        'Ubuntu',
        '--',
        toWslMountPath(mkfsFat),
        '-C',
        '-F',
        '16',
        '--invariant',
        toWslMountPath(fatImage),
        '16384',
      ],
      'FAT16',
    )
    runFixtureCommand(
      'wsl.exe',
      [
        '-d',
        'Ubuntu',
        '--',
        toWslMountPath(mcopy),
        '-i',
        toWslMountPath(fatImage),
        toWslMountPath(extractOnlySource),
        '::extract-only-payload.txt',
      ],
      'FAT16 payload copy',
    )
    addExtractOnlyCase('fat', fatImage, 'extract-only-payload.txt')

    const ntfsImage = path.join(fixtureDirectory, 'extract-only.ntfs')
    runFixtureCommand(qemuImg, ['create', '-f', 'raw', ntfsImage, '32M'], 'NTFS raw image')
    const ntfsEnvironment = [
      '-d',
      'Ubuntu',
      '--',
      '/usr/bin/env',
      `LD_LIBRARY_PATH=${toWslMountPath(wslFsLibraryPath)}`,
    ]
    runFixtureCommand(
      'wsl.exe',
      [
        ...ntfsEnvironment,
        toWslMountPath(mkntfs),
        '-F',
        '-Q',
        '-L',
        'LONGTEST',
        toWslMountPath(ntfsImage),
      ],
      'NTFS',
    )
    runFixtureCommand(
      'wsl.exe',
      [
        ...ntfsEnvironment,
        toWslMountPath(ntfscp),
        toWslMountPath(ntfsImage),
        toWslMountPath(extractOnlySource),
        '/extract-only-payload.txt',
      ],
      'NTFS payload copy',
    )
    addExtractOnlyCase('ntfs', ntfsImage, 'extract-only-payload.txt')
    console.log('[desktop-e2e] generated FAT16 and NTFS images with known payloads')
  } else {
    recordMissingFullFormatCapability(
      'FAT16/NTFS generators',
      'npm run test:tools:qemu-img && npm run test:tools:wsl-fs',
    )
  }
  const wslSquashFsProbe = spawnSync(
    'wsl.exe',
    ['-d', 'Ubuntu', '--', 'test', '-x', '/usr/bin/mksquashfs'],
    { encoding: 'utf8', timeout: 30_000, windowsHide: true },
  )
  if (wslSquashFsProbe.status === 0) {
    const squashFsSourceDirectory = path.join(fixtureDirectory, 'squashfs-source')
    mkdirSync(squashFsSourceDirectory, { recursive: true })
    copyFileSync(
      extractOnlySource,
      path.join(squashFsSourceDirectory, 'extract-only-payload.txt'),
    )
    const squashFsArchive = path.join(fixtureDirectory, 'extract-only.squashfs')
    runFixtureCommand(
      'wsl.exe',
      [
        '-d',
        'Ubuntu',
        '--',
        '/usr/bin/mksquashfs',
        toWslMountPath(squashFsSourceDirectory),
        toWslMountPath(squashFsArchive),
        '-noappend',
        '-quiet',
        '-no-progress',
      ],
      'SquashFS',
    )
    console.log('[desktop-e2e] generated a real SquashFS image with a known payload')
    addExtractOnlyCase('squashfs', squashFsArchive, 'extract-only-payload.txt')
    const squashFsAlias = path.join(fixtureDirectory, 'extract-only.sfs')
    copyFileSync(squashFsArchive, squashFsAlias)
    addExtractOnlyCase('sfs', squashFsAlias, 'extract-only-payload.txt')
  } else {
    recordMissingFullFormatCapability(
      'SquashFS generator',
      'install squashfs-tools in the Ubuntu WSL distribution',
    )
  }

  const apfsGo = path.join(apfsToolRoot, 'go-sdk', 'go', 'bin', 'go.exe')
  const apfsSource = path.join(apfsToolRoot, 'source', 'go.mod')
  const apfsGenerator = path.join(root, 'tests', 'fixtures', 'apfs-generator')
  if (existsSync(apfsGo) && existsSync(apfsSource)) {
    const apfsArchive = path.join(fixtureDirectory, 'extract-only.apfs')
    runFixtureCommand(
      apfsGo,
      ['run', '.', apfsArchive, extractOnlySource],
      'APFS',
      {
        cwd: apfsGenerator,
        env: {
          ...process.env,
          GOTOOLCHAIN: 'local',
          GOMODCACHE: path.join(apfsToolRoot, 'mod-cache'),
          GOCACHE: path.join(apfsToolRoot, 'build-cache'),
        },
        timeout: 180_000,
      },
    )
    addExtractOnlyCase('apfs', apfsArchive, 'extract-only-payload.txt')
    console.log('[desktop-e2e] generated a real APFS image with a known payload')
  } else {
    recordMissingFullFormatCapability('APFS generator', 'npm run test:tools:apfs')
  }

  if (existsSync(hfsxFixture)) {
    addExtractOnlyCase(
      'hfsx',
      hfsxFixture,
      path.join('Firefox', 'known-payload.txt'),
      Buffer.from('Long Decompress HFSX real payload\n', 'utf8'),
    )
    console.log('[desktop-e2e] prepared a non-empty HFSX image with a known payload')
  } else {
    recordMissingFullFormatCapability('HFSX generator', 'npm run test:fixtures:hfsx')
  }
  if (existsSync(hfsFixture)) {
    addExtractOnlyCase(
      'hfs',
      hfsFixture,
      path.join('Firefox', 'known-payload.txt'),
      Buffer.from('Long Decompress HFSX real payload\n', 'utf8'),
    )
  } else {
    recordMissingFullFormatCapability('HFS generator', 'npm run test:fixtures:hfsx')
  }

  const wix3Tools = Object.fromEntries(
    ['candle', 'light', 'torch', 'pyro'].map((tool) => [
      tool,
      path.join(wix3ToolRoot, `${tool}.exe`),
    ]),
  )
  if (Object.values(wix3Tools).every(existsSync)) {
    const installerFixtureRoot = path.join(fixtureDirectory, 'windows-installer-fixtures')
    const productV1Root = path.join(installerFixtureRoot, 'v1')
    const productV2Root = path.join(installerFixtureRoot, 'v2')
    mkdirSync(productV1Root, { recursive: true })
    mkdirSync(productV2Root, { recursive: true })
    const productV1Payload = path.join(productV1Root, 'extract-only-payload.txt')
    const productV2Payload = path.join(productV2Root, 'extract-only-payload.txt')
    const updatedInstallerPayload = Buffer.from(
      `Long Decompress updated MSP payload ${new Date().toISOString()}\n`,
      'utf8',
    )
    writeFileSync(productV1Payload, extractOnlyPayload)
    writeFileSync(productV2Payload, updatedInstallerPayload)
    const productSource = path.join(root, 'tests', 'fixtures', 'minimal-product.wxs')
    const moduleSource = path.join(root, 'tests', 'fixtures', 'minimal-module.wxs')
    const patchSource = path.join(root, 'tests', 'fixtures', 'minimal-patch.wxs')

    for (const [version, source] of [
      ['1', productV1Payload],
      ['2', productV2Payload],
    ]) {
      const productVersion = version === '1' ? '1.0.0' : '1.0.1'
      runFixtureCommand(
        wix3Tools.candle,
        [
          '-nologo',
          `-dSourceFile=${source}`,
          `-dProductVersion=${productVersion}`,
          '-out',
          path.join(installerFixtureRoot, `product-v${version}.wixobj`),
          productSource,
        ],
        `MSI v${version} compile`,
      )
      runFixtureCommand(
        wix3Tools.light,
        [
          '-nologo',
          '-sval',
          '-out',
          path.join(installerFixtureRoot, `product-v${version}.msi`),
          path.join(installerFixtureRoot, `product-v${version}.wixobj`),
        ],
        `MSI v${version}`,
      )
    }

    runFixtureCommand(
      wix3Tools.candle,
      [
        '-nologo',
        `-dSourceFile=${productV1Payload}`,
        '-out',
        path.join(installerFixtureRoot, 'module.wixobj'),
        moduleSource,
      ],
      'MSM compile',
    )
    runFixtureCommand(
      wix3Tools.light,
      [
        '-nologo',
        '-sval',
        '-out',
        path.join(installerFixtureRoot, 'fixture.msm'),
        path.join(installerFixtureRoot, 'module.wixobj'),
      ],
      'MSM',
    )
    runFixtureCommand(
      wix3Tools.torch,
      [
        '-nologo',
        '-p',
        '-xi',
        path.join(installerFixtureRoot, 'product-v1.wixpdb'),
        path.join(installerFixtureRoot, 'product-v2.wixpdb'),
        '-out',
        path.join(installerFixtureRoot, 'fixture.wixmst'),
      ],
      'MSP transform',
    )
    runFixtureCommand(
      wix3Tools.candle,
      [
        '-nologo',
        '-out',
        path.join(installerFixtureRoot, 'patch.wixobj'),
        patchSource,
      ],
      'MSP compile',
    )
    runFixtureCommand(
      wix3Tools.light,
      [
        '-nologo',
        '-sval',
        '-out',
        path.join(installerFixtureRoot, 'patch.wixmsp'),
        path.join(installerFixtureRoot, 'patch.wixobj'),
      ],
      'MSP link',
    )
    runFixtureCommand(
      wix3Tools.pyro,
      [
        '-nologo',
        path.join(installerFixtureRoot, 'patch.wixmsp'),
        '-out',
        path.join(installerFixtureRoot, 'fixture.msp'),
        '-t',
        'RTM',
        path.join(installerFixtureRoot, 'fixture.wixmst'),
      ],
      'MSP',
    )

    addExtractOnlyCase(
      'msi',
      path.join(installerFixtureRoot, 'product-v1.msi'),
      'PayloadFile',
    )
    addExtractOnlyCase(
      'msm',
      path.join(installerFixtureRoot, 'fixture.msm'),
      'PayloadFile.719C727A_2D5C_4ED6_A487_F2BEA6D8094F',
    )
    addExtractOnlyCase(
      'msp',
      path.join(installerFixtureRoot, 'fixture.msp'),
      'PayloadFile',
      updatedInstallerPayload,
    )
    console.log('[desktop-e2e] generated real MSI, MSM and MSP containers with known payloads')
  } else {
    recordMissingFullFormatCapability('MSI/MSM/MSP generators', 'npm run test:tools:wix3')
  }

  const wslDiskLayoutProbe = spawnSync(
    'wsl.exe',
    ['-d', 'Ubuntu', '--', 'sh', '-lc', 'command -v sfdisk >/dev/null && command -v dd >/dev/null'],
    { encoding: 'utf8', timeout: 30_000, windowsHide: true },
  )
  if (ext4Archive && wslDiskLayoutProbe.status === 0) {
    for (const layout of ['gpt', 'mbr']) {
      const diskImage = path.join(fixtureDirectory, `extract-only.${layout}`)
      const wslDiskImage = toWslMountPath(diskImage)
      const wslPartitionImage = toWslMountPath(ext4Archive)
      runFixtureCommand(
        'wsl.exe',
        [
          '-d',
          'Ubuntu',
          '--',
          'bash',
          '-lc',
          [
            'set -euo pipefail',
            `truncate -s 20M '${wslDiskImage}'`,
            `printf '2048,32768,L\\n' | sfdisk --label ${layout === 'gpt' ? 'gpt' : 'dos'} '${wslDiskImage}' >/dev/null`,
            `dd if='${wslPartitionImage}' of='${wslDiskImage}' bs=512 seek=2048 conv=notrunc status=none`,
          ].join('; '),
        ],
        layout.toUpperCase(),
        { timeout: 120_000 },
      )
      addExtractOnlyCase(layout, diskImage, 'extract-only-payload.txt')
    }
    console.log('[desktop-e2e] generated real GPT and MBR disk images with EXT4 payloads')
  } else {
    recordMissingFullFormatCapability(
      'GPT/MBR generators',
      'install util-linux in the Ubuntu WSL distribution',
    )
  }

  const wslCramFsProbe = spawnSync(
    'wsl.exe',
    ['-d', 'Ubuntu', '--', 'test', '-x', '/usr/sbin/mkfs.cramfs'],
    { encoding: 'utf8', timeout: 30_000, windowsHide: true },
  )
  if (wslCramFsProbe.status === 0) {
    const cramFsSourceDirectory = path.join(fixtureDirectory, 'cramfs-source')
    const cramFsArchive = path.join(fixtureDirectory, 'extract-only.cramfs')
    mkdirSync(cramFsSourceDirectory, { recursive: true })
    copyFileSync(
      extractOnlySource,
      path.join(cramFsSourceDirectory, 'extract-only-payload.txt'),
    )
    runFixtureCommand(
      'wsl.exe',
      [
        '-d',
        'Ubuntu',
        '--',
        '/usr/sbin/mkfs.cramfs',
        toWslMountPath(cramFsSourceDirectory),
        toWslMountPath(cramFsArchive),
      ],
      'CRAMFS',
    )
    addExtractOnlyCase('cramfs', cramFsArchive, 'extract-only-payload.txt')
  } else {
    recordMissingFullFormatCapability(
      'CRAMFS generator',
      'install cramfsprogs in the Ubuntu WSL distribution',
    )
  }

  const ihexArchive = path.join(fixtureDirectory, 'extract-only.ihex')
  writeFileSync(ihexArchive, ':0500000048656C6C6F07\n:00000001FF', 'ascii')
  addExtractOnlyCase('ihex', ihexArchive, 'extract-only', Buffer.from('Hello', 'ascii'))

  if (existsSync(nsisFixture)) {
    addExtractOnlyCase(
      'nsis',
      nsisFixture,
      path.join('resources', 'archive-engine', '7-Zip-License.txt'),
      readFileSync(path.join(root, 'src-tauri', 'resources', 'archive-engine', '7-Zip-License.txt')),
    )
  } else {
    recordMissingFullFormatCapability(
      'NSIS installer fixture',
      'build the current release installer or set NSIS_FIXTURE_PATH',
    )
  }

  for (const [label, archive, extractedName, expectedPayload = extractOnlyPayload] of extractOnlyMatrix) {
    const output = path.join(fixtureDirectory, `extract-only-${label}-output`)
    await callDesktopBridge('extractArchive', archive, output)
    assert.deepEqual(
      readFileSync(path.join(output, extractedName)),
      expectedPayload,
      `${label} extraction must reproduce the real sample byte-for-byte`,
    )
  }
  await callDesktopBridge('clearTasks')

  if (existsSync(ovmfFirmware)) {
    console.log('[desktop-e2e] verifying a pinned Ubuntu OVMF UEFI firmware image')
    const firmwareFixture = path.join(fixtureDirectory, 'extract-only.uefif')
    copyFileSync(ovmfFirmware, firmwareFixture)
    const firmwareOutput = path.join(fixtureDirectory, 'uefi-firmware-output')
    await callDesktopBridge('extractArchive', firmwareFixture, firmwareOutput)
    const peiCore = path.join(
      firmwareOutput,
      '9E21FD93',
      'EE4E5898',
      'VOLUME',
      'PeiCore',
      '1.efi',
    )
    assert.equal(
      fileSha256(peiCore),
      'bb229cf4e15c4d96e67dff30770f5ca47e2e513f496b5d09c0daf96a53c12e9d',
      'UEFI firmware extraction must reproduce the pinned PeiCore module',
    )
    verifiedDeclaredExtensions.add('uefif')
    await callDesktopBridge('clearTasks')
  } else {
    recordMissingFullFormatCapability('UEFI firmware fixture', 'npm run test:fixtures:ovmf')
  }

  console.log('[desktop-e2e] verifying a real Debian package container')
  const debianBinary = Buffer.from('2.0\n', 'ascii')
  const controlSource = path.join(fixtureDirectory, 'control.txt')
  const controlTar = path.join(fixtureDirectory, 'control.tar')
  const dataTar = path.join(fixtureDirectory, 'data.tar')
  writeFileSync(controlSource, 'Package: long-decompress-e2e\nVersion: 1.0\n', 'utf8')
  runFixtureCommand(
    'tar.exe',
    ['-cf', controlTar, '--format', 'pax', path.basename(controlSource)],
    'DEB control.tar',
  )
  runFixtureCommand(
    'tar.exe',
    ['-cf', dataTar, '--format', 'pax', path.basename(extractOnlySource)],
    'DEB data.tar',
  )
  const debArchive = path.join(fixtureDirectory, 'extract-only.deb')
  createArEntries(debArchive, [
    ['debian-binary', debianBinary],
    ['control.tar', readFileSync(controlTar)],
    ['data.tar', readFileSync(dataTar)],
  ])
  const udebArchive = path.join(fixtureDirectory, 'extract-only.udeb')
  copyFileSync(debArchive, udebArchive)
  for (const [extension, archive] of [['deb', debArchive], ['udeb', udebArchive]]) {
    const debOutput = path.join(fixtureDirectory, `extract-only-${extension}-output`)
    await callDesktopBridge('extractArchive', archive, debOutput)
    const debPayloadOutput = path.join(
      fixtureDirectory,
      `extract-only-${extension}-payload-output`,
    )
    await callDesktopBridge('extractArchive', path.join(debOutput, 'data.tar'), debPayloadOutput)
    assert.deepEqual(
      readFileSync(path.join(debPayloadOutput, 'extract-only-payload.txt')),
      extractOnlyPayload,
      `${extension} data archive must reproduce the package payload byte-for-byte`,
    )
    verifiedDeclaredExtensions.add(extension)
  }
  await callDesktopBridge('clearTasks')

  console.log('[desktop-e2e] verifying pinned upstream RAR, LHA, RPM and DMG/HFS samples')
  runFixtureCommand(
    process.execPath,
    [path.join(root, 'scripts', 'fetch-archive-test-fixtures.mjs')],
    'external archive',
  )
  const upstreamMatrix = [
    [
      'rar5',
      'libarchive-rar5-stored.rar',
      'helloworld.txt',
      'fef9ad8cf601b43f76c6320075f62267c6e5c0a526d750a70b80c919a4a0aad8',
    ],
    [
      'lha',
      'libarchive-lha-lh0.lzh',
      'file1',
      'd0c504f06bbd64d183524eb35e5482ee5d966d456b905a24147165b2904d301b',
    ],
    [
      'rpm',
      'libarchive-cpio-svr4-gzip.rpm',
      'rpmsample-1.0.0-1.noarch.cpio',
      '0e74cd48811782ad214e89ddeb478ebdcd17f2274f2a86e580fc6d1ac0e6d67d',
    ],
    [
      'dmg-hfs',
      'qemu-simple-hfs.dmg',
      path.join('qemu-iotest', 'simple'),
      '42eb54fc42befa10ed033996f1c15295751f22993c18dd0a7e4bf7c75b6acae3',
    ],
  ]
  for (const [label, fixtureName, extractedName, expectedSha256] of upstreamMatrix) {
    const archive = path.join(externalFixtureDirectory, fixtureName)
    const output = path.join(fixtureDirectory, `upstream-${label}-output`)
    await callDesktopBridge('extractArchive', archive, output)
    assert.equal(
      fileSha256(path.join(output, extractedName)),
      expectedSha256.toLowerCase(),
      `${label} upstream sample output must match its known SHA-256`,
    )
    if (label === 'rar5') {
      verifiedDeclaredExtensions.add('rar')
    } else if (label === 'lha') {
      verifiedDeclaredExtensions.add('lzh')
      const lhaAlias = path.join(fixtureDirectory, 'extract-only.lha')
      copyFileSync(archive, lhaAlias)
      const lhaOutput = path.join(fixtureDirectory, 'upstream-lha-alias-output')
      await callDesktopBridge('extractArchive', lhaAlias, lhaOutput)
      assert.equal(
        fileSha256(path.join(lhaOutput, extractedName)),
        expectedSha256.toLowerCase(),
        'lha alias extraction must reproduce the pinned output',
      )
      verifiedDeclaredExtensions.add('lha')
    } else if (label === 'rpm') {
      verifiedDeclaredExtensions.add('rpm')
    } else if (label === 'dmg-hfs') {
      verifiedDeclaredExtensions.add('dmg')
    }
  }
  await callDesktopBridge('clearTasks')

  const encryptedRar = path.join(
    externalFixtureDirectory,
    'libarchive-rar-encrypted.rar',
  )
  if (existsSync(encryptedRar)) {
    console.log('[desktop-e2e] verifying encrypted RAR wrong-password rejection')
    const wrongPasswordStartedAt = Date.now()
    const wrongPasswordOutput = path.join(fixtureDirectory, 'rar-encrypted-wrong-password')
    const wrongPasswordError = await callDesktopBridgeFailure(
      'extractArchive',
      encryptedRar,
      wrongPasswordOutput,
      'wrong-password',
    )
    assert.match(
      wrongPasswordError,
      /password|encrypted|decrypt|checksum|crc|密码|解密/i,
      `encrypted RAR must report a password-related failure: ${wrongPasswordError}`,
    )
    assert.equal(
      existsSync(path.join(wrongPasswordOutput, 'foo.txt')),
      false,
      'wrong RAR password must not publish decrypted output',
    )
    assert.ok(
      Date.now() - wrongPasswordStartedAt < 60_000,
      'wrong RAR password rejection must complete within 60 seconds',
    )
    await callDesktopBridge('clearTasks')

    console.log('[desktop-e2e] verifying encrypted RAR correct-password extraction')
    const encryptedRarOutput = path.join(fixtureDirectory, 'rar-encrypted-correct-password')
    await callDesktopBridge('extractArchive', encryptedRar, encryptedRarOutput, '12345678')
    assert.equal(
      fileSha256(path.join(encryptedRarOutput, 'foo.txt')),
      '325d7b459b439684cad8825cbf2e488de15518103de09c56a42d6b1875081ee7',
      'encrypted RAR foo.txt must match the pinned plaintext',
    )
    assert.equal(
      fileSha256(path.join(encryptedRarOutput, 'bar.txt')),
      '7113d093a90b4a5cbac15a3bc8e85efbac50556c2a1f58f70a283cb2c373f1d5',
      'encrypted RAR bar.txt must match the pinned plaintext',
    )
    await callDesktopBridge('clearTasks')
  } else {
    recordMissingFullFormatCapability(
      'encrypted RAR fixture',
      'npm run test:fixtures:archives',
    )
  }
  const declaredExtensions = [
    ...capabilitySource.matchAll(/extensions:\s*\[([^\]]+)\]/g),
  ].flatMap((match) => [...match[1].matchAll(/'([^']+)'/g)].map((entry) => entry[1]))
  const unverifiedDeclaredExtensions = [...new Set(declaredExtensions)]
    .filter((extension) => !verifiedDeclaredExtensions.has(extension))
    .sort()
  for (const extension of unverifiedDeclaredExtensions) {
    recordMissingFullFormatCapability(
      `declared .${extension} extraction`,
      'add a non-empty real fixture and byte-for-byte desktop assertion',
    )
  }
  assertFullFormatMatrixReady()

  navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[navigation.length - 1].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/settings'), 30_000)
  assert.ok(await waitForNonEmptyText('main h1'), 'the settings heading is empty')

  const cancellableOutput = path.join(fixtureDirectory, 'cancelled-task.partial')
  console.log('[desktop-e2e] verifying cancellable task cleanup')
  const cancellableTaskId = await callDesktopBridge('startCancellableTask', cancellableOutput)
  await waitForNonEmptyFile(cancellableOutput)
  console.log('[desktop-e2e] cancellable output is active')
  await driver.executeScript("document.querySelector('.progress-summary')?.click()")
  await waitForElement('.progress-panel button[data-testid="cancel-task"]')
  console.log('[desktop-e2e] cancel action is visible')
  await driver.executeScript(
    "document.querySelector('.progress-panel button[data-testid=\"cancel-task\"]')?.click()",
  )
  await driver.wait(
    async () => (await callDesktopBridge('taskStatus', cancellableTaskId)) === 'cancelled',
    30_000,
  )
  console.log('[desktop-e2e] task store observed cancellation')
  await driver.wait(() => !existsSync(cancellableOutput), 30_000)
  console.log('[desktop-e2e] partial output was removed')
  await callDesktopBridge('clearTasks')

  console.log('[desktop-e2e] verifying active-task exit confirmation')
  await callDesktopBridge('setCloseToTray', false)
  await callDesktopBridge('seedActiveTask')
  assert.deepEqual(
    await callDesktopBridge('desktopBehaviorState'),
    { close_to_tray: false, has_active_tasks: true },
    'the native close handler must observe the active task before a close request',
  )
  assert.equal(
    await callDesktopBridge('requestExitConfirmation'),
    true,
    'the native close decision must request confirmation for an active task',
  )
  const exitDialog = await waitForElement('[role="dialog"]')
  assert.ok(
    (await exitDialog.findElements(By.css('button'))).length >= 3,
    'active-task exit confirmation must provide cancel, background, and stop-and-exit actions',
  )
  assert.equal(
    await callDesktopBridge('isWindowVisible'),
    true,
    'the window must remain visible while active-task exit confirmation is open',
  )
  const exitButtons = await exitDialog.findElements(By.css('button'))
  await exitButtons[0].click()
  await driver.wait(async () => (await driver.findElements(By.css('[role="dialog"]'))).length === 0, 30_000)
  await callDesktopBridge('clearTasks')

  console.log('[desktop-e2e] verifying update blocking while a task is active')
  await callDesktopBridge('seedActiveTask')
  await callDesktopBridge('showAvailableUpdate')
  const updateDialog = await waitForElement('[role="dialog"]')
  assert.ok(
    (await updateDialog.findElements(By.css('button:disabled'))).length > 0,
    'the install action must be disabled while a desktop task is active',
  )
  await callDesktopBridge('clearTasks')
  await driver.wait(
    async () => (await updateDialog.findElements(By.css('button:disabled'))).length === 0,
    30_000,
  )
  await callDesktopBridge('reset')
  await callDesktopBridge('setCloseToTray', true)

  console.log('[desktop-e2e] verifying close-to-tray and second-instance restore')
  assert.deepEqual(
    await callDesktopBridge('desktopBehaviorState'),
    { close_to_tray: true, has_active_tasks: false },
    'the native close handler must be configured to hide an idle window to the tray',
  )
  const hiddenMarker = path.join(fixtureDirectory, 'window-hidden.marker')
  const restoredMarker = path.join(fixtureDirectory, 'window-restored.marker')
  await hideDesktopWindow(hiddenMarker)
  await waitForLocalFileContent(hiddenMarker, 'hidden')
  forwardContextAction('--desktop-e2e-restore', [restoredMarker])
  await waitForLocalFileContent(restoredMarker, 'visible')

  completedSuccessfully = true
  console.log('Real Windows Tauri desktop archive and lifecycle tests passed.')
  }
  }
  }
  }
  }
  }
  }
} catch (error) {
  await captureFailure()
  throw error
} finally {
  if (autoStartOnly && autoStartRegistryOwnedByTest) {
    removeAutoStartRegistryValue()
  }
  if (devToolsPortMirror) clearInterval(devToolsPortMirror)
  if (driver) {
    try {
      await driver.quit()
    } catch {
      // Continue with process-tree cleanup below.
    }
  }
  terminateProcessTree(tauriDriverProcess?.pid)
  if (completedSuccessfully && fixtureDirectory) {
    const expectedPrefix = `${path.resolve(tmpdir())}${path.sep}`
    const resolvedFixtureDirectory = path.resolve(fixtureDirectory)
    if (resolvedFixtureDirectory.startsWith(expectedPrefix)) {
      rmSync(resolvedFixtureDirectory, { recursive: true, force: true })
    }
  }
}

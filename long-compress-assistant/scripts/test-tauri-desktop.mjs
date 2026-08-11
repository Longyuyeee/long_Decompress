import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import {
  copyFileSync,
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
import { deflateSync } from 'node:zlib'
import { Builder, By, Capabilities } from 'selenium-webdriver'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const executableSuffix = process.platform === 'win32' ? '.exe' : ''
const tauriConfig = JSON.parse(
  readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'),
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
const edgeDriver = process.env.EDGE_DRIVER_PATH
const webdriverUrl = 'http://127.0.0.1:4444/'
const artifactDirectory = path.join(root, 'test-results', 'desktop-e2e')
const e2eInstanceId =
  process.env.LONG_DECOMPRESS_E2E_INSTANCE_ID || randomBytes(12).toString('hex')
const e2eDataDirectory =
  process.env.LONG_DECOMPRESS_E2E_DATA_DIR ||
  path.join(root, 'test-results', `desktop-e2e-data-${e2eInstanceId}`)
let desktopSessionIndex = 0
let webviewUserDataDirectory = path.join(e2eDataDirectory, `webview2-session-${desktopSessionIndex}`)
const bundledSevenZip = path.join(root, 'src-tauri', 'resources', 'archive-engine', '7z.exe')
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
const markOfWebOnly = process.argv.includes('--mark-of-web-only')
const compressionVerificationOnly = process.argv.includes('--compression-verification-only')
const missingFullFormatCapabilities = new Set()

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

async function startTauriDriver() {
  tauriDriverProcess = spawn(
    tauriDriver,
    ['--native-driver', edgeDriver],
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

  let navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[2].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/browser'), 30_000)

  const openArchive = async (archivePath, outputPath, password, expectedText) => {
    await callDesktopBridge('queueDesktopDialogSelections', [archivePath, outputPath])
    const passwordInput = await waitForElement('.browser-toolbar input[type="password"]')
    await passwordInput.clear()
    if (password) await passwordInput.sendKeys(password)
    await (await waitForElement('.browser-page > header .browser-primary')).click()
    await driver.wait(async () => {
      const pages = await driver.findElements(By.css('.browser-page'))
      return pages.length > 0 && (await pages[0].getText()).includes(expectedText)
    }, 30_000)
    const fields = await driver.findElements(By.css('.browser-toolbar .browser-field'))
    assert.equal(fields.length, 3)
    await (await fields[2].findElement(By.css('button'))).click()
    await driver.wait(async () => (await fields[2].getText()).includes(outputPath), 10_000)
    const dimensions = await driver.executeScript(
      'const page = document.querySelector(\'.browser-page\'); return page ? { scrollWidth: page.scrollWidth, clientWidth: page.clientWidth } : null;',
    )
    assert.ok(dimensions)
    assert.ok(
      dimensions.scrollWidth <= dimensions.clientWidth + 1,
      `the archive-browser page must not scroll horizontally: ${JSON.stringify(dimensions)}`,
    )
  }

  const extractOnly = async (query, expectedPath, expectedContent, excludedPath) => {
    await (await waitForElement('.browser-table-head .browser-checkbox')).click()
    const search = await waitForElement('.browser-search input')
    await search.clear()
    await search.sendKeys(query)
    const rows = await driver.wait(async () => {
      const entries = await driver.findElements(By.css('.browser-row'))
      return entries.length === 1 ? entries : false
    }, 10_000)
    await (await rows[0].findElement(By.css('.browser-checkbox'))).click()
    assert.match(await (await waitForElement('.browser-page > footer')).getText(), /已选择\s+1\s+\//)
    await (await waitForElement('.browser-page > footer .browser-primary')).click()
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
  await openArchive(browserZip, zipOutput, '', '保留文件.txt')
  assert.match(await (await waitForElement('.browser-page')).getText(), /ZIP[\s\S]*未加密/)
  await extractOnly(
    '保留文件',
    path.join(zipOutput, zipKeepRelative),
    zipKeepPayload,
    path.join(zipOutput, zipSkipRelative),
  )

  const sevenZipOutput = path.join(browserFixtureRoot, '7z-selected-output')
  await openArchive(browser7z, sevenZipOutput, 'desktop-browser-secret', '只解压这一项.txt')
  assert.match(await (await waitForElement('.browser-page')).getText(), /7Z[\s\S]*已加密/)
  await extractOnly(
    '只解压这一项',
    path.join(sevenZipOutput, passwordKeepRelative),
    'password 7z selected payload',
    path.join(sevenZipOutput, passwordSkipRelative),
  )

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
  const text = (await card.getAttribute('textContent')).trim()
  assert.match(text, /资源预检/)
  assert.match(text, new RegExp(expectedLabel))
  const dimensions = await driver.executeScript(
    'const card = arguments[0]; return { scrollWidth: card.scrollWidth, clientWidth: card.clientWidth };',
    card,
  )
  assert.ok(dimensions, 'the resource-preflight card must remain mounted')
  assert.ok(
    dimensions.scrollWidth <= dimensions.clientWidth + 1,
    `the resource-preflight card must not scroll horizontally: ${JSON.stringify(dimensions)}`,
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

async function createDesktopSession() {
  const capabilities = new Capabilities()
  capabilities.setBrowserName('wry')
  const webviewOptions = {
    userDataFolder: webviewUserDataDirectory,
  }
  if (process.env.CI) {
    webviewOptions.additionalBrowserArguments = ['--headless=new', '--disable-gpu']
  }
  capabilities.set('tauri:options', { application, webviewOptions })
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
  console.log('[desktop-e2e] waiting for desktop heading')
  assert.ok(await waitForNonEmptyText('main h1'), 'the decompression workspace heading is empty')
  console.log('[desktop-e2e] desktop heading is ready; waiting for the E2E bridge')
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
  driver = await createDesktopSession()
  console.log('[desktop-e2e] replacement desktop session created')
  await waitForDesktopReady()
}

try {
  mkdirSync(webviewUserDataDirectory, { recursive: true })
  await startTauriDriver()

  driver = await createDesktopSession()
  await waitForDesktopReady()

  let navigation = await driver.findElements(By.css('aside nav > button'))
  assert.equal(navigation.length, 6, 'the real desktop shell must expose six navigation buttons')
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
  if (archiveBrowserOnly) {
    completedSuccessfully = true
    console.log('Real Windows Tauri archive-browser gate passed.')
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
} catch (error) {
  await captureFailure()
  throw error
} finally {
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

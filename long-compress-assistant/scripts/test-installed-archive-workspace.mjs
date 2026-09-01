import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir, homedir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Builder, By, Capabilities, Key } from 'selenium-webdriver'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const application = process.env.TAURI_APP_BINARY
const edgeDriver = process.env.EDGE_DRIVER_PATH
const tauriDriver = process.env.TAURI_DRIVER_PATH || path.join(homedir(), '.cargo', 'bin', 'tauri-driver.exe')
const webdriverUrl = 'http://127.0.0.1:4444/'
const fixtureRoot = mkdtempSync(path.join(tmpdir(), 'long-decompress-installed-a06-'))
const evidenceRoot = path.join(root, 'test-results', 'installed-archive-workspace')
let webviewData = path.join(fixtureRoot, 'webview2-1')
const installedSevenZip = application
  ? path.join(path.dirname(application), 'resources', 'archive-engine', '7z.exe')
  : ''
const encryptedRarFixture = path.join(root, 'test-results', 'external-archive-fixtures', 'libarchive-rar-encrypted.rar')
const checks = []
let driver
let tauriDriverProcess
let mirrorTimer
let driverOutput = ''

const record = (name, detail) => {
  checks.push({ name, passed: true, detail })
  console.log(`[installed-a06] ${name}: ${detail}`)
}

for (const [label, target] of [
  ['installed application', application],
  ['Microsoft EdgeDriver', edgeDriver],
  ['tauri-driver', tauriDriver],
  ['installed 7-Zip engine', installedSevenZip],
  ['pinned encrypted RAR fixture', encryptedRarFixture],
]) {
  assert.ok(target && existsSync(target), `${label} was not found: ${target || '<unset>'}`)
}

mkdirSync(evidenceRoot, { recursive: true })
mkdirSync(webviewData, { recursive: true })

const run = (command, args, label, options = {}) => {
  const result = spawnSync(command, args, {
    cwd: options.cwd || fixtureRoot,
    encoding: 'utf8',
    timeout: options.timeout || 60_000,
    windowsHide: true,
  })
  assert.ifError(result.error)
  assert.equal(result.status, 0, `${label} failed: ${result.stderr || result.stdout}`)
}

const createLargeMetadataTar = (targetPath, entryCount) => {
  const archive = Buffer.alloc(entryCount * 512 + 1024)
  const writeOctal = (header, offset, length, value) => {
    header.write(`${value.toString(8).padStart(length - 1, '0')}\0`, offset, length, 'ascii')
  }
  for (let index = 0; index < entryCount; index += 1) {
    const header = archive.subarray(index * 512, (index + 1) * 512)
    header.write(`bulk/entry-${String(index).padStart(6, '0')}.txt`, 0, 100, 'ascii')
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

const sourceRoot = path.join(fixtureRoot, 'source')
const normalCase = path.join(fixtureRoot, 'normal-case')
const encryptedCase = path.join(fixtureRoot, 'encrypted-case')
const tarCase = path.join(fixtureRoot, 'tar-case')
const nestedCase = path.join(fixtureRoot, 'nested-case')
const rarCase = path.join(fixtureRoot, 'rar-case')
for (const directory of [sourceRoot, normalCase, encryptedCase, tarCase, nestedCase, rarCase]) mkdirSync(directory, { recursive: true })

let longDirectory = path.join(sourceRoot, '资料集合')
for (let index = 1; index <= 8; index += 1) longDirectory = path.join(longDirectory, `中文长目录-${index}`)
mkdirSync(longDirectory, { recursive: true })
writeFileSync(path.join(longDirectory, '保留文件.txt'), 'installed archive workspace exact payload', 'utf8')
writeFileSync(path.join(sourceRoot, '说明 文档.txt'), 'Long解压 A-06 installed preview', 'utf8')
writeFileSync(path.join(sourceRoot, '危险脚本.cmd'), '@echo off\r\necho should-not-run\r\n', 'utf8')
writeFileSync(path.join(sourceRoot, '空白 文档.pdf'), '%PDF-1.4\n%%EOF\n', 'ascii')
writeFileSync(path.join(sourceRoot, '媒体样本.mp4'), Buffer.alloc(4096, 0x5a))
copyFileSync(path.join(root, 'src-tauri', 'icons', 'icon.png'), path.join(sourceRoot, '界面 图标.png'))

const normalZip = path.join(normalCase, '安装态混合内容.zip')
run(installedSevenZip, ['a', '-tzip', '-y', normalZip, '.\\*', '-r'], 'normal ZIP fixture', { cwd: sourceRoot })

const encrypted7z = path.join(encryptedCase, '安装态加密.7z')
run(installedSevenZip, ['a', '-t7z', '-y', '-pinstalled-secret', '-mhe=off', encrypted7z, '.\\*', '-r'], 'encrypted 7Z fixture', { cwd: sourceRoot })

const encryptedRar = path.join(rarCase, '安装态加密.rar')
copyFileSync(encryptedRarFixture, encryptedRar)

const plainTar = path.join(tarCase, '安装态目录.tar')
run(installedSevenZip, ['a', '-ttar', '-y', plainTar, '.\\*', '-r'], 'TAR fixture', { cwd: sourceRoot })
const tarGz = path.join(tarCase, '安装态目录.tar.gz')
run(installedSevenZip, ['a', '-tgzip', '-y', tarGz, path.basename(plainTar)], 'TAR.GZ fixture', { cwd: tarCase })

const leafRoot = path.join(fixtureRoot, 'leaf-source')
mkdirSync(leafRoot)
writeFileSync(path.join(leafRoot, '最内层.txt'), 'three-level installed nested payload', 'utf8')
const leafZip = path.join(fixtureRoot, '内层.zip')
run(installedSevenZip, ['a', '-tzip', '-y', leafZip, '.\\*'], 'nested ZIP fixture', { cwd: leafRoot })
const middle7z = path.join(fixtureRoot, '加密中层.7z')
run(installedSevenZip, ['a', '-t7z', '-y', '-pinner-secret', '-mhe=on', middle7z, path.basename(leafZip)], 'nested encrypted 7Z fixture', { cwd: fixtureRoot })
const outerZip = path.join(nestedCase, '外层工作区.zip')
run(installedSevenZip, ['a', '-tzip', '-y', outerZip, path.basename(middle7z)], 'outer ZIP fixture', { cwd: fixtureRoot })

const damagedZip = path.join(fixtureRoot, '损坏归档.zip')
writeFileSync(damagedZip, Buffer.from('PK\x03\x04truncated-installed-gate', 'binary'))
const cancellableTar = path.join(fixtureRoot, '大量条目取消.tar')
createLargeMetadataTar(cancellableTar, 180_000)

const registryCommand = spawnSync('powershell.exe', [
  '-NoProfile', '-NonInteractive', '-Command',
  "$command=(Get-Item -LiteralPath 'HKCU:\\Software\\Classes\\SystemFileAssociations\\.zip\\shell\\LongDecompress\\shell\\01.LongDecompress.open\\command').GetValue(''); $expected='\"'+$env:TAURI_APP_BINARY+'\" --browse-archive \"%1\"'; if(-not $command.Equals($expected,[StringComparison]::OrdinalIgnoreCase)){exit 7}; Write-Output 'OK'",
], { encoding: 'utf8', windowsHide: true, env: { ...process.env, TAURI_APP_BINARY: application } })
assert.equal(registryCommand.status, 0, registryCommand.stderr || registryCommand.stdout)
assert.equal(registryCommand.stdout.trim(), 'OK')
record('Explorer classic browse command', 'Unicode target and --browse-archive "%1" matched the installed executable')

const appendDriverOutput = chunk => {
  driverOutput = `${driverOutput}${chunk}`.slice(-32_768)
  process.stdout.write(chunk)
}

const mirrorDevToolsPort = () => {
  const source = path.join(webviewData, 'EBWebView', 'DevToolsActivePort')
  const destination = path.join(webviewData, 'DevToolsActivePort')
  if (!existsSync(source)) return
  try { copyFileSync(source, destination) } catch { /* EdgeDriver may be reading it. */ }
}

const waitForDriver = async () => {
  const deadline = Date.now() + 30_000
  while (Date.now() < deadline) {
    if (tauriDriverProcess.exitCode !== null) throw new Error(`tauri-driver exited early: ${driverOutput}`)
    try {
      const response = await fetch(`${webdriverUrl}status`)
      if (response.ok) return
    } catch { /* not ready */ }
    await new Promise(resolve => setTimeout(resolve, 250))
  }
  throw new Error(`tauri-driver did not become ready: ${driverOutput}`)
}

const waitForElement = selector => driver.wait(async () => {
  const elements = await driver.findElements(By.css(selector))
  return elements[0] || false
}, 30_000)

const forwardArchive = archivePath => {
  const result = spawnSync(application, ['--browse-archive', archivePath], {
    encoding: 'utf8', timeout: 30_000, windowsHide: true,
  })
  assert.ifError(result.error)
  assert.equal(result.status, 0, result.stderr || result.stdout)
}

const waitForArchive = async (archivePath, formatPattern) => {
  forwardArchive(archivePath)
  const name = path.basename(archivePath)
  await driver.wait(async () => (await waitForElement('[data-testid="archive-chain"]')).getText().then(text => text.includes(name)), 30_000)
  await driver.wait(async () => {
    const summaries = await driver.findElements(By.css('.browser-summary'))
    if (summaries.length === 0) return false
    return formatPattern.test(await summaries[0].getText())
  }, 30_000)
  return waitForElement('.browser-summary')
}

const sha256 = filePath => createHash('sha256').update(readFileSync(filePath)).digest('hex')

const setWorkspacePassword = async password => {
  const input = await waitForElement('.browser-toolbar input[type="password"]')
  await driver.executeScript(
    "const input=arguments[0]; input.value=''; input.dispatchEvent(new Event('input',{bubbles:true}));",
    input,
  )
  if (password) await input.sendKeys(password)
}

const extractSelectedAndWait = async outputReady => {
  const button = await waitForElement('.browser-page > footer .browser-primary')
  await button.click()
  await driver.wait(async () => outputReady() && await button.isEnabled(), 120_000)
}

const terminateApplication = () => {
  const escaped = application.replaceAll("'", "''")
  spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }`], { windowsHide: true })
}

try {
  tauriDriverProcess = spawn(tauriDriver, ['--native-driver', edgeDriver], {
    cwd: root,
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--force-device-scale-factor=1.5' },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  })
  tauriDriverProcess.stdout.on('data', appendDriverOutput)
  tauriDriverProcess.stderr.on('data', appendDriverOutput)
  await waitForDriver()

  let startupAttempts = 0
  while (!driver && startupAttempts < 3) {
    startupAttempts += 1
    webviewData = path.join(fixtureRoot, `webview2-${startupAttempts}`)
    mkdirSync(webviewData, { recursive: true })
    const capabilities = new Capabilities()
    capabilities.setBrowserName('wry')
    capabilities.set('tauri:options', {
      application,
      args: [],
      webviewOptions: { userDataFolder: webviewData, additionalBrowserArguments: ['--force-device-scale-factor=1.5'] },
    })
    mirrorTimer = setInterval(mirrorDevToolsPort, 50)
    try {
      driver = await new Builder().usingServer(webdriverUrl).withCapabilities(capabilities).build()
    } catch (error) {
      if (startupAttempts >= 3 || !/DevToolsActivePort|session not created/i.test(String(error))) throw error
      terminateApplication()
      await new Promise(resolve => setTimeout(resolve, 750))
    } finally {
      clearInterval(mirrorTimer)
      mirrorTimer = undefined
    }
  }
  assert.ok(driver, 'installed WebView2 session was not created')
  if (startupAttempts > 1) record('installed WebView2 startup recovery', `session created on attempt ${startupAttempts}`)
  await driver.manage().setTimeouts({ implicit: 1_000, pageLoad: 60_000, script: 120_000 })
  await driver.manage().window().setRect({ x: -32_000, y: -32_000, width: 1280, height: 800 })

  await driver.wait(async () => (await waitForElement('main h1')).getText().then(text => text.length > 0), 30_000)
  assert.equal(await driver.executeScript('return Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__)'), false)
  record('formal installed executable', 'normal production build; desktop E2E bridge absent')

  await waitForArchive(normalZip, /ZIP/)
  const dimensions = await driver.executeScript('const page=document.querySelector(".browser-page"); return {page:[page.scrollWidth,page.clientWidth], body:[document.body.scrollWidth,document.body.clientWidth], dpr:window.devicePixelRatio}')
  assert.ok(dimensions.page[0] <= dimensions.page[1] + 1, JSON.stringify(dimensions))
  assert.ok(dimensions.body[0] <= dimensions.body[1] + 1, JSON.stringify(dimensions))
  record('150% scale responsive layout', JSON.stringify(dimensions))

  const search = await waitForElement('.browser-search input')
  await search.sendKeys('说明 文档.txt')
  await (await waitForElement('.preview-trigger')).click()
  assert.match(await (await waitForElement('[data-testid="archive-text-preview"]')).getText(), /A-06 installed preview/)
  await driver.actions().sendKeys(Key.ESCAPE).perform()
  record('bounded installed text preview', 'real ZIP entry rendered through the internal viewer')

  await search.clear()
  await search.sendKeys('危险脚本.cmd')
  const dangerousRow = await waitForElement('[data-entry-path$="危险脚本.cmd"]')
  await driver.actions().contextClick(dangerousRow).perform()
  await (await waitForElement('[data-testid="archive-context-default-open"]')).click()
  await waitForElement('[data-testid="archive-dangerous-open-dialog"]')
  await (await waitForElement('[data-testid="archive-dangerous-cancel"]')).click()
  record('active-content negative gate', 'CMD stayed unopened until explicit confirmation; cancel path passed')

  await search.clear()
  const expectedPayload = 'installed archive workspace exact payload'
  const normalOutput = path.join(normalCase, path.relative(sourceRoot, longDirectory), '保留文件.txt')
  await extractSelectedAndWait(() => existsSync(normalOutput) && readFileSync(normalOutput, 'utf8') === expectedPayload)
  record('installed selected extraction', 'long Chinese path output matched byte-for-byte')

  const encryptedSummary = await waitForArchive(encrypted7z, /7Z/)
  assert.match(await encryptedSummary.getText(), /已加密/)
  const passwordInput = await waitForElement('.browser-toolbar input[type="password"]')
  await passwordInput.sendKeys('installed-secret')
  const encryptedOutput = path.join(encryptedCase, path.relative(sourceRoot, longDirectory), '保留文件.txt')
  await extractSelectedAndWait(() => existsSync(encryptedOutput) && readFileSync(encryptedOutput, 'utf8') === expectedPayload)
  record('installed encrypted 7Z extraction', 'password-protected real payload matched')

  await waitForArchive(plainTar, /TAR/)
  await waitForArchive(tarGz, /TAR/)
  record('installed TAR family browsing', 'real TAR and TAR.GZ metadata loaded')

  await setWorkspacePassword('12345678')
  await waitForArchive(encryptedRar, /RAR/)
  await extractSelectedAndWait(() => existsSync(path.join(rarCase, 'foo.txt')) && existsSync(path.join(rarCase, 'bar.txt')))
  assert.equal(sha256(path.join(rarCase, 'foo.txt')), '325d7b459b439684cad8825cbf2e488de15518103de09c56a42d6b1875081ee7')
  record('installed encrypted RAR extraction', 'correct password produced both files and foo.txt matched pinned SHA-256')

  await waitForArchive(outerZip, /ZIP/)
  await driver.actions().doubleClick(await waitForElement('[data-entry-path="加密中层.7z"]')).perform()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="archive-nested-retry"]'))).length === 1, 30_000)
  const nestedPassword = await waitForElement('.browser-toolbar input[type="password"]')
  await nestedPassword.sendKeys('inner-secret')
  await (await waitForElement('[data-testid="archive-nested-retry"]')).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-entry-path="内层.zip"]'))).length === 1, 30_000)
  await driver.actions().doubleClick(await waitForElement('[data-entry-path="内层.zip"]')).perform()
  await driver.wait(async () => (await waitForElement('[data-testid="archive-chain"]')).getText().then(text => /3\s*\/\s*3\s*层/.test(text)), 30_000)
  assert.match(await (await waitForElement('.browser-page')).getText(), /最内层\.txt/)
  record('installed three-level nested workspace', 'ZIP → encrypted 7Z → ZIP with per-layer password passed')

  forwardArchive(damagedZip)
  const damagedMessage = await driver.wait(async () => {
    const alerts = await driver.findElements(By.css('[role="alert"]'))
    for (const alert of alerts) {
      const text = await alert.getText()
      if (/损坏|无法读取|结构|格式/.test(text)) return text
    }
    return false
  }, 30_000)
  record('damaged archive negative gate', damagedMessage)

  forwardArchive(cancellableTar)
  const cancelButton = await waitForElement('[data-testid="archive-browse-cancel"]')
  const cancelStartedAt = Date.now()
  await cancelButton.click()
  await driver.wait(async () => (await waitForElement('[data-testid="archive-browse-notice"]')).getText().then(text => text.includes('已取消')), 5_000)
  record('installed large-archive cancellation', `${Date.now() - cancelStartedAt} ms`)

  writeFileSync(path.join(evidenceRoot, 'a06-installed-workspace.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  writeFileSync(path.join(evidenceRoot, 'result.json'), JSON.stringify({ schemaVersion: 1, application, checks }, null, 2))
  console.log('Real installed Windows archive-workspace matrix passed.')
} catch (error) {
  if (driver) {
    try { writeFileSync(path.join(evidenceRoot, 'a06-installed-workspace-failure.png'), Buffer.from(await driver.takeScreenshot(), 'base64')) } catch { /* session unavailable */ }
  }
  writeFileSync(path.join(evidenceRoot, 'result.json'), JSON.stringify({ schemaVersion: 1, application, checks, error: String(error), driverOutput }, null, 2))
  throw error
} finally {
  if (mirrorTimer) clearInterval(mirrorTimer)
  if (driver) {
    try { await driver.quit() } catch { /* application may close first */ }
  }
  if (tauriDriverProcess?.pid) spawnSync('taskkill.exe', ['/pid', String(tauriDriverProcess.pid), '/t', '/f'], { stdio: 'ignore', windowsHide: true })
  terminateApplication()
  rmSync(fixtureRoot, { recursive: true, force: true })
}

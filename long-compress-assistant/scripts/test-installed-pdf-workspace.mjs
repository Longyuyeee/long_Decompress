import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Builder, By, Capabilities } from 'selenium-webdriver'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const application = process.env.TAURI_APP_BINARY
const edgeDriver = process.env.EDGE_DRIVER_PATH
const tauriDriver = process.env.TAURI_DRIVER_PATH || path.join(homedir(), '.cargo', 'bin', 'tauri-driver.exe')
const webdriverUrl = 'http://127.0.0.1:4444/'
const fixtureRoot = mkdtempSync(path.join(tmpdir(), 'long-decompress-installed-d043-'))
const sourceRoot = path.join(fixtureRoot, '安装版 PDF 输入')
const evidenceRoot = path.resolve(process.env.PDF_WORKSPACE_EVIDENCE_DIRECTORY || path.join(root, 'test-results', 'installed-pdf-workspace'))
const pinnedRoot = path.join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'pdfs')
const runSuffix = path.basename(fixtureRoot).slice(-6)
const checks = []
let driver
let tauriDriverProcess
let mirrorTimer
let driverOutput = ''
let webviewData = ''

const expected = { productionBridge: false, cancelled: 1, completed: 2, failed: 1, persistedHistory: 4, reopenedOutputs: 2 }
const verify = (name, wanted, actual, predicate = value => value === wanted) => {
  const passed = Boolean(predicate(actual))
  checks.push({ name, expected: wanted, actual, passed })
  console.log(`[installed-d043] ${name}: expected=${JSON.stringify(wanted)}; actual=${JSON.stringify(actual)}`)
  assert.ok(passed, `${name}: expected=${JSON.stringify(wanted)}; actual=${JSON.stringify(actual)}`)
}

for (const [label, target] of [
  ['installed application', application], ['Microsoft EdgeDriver', edgeDriver], ['tauri-driver', tauriDriver],
]) assert.ok(target && existsSync(target), `${label} was not found: ${target || '<unset>'}`)

mkdirSync(sourceRoot, { recursive: true })
mkdirSync(evidenceRoot, { recursive: true })
const fixture = (pinnedName, label) => {
  const pinned = path.join(pinnedRoot, pinnedName)
  assert.ok(existsSync(pinned), `pinned PDF fixture was not found: ${pinned}`)
  const target = path.join(sourceRoot, `${label}-${runSuffix}.pdf`)
  copyFileSync(pinned, target)
  return { pinnedName, path: target, name: path.basename(target) }
}
const largeImage = fixture('large-image.pdf', '安装版取消大图')
const form = fixture('form.pdf', '安装版失败表单')
const text = fixture('text-vector.pdf', '安装版完成文本')
const mixed = fixture('mixed-content.pdf', '安装版完成混合')
const signed = fixture('signed.pdf', '安装版签名阻断')
const encrypted = fixture('encrypted.pdf', '安装版加密阻断')
const allSources = [largeImage, form, text, mixed, signed, encrypted]
const sha256 = filePath => createHash('sha256').update(readFileSync(filePath)).digest('hex')
const sourceHashes = Object.fromEntries(allSources.map(source => [source.path, sha256(source.path)]))
const organizedOutput = source => source.path.replace(/\.pdf$/iu, '.organized.pdf')
const optimizedOutput = source => source.path.replace(/\.pdf$/iu, '.optimized.pdf')

const appendDriverOutput = chunk => { driverOutput = `${driverOutput}${chunk}`.slice(-32_768); process.stdout.write(chunk) }
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
    try { if ((await fetch(`${webdriverUrl}status`)).ok) return } catch { /* not ready */ }
    await new Promise(resolve => setTimeout(resolve, 250))
  }
  throw new Error(`tauri-driver did not become ready: ${driverOutput}`)
}
const terminateApplication = () => {
  const escaped = application.replaceAll("'", "''")
  spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }`], { windowsHide: true })
}
const terminateFixtureWebViews = () => {
  const escaped = fixtureRoot.replaceAll("'", "''")
  spawnSync('powershell.exe', [
    '-NoProfile', '-NonInteractive', '-Command',
    `Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'msedge.exe' -and $_.CommandLine -like '*${escaped}*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }`,
  ], { windowsHide: true })
}
const createSession = async attemptOffset => {
  let created
  for (let attempt = 1; !created && attempt <= 3; attempt += 1) {
    webviewData = path.join(fixtureRoot, `webview2-${attemptOffset + attempt}`)
    mkdirSync(webviewData, { recursive: true })
    const capabilities = new Capabilities()
    capabilities.setBrowserName('wry')
    capabilities.set('tauri:options', { application, args: [], webviewOptions: { userDataFolder: webviewData, additionalBrowserArguments: ['--force-device-scale-factor=1.5'] } })
    mirrorTimer = setInterval(mirrorDevToolsPort, 50)
    try {
      created = await new Builder().usingServer(webdriverUrl).withCapabilities(capabilities).build()
    } catch (error) {
      if (attempt === 3 || !/DevToolsActivePort|session not created/iu.test(String(error))) throw error
      terminateApplication(); await new Promise(resolve => setTimeout(resolve, 750))
    } finally { clearInterval(mirrorTimer); mirrorTimer = undefined }
  }
  assert.ok(created, 'installed WebView2 session was not created')
  await created.manage().setTimeouts({ implicit: 1_000, pageLoad: 60_000, script: 120_000 })
  await created.manage().window().setRect({ x: -32_000, y: -32_000, width: 1280, height: 800 })
  return created
}
const waitForElement = (selector, timeout = 30_000) => driver.wait(async () => (await driver.findElements(By.css(selector)))[0] || false, timeout)
const navigateToPdfWorkspace = async () => {
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await (await waitForElement('[data-testid="compression-mode-pdf"]')).click()
  await waitForElement('[data-testid="pdf-compression-workspace"]')
  // The production dropzone subscribes through Tauri's async event API after
  // Vue mounts. A visible workspace alone does not prove that subscription is
  // ready, especially after a complete WebView2 session restart.
  await new Promise(resolve => setTimeout(resolve, 1_000))
}
const emitThroughProductionTauriIpc = async paths => driver.executeAsyncScript(`
  const paths = arguments[0], done = arguments[arguments.length - 1]
  if (typeof window.__TAURI_IPC__ !== 'function') return done({ ok: false, error: 'production Tauri IPC transport is unavailable' })
  const callback = window.crypto.getRandomValues(new Uint32Array(1))[0]
  const error = window.crypto.getRandomValues(new Uint32Array(1))[0]
  const cleanup = () => { Reflect.deleteProperty(window, '_' + callback); Reflect.deleteProperty(window, '_' + error) }
  Object.defineProperty(window, '_' + callback, { configurable: true, value: () => { cleanup(); done({ ok: true, count: paths.length, transport: '__TAURI_IPC__/Event.emit' }) } })
  Object.defineProperty(window, '_' + error, { configurable: true, value: reason => { cleanup(); done({ ok: false, error: String(reason) }) } })
  window.__TAURI_IPC__({ cmd: 'tauri', callback, error, __tauriModule: 'Event', message: { cmd: 'emit', event: 'tauri://file-drop', windowLabel: null, payload: paths } })
`, paths)
const waitForPdfCards = async (count, status) => {
  try {
    return await driver.wait(async () => {
      const cards = await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))
      if (cards.length !== count) return false
      return (await Promise.all(cards.map(card => card.getText()))).every(value => value.includes(status)) ? cards : false
    }, 60_000)
  } catch (error) {
    const cards = await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))
    const texts = await Promise.all(cards.map(card => card.getText()))
    throw new Error(`PDF cards did not reach ${count} × ${status}: actual=${JSON.stringify(texts)}; ${error}`)
  }
}
const historyFacts = async names => driver.executeScript(`
  const names = arguments[0]
  return Array.from(document.querySelectorAll('[data-testid="history-record-row"]')).filter(row => names.some(name => (row.textContent || '').includes(name))).map(row => ({ text: row.textContent || '', status: row.querySelector('[data-testid="history-status-badge"]')?.textContent?.trim() || '' }))
`, names)
const openHistoryAndWait = async (names, count) => {
  await (await waitForElement('[data-testid="nav-History"]')).click(); await waitForElement('[data-testid="history-list"]')
  return driver.wait(async () => { const records = await historyFacts(names); return records.length === count ? records : false }, 30_000)
}
const restartSession = async offset => {
  await driver.quit(); driver = undefined; terminateApplication(); await new Promise(resolve => setTimeout(resolve, 750))
  driver = await createSession(offset)
  await driver.wait(async () => (await waitForElement('main h1')).getText().then(value => value.length > 0), 30_000)
}
const waitForBatchCompletion = async () => {
  const summary = 'PDF 处理结束：2 个完成，1 个失败，0 个取消'
  try {
    await driver.wait(async () => (await driver.findElement(By.css('body')).getText()).includes(summary), 120_000)
  } catch (error) {
    const cards = await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))
    const cardTexts = await Promise.all(cards.map(card => card.getText()))
    const bodyText = await driver.findElement(By.css('body')).getText()
    throw new Error(`PDF installed batch did not reach its terminal summary: cards=${JSON.stringify(cardTexts)}; bodyTail=${JSON.stringify(bodyText.slice(-2_000))}; ${error}`)
  }
}

try {
  tauriDriverProcess = spawn(tauriDriver, ['--native-driver', edgeDriver], { cwd: root, env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--force-device-scale-factor=1.5' }, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true })
  tauriDriverProcess.stdout.on('data', appendDriverOutput); tauriDriverProcess.stderr.on('data', appendDriverOutput); await waitForDriver()
  driver = await createSession(0)
  await driver.wait(async () => (await waitForElement('main h1')).getText().then(value => value.length > 0), 30_000)
  verify('formal installed executable excludes desktop E2E bridge', expected.productionBridge, await driver.executeScript('return Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__)'))

  await navigateToPdfWorkspace()
  let drop = await emitThroughProductionTauriIpc([largeImage.path]); verify('production Tauri file-drop IPC accepted cancellation input', true, drop.ok)
  await waitForPdfCards(1, '可配置')
  await (await waitForElement('[data-testid="pdf-mode-image"]')).click(); await (await waitForElement('[data-testid="pdf-risk-confirmation"]')).click(); await (await waitForElement('[data-testid="pdf-allow-larger-output"]')).click(); await (await waitForElement('[data-testid="pdf-freeze-configuration"]')).click()
  await (await waitForElement('[data-testid="pdf-start-batch"]')).click(); await (await waitForElement('[data-testid="pdf-cancel-batch"]')).click()
  await driver.wait(async () => (await driver.findElement(By.css('body')).getText()).includes('1 个取消'), 60_000)
  verify('cancelled installed task publishes no output', false, existsSync(optimizedOutput(largeImage)))
  verify('cancelled installed task preserves source SHA-256', sourceHashes[largeImage.path], sha256(largeImage.path))
  const cancelledHistory = await openHistoryAndWait([largeImage.name], 1)
  verify('installed cancellation persists cancelled history', expected.cancelled, cancelledHistory.filter(record => /已取消/u.test(record.status)).length)
  writeFileSync(path.join(evidenceRoot, 'd043-installed-pdf-cancelled.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))

  await restartSession(10); await navigateToPdfWorkspace()
  for (const [index, source] of [form, text, mixed].entries()) {
    drop = await emitThroughProductionTauriIpc([source.path])
    verify(`production Tauri file-drop IPC accepted batch input ${index + 1}`, true, drop.ok)
    await waitForPdfCards(index + 1, '可配置')
  }
  const batchCards = await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))
  for (const card of batchCards) await card.findElement(By.css('[data-testid="pdf-freeze-configuration"]')).click()
  rmSync(form.path); await (await waitForElement('[data-testid="pdf-start-batch"]')).click()
  await waitForBatchCompletion()
  const outputPaths = [organizedOutput(text), organizedOutput(mixed)]
  verify('failed installed item publishes no output', false, existsSync(organizedOutput(form)))
  verify('remaining installed batch outputs are published', expected.completed, outputPaths.filter(output => existsSync(output) && statSync(output).size > 0).length)
  verify('completed installed sources preserve SHA-256', 0, [text, mixed].filter(source => sha256(source.path) !== sourceHashes[source.path]).length)
  const defaultOpen = (await driver.findElements(By.css('[data-testid="pdf-open-default-app"]')))[0]
  assert.ok(defaultOpen, 'completed installed PDF does not expose the default-reader action')
  verify('published PDF exposes an enabled default-reader action without launching an external window', true, await defaultOpen.isEnabled())
  const historyNames = [largeImage.name, form.name, text.name, mixed.name]
  const currentHistory = await openHistoryAndWait(historyNames, 4)
  verify('current installed history completed count', expected.completed, currentHistory.filter(record => /已完成/u.test(record.status)).length)
  verify('current installed history failed count', expected.failed, currentHistory.filter(record => /失败/u.test(record.status)).length)
  verify('current installed history cancelled count', expected.cancelled, currentHistory.filter(record => /已取消/u.test(record.status)).length)
  writeFileSync(path.join(evidenceRoot, 'd043-installed-pdf-completed.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))

  await restartSession(20)
  const restartedHistory = await openHistoryAndWait(historyNames, 4); verify('PDF history survives complete installed-app restart', expected.persistedHistory, restartedHistory.length)
  await navigateToPdfWorkspace()
  for (const [index, output] of outputPaths.entries()) {
    drop = await emitThroughProductionTauriIpc([output])
    verify(`published output ${index + 1} reopens through production Tauri IPC`, true, drop.ok)
    await waitForPdfCards(index + 1, '可配置')
  }
  const reopenedCards = await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))
  verify('reopened outputs expose page facts', expected.reopenedOutputs, (await Promise.all(reopenedCards.map(card => card.getText()))).filter(value => /页数\s*1/u.test(value)).length)

  await restartSession(30); await navigateToPdfWorkspace()
  drop = await emitThroughProductionTauriIpc([signed.path]); verify('production Tauri file-drop IPC accepted signed input', true, drop.ok)
  await waitForPdfCards(1, '仅可分析')
  drop = await emitThroughProductionTauriIpc([encrypted.path]); verify('production Tauri file-drop IPC accepted encrypted input', true, drop.ok)
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))).length === 2, 30_000)
  await driver.wait(async () => (await driver.findElement(By.css('[data-testid="pdf-compression-workspace"]')).getText()).includes('需要正确密码'), 30_000)
  const blockedCards = await driver.findElements(By.css('[data-testid="pdf-draft-card"]'))
  verify('signed installed PDF cannot freeze execution', true, await blockedCards[0].findElement(By.css('[data-testid="pdf-freeze-configuration"]')).getAttribute('disabled') !== null)
  let password = await waitForElement('[data-testid="pdf-password-input"]'); await password.sendKeys('wrong-password'); await (await waitForElement('[data-testid="pdf-password-analyze"]')).click()
  await driver.wait(async () => (await driver.findElement(By.css('[data-testid="pdf-compression-workspace"]')).getText()).includes('PDF_ANALYSIS_INVALID_PASSWORD'), 30_000)
  password = await waitForElement('[data-testid="pdf-password-input"]'); verify('wrong installed PDF password is cleared', '', await password.getAttribute('value'))
  await password.sendKeys('fixture-user'); await (await waitForElement('[data-testid="pdf-password-analyze"]')).click()
  await driver.wait(async () => (await driver.findElement(By.css('[data-testid="pdf-compression-workspace"]')).getText()).includes('密码已验证'), 30_000)
  const encryptedCard = (await driver.findElements(By.css('[data-testid="pdf-draft-card"]')))[1]
  verify('encrypted installed PDF remains execution-blocked', true, /PDF_ENCRYPTED_EXECUTION_UNSUPPORTED/u.test(await encryptedCard.getText()))
  writeFileSync(path.join(evidenceRoot, 'd043-installed-pdf-blocked.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))

  const outputFacts = outputPaths.map(output => ({ path: output, bytes: statSync(output).size, sha256: sha256(output) }))
  writeFileSync(path.join(evidenceRoot, 'result.json'), JSON.stringify({ schemaVersion: 1, node: 'D-04.3', application, expected, actual: { sources: allSources, outputs: outputFacts, cancelledHistory, currentHistory, restartedHistory }, checks }, null, 2))
  console.log('Real installed Windows PDF-workspace full flow passed.')
} catch (error) {
  if (driver) { try { writeFileSync(path.join(evidenceRoot, 'd043-installed-pdf-failure.png'), Buffer.from(await driver.takeScreenshot(), 'base64')) } catch { /* unavailable */ } }
  writeFileSync(path.join(evidenceRoot, 'result.json'), JSON.stringify({ schemaVersion: 1, node: 'D-04.3', application, expected, checks, error: String(error), driverOutput }, null, 2))
  throw error
} finally {
  if (driver) { try { await driver.quit() } catch { /* session may already be closed */ } }
  terminateApplication()
  if (tauriDriverProcess && tauriDriverProcess.exitCode === null) tauriDriverProcess.kill()
  terminateFixtureWebViews()
  await new Promise(resolve => setTimeout(resolve, 1_000))
  rmSync(fixtureRoot, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 })
}

import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Builder, By, Capabilities } from 'selenium-webdriver'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const application = process.env.TAURI_APP_BINARY
const edgeDriver = process.env.EDGE_DRIVER_PATH
const tauriDriver = process.env.TAURI_DRIVER_PATH || path.join(homedir(), '.cargo', 'bin', 'tauri-driver.exe')
const webdriverUrl = 'http://127.0.0.1:4444/'
const fixtureRoot = mkdtempSync(path.join(tmpdir(), 'long-decompress-installed-b053-'))
const sourceRoot = path.join(fixtureRoot, '安装版 图片输入')
const evidenceRoot = path.join(root, 'test-results', 'installed-image-workspace')
const pinnedFixtureRoot = path.join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'images')
const checks = []
let driver
let tauriDriverProcess
let mirrorTimer
let driverOutput = ''
let webviewData = ''

const expected = {
  productionBridge: false,
  readyInputs: 3,
  configured: {
    mode: 'lossy',
    quality: '67',
    outputFormat: 'keep',
    resizeMode: 'limit',
    conflictPolicy: 'rename',
  },
  completedOutputs: 3,
  persistedHistoryRecords: 3,
  reopenedOutputs: 3,
}

const verify = (name, wanted, actual, predicate = value => value === wanted) => {
  const passed = Boolean(predicate(actual))
  checks.push({ name, expected: wanted, actual, passed })
  console.log(`[installed-b053] ${name}: expected=${JSON.stringify(wanted)}; actual=${JSON.stringify(actual)}`)
  assert.ok(passed, `${name}: expected=${JSON.stringify(wanted)}; actual=${JSON.stringify(actual)}`)
}

for (const [label, target] of [
  ['installed application', application],
  ['Microsoft EdgeDriver', edgeDriver],
  ['tauri-driver', tauriDriver],
]) assert.ok(target && existsSync(target), `${label} was not found: ${target || '<unset>'}`)

const fixtureSpecs = [
  ['large-photo.jpg', '安装版 大照片.jpg', 'jpeg'],
  ['large-alpha.png', '安装版 透明图.png', 'png'],
  ['photo.webp', '安装版 WebP.webp', 'webp'],
]
mkdirSync(sourceRoot, { recursive: true })
mkdirSync(evidenceRoot, { recursive: true })
const sources = fixtureSpecs.map(([fixture, name, format]) => {
  const fixturePath = path.join(pinnedFixtureRoot, fixture)
  assert.ok(existsSync(fixturePath), `pinned image fixture was not found: ${fixturePath}`)
  const target = path.join(sourceRoot, name)
  copyFileSync(fixturePath, target)
  return { fixture, name, format, path: target }
})

const sha256 = filePath => createHash('sha256').update(readFileSync(filePath)).digest('hex')
const sourceHashes = Object.fromEntries(sources.map(source => [source.path, sha256(source.path)]))
const outputFor = source => path.join(
  sourceRoot,
  `${path.basename(source.name, path.extname(source.name))}.compressed.${source.format === 'jpeg' ? 'jpg' : source.format}`,
)

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

const terminateApplication = () => {
  const escaped = application.replaceAll("'", "''")
  spawnSync('powershell.exe', [
    '-NoProfile', '-NonInteractive', '-Command',
    `Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }`,
  ], { windowsHide: true })
}

const createSession = async attemptOffset => {
  let created
  for (let attempt = 1; !created && attempt <= 3; attempt += 1) {
    webviewData = path.join(fixtureRoot, `webview2-${attemptOffset + attempt}`)
    mkdirSync(webviewData, { recursive: true })
    const capabilities = new Capabilities()
    capabilities.setBrowserName('wry')
    capabilities.set('tauri:options', {
      application,
      args: [],
      webviewOptions: {
        userDataFolder: webviewData,
        additionalBrowserArguments: ['--force-device-scale-factor=1.5'],
      },
    })
    mirrorTimer = setInterval(mirrorDevToolsPort, 50)
    try {
      created = await new Builder().usingServer(webdriverUrl).withCapabilities(capabilities).build()
    } catch (error) {
      if (attempt === 3 || !/DevToolsActivePort|session not created/i.test(String(error))) throw error
      terminateApplication()
      await new Promise(resolve => setTimeout(resolve, 750))
    } finally {
      clearInterval(mirrorTimer)
      mirrorTimer = undefined
    }
  }
  assert.ok(created, 'installed WebView2 session was not created')
  await created.manage().setTimeouts({ implicit: 1_000, pageLoad: 60_000, script: 120_000 })
  await created.manage().window().setRect({ x: -32_000, y: -32_000, width: 1280, height: 800 })
  return created
}

const waitForElement = selector => driver.wait(async () => {
  const elements = await driver.findElements(By.css(selector))
  return elements[0] || false
}, 30_000)

const setControl = async (selector, value) => {
  const control = await waitForElement(selector)
  await driver.executeScript(`
    const control = arguments[0]
    const value = arguments[1]
    control.value = value
    control.dispatchEvent(new Event(control.type === 'range' ? 'input' : 'change', { bubbles: true }))
  `, control, value)
  return control.getAttribute('value')
}

const navigateToImageWorkspace = async () => {
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await (await waitForElement('[data-testid="compression-mode-image"]')).click()
  await waitForElement('[data-testid="image-compression-workspace"]')
}

const emitThroughProductionTauriIpc = async paths => driver.executeAsyncScript(`
  const paths = arguments[0]
  const done = arguments[arguments.length - 1]
  if (typeof window.__TAURI_IPC__ !== 'function') {
    done({ ok: false, error: 'production Tauri IPC transport is unavailable' })
    return
  }
  const callback = window.crypto.getRandomValues(new Uint32Array(1))[0]
  const error = window.crypto.getRandomValues(new Uint32Array(1))[0]
  const cleanup = () => {
    Reflect.deleteProperty(window, '_' + callback)
    Reflect.deleteProperty(window, '_' + error)
  }
  Object.defineProperty(window, '_' + callback, {
    configurable: true,
    value: () => {
      cleanup()
      done({ ok: true, count: paths.length, transport: '__TAURI_IPC__/Event.emit' })
    },
  })
  Object.defineProperty(window, '_' + error, {
    configurable: true,
    value: reason => {
      cleanup()
      done({ ok: false, error: String(reason) })
    },
  })
  window.__TAURI_IPC__({
    cmd: 'tauri',
    callback,
    error,
    __tauriModule: 'Event',
    message: {
      cmd: 'emit',
      event: 'tauri://file-drop',
      windowLabel: null,
      payload: paths,
    },
  })
`, paths)

const visibleImageFacts = async () => driver.executeScript(`
  return Array.from(document.querySelectorAll('.image-task')).map(row => ({
    name: row.querySelector('.image-name strong')?.textContent?.trim() || '',
    status: row.querySelector('.image-status strong')?.textContent?.trim() || '',
    dimensions: row.querySelector('.image-dimensions')?.textContent?.trim() || '',
    outputPath: row.querySelector('.result-path')?.getAttribute('title') || '',
  }))
`)

const waitForReady = async count => {
  try {
    return await driver.wait(async () => {
      const facts = await visibleImageFacts()
      return facts.length === count && facts.every(item => item.status === '待处理') ? facts : false
    }, 60_000)
  } catch (error) {
    throw new Error(`real inputs did not reach ready state: actual=${JSON.stringify(await visibleImageFacts())}; ${error}`)
  }
}

const waitForCompleted = async count => {
  try {
    return await driver.wait(async () => {
      const facts = await visibleImageFacts()
      return facts.length === count && facts.every(item => item.status === '已完成') ? facts : false
    }, 180_000)
  } catch (error) {
    throw new Error(`real inputs did not complete: actual=${JSON.stringify(await visibleImageFacts())}; ${error}`)
  }
}

const verifyPreviewPair = async rowIndex => {
  const rows = await driver.findElements(By.css('.image-task'))
  const expanded = await rows[rowIndex].getAttribute('class').then(value => value.includes('expanded'))
  if (!expanded) {
    const button = await rows[rowIndex].findElement(By.css('.image-row'))
    await driver.executeScript('arguments[0].click()', button)
  }
  await driver.wait(async () => {
    const images = await rows[rowIndex].findElements(By.css('.preview-card img'))
    if (images.length !== 2) return false
    return driver.executeScript('return arguments[0].complete && arguments[0].naturalWidth > 0 && arguments[1].complete && arguments[1].naturalWidth > 0', images[0], images[1])
  }, 30_000)
  return driver.executeScript(`
    const row = arguments[0]
    const images = row.querySelectorAll('.preview-card img')
    return {
      original: [images[0].naturalWidth, images[0].naturalHeight],
      result: [images[1].naturalWidth, images[1].naturalHeight],
      path: row.querySelector('.result-path')?.getAttribute('title') || '',
    }
  `, rows[rowIndex])
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

  driver = await createSession(0)
  await driver.wait(async () => (await waitForElement('main h1')).getText().then(text => text.length > 0), 30_000)
  const bridgePresent = await driver.executeScript('return Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__)')
  verify('formal installed executable excludes desktop E2E bridge', expected.productionBridge, bridgePresent)

  await navigateToImageWorkspace()
  const configured = {
    mode: await setControl('[data-testid="image-setting-mode"]', expected.configured.mode),
    quality: await setControl('[data-testid="image-setting-quality"]', expected.configured.quality),
    outputFormat: await setControl('[data-testid="image-setting-format"]', expected.configured.outputFormat),
    resizeMode: await setControl('[data-testid="image-setting-resize"]', expected.configured.resizeMode),
    conflictPolicy: await setControl('[data-testid="image-setting-conflict"]', expected.configured.conflictPolicy),
  }
  verify('visible global configuration values', expected.configured, configured, actual => JSON.stringify(actual) === JSON.stringify(expected.configured))

  const dropResult = await emitThroughProductionTauriIpc(sources.map(source => source.path))
  verify('production Tauri file-drop IPC accepted real paths', { ok: true, count: 3 }, dropResult, actual => actual.ok && actual.count === 3)
  const readyFacts = await waitForReady(3)
  verify('real inputs reached ready state', expected.readyInputs, readyFacts.length)
  verify('original preview is decoded before execution', true, await driver.executeScript(`
    const image = document.querySelector('.image-task.expanded .preview-card img')
    return Boolean(image && image.complete && image.naturalWidth > 0 && image.naturalHeight > 0)
  `))
  verify('result preview is absent before execution', 0, await driver.findElements(By.css('.image-task.expanded .result-ready img')).then(items => items.length))

  await (await waitForElement('[data-testid="image-compression-workspace"] .primary-action')).click()
  const completedFacts = await waitForCompleted(3)
  const outputPaths = sources.map(outputFor)
  const outputFacts = outputPaths.map(outputPath => ({
    path: outputPath,
    exists: existsSync(outputPath),
    bytes: existsSync(outputPath) ? statSync(outputPath).size : 0,
    sha256: existsSync(outputPath) ? sha256(outputPath) : '',
  }))
  verify('real published output count', expected.completedOutputs, outputFacts.filter(fact => fact.exists && fact.bytes > 0).length)
  verify('source bytes remain unchanged', 0, sources.filter(source => sha256(source.path) !== sourceHashes[source.path]).length)
  const previewPairs = []
  for (let index = 0; index < sources.length; index += 1) previewPairs.push(await verifyPreviewPair(index))
  verify('real original/result comparison pairs', 3, previewPairs.length, actual => actual === 3 && previewPairs.every(pair => pair.original.every(Boolean) && pair.result.every(Boolean) && outputPaths.includes(pair.path)))
  verify('visible completed output paths match disk outputs', outputPaths, previewPairs.map(pair => pair.path), actual => outputPaths.every(output => actual.includes(output)))

  verify('result-location action is available for a published output', 1, await driver.findElements(By.css('.image-task.expanded .open-result')).then(items => items.length), actual => actual >= 1)

  await (await waitForElement('[data-testid="nav-History"]')).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="history-record-row"]'))).length >= 3, 30_000)
  const currentHistory = await driver.executeScript(`
    const outputs = arguments[0]
    return Array.from(document.querySelectorAll('[data-testid="history-record-row"]'))
      .map(row => ({ name: row.querySelector('h3')?.textContent?.trim() || '', text: row.textContent || '' }))
      .filter(record => outputs.some(output => record.text.includes(output)))
  `, outputPaths)
  verify('current-session image history records', expected.persistedHistoryRecords, currentHistory.length)
  verify('current-session history statuses are completed', 3, currentHistory.filter(record => /已完成/.test(record.text)).length)

  writeFileSync(path.join(evidenceRoot, 'b053-installed-image-completed.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  await driver.quit()
  driver = undefined
  terminateApplication()
  await new Promise(resolve => setTimeout(resolve, 750))

  driver = await createSession(10)
  await driver.wait(async () => (await waitForElement('main h1')).getText().then(text => text.length > 0), 30_000)
  await (await waitForElement('[data-testid="nav-History"]')).click()
  const reopenedHistory = await driver.wait(async () => {
    const records = await driver.executeScript(`
      const outputs = arguments[0]
      return Array.from(document.querySelectorAll('[data-testid="history-record-row"]'))
        .map(row => ({ name: row.querySelector('h3')?.textContent?.trim() || '', text: row.textContent || '' }))
        .filter(record => outputs.some(output => record.text.includes(output)))
    `, outputPaths)
    return records.length === 3 ? records : false
  }, 30_000)
  verify('history survives complete installed-app restart', expected.persistedHistoryRecords, reopenedHistory.length)

  await navigateToImageWorkspace()
  const reopenDrop = await emitThroughProductionTauriIpc(outputPaths)
  verify('published outputs can be reopened through production Tauri file-drop IPC', { ok: true, count: 3 }, reopenDrop, actual => actual.ok && actual.count === 3)
  const reopenedFacts = await waitForReady(3)
  verify('reopened output count', expected.reopenedOutputs, reopenedFacts.length)
  verify('reopened outputs decode to visible dimensions', 3, reopenedFacts.filter(item => /^\d+\s*×\s*\d+$/.test(item.dimensions)).length)

  writeFileSync(path.join(evidenceRoot, 'b053-installed-image-reopened.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  writeFileSync(path.join(evidenceRoot, 'result.json'), JSON.stringify({
    schemaVersion: 1,
    application,
    expected,
    actual: { sources, sourceHashes, outputs: outputFacts, completedFacts, previewPairs, currentHistory, reopenedHistory, reopenedFacts },
    checks,
  }, null, 2))
  console.log('Real installed Windows image-workspace full flow passed.')
} catch (error) {
  if (driver) {
    try { writeFileSync(path.join(evidenceRoot, 'b053-installed-image-failure.png'), Buffer.from(await driver.takeScreenshot(), 'base64')) } catch { /* session unavailable */ }
  }
  writeFileSync(path.join(evidenceRoot, 'result.json'), JSON.stringify({
    schemaVersion: 1,
    application,
    expected,
    checks,
    error: String(error),
    driverOutput,
  }, null, 2))
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

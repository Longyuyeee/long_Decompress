import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
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
const fixtureRoot = mkdtempSync(path.join(tmpdir(), 'long-decompress-installed-c054-'))
const sourceRoot = path.join(fixtureRoot, '安装版 视频输入')
const evidenceRoot = path.resolve(
  process.env.VIDEO_WORKSPACE_EVIDENCE_DIRECTORY
    || path.join(root, 'test-results', 'installed-video-workspace'),
)
const pinnedCancellationFixture = path.join(
  root,
  'test-results',
  'c05-video-long-large-matrix',
  'inputs',
  'avi-10-minute.avi',
)
const pinnedCompletionFixture = path.join(
  root,
  'test-results',
  'c05-video-long-large-matrix',
  'inputs',
  'avi-100mib-1080p.avi',
)
const runSuffix = path.basename(fixtureRoot).slice(-6)
const cancellationSource = path.join(sourceRoot, `安装版 取消 10分钟-${runSuffix}.avi`)
const cancellationOutput = path.join(sourceRoot, `安装版 取消 10分钟-${runSuffix}.compressed.mp4`)
const completionSource = path.join(sourceRoot, `安装版 完成 1080p-${runSuffix}.avi`)
const completionOutput = path.join(sourceRoot, `安装版 完成 1080p-${runSuffix}.compressed.mp4`)
const checks = []
let driver
let tauriDriverProcess
let mirrorTimer
let driverOutput = ''
let webviewData = ''

const expected = {
  productionBridge: false,
  cancellationSourceBytes: 30_163_318,
  completionSourceBytes: 114_842_332,
  cancelledHistoryRecords: 1,
  completedHistoryRecords: 1,
  output: { container: 'mp4', videoCodec: 'h264', width: 1920, height: 1080, durationSeconds: 32 },
}

const verify = (name, wanted, actual, predicate = value => value === wanted) => {
  const passed = Boolean(predicate(actual))
  checks.push({ name, expected: wanted, actual, passed })
  console.log(`[installed-c054] ${name}: expected=${JSON.stringify(wanted)}; actual=${JSON.stringify(actual)}`)
  assert.ok(passed, `${name}: expected=${JSON.stringify(wanted)}; actual=${JSON.stringify(actual)}`)
}

for (const [label, target] of [
  ['installed application', application],
  ['Microsoft EdgeDriver', edgeDriver],
  ['tauri-driver', tauriDriver],
  ['pinned C-05 ten-minute cancellation fixture', pinnedCancellationFixture],
  ['pinned C-05 large completion fixture', pinnedCompletionFixture],
]) assert.ok(target && existsSync(target), `${label} was not found: ${target || '<unset>'}`)

mkdirSync(sourceRoot, { recursive: true })
mkdirSync(evidenceRoot, { recursive: true })
copyFileSync(pinnedCancellationFixture, cancellationSource)
copyFileSync(pinnedCompletionFixture, completionSource)
const sha256 = filePath => createHash('sha256').update(readFileSync(filePath)).digest('hex')
const cancellationSourceSha256 = sha256(cancellationSource)
const completionSourceSha256 = sha256(completionSource)
const runtimeRoot = path.join(path.dirname(application), 'resources', 'video-engine')
const productFfprobe = path.join(runtimeRoot, 'ffprobe.exe')
assert.ok(existsSync(productFfprobe), `installed product ffprobe was not found: ${productFfprobe}`)

const appendDriverOutput = chunk => {
  driverOutput = `${driverOutput}${chunk}`.slice(-32_768)
  process.stdout.write(chunk)
}

const mirrorDevToolsPort = () => {
  const sourcePath = path.join(webviewData, 'EBWebView', 'DevToolsActivePort')
  const destination = path.join(webviewData, 'DevToolsActivePort')
  if (!existsSync(sourcePath)) return
  try { copyFileSync(sourcePath, destination) } catch { /* EdgeDriver may be reading it. */ }
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
    message: { cmd: 'emit', event: 'tauri://file-drop', windowLabel: null, payload: paths },
  })
`, paths)

const videoFfmpegProcessIds = sourcePath => {
  const sourceName = path.basename(sourcePath).replaceAll("'", "''")
  const result = spawnSync('powershell.exe', [
    '-NoProfile', '-NonInteractive', '-Command',
    `Get-CimInstance Win32_Process | Where-Object { $_.Name -ieq 'ffmpeg.exe' -and $_.CommandLine -like '*${sourceName}*' } | ForEach-Object { $_.ProcessId }`,
  ], { encoding: 'utf8', windowsHide: true })
  assert.ifError(result.error)
  assert.equal(result.status, 0, `failed to inspect installed product FFmpeg: ${result.stderr}`)
  return result.stdout.split(/\r?\n/u).map(value => Number.parseInt(value.trim(), 10)).filter(Number.isInteger)
}

const navigateToVideoWorkspace = async () => {
  await (await waitForElement('[data-testid="nav-SpecialCompression"]')).click()
  await (await waitForElement('[data-testid="compression-mode-video"]')).click()
  await waitForElement('[data-testid="video-compression-workspace"]')
}

const addSourceAndWaitForReady = async sourcePath => {
  const dropResult = await emitThroughProductionTauriIpc([sourcePath])
  verify('production Tauri file-drop IPC accepted the real video path', true, dropResult.ok)
  return driver.wait(async () => {
    const cards = await driver.findElements(By.css('[data-testid="video-draft-card"]'))
    return cards.length === 1 && await cards[0].getAttribute('data-status') === 'ready' ? cards[0] : false
  }, 60_000)
}

const matchingHistory = async () => driver.executeScript(`
  const names = arguments[0]
  return Array.from(document.querySelectorAll('[data-testid="history-record-row"]'))
    .filter(row => names.some(name => (row.textContent || '').includes(name)))
    .map(row => ({
      text: row.textContent || '',
      status: row.querySelector('[data-testid="history-status-badge"]')?.textContent?.trim() || '',
    }))
`, [path.basename(cancellationSource), path.basename(completionSource)])

const openHistoryAndWait = async expectedCount => {
  await (await waitForElement('[data-testid="nav-History"]')).click()
  await waitForElement('[data-testid="history-list"]')
  return driver.wait(async () => {
    const records = await matchingHistory()
    return records.length === expectedCount ? records : false
  }, 30_000)
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
  verify(
    'formal installed executable excludes desktop E2E bridge',
    expected.productionBridge,
    await driver.executeScript('return Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__)'),
  )
  verify('real ten-minute cancellation source byte identity', expected.cancellationSourceBytes, statSync(cancellationSource).size)
  verify('real large completion source byte identity', expected.completionSourceBytes, statSync(completionSource).size)

  await navigateToVideoWorkspace()
  await addSourceAndWaitForReady(cancellationSource)
  await (await waitForElement('[data-testid="video-compression-workspace"] .primary-action')).click()
  await driver.wait(async () => {
    const card = await waitForElement('[data-testid="video-draft-card"]')
    return await card.getAttribute('data-status') === 'compressing'
  }, 30_000)
  const observedFfmpegProcessIds = await driver.wait(() => {
    const processIds = videoFfmpegProcessIds(cancellationSource)
    return processIds.length > 0 ? processIds : false
  }, 30_000)
  await (await waitForElement('[data-testid="video-compression-workspace"] .danger-action')).click()
  await driver.wait(async () => {
    const card = await waitForElement('[data-testid="video-draft-card"]')
    return await card.getAttribute('data-status') === 'cancelled'
  }, 30_000)
  await driver.wait(() => videoFfmpegProcessIds(cancellationSource).length === 0, 30_000)
  verify('cancelled installed task does not publish output', false, existsSync(cancellationOutput))
  verify(
    'cancelled installed task removes video staging',
    0,
    readdirSync(sourceRoot).filter(name => name.includes('.video-encode-')).length,
  )
  verify('cancelled installed task preserves source SHA-256', cancellationSourceSha256, sha256(cancellationSource))
  const cancelledHistory = await openHistoryAndWait(1)
  verify(
    'installed cancellation persists cancelled history',
    expected.cancelledHistoryRecords,
    cancelledHistory.filter(record => record.status === '已取消').length,
  )
  writeFileSync(
    path.join(evidenceRoot, 'c054-installed-video-cancelled.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  await navigateToVideoWorkspace()
  const cancelledCard = await waitForElement('[data-testid="video-draft-card"]')
  await cancelledCard.findElement(By.css('.remove')).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="video-draft-card"]'))).length === 0, 15_000)
  await addSourceAndWaitForReady(completionSource)
  await (await waitForElement('[data-testid="video-compression-workspace"] .primary-action')).click()
  const completedCard = await driver.wait(async () => {
    const card = await waitForElement('[data-testid="video-draft-card"]')
    return await card.getAttribute('data-status') === 'completed' ? card : false
  }, 180_000)
  verify('installed video output exists', true, existsSync(completionOutput))
  verify('installed video output is non-empty', true, statSync(completionOutput).size > 0)
  verify('completed installed task preserves source SHA-256', completionSourceSha256, sha256(completionSource))
  const probeResult = spawnSync(productFfprobe, [
    '-v', 'error', '-show_entries',
    'format=format_name,duration,size:stream=codec_type,codec_name,width,height',
    '-of', 'json', completionOutput,
  ], { encoding: 'utf8', windowsHide: true, maxBuffer: 32 * 1024 * 1024 })
  assert.equal(probeResult.status, 0, probeResult.stderr || probeResult.stdout)
  const probe = JSON.parse(probeResult.stdout)
  const video = probe.streams.find(stream => stream.codec_type === 'video')
  verify('installed output container is MP4', true, probe.format.format_name.includes('mp4'))
  verify('installed output video codec', expected.output.videoCodec, video?.codec_name)
  verify('installed output dimensions', [expected.output.width, expected.output.height], [video?.width, video?.height], actual => actual[0] === expected.output.width && actual[1] === expected.output.height)
  verify('installed output duration', expected.output.durationSeconds, Number(probe.format.duration), actual => Math.abs(actual - expected.output.durationSeconds) <= 0.32)

  const defaultPlayback = await completedCard.findElement(By.css('[data-testid="video-open-default-app"]'))
  const defaultPlaybackActionAvailable = await defaultPlayback.isEnabled()
  verify('published MP4 exposes an enabled default-application action without launching an external window', true, defaultPlaybackActionAvailable)
  const currentHistory = await openHistoryAndWait(2)
  verify('current installed session has one cancelled record', 1, currentHistory.filter(record => record.status === '已取消').length)
  verify('current installed session has one completed record', 1, currentHistory.filter(record => record.status === '已完成').length)
  writeFileSync(
    path.join(evidenceRoot, 'c054-installed-video-completed.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  await driver.quit()
  driver = undefined
  terminateApplication()
  await new Promise(resolve => setTimeout(resolve, 750))
  driver = await createSession(10)
  await driver.wait(async () => (await waitForElement('main h1')).getText().then(text => text.length > 0), 30_000)
  const restartedHistory = await openHistoryAndWait(2)
  verify('cancelled history survives complete installed-app restart', 1, restartedHistory.filter(record => record.status === '已取消').length)
  verify('completed history survives complete installed-app restart', 1, restartedHistory.filter(record => record.status === '已完成').length)
  writeFileSync(
    path.join(evidenceRoot, 'c054-installed-video-history-restart.png'),
    Buffer.from(await driver.takeScreenshot(), 'base64'),
  )

  writeFileSync(path.join(evidenceRoot, 'result.json'), `${JSON.stringify({
    schemaVersion: 1,
    application,
    expected,
    actual: {
      cancellationSource: { path: cancellationSource, bytes: statSync(cancellationSource).size, sha256: cancellationSourceSha256 },
      completionSource: { path: completionSource, bytes: statSync(completionSource).size, sha256: completionSourceSha256 },
      cancellation: { observedFfmpegProcessIds, outputAbsent: true, stagingCleaned: true },
      publication: { path: completionOutput, bytes: statSync(completionOutput).size, sha256: sha256(completionOutput), probe },
      defaultPlaybackActionAvailable,
      currentHistory,
      restartedHistory,
    },
    checks,
    passed: checks.every(check => check.passed),
  }, null, 2)}\n`)
  console.log('Real installed Windows C-05.4 video workspace full flow passed.')
} catch (error) {
  if (driver) {
    try {
      writeFileSync(
        path.join(evidenceRoot, 'c054-installed-video-failure.png'),
        Buffer.from(await driver.takeScreenshot(), 'base64'),
      )
    } catch { /* session unavailable */ }
  }
  writeFileSync(path.join(evidenceRoot, 'result.json'), `${JSON.stringify({
    schemaVersion: 1,
    application,
    expected,
    checks,
    passed: false,
    error: String(error),
    driverOutput,
  }, null, 2)}\n`)
  throw error
} finally {
  if (mirrorTimer) clearInterval(mirrorTimer)
  if (driver) {
    try { await driver.quit() } catch { /* application may close first */ }
  }
  if (tauriDriverProcess?.pid) {
    spawnSync('taskkill.exe', ['/pid', String(tauriDriverProcess.pid), '/t', '/f'], {
      stdio: 'ignore',
      windowsHide: true,
    })
  }
  terminateApplication()
  terminateFixtureWebViews()
  await new Promise(resolve => setTimeout(resolve, 1_000))
  rmSync(fixtureRoot, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 })
}

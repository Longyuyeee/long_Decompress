import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
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
import { randomBytes } from 'node:crypto'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Builder, By, Capabilities } from 'selenium-webdriver'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const executableSuffix = process.platform === 'win32' ? '.exe' : ''
const application =
  process.env.TAURI_APP_BINARY ||
  path.join(root, 'src-tauri', 'target', 'release', `long-compress-assistant${executableSuffix}`)
const tauriDriver =
  process.env.TAURI_DRIVER_PATH ||
  path.join(homedir(), '.cargo', 'bin', `tauri-driver${executableSuffix}`)
const edgeDriver = process.env.EDGE_DRIVER_PATH
const webdriverUrl = 'http://127.0.0.1:4444/'
const artifactDirectory = path.join(root, 'test-results', 'desktop-e2e')
const e2eDataDirectory =
  process.env.LONG_DECOMPRESS_E2E_DATA_DIR ||
  path.join(root, 'test-results', 'desktop-e2e-data')
const webviewUserDataDirectory = path.join(e2eDataDirectory, 'webview2')
const bundledSevenZip = path.join(root, 'src-tauri', 'resources', 'archive-engine', '7z.exe')

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

function forwardContextAction(flag, files) {
  const result = spawnSync(application, [flag, ...files], {
    cwd: root,
    env: {
      ...process.env,
      LONG_DECOMPRESS_E2E_DATA_DIR: e2eDataDirectory,
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

function runFixtureCommand(command, args, label) {
  const result = spawnSync(command, args, {
    cwd: fixtureDirectory,
    encoding: 'utf8',
    timeout: 30_000,
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
  const identifier = `${entryName}/`.padEnd(16, ' ')
  const timestamp = '0'.padEnd(12, ' ')
  const owner = '0'.padEnd(6, ' ')
  const group = '0'.padEnd(6, ' ')
  const mode = '100644'.padEnd(8, ' ')
  const size = String(payload.length).padEnd(10, ' ')
  const header = Buffer.from(`${identifier}${timestamp}${owner}${group}${mode}${size}\x60\n`, 'ascii')
  const padding = payload.length % 2 === 0 ? Buffer.alloc(0) : Buffer.from('\n')
  writeFileSync(outputPath, Buffer.concat([Buffer.from('!<arch>\n', 'ascii'), header, payload, padding]))
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

try {
  mkdirSync(webviewUserDataDirectory, { recursive: true })
  tauriDriverProcess = spawn(
    tauriDriver,
    ['--native-driver', edgeDriver],
    {
      cwd: root,
      env: {
        ...process.env,
        LONG_DECOMPRESS_E2E_DATA_DIR: e2eDataDirectory,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    },
  )
  tauriDriverProcess.stdout.on('data', appendDriverOutput)
  tauriDriverProcess.stderr.on('data', appendDriverOutput)
  await waitForWebDriver()

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
  driver = await new Builder().usingServer(webdriverUrl).withCapabilities(capabilities).build()
  clearInterval(devToolsPortMirror)
  devToolsPortMirror = undefined
  await driver.manage().setTimeouts({ implicit: 1_000, pageLoad: 60_000, script: 30_000 })

  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/decompress'), 60_000)
  assert.ok(await waitForNonEmptyText('main h1'), 'the decompression workspace heading is empty')
  await driver.wait(
    () => driver.executeScript('return Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__)'),
    30_000,
  )

  let navigation = await driver.findElements(By.css('aside nav > button'))
  assert.equal(navigation.length, 5, 'the real desktop shell must expose five navigation buttons')
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

  forwardContextAction('--quick-extract', [archivePath])
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/decompress'), 30_000)
  await waitForFileContent(extractedPath, payload)
  assert.equal(
    readFileSync(extractedPath, 'utf8'),
    payload,
    'the extracted file must match the source payload byte-for-byte',
  )

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
    ['zip', 'zip', null],
    ['7z', '7z', null],
    ['wim', 'wim', null],
    ['tar', 'tar', null],
    ['tar.gz', 'tar.gz', null],
    ['tar.bz2', 'tar.bz2', null],
    ['tar.xz', 'tar.xz', null],
    ['tar.zst', 'tar.zst', null],
    ['gz', 'txt.gz', null],
    ['bz2', 'txt.bz2', null],
    ['xz', 'txt.xz', null],
    ['zst', 'txt.zst', null],
    ['zstd', 'txt.zstd', null],
    ['lzma', 'txt.lzma', null],
    ['zip-password', 'zip', 'desktop-e2e-password'],
    ['7z-password', '7z', 'desktop-e2e-password'],
    ['tar.aes', 'tar.aes', 'desktop-e2e-password'],
    ['tar.gz.aes', 'tar.gz.aes', 'desktop-e2e-password'],
    ['tar.bz2.aes', 'tar.bz2.aes', 'desktop-e2e-password'],
    ['tar.xz.aes', 'tar.xz.aes', 'desktop-e2e-password'],
    ['tar.zst.aes', 'tar.zst.aes', 'desktop-e2e-password'],
    ['gz.aes', 'txt.gz.aes', 'desktop-e2e-password'],
    ['bz2.aes', 'txt.bz2.aes', 'desktop-e2e-password'],
    ['xz.aes', 'txt.xz.aes', 'desktop-e2e-password'],
    ['zst.aes', 'txt.zst.aes', 'desktop-e2e-password'],
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
  for (const [label, extension, password] of archiveMatrix) {
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
  for (const extension of ['jar', 'xpi', 'ipa', 'apk', 'appx']) {
    const archive = path.join(fixtureDirectory, `extract-only.${extension}`)
    createZipCompatibleFixture(archive, extractOnlySource)
    extractOnlyMatrix.push([extension, archive, 'extract-only-payload.txt'])
  }
  const cabArchive = path.join(fixtureDirectory, 'extract-only.cab')
  runFixtureCommand(
    'makecab.exe',
    ['/D', 'CompressionType=LZX', extractOnlySource, cabArchive],
    'CAB',
  )
  extractOnlyMatrix.push(['cab', cabArchive, 'extract-only-payload.txt'])
  const arArchive = path.join(fixtureDirectory, 'extract-only.ar')
  createArFixture(arArchive, 'payload.txt', extractOnlyPayload)
  extractOnlyMatrix.push(['ar', arArchive, 'payload.txt'])

  for (const [label, archive, extractedName] of extractOnlyMatrix) {
    const output = path.join(fixtureDirectory, `extract-only-${label}-output`)
    await callDesktopBridge('extractArchive', archive, output)
    assert.deepEqual(
      readFileSync(path.join(output, extractedName)),
      extractOnlyPayload,
      `${label} extraction must reproduce the real sample byte-for-byte`,
    )
  }
  await callDesktopBridge('clearTasks')

  navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[4].click()
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

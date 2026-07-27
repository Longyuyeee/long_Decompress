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

  navigation = await driver.findElements(By.css('aside nav > button'))
  await navigation[4].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/settings'), 30_000)
  assert.ok(await waitForNonEmptyText('main h1'), 'the settings heading is empty')
  completedSuccessfully = true
  console.log('Real Windows Tauri desktop archive round-trip test passed.')
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

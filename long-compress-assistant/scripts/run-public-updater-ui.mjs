import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { Builder, By, Capabilities, Key } from 'selenium-webdriver'

const application = process.env.PUBLIC_UPDATER_APP
const expectedVersion = process.env.PUBLIC_UPDATER_EXPECTED_VERSION
const tauriDriver = process.env.TAURI_DRIVER_PATH
const edgeDriver = process.env.EDGE_DRIVER_PATH
const webdriverUrl = 'http://127.0.0.1:4444/'
const artifactDirectory = path.resolve('test-results', 'public-updater-ui')
const webviewUserDataDirectory = mkdtempSync(
  path.join(tmpdir(), 'long-decompress-public-updater-webview-'),
)

for (const [label, target] of [
  ['installed application', application],
  ['expected version', expectedVersion],
  ['tauri-driver', tauriDriver],
  ['Microsoft EdgeDriver', edgeDriver],
]) {
  if (!target || (label !== 'expected version' && !existsSync(target))) {
    throw new Error(`${label} was not found: ${target || '<unset>'}`)
  }
}

let driver
let tauriDriverProcess
let devToolsPortMirror
let driverOutput = ''
let completedSuccessfully = false

function terminateProcessTree(processId) {
  if (!processId) return
  spawnSync('taskkill.exe', ['/pid', String(processId), '/t', '/f'], {
    stdio: 'ignore',
    windowsHide: true,
  })
}

function appendDriverOutput(chunk) {
  driverOutput = `${driverOutput}${chunk}`.slice(-32_768)
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

function mirrorDevToolsActivePort() {
  const source = path.join(webviewUserDataDirectory, 'EBWebView', 'DevToolsActivePort')
  const destination = path.join(webviewUserDataDirectory, 'DevToolsActivePort')
  if (!existsSync(source)) return
  try {
    copyFileSync(source, destination)
  } catch {
    // EdgeDriver may be reading or replacing the compatibility copy.
  }
}

async function sessionIsAlive() {
  try {
    await driver.getCurrentUrl()
    return true
  } catch {
    return false
  }
}

async function waitForInstalledVersion(timeoutMs = 240_000) {
  const registryKey =
    'HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Long解压'
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const result = spawnSync(
      'reg.exe',
      ['query', registryKey, '/v', 'DisplayVersion'],
      { encoding: 'utf8', windowsHide: true },
    )
    if (result.status === 0 && result.stdout.includes(expectedVersion)) return
    await new Promise((resolve) => setTimeout(resolve, 500))
  }
  throw new Error(`Timed out waiting for installed version ${expectedVersion}.`)
}

async function waitForVisibleDialog(timeoutMs = 60_000) {
  return driver.wait(async () => {
    const dialogs = await driver.findElements(By.css('[role="dialog"]'))
    for (const dialog of dialogs) {
      if (await dialog.isDisplayed()) return dialog
    }
    return false
  }, timeoutMs)
}

try {
  mkdirSync(artifactDirectory, { recursive: true })
  tauriDriverProcess = spawn(tauriDriver, ['--native-driver', edgeDriver], {
    cwd: process.cwd(),
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  })
  tauriDriverProcess.stdout.on('data', appendDriverOutput)
  tauriDriverProcess.stderr.on('data', appendDriverOutput)
  await waitForWebDriver()

  const capabilities = new Capabilities()
  capabilities.setBrowserName('wry')
  capabilities.set('tauri:options', {
    application,
    webviewOptions: { userDataFolder: webviewUserDataDirectory },
  })
  devToolsPortMirror = setInterval(mirrorDevToolsActivePort, 50)
  driver = await new Builder().usingServer(webdriverUrl).withCapabilities(capabilities).build()
  clearInterval(devToolsPortMirror)
  devToolsPortMirror = undefined
  await driver.manage().setTimeouts({ implicit: 1_000, pageLoad: 60_000, script: 30_000 })

  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/decompress'), 60_000)
  await driver.actions().keyDown(Key.CONTROL).sendKeys(',').keyUp(Key.CONTROL).perform()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/settings'), 30_000)

  const refreshIcon = await driver.findElement(By.css('main button .pi-refresh'))
  await refreshIcon.findElement(By.xpath('..')).click()
  const dialog = await waitForVisibleDialog()
  await driver.wait(async () => (await dialog.getText()).includes(expectedVersion), 60_000)

  const buttons = await dialog.findElements(By.css('button'))
  const installButtons = []
  for (const button of buttons) {
    if ((await button.isDisplayed()) && (await button.isEnabled())) installButtons.push(button)
  }
  assert.ok(installButtons.length >= 3, 'the signed update dialog must expose its actions')
  await installButtons.at(-1).click()

  const deadline = Date.now() + 180_000
  while (Date.now() < deadline && (await sessionIsAlive())) {
    await new Promise((resolve) => setTimeout(resolve, 500))
  }
  assert.equal(await sessionIsAlive(), false, 'the application did not hand off to the updater')
  await waitForInstalledVersion()
  completedSuccessfully = true
} catch (error) {
  mkdirSync(artifactDirectory, { recursive: true })
  if (driver && (await sessionIsAlive())) {
    try {
      const screenshot = await driver.takeScreenshot()
      writeFileSync(
        path.join(artifactDirectory, 'public-updater-failure.png'),
        Buffer.from(screenshot, 'base64'),
      )
    } catch {
      // The WebDriver session may disappear while the updater takes over.
    }
  }
  throw error
} finally {
  if (devToolsPortMirror) clearInterval(devToolsPortMirror)
  if (driver) {
    try {
      await driver.quit()
    } catch {
      // A successful updater hand-off terminates the WebDriver session.
    }
  }
  if (completedSuccessfully && tauriDriverProcess) {
    // Keep the driver job alive until the outer validator has observed the updater's
    // restart. Closing the Windows job earlier also terminates the spawned installer.
    tauriDriverProcess.stdout.destroy()
    tauriDriverProcess.stderr.destroy()
    tauriDriverProcess.unref()
    writeFileSync(
      path.join(artifactDirectory, 'tauri-driver.pid'),
      String(tauriDriverProcess.pid),
      'utf8',
    )
  } else {
    terminateProcessTree(tauriDriverProcess?.pid)
  }
  mkdirSync(artifactDirectory, { recursive: true })
  writeFileSync(path.join(artifactDirectory, 'tauri-driver.log'), driverOutput, 'utf8')
  if (completedSuccessfully) {
    writeFileSync(
      path.join(artifactDirectory, 'active-webview-profile.txt'),
      webviewUserDataDirectory,
      'utf8',
    )
  } else {
    writeFileSync(
      path.join(artifactDirectory, 'retained-webview-profile.txt'),
      webviewUserDataDirectory,
      'utf8',
    )
  }
}

console.log(`Public updater UI handed off v${expectedVersion} successfully.`)

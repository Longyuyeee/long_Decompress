import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Builder, By, Capabilities, until } from 'selenium-webdriver'

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
let driverOutput = ''

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

async function captureFailure() {
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

try {
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
  capabilities.set('tauri:options', { application })
  driver = await new Builder().usingServer(webdriverUrl).withCapabilities(capabilities).build()
  await driver.manage().setTimeouts({ implicit: 1_000, pageLoad: 60_000, script: 30_000 })

  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/decompress'), 60_000)
  const heading = await driver.wait(until.elementLocated(By.css('main h1')), 30_000)
  assert.ok((await heading.getText()).trim(), 'the decompression workspace heading is empty')

  const navigation = await driver.findElements(By.css('aside nav > button'))
  assert.equal(navigation.length, 5, 'the real desktop shell must expose five navigation buttons')
  assert.equal(
    await navigation[0].getAttribute('aria-current'),
    'page',
    'the decompression workspace must be selected by default',
  )

  await navigation[4].click()
  await driver.wait(async () => (await driver.getCurrentUrl()).includes('#/settings'), 30_000)
  const settingsHeading = await driver.wait(until.elementLocated(By.css('main h1')), 30_000)
  assert.ok((await settingsHeading.getText()).trim(), 'the settings heading is empty')
  console.log('Real Windows Tauri desktop smoke test passed.')
} catch (error) {
  await captureFailure()
  throw error
} finally {
  if (driver) {
    try {
      await driver.quit()
    } catch {
      // Continue with process-tree cleanup below.
    }
  }
  terminateProcessTree(tauriDriverProcess?.pid)
}

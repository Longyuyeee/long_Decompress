import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import path from 'node:path'
import { chromium } from '@playwright/test'

const application = process.env.PUBLIC_UPDATER_APP
const expectedVersion = process.env.PUBLIC_UPDATER_EXPECTED_VERSION
const artifactDirectory = path.resolve(
  process.env.PUBLIC_UPDATER_ARTIFACT_DIR ||
    path.join('test-results', 'public-updater-ui'),
)

for (const [label, target] of [
  ['installed application', application],
  ['expected version', expectedVersion],
]) {
  if (!target || (label !== 'expected version' && !existsSync(target))) {
    throw new Error(`${label} was not found: ${target || '<unset>'}`)
  }
}

let browser
let page
let applicationProcess
let handedOff = false

function terminateProcessTree(processId) {
  if (!processId) return
  spawnSync('taskkill.exe', ['/pid', String(processId), '/t', '/f'], {
    stdio: 'ignore',
    windowsHide: true,
  })
}

async function reserveLoopbackPort() {
  const server = createServer()
  await new Promise((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', resolve)
  })
  const address = server.address()
  const port = typeof address === 'object' && address ? address.port : 0
  await new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()))
  })
  assert.ok(port > 0, 'failed to reserve a WebView2 debugging port')
  return port
}

async function waitForCdpEndpoint(endpoint, timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    if (applicationProcess.exitCode !== null) {
      throw new Error(
        `installed application exited before WebView2 was ready: ${applicationProcess.exitCode}`,
      )
    }
    try {
      const response = await fetch(`${endpoint}/json/version`)
      if (response.ok) {
        const version = await response.json()
        if (version.webSocketDebuggerUrl) return version.webSocketDebuggerUrl
      }
    } catch {
      // WebView2 has not opened the debugging endpoint yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error('Timed out waiting for the independent WebView2 process.')
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

try {
  mkdirSync(artifactDirectory, { recursive: true })
  const debuggingPort = await reserveLoopbackPort()
  const endpoint = `http://127.0.0.1:${debuggingPort}`
  const existingBrowserArguments =
    process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS?.trim()
  const browserArguments = [
    existingBrowserArguments,
    `--remote-debugging-port=${debuggingPort}`,
  ]
    .filter(Boolean)
    .join(' ')

  applicationProcess = spawn(application, [], {
    detached: true,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: browserArguments,
    },
    stdio: 'ignore',
    windowsHide: true,
  })
  applicationProcess.unref()
  writeFileSync(
    path.join(artifactDirectory, 'independent-application.json'),
    JSON.stringify(
      {
        processId: applicationProcess.pid,
        debuggingPort,
        launchedAt: new Date().toISOString(),
      },
      null,
      2,
    ),
    'utf8',
  )

  const browserWebSocketUrl = await waitForCdpEndpoint(endpoint)
  browser = await chromium.connectOverCDP(browserWebSocketUrl)
  const context = browser.contexts()[0]
  assert.ok(context, 'WebView2 did not expose a browser context')
  page = context.pages()[0] || (await context.waitForEvent('page'))
  await page.waitForURL(/#\/decompress/, { timeout: 60_000 })

  await page.keyboard.press('Control+,')
  await page.waitForURL(/#\/settings/, { timeout: 30_000 })
  await page.locator('main button:has(.pi-refresh)').first().click()

  const dialog = page.locator('[role="dialog"]:visible')
  await dialog.waitFor({ state: 'visible', timeout: 60_000 })
  await assert.doesNotReject(
    dialog.getByText(expectedVersion, { exact: true }).waitFor({
      state: 'visible',
      timeout: 60_000,
    }),
  )

  const installButtons = dialog.locator('button:visible:enabled')
  const buttonCount = await installButtons.count()
  assert.ok(
    buttonCount >= 3,
    'the signed update dialog must expose its actions',
  )
  await installButtons.nth(buttonCount - 1).click()

  await page.waitForEvent('close', { timeout: 180_000 })
  handedOff = true
  await waitForInstalledVersion()
} catch (error) {
  mkdirSync(artifactDirectory, { recursive: true })
  if (page && !page.isClosed()) {
    try {
      await page.screenshot({
        path: path.join(artifactDirectory, 'public-updater-failure.png'),
        fullPage: true,
      })
    } catch {
      // The WebView may disappear while the updater takes over.
    }
  }
  throw error
} finally {
  if (browser) {
    try {
      await browser.close()
    } catch {
      // The successful updater hand-off closes the CDP connection itself.
    }
  }
  if (!handedOff) terminateProcessTree(applicationProcess?.pid)
}

console.log(
  `Independent WebView2 updater UI handed off v${expectedVersion} successfully.`,
)

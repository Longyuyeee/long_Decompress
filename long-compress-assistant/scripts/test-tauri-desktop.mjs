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
const webviewUserDataDirectory = path.join(e2eDataDirectory, 'webview2')
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
const requireFullFormatMatrix =
  process.argv.includes('--require-full-format-matrix') ||
  process.env.LONG_DECOMPRESS_REQUIRE_FULL_FORMAT_MATRIX === '1'
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
        LONG_DECOMPRESS_E2E_INSTANCE_ID: e2eInstanceId,
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
  for (const format of ['iso9660', 'cpio']) {
    const extension = format === 'iso9660' ? 'iso' : format
    const archive = path.join(fixtureDirectory, `extract-only.${extension}`)
    runFixtureCommand(
      'tar.exe',
      ['-cf', archive, '--format', format, path.basename(extractOnlySource)],
      format.toUpperCase(),
    )
    extractOnlyMatrix.push([extension, archive, 'extract-only-payload.txt'])
  }
  const xarArchive = path.join(fixtureDirectory, 'extract-only.xar')
  createXarFixture(xarArchive, 'extract-only-payload.txt', extractOnlyPayload)
  extractOnlyMatrix.push(['xar', xarArchive, 'extract-only-payload.txt'])
  const wslExtProbe = spawnSync(
    'wsl.exe',
    ['-d', 'Ubuntu', '--', 'test', '-x', '/sbin/mkfs.ext4'],
    { encoding: 'utf8', timeout: 30_000, windowsHide: true },
  )
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
      extractOnlyMatrix.push([`ext${version}`, archive, 'extract-only-payload.txt'])
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
      extractOnlyMatrix.push([extension, image, 'extract-only-payload.txt'])
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
    extractOnlyMatrix.push(['fat', fatImage, 'extract-only-payload.txt'])

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
    extractOnlyMatrix.push(['ntfs', ntfsImage, 'extract-only-payload.txt'])
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
    extractOnlyMatrix.push(['squashfs', squashFsArchive, 'extract-only-payload.txt'])
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
    extractOnlyMatrix.push(['apfs', apfsArchive, 'extract-only-payload.txt'])
    console.log('[desktop-e2e] generated a real APFS image with a known payload')
  } else {
    recordMissingFullFormatCapability('APFS generator', 'npm run test:tools:apfs')
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

    extractOnlyMatrix.push([
      'msi',
      path.join(installerFixtureRoot, 'product-v1.msi'),
      'PayloadFile',
    ])
    extractOnlyMatrix.push([
      'msm',
      path.join(installerFixtureRoot, 'fixture.msm'),
      'PayloadFile.719C727A_2D5C_4ED6_A487_F2BEA6D8094F',
    ])
    extractOnlyMatrix.push([
      'msp',
      path.join(installerFixtureRoot, 'fixture.msp'),
      'PayloadFile',
      updatedInstallerPayload,
    ])
    console.log('[desktop-e2e] generated real MSI, MSM and MSP containers with known payloads')
  } else {
    recordMissingFullFormatCapability('MSI/MSM/MSP generators', 'npm run test:tools:wix3')
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
  const debOutput = path.join(fixtureDirectory, 'extract-only-deb-output')
  await callDesktopBridge('extractArchive', debArchive, debOutput)
  const debPayloadOutput = path.join(fixtureDirectory, 'extract-only-deb-payload-output')
  await callDesktopBridge('extractArchive', path.join(debOutput, 'data.tar'), debPayloadOutput)
  assert.deepEqual(
    readFileSync(path.join(debPayloadOutput, 'extract-only-payload.txt')),
    extractOnlyPayload,
    'DEB data archive must reproduce the package payload byte-for-byte',
  )
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
  }
  await callDesktopBridge('clearTasks')
  assertFullFormatMatrixReady()

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

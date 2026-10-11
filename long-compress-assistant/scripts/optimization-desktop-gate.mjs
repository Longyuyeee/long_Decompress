import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, copyFileSync } from 'node:fs'
import { randomBytes, createHash } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runSavedLocationDesktopGate({ driver, waitForElement, fixtureDirectory, artifactDirectory }) {
  const folder = path.join(fixtureDirectory, '我的常用目录')
  mkdirSync(folder)
  await (await waitForElement('[data-testid="nav-ArchiveBrowser"]')).click()
  const left = await waitForElement('[data-testid="file-manager-path-input-left"]')
  await left.sendKeys(Key.chord(Key.CONTROL, 'a'), folder, Key.ENTER)
  const bookmark = await waitForElement('[data-testid="bookmark-current-left"]')
  await driver.wait(async () => (await left.getAttribute('value')) === folder, 10_000)
  await bookmark.click()
  await driver.wait(async () => await bookmark.getAttribute('aria-pressed') === 'true', 10_000)
  const right = await waitForElement('[data-testid="file-manager-path-input-right"]')
  await right.click()
  const saved = await driver.wait(async () => {
    const locations = await driver.findElements(By.css('[data-testid="saved-location-right"]'))
    for (const item of locations) if ((await item.getText()).includes('我的常用目录')) return item
    return false
  }, 10_000)
  await saved.click()
  await driver.wait(async () => await right.getAttribute('value') === folder, 10_000)
  await driver.navigate().refresh()
  await (await waitForElement('[data-testid="file-manager-path-input-left"]')).click()
  const savedAfterReload = await driver.wait(async () => {
    const locations = await driver.findElements(By.css('[data-testid="saved-location-left"]'))
    for (const item of locations) if ((await item.getText()).includes('我的常用目录')) return item
    return false
  }, 10_000)
  await savedAfterReload.click()
  assert.equal(await (await waitForElement('[data-testid="file-manager-path-input-left"]')).getAttribute('value'), folder)
  writeFileSync(path.join(artifactDirectory, 'saved-location-native-result.json'), JSON.stringify({
    verifiedAt: new Date().toISOString(), realDirectory: true, crossPane: true, webviewReload: true,
  }, null, 2))
}

// Real UI, commands, ZIP engine and SQLite. Only the initial historical record
// and OS picker response are supplied as fixtures; no execution is mocked.
export async function runOptimizationDesktopGate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, binarySha256 }) {
  const clickVisible = async element => {
    await driver.executeScript('arguments[0].scrollIntoView({block:"center",behavior:"instant"})', element)
    await driver.wait(() => driver.executeScript(el => {
      const r = el.getBoundingClientRect()
      return el.contains(document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2))
    }, element), 10_000, 'wait until the control is reachable after the drawer animation')
    await element.click()
  }
  const root = path.join(fixtureDirectory, 'optimization-user')
  const sources = path.join(root, 'sources')
  const output = path.join(root, 'archives')
  mkdirSync(sources, { recursive: true })
  mkdirSync(output)
  const files = ['照片数据.bin', '视频数据.bin'].map(name => path.join(sources, name))
  files.forEach(file => writeFileSync(file, randomBytes(8 * 1024 * 1024)))
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const original = {
    id: 'optimization-prior', name: '上次打包', taskType: 'compression', workloadKind: 'archive', status: 'failed',
    sourcePaths: files, outputPath: path.join(output, '重试打包.zip'), completedAt: new Date().toISOString(),
    durationMs: 1, processedBytes: 0, totalBytes: 0, logs: [], errorMessage: '上次输出目录不可用',
    executionConfig: { schemaVersion: 1, kind: 'archive-compression', outputDirectory: output, passwordRequired: false,
      settings: { format: 'zip', level: 6, filename: '重试打包', splitArchive: false, splitSize: '100', keepStructure: true,
        verifyAfter: true, createSolidArchive: false, fastPack: true } },
  }
  await bridge('seedHistoryRecords', [original])
  await (await waitForElement('[data-testid="nav-History"]')).click()
  await (await waitForElement('[data-testid="history-compression-retry"]')).click()
  await waitForElement('[data-testid="history-compression-draft"]')
  assert.equal((await driver.findElements(By.css('[data-testid="compression-fast-pack"]'))).length, 0, 'Removed fast-pack option must not return for legacy history')
  await clickVisible(await waitForElement('[data-testid="compression-retry-create"]'))
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="history-compression-draft"] a'))).length > 0, 15_000)
  assert.deepEqual(readdirSync(output), [], 'draft creation must not execute')
  const draftLink = await driver.findElement(By.css('[data-testid="history-compression-draft"] a'))
  await clickVisible(draftLink)
  await (await waitForElement('[data-testid="start-compression"]')).click()
  const compression = await driver.wait(async () => (await bridge('taskHistory')).find(row => row.retryOfTaskId === original.id && row.status === 'completed'), 60_000)
  assert.equal(compression.executionConfig.settings.fastPack, false, 'Legacy fastPack=true must not be enabled by retry')
  assert.deepEqual(compression.sourcePaths, files)
  const archive = compression.outputPath
  const savedOriginal = (await bridge('taskHistory')).find(row => row.id === original.id)
  assert.equal(savedOriginal.status, 'failed')

  await (await waitForElement('[data-testid="nav-Settings"]')).click()
  const parallel = await waitForElement('[data-testid="parallel-zip-switch"]')
  assert.equal(await parallel.getAttribute('aria-checked'), 'false')
  const parallelEnabled = process.env.LONG_OPTIMIZATION_PARALLEL !== '0'
  if (parallelEnabled) await parallel.click()
  assert.equal(await parallel.getAttribute('aria-checked'), String(parallelEnabled))
  const smart = await waitForElement('[data-testid="smart-directory-switch"]')
  assert.equal(await smart.getAttribute('aria-checked'), 'false')
  await (await waitForElement('[data-testid="nav-Decompress"]')).click()
  await bridge('queueDesktopDialogSelections', [[archive]])
  await (await waitForElement('[data-testid="dropzone-file"]')).click()
  const subfolder = await waitForElement('[data-testid="global-subfolder"]')
  if ((await subfolder.findElements(By.css('.pi-check'))).length) await subfolder.click()
  const target = path.join(output, path.basename(files[0]))
  writeFileSync(target, '我已有的旧文件')
  await (await driver.findElement(By.xpath('//header[@data-testid="decompression-header"]//button[contains(., "开始解压队列")]'))).click()
  await waitForElement('[data-testid="conflict-overwrite"]')
  writeFileSync(target, '我在弹窗打开以后又保存了一次，这次更改不能被旧选择覆盖。')
  const changedHash = hash(target)
  await (await waitForElement('[data-testid="conflict-overwrite"]')).click()
  await driver.wait(async () => {
    const dialogs = await driver.findElements(By.css('[role="dialog"]'))
    return dialogs.length && (await dialogs[0].getText()).includes('自动重命名保留两者')
      && (await driver.findElements(By.css('[data-testid="conflict-overwrite"]:not([disabled])'))).length
  }, 15_000)
  assert.equal(hash(target), changedHash, 'stale overwrite must not publish')
  await (await waitForElement('[data-testid="conflict-apply-remaining"]')).click()
  await (await waitForElement('[data-testid="conflict-rename"]')).click()
  const extraction = await driver.wait(async () => (await bridge('taskHistory')).find(row => row.taskType === 'decompression' && row.sourcePaths.includes(archive) && row.status === 'completed'), 60_000)
  assert.equal(extraction.logs.some(log => log.message.includes('ZIP 并行解压')), parallelEnabled, 'real engine must follow the setting')
  assert.equal(hash(target), changedHash)
  assert.equal(hash(path.join(output, '照片数据 (1).bin')), hash(files[0]))
  assert.equal(hash(path.join(output, path.basename(files[1]))), hash(files[1]))
  assert.deepEqual((await bridge('taskHistory')).find(row => row.id === original.id), savedOriginal)
  // Smart directories apply only after opting in. A fresh multi-root archive
  // must preview and create exactly one archive-named output directory.
  await (await waitForElement('[data-testid="nav-Settings"]')).click()
  await (await waitForElement('[data-testid="smart-directory-switch"]')).click()
  const smartRoot = path.join(root, 'smart-output')
  mkdirSync(smartRoot)
  const smartArchive = path.join(smartRoot, '多个项目.zip')
  copyFileSync(archive, smartArchive)
  await (await waitForElement('[data-testid="nav-Decompress"]')).click()
  await bridge('queueDesktopDialogSelections', [[smartArchive]])
  await (await waitForElement('[data-testid="dropzone-file"]')).click()
  const smartRow = await driver.wait(async () => {
    const rows = await driver.findElements(By.css('[data-testid="task-row"]'))
    for (const row of rows) if ((await row.getText()).includes('多个项目.zip')) return row
    return false
  }, 10_000)
  await smartRow.click()
  const planned = path.join(smartRoot, '多个项目')
  await driver.wait(async () => {
    const previews = await driver.findElements(By.css('[data-testid="extraction-output-path"]'))
    for (const preview of previews) if (await preview.getAttribute('title') === planned) return true
    return false
  }, 15_000, 'smart directory preview must resolve before starting')
  await (await driver.findElement(By.xpath('//header[@data-testid="decompression-header"]//button[contains(., "开始解压队列")]'))).click()
  await driver.wait(async () => (await bridge('taskHistory')).find(row => row.sourcePaths.includes(smartArchive) && row.status === 'completed'), 60_000)
  for (const source of files) assert.equal(hash(path.join(planned, path.basename(source))), hash(source))
  assert.equal(await (await waitForElement('[data-testid="decompression-detail-progress"]')).getText(), '100%', 'Completed detail must settle immediately, without waiting for numeric animation')
  writeFileSync(path.join(artifactDirectory, 'optimization-user-result.json'), JSON.stringify({
    verifiedAt: new Date().toISOString(), binarySha256, testKind: 'real-ui-engine-sqlite-with-seeded-prior-record',
    compressionId: compression.id, extractionId: extraction.id, fastPack: false, legacyFastPackDisabled: true, parallelOptIn: parallelEnabled, smartOptInPreviewMatchesOutput: true,
    draftDidNotExecute: true, modifiedTargetPreserved: true, outputHashesMatch: true,
    nativePickerVerified: false,
  }, null, 2))
  writeFileSync(path.join(artifactDirectory, 'optimization-user.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
}

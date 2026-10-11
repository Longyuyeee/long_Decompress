import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync } from 'node:fs'
import { randomBytes } from 'node:crypto'
import path from 'node:path'
import { By } from 'selenium-webdriver'
import { spawnSync } from 'node:child_process'

export async function runPasswordSplitCreationGate({ driver, callDesktopBridge, waitForElement, fixtureDirectory, artifactDirectory, productionIpc, format = 'zip', password = '中文密码 + !测试', solid = false, bundledSevenZip }) {
  if (format !== 'zip') {
    fixtureDirectory = path.join(fixtureDirectory, format)
    artifactDirectory = path.join(artifactDirectory, format)
    mkdirSync(fixtureDirectory, { recursive: true })
    mkdirSync(artifactDirectory, { recursive: true })
  }
  if (format === '7z') {
    const scenario = `${password ? 'password' : 'plain'}-${solid ? 'solid' : 'non-solid'}`
    fixtureDirectory = path.join(fixtureDirectory, scenario)
    artifactDirectory = path.join(artifactDirectory, scenario)
    mkdirSync(fixtureDirectory, { recursive: true })
    mkdirSync(artifactDirectory, { recursive: true })
  }
  const folder = path.join(fixtureDirectory, '真实密码分卷目录')
  mkdirSync(path.join(folder, '空目录'), { recursive: true })
  const payload = randomBytes(2 * 1024 * 1024)
  writeFileSync(path.join(folder, '数据.bin'), payload)
  writeFileSync(path.join(folder, '说明.txt'), '中文内容与空格密码')
  const output = path.join(fixtureDirectory, '创建分卷输出')
  mkdirSync(output)
  if (!productionIpc) {
    await callDesktopBridge('clearCompressionWorkspace')
    await callDesktopBridge('clearTasks')
  }
  await (await waitForElement('[data-testid="nav-Compress"]')).click()
  await waitForElement('[data-testid="compression-header"]')
  if (productionIpc) {
    await productionIpc('tauri', { __tauriModule: 'Event', message: { cmd: 'emit', event: 'tauri://file-drop', windowLabel: null, payload: [folder] } })
  } else {
    await callDesktopBridge('queueDesktopDialogSelections', [[folder]])
    await (await waitForElement('[data-testid="dropzone-folder"]')).click()
  }
  await waitForElement('[data-testid="compression-draft-row"]')
  await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
  if (format === '7z') {
    await (await waitForElement('[data-testid="compression-format-select"]')).click()
    await (await driver.findElement(By.xpath('//button[@role="option" and normalize-space(.)="7Z"]'))).click()
  }
  const fill = async (id, value) => {
    const input = await waitForElement(`[data-testid="${id}"]`)
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', input)
    await input.clear()
    if (value) await input.sendKeys(value)
    return input
  }
  await fill('compression-password-input', password)
  if (format === '7z') {
    const input = await waitForElement('[data-testid="compression-solid-toggle"]')
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', input)
    if ((await input.isSelected()) !== solid) await input.click()
    writeFileSync(path.join(artifactDirectory, 'solid-settings.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  const split = await waitForElement('[data-testid="compression-split-toggle"]')
  assert.equal(await split.isEnabled(), true)
  await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', split)
  if (!(await split.isSelected())) await split.click()
  await fill('compression-split-size', '1')
  await fill('compression-output-path', output)
  for (const [id, checked] of [['compression-keep-structure', true], ['compression-delete-after', false], ['compression-verify-after', true]]) {
    const input = await waitForElement(`[data-testid="${id}"]`)
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', input)
    if ((await input.isSelected()) !== checked) await input.click()
  }
  await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', split)
  writeFileSync(path.join(artifactDirectory, 'password-split-create-settings.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  await (await waitForElement('[data-testid="save-global-compression-settings"]')).click()
  await (await waitForElement('[data-testid="start-compression"]')).click()
  const readHistory = async () => productionIpc
    ? (await productionIpc('list_task_history_page', { limit: 100, query: { text: folder } })).records
    : callDesktopBridge('taskHistory')
  const task = await driver.wait(async () => {
    if (productionIpc) return (await readHistory()).find(row => row.sourcePaths.includes(folder)) || false
    const rows = await callDesktopBridge('compressionVerificationTaskState')
    return rows.find(row => ['completed', 'failed', 'cancelled'].includes(row.status)) || false
  }, 90_000)
  assert.equal(task.status, 'completed', JSON.stringify(task))
  const summaryValue = await waitForElement('[data-testid="summary-progress-value"]')
  assert.equal(await summaryValue.getText(), '100%', 'Completed UI progress must settle without an animation delay')
  assert.equal(productionIpc ? task.executionConfig?.settings?.verifyAfter : task.verifyAfter, true)
  const parts = readdirSync(output).filter(name => new RegExp(`\\.${format}\\.\\d+$`).test(name)).sort().map(name => path.join(output, name))
  assert.equal(parts.length, 3, JSON.stringify(readdirSync(output)))
  const snapshots = parts.map(file => readFileSync(file))
  assert.ok(snapshots.every(bytes => bytes.length <= 1024 * 1024))
  assert.equal(snapshots[0].length, 1024 * 1024)
  assert.equal(snapshots[1].length, 1024 * 1024)
  if (format === '7z') {
    assert.ok(bundledSevenZip, '7Z matrix needs an independent archive metadata check')
    const listing = spawnSync(bundledSevenZip, ['l', '-slt', ...(password ? [`-p${password}`] : []), parts[0]], { encoding: 'utf8', timeout: 30_000 })
    assert.equal(listing.status, 0, listing.stderr || listing.stdout)
    assert.match(listing.stdout, solid ? /Solid = \+/ : /Solid = -/)
    assert.match(listing.stdout, password ? /Encrypted = \+/ : /Encrypted = -/)
  }
  const extracted = path.join(fixtureDirectory, '创建后解压')
  if (productionIpc) {
    await productionIpc('extract_file', { taskId: `installed-create-roundtrip-${Date.now()}`, filePath: parts[0], outputPath: extracted, password,
      options: { preserve_paths: true, create_subdirectory: false, delete_after: false, conflict_policy: 'rename',
        overwrite_existing: false, preserve_timestamps: true, skip_corrupted: false, extract_only_newer: false, file_filter: null } })
  } else {
    await callDesktopBridge('extractArchive', parts[0], extracted, password)
  }
  const restored = path.join(extracted, path.basename(folder))
  assert.deepEqual(readFileSync(path.join(restored, '数据.bin')), payload)
  assert.equal(readFileSync(path.join(restored, '说明.txt'), 'utf8'), '中文内容与空格密码')
  assert.deepEqual(readdirSync(path.join(restored, '空目录')), [])
  assert.deepEqual(readFileSync(path.join(folder, '数据.bin')), payload)
  parts.forEach((file, index) => assert.deepEqual(readFileSync(file), snapshots[index]))
  assert.equal(await summaryValue.getText(), '100%', 'Late progress must not overwrite completed UI progress')
  writeFileSync(path.join(artifactDirectory, 'password-split-create-result.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  const history = await readHistory()
  assert.ok(history.some(row => row.status === 'completed' && row.sourcePaths.includes(folder)))
  writeFileSync(path.join(artifactDirectory, 'password-split-create.json'), JSON.stringify({
    format, encrypted: Boolean(password), solid, volumeCount: parts.length, task, buildKind: productionIpc ? 'installed-production' : 'isolated-desktop-e2e', nativePickerVerified: false, extractionViaProductionCommand: true,
    checks: ['UI password and split settings', ...(format === '7z' ? ['UI solid toggle', 'independent 7-Zip solid/encryption metadata'] : []), 'UI start compression', 'real directory and multiple files', '1 MiB numeric volumes', 'verified publication', 'extracted payload and empty directory', 'source and volumes preserved', 'completed compression history'],
  }, null, 2))
}

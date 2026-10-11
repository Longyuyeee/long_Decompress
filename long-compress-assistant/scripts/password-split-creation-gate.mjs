import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { randomBytes } from 'node:crypto'
import path from 'node:path'
import { By } from 'selenium-webdriver'
import { spawnSync } from 'node:child_process'

export async function runPasswordSplitCreationGate({ driver, callDesktopBridge, waitForElement, fixtureDirectory, artifactDirectory, productionIpc, format = 'zip', password = '中文密码 + !测试', solid = false, bundledSevenZip, multipleSources = false, fault = null }) {
  assert.ok(!fault || (!productionIpc && format === '7z' && !multipleSources && ['cancel', 'output-file'].includes(fault)))
  assert.ok(!multipleSources || (!productionIpc && format === '7z'), 'Multi-source gate currently uses the isolated 7Z desktop workflow')
  if (format !== 'zip') {
    fixtureDirectory = path.join(fixtureDirectory, format)
    artifactDirectory = path.join(artifactDirectory, format)
    mkdirSync(fixtureDirectory, { recursive: true })
    mkdirSync(artifactDirectory, { recursive: true })
  }
  if (format === '7z') {
    const scenario = `${password ? 'password' : 'plain'}-${solid ? 'solid' : 'non-solid'}${multipleSources ? '-multi-source' : ''}${fault ? `-${fault}` : ''}`
    fixtureDirectory = path.join(fixtureDirectory, scenario)
    artifactDirectory = path.join(artifactDirectory, scenario)
    mkdirSync(fixtureDirectory, { recursive: true })
    mkdirSync(artifactDirectory, { recursive: true })
  }
  const folder = path.join(fixtureDirectory, '真实密码分卷目录')
  mkdirSync(path.join(folder, '空目录'), { recursive: true })
  const payload = randomBytes((fault === 'cancel' ? 96 : 2) * 1024 * 1024)
  writeFileSync(path.join(folder, '数据.bin'), payload)
  writeFileSync(path.join(folder, '说明.txt'), '中文内容与空格密码')
  const extraSource = path.join(fixtureDirectory, '独立源文件.txt')
  const extraContent = '这个文件不在源目录中，应与目录一起打入同一个分卷归档。'
  if (multipleSources) writeFileSync(extraSource, extraContent)
  const output = path.join(fixtureDirectory, '创建分卷输出')
  mkdirSync(output)
  const blockedPath = path.join(output, '已有文件不是目录.txt')
  const existingContent = '用户原有内容，失败后不能覆盖或删除'
  if (fault === 'output-file') writeFileSync(blockedPath, existingContent)
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
  if (multipleSources) {
    await callDesktopBridge('queueDesktopDialogSelections', [[extraSource]])
    await (await waitForElement('[data-testid="dropzone-file"]')).click()
    await driver.wait(async () => (await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))).length === 2, 10_000)
    for (const checkbox of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))) await checkbox.click()
    await (await waitForElement('[data-testid="compression-grouping-actions"] button')).click()
    await waitForElement('[data-testid="compression-group-row"]')
    assert.equal((await driver.findElements(By.css('[data-testid="compression-draft-row"]'))).length, 0, 'Both sources must be grouped into one job')
    writeFileSync(path.join(artifactDirectory, 'multi-source-group.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
  if (format === '7z') {
    await (await waitForElement('[role="dialog"] [data-testid="compression-format-select"]')).click()
    await (await driver.findElement(By.xpath('//button[@role="option" and normalize-space(.)="7Z"]'))).click()
  }
  const fill = async (id, value) => {
    const input = await waitForElement(`[role="dialog"] [data-testid="${id}"]`)
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', input)
    await input.clear()
    if (value) await input.sendKeys(value)
    return input
  }
  await fill('compression-password-input', password)
  if (format === '7z') {
    const input = await waitForElement('[role="dialog"] [data-testid="compression-solid-toggle"]')
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', input)
    if ((await input.isSelected()) !== solid) await input.click()
    writeFileSync(path.join(artifactDirectory, 'solid-settings.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  const split = await waitForElement('[role="dialog"] [data-testid="compression-split-toggle"]')
  assert.equal(await split.isEnabled(), true)
  await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', split)
  if (!(await split.isSelected())) await split.click()
  await fill('compression-split-size', '1')
  await fill('compression-output-path', fault === 'output-file' ? blockedPath : output)
  for (const [id, checked] of [['compression-keep-structure', true], ['compression-delete-after', false], ['compression-verify-after', true]]) {
    const input = await waitForElement(`[role="dialog"] [data-testid="${id}"]`)
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', input)
    if ((await input.isSelected()) !== checked) await input.click()
  }
  await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', split)
  writeFileSync(path.join(artifactDirectory, 'password-split-create-settings.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  await (await waitForElement('[data-testid="save-global-compression-settings"]')).click()
  await (await waitForElement('[data-testid="start-compression"]')).click()
  if (fault === 'cancel') {
    await driver.wait(() => readdirSync(output).some(name => {
      try { return statSync(path.join(output, name)).size > 0 } catch { return false }
    }), 30_000, 'Must observe real archive bytes before cancelling, not cancel a pending task')
    const state = await callDesktopBridge('compressionVerificationTaskState')
    assert.ok(state.some(row => ['running', 'compressing'].includes(row.status)), JSON.stringify(state))
    await (await waitForElement('[data-testid="compression-job-cancel"]')).click()
  }
  const readHistory = async () => productionIpc
    ? (await productionIpc('list_task_history_page', { limit: 100, query: { text: folder } })).records
    : callDesktopBridge('taskHistory')
  const task = await driver.wait(async () => {
    if (productionIpc) return (await readHistory()).find(row => row.sourcePaths.includes(folder)) || false
    const rows = await callDesktopBridge('compressionVerificationTaskState')
    return rows.find(row => ['completed', 'failed', 'cancelled'].includes(row.status)) || false
  }, 90_000)
  if (fault) {
    const expectedStatus = fault === 'cancel' ? 'cancelled' : 'failed'
    assert.equal(task.status, expectedStatus, JSON.stringify(task))
    const history = await driver.wait(async () => (await readHistory()).find(row => row.taskType === 'compression' && row.sourcePaths.includes(folder) && row.status === expectedStatus) || false, 30_000)
    if (fault === 'output-file') {
      assert.ok(task.error?.trim(), 'Failed task must expose a final reason')
      assert.match(task.error, /是文件，不是文件夹.*请选择其他输出目录/)
      assert.equal((task.error.match(/压缩失败[:：]/g) || []).length, 1, 'Exactly one compression failure prefix')
      assert.equal(history.errorMessage, task.error, 'History must retain the final failure reason')
      assert.equal(readFileSync(blockedPath, 'utf8'), existingContent)
    }
    const expectedFiles = fault === 'output-file' ? [path.basename(blockedPath)] : []
    await driver.wait(() => JSON.stringify(readdirSync(output).sort()) === JSON.stringify(expectedFiles), 30_000, 'No archive, numbered volumes or temporary output may remain')
    if (fault === 'cancel') {
      await driver.wait(async () => (await callDesktopBridge('compressionVerificationTaskState')).some(row => row.logs.includes('压缩已取消')), 10_000)
      const cancelled = (await callDesktopBridge('compressionVerificationTaskState'))[0]
      assert.ok(cancelled.logs.every(message => !message.startsWith('压缩失败')), JSON.stringify(cancelled))
      Object.assign(task, cancelled)
    }
    assert.deepEqual(readFileSync(path.join(folder, '数据.bin')), payload)
    assert.equal(readFileSync(path.join(folder, '说明.txt'), 'utf8'), '中文内容与空格密码')
    assert.deepEqual(readdirSync(path.join(folder, '空目录')), [])
    await (await waitForElement('[data-testid="compression-draft-row"]')).click()
    await waitForElement('[data-testid="compression-draft-details"]')
    await driver.wait(async () => (await (await waitForElement('[data-testid="compression-draft-row"]')).getText()).includes(fault === 'cancel' ? '取消' : '失败'), 10_000)
    await driver.wait(async () => driver.executeScript('return document.querySelector("[data-testid=compression-draft-details]").getAnimations({subtree:true}).every(a => a.playState !== "running")'), 10_000)
    writeFileSync(path.join(artifactDirectory, 'failure-or-cancel.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
    writeFileSync(path.join(artifactDirectory, 'failure-or-cancel.json'), JSON.stringify({ fault, task, history, outputFiles: readdirSync(output), sourcePreserved: true, buildKind: 'isolated-desktop-e2e', nativePickerVerified: false }, null, 2))
    return
  }
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
  if (multipleSources) {
    assert.equal(readFileSync(path.join(extracted, path.basename(extraSource)), 'utf8'), extraContent)
    assert.equal(readFileSync(extraSource, 'utf8'), extraContent)
    assert.deepEqual(readdirSync(extracted).sort(), [path.basename(folder), path.basename(extraSource)].sort())
  }
  parts.forEach((file, index) => assert.deepEqual(readFileSync(file), snapshots[index]))
  assert.equal(await summaryValue.getText(), '100%', 'Late progress must not overwrite completed UI progress')
  writeFileSync(path.join(artifactDirectory, 'password-split-create-result.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  const history = await readHistory()
  assert.ok(history.some(row => row.status === 'completed' && row.sourcePaths.includes(folder)))
  if (multipleSources) {
    const matching = history.filter(row => row.taskType === 'compression' && row.sourcePaths.includes(folder))
    assert.equal(matching.length, 1, 'One group must produce one compression history record')
    assert.equal(matching[0].status, 'completed')
    assert.deepEqual([...matching[0].sourcePaths].sort(), [folder, extraSource].sort())
  }
  writeFileSync(path.join(artifactDirectory, 'password-split-create.json'), JSON.stringify({
    format, encrypted: Boolean(password), solid, multipleSources, volumeCount: parts.length, task, buildKind: productionIpc ? 'installed-production' : 'isolated-desktop-e2e', nativePickerVerified: false, extractionViaProductionCommand: true,
    checks: ['UI password and split settings', ...(format === '7z' ? ['UI solid toggle', 'independent 7-Zip solid/encryption metadata'] : []), 'UI start compression', 'real directory and multiple files', '1 MiB numeric volumes', 'verified publication', 'extracted payload and empty directory', 'source and volumes preserved', 'completed compression history'],
  }, null, 2))
}

import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { randomBytes } from 'node:crypto'
import path from 'node:path'
import { By } from 'selenium-webdriver'
import { spawnSync } from 'node:child_process'

export async function runPasswordSplitCreationGate({ driver, callDesktopBridge, waitForElement, fixtureDirectory, artifactDirectory, productionIpc, format = 'zip', password = '中文密码 + !测试', solid = false, bundledSevenZip, multipleSources = false, fault = null, encryptFilenames = false }) {
  assert.ok(!encryptFilenames || (format === '7z' && password && !productionIpc))
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
    fixtureDirectory = path.join(fixtureDirectory, scenario + (encryptFilenames ? '-hidden-names' : ''))
    artifactDirectory = path.join(artifactDirectory, scenario + (encryptFilenames ? '-hidden-names' : ''))
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
    const names = await waitForElement('[role="dialog"] [data-testid="compression-encrypt-filenames"]')
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', names)
    if ((await names.isSelected()) !== encryptFilenames) await names.click()
    if (encryptFilenames) {
      const help = await waitForElement('[role="dialog"] button[aria-label="加密文件名说明"]')
      await help.click()
      assert.match(await (await waitForElement('[role="note"][aria-label="加密文件名说明"]')).getText(), /查看包内文件名也需要密码/)
      await driver.wait(() => driver.executeScript(element => element.closest('[role="dialog"]').getAnimations({ subtree: true }).every(animation => animation.playState !== 'running'), names), 10_000)
      assert.equal(await names.isSelected(), true, 'Help must not toggle filename encryption')
      writeFileSync(path.join(artifactDirectory, 'filename-encryption-help.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
      await help.click()
    }
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
  await driver.wait(async () => (await driver.findElements(By.css('[role="dialog"]'))).length === 0,
    10_000, 'Settings dialog must finish closing before starting compression')
  const clickGeometry = await driver.executeScript(() => {
    const buttons = ['start-compression', 'open-global-compression-settings'].map(id => {
      const element = document.querySelector(`[data-testid="${id}"]`)
      const rect = element.getBoundingClientRect()
      const hit = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2)
      return { id, rect: rect.toJSON(), centerHitsTarget: element.contains(hit), hitTestId: hit?.closest('[data-testid]')?.getAttribute('data-testid') }
    })
    return { viewport: { width: innerWidth, height: innerHeight, devicePixelRatio, scale: visualViewport?.scale }, buttons }
  })
  writeFileSync(path.join(artifactDirectory, 'start-click-geometry.json'), JSON.stringify(clickGeometry, null, 2))
  assert.ok(clickGeometry.buttons.every(button => button.centerHitsTarget), JSON.stringify(clickGeometry))
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
    if (encryptFilenames) {
      const wrong = spawnSync(bundledSevenZip, ['l', '-slt', '-pwrong-password', parts[0]], { encoding: 'utf8', timeout: 30_000 })
      assert.notEqual(wrong.status, 0, 'Independent listing must require the header password')
      assert.ok(!wrong.stdout.includes('说明.txt'), 'Wrong password must not expose filenames')
    }
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
  if (encryptFilenames) {
    const saved = history.find(row => row.status === 'completed' && row.sourcePaths.includes(folder))
    assert.equal(saved?.executionConfig?.settings?.encryptFilenames, true)
    assert.equal(saved?.executionConfig?.passwordRequired, true)
    assert.equal(saved?.executionConfig?.settings?.password, undefined)
    if (!solid) {
      await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
      const manage = await waitForElement('[role="dialog"] [data-testid="manage-compression-profiles"]')
      await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', manage)
      await manage.click()
      const editSelector = '//article[.//span[normalize-space(.)="7Z"]]//button[.//i[contains(@class,"pi-pencil")]]'
      const edit = await driver.wait(async () => (await driver.findElements(By.xpath(editSelector)))[0] || false, 10_000)
      await edit.click()
      let toggle = await waitForElement('[data-testid="profile-encrypt-filenames"]')
      await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', toggle)
      if (!(await toggle.isSelected())) await toggle.click()
      assert.equal(await (await waitForElement('[role="dialog"] form input[type="password"]')).getAttribute('placeholder'), '运行前需填写密码')
      const help = await waitForElement('button[aria-label="配置组文件名加密说明"]')
      await help.click()
      await waitForElement('[role="note"][aria-label="配置组文件名加密说明"]')
      await driver.wait(() => driver.executeScript(element => element.closest('[role="dialog"]').getAnimations({ subtree: true }).every(animation => animation.playState !== 'running'), toggle), 10_000)
      writeFileSync(path.join(artifactDirectory, 'profile-filename-encryption.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
      await help.click()
      await (await waitForElement('[role="dialog"] form button[type="submit"]')).click()
      await driver.wait(async () => (await driver.findElements(By.css('[data-testid="profile-encrypt-filenames"]'))).length === 0, 10_000)
      await (await driver.findElement(By.xpath(editSelector))).click()
      toggle = await waitForElement('[data-testid="profile-encrypt-filenames"]')
      assert.equal(await toggle.isSelected(), true, 'Saved profile must restore filename encryption')
      assert.equal(await (await waitForElement('[role="dialog"] form input[type="password"]')).getAttribute('placeholder'), '运行前需填写密码')
      const close = await toggle.findElement(By.xpath('ancestor::div[@role="dialog"]//button[@aria-label]'))
      await close.click()
      await driver.wait(async () => (await driver.findElements(By.css('[data-testid="profile-encrypt-filenames"]'))).length === 0, 10_000)
      const templatePath = path.join(fixtureDirectory, '隐藏名称.longtask.json')
      const reexportPath = path.join(fixtureDirectory, '再次导出.longtask.json')
      await callDesktopBridge('queueTaskTemplateDialogSelections', [templatePath, templatePath, reexportPath])
      const exportSelector = '[data-testid^="export-task-template-"]'
      const previousExportIds = new Set(await Promise.all((await driver.findElements(By.css(exportSelector))).map(button => button.getAttribute('data-testid'))))
      const sourceExport = await driver.findElement(By.xpath('//article[.//span[normalize-space(.)="7Z"]]//button[starts-with(@data-testid,"export-task-template-")]'))
      await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', sourceExport)
      await sourceExport.click()
      const readTemplate = async file => driver.wait(() => {
        try { return JSON.parse(readFileSync(file, 'utf8')) } catch { return false }
      }, 10_000)
      const exported = await readTemplate(templatePath)
      assert.equal(exported.compression.encryptFilenames, true)
      assert.equal(exported.passwordStrategy.mode, 'prompt_at_runtime')
      assert.ok(!readFileSync(templatePath, 'utf8').includes(password))
      const importButton = await waitForElement('[data-testid="import-task-template"]')
      await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', importButton)
      await importButton.click()
      const preview = await waitForElement('[data-testid="task-template-preview"]')
      await driver.wait(() => driver.executeScript(element => element.closest('[role="dialog"]').getAnimations({subtree:true}).every(animation => animation.playState !== 'running'), preview), 10_000)
      await driver.wait(() => preview.isDisplayed(), 10_000)
      assert.match(await preview.getText(), /执行时询问密码/)
      assert.match(await (await waitForElement('[data-testid="template-filename-encryption"]')).getText(), /文件名加密\s*开启/)
      writeFileSync(path.join(artifactDirectory, 'filename-template-preview.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
      await (await waitForElement('[data-testid="confirm-task-template-import"]')).click()
      await driver.wait(async () => (await driver.findElements(By.css('[data-testid="task-template-preview"]'))).length === 0, 10_000)
      const importedExport = await driver.wait(async () => {
        for (const button of await driver.findElements(By.css(exportSelector))) {
          if (!previousExportIds.has(await button.getAttribute('data-testid'))) return button
        }
        return false
      }, 10_000)
      await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', importedExport)
      await importedExport.click()
      const importedProfileId = (await importedExport.getAttribute('data-testid')).replace('export-task-template-', '')
      const reexported = await readTemplate(reexportPath)
      assert.equal(reexported.compression.encryptFilenames, true)
      assert.equal(reexported.passwordStrategy.mode, 'prompt_at_runtime')
      assert.equal(await callDesktopBridge('taskTemplateDialogQueueLength'), 0)
      writeFileSync(path.join(artifactDirectory, 'filename-template-roundtrip.json'), JSON.stringify({ exported, reexported, executionVerified: false }, null, 2))
      await (await waitForElement('[data-testid="close-compression-profiles"]')).click()
      await driver.wait(async () => (await driver.findElements(By.css('[data-testid="close-compression-profiles"]'))).length === 0, 10_000)
      await (await waitForElement('[data-testid="save-global-compression-settings"]')).click()
      await driver.wait(async () => (await driver.findElements(By.css('[role="dialog"]'))).length === 0, 10_000)
      // Retry the real completed record, not a fabricated history entry.
      const clickVisible = async element => {
        await driver.executeScript('arguments[0].scrollIntoView({block:"center",behavior:"instant"})', element)
        await driver.wait(() => driver.executeScript(el => {
          const r = el.getBoundingClientRect()
          return el.contains(document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2))
        }, element), 10_000)
        await element.click()
      }
      await (await waitForElement('[data-testid="nav-History"]')).click()
      const retryRow = await driver.wait(async () => {
        for (const row of await driver.findElements(By.css('[data-testid="history-record-row"]'))) {
          if ((await row.getText()).includes(saved.outputPath)) return row
        }
        return false
      }, 10_000)
      await clickVisible(await retryRow.findElement(By.css('[data-testid="history-compression-retry"]')))
      const draft = '[data-testid="history-compression-draft"]'
      assert.equal(await (await waitForElement(`${draft} [data-testid="compression-encrypt-filenames"]`)).isSelected(), true)
      const retryPassword = await waitForElement(`${draft} [data-testid="compression-password-input"]`)
      assert.equal(await retryPassword.getAttribute('value'), '', 'History must not restore a password')
      await clickVisible(await waitForElement('[data-testid="compression-retry-create"]'))
      assert.match(await (await waitForElement(`${draft} [role="alert"]`)).getText(), /重新填写本次密码/)
      const retryOutput = path.join(fixtureDirectory, '历史再次压缩')
      mkdirSync(retryOutput)
      const destination = await waitForElement('[data-testid="compression-retry-output"]')
      await destination.clear()
      await destination.sendKeys(retryOutput)
      await clickVisible(retryPassword)
      await retryPassword.sendKeys(password)
      await clickVisible(await waitForElement('[data-testid="compression-retry-create"]'))
      const link = await waitForElement(`${draft} a`)
      assert.deepEqual(readdirSync(retryOutput), [], 'Creating a draft must not execute it')
      writeFileSync(path.join(artifactDirectory, 'history-retry-draft.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
      await clickVisible(link)
      await (await waitForElement('[data-testid="start-compression"]')).click()
      const retried = await driver.wait(async () => (await readHistory()).find(row => row.retryOfTaskId === saved.id && row.status === 'completed'), 60_000)
      assert.equal(retried.executionConfig.settings.encryptFilenames, true)
      assert.equal(retried.executionConfig.passwordRequired, true)
      assert.equal(retried.executionConfig.settings.password, undefined)
      assert.deepEqual((await readHistory()).find(row => row.id === saved.id), saved, 'Original history must remain unchanged')
      const retryParts = readdirSync(retryOutput).filter(name => /\.7z\.\d+$/.test(name)).sort()
      assert.equal(retryParts.length, 3)
      const retryArchive = path.join(retryOutput, retryParts[0])
      const wrongRetry = spawnSync(bundledSevenZip, ['l', '-slt', '-pwrong-password', retryArchive], { encoding: 'utf8', timeout: 30_000 })
      assert.notEqual(wrongRetry.status, 0)
      assert.ok(!wrongRetry.stdout.includes('说明.txt'))
      const retryRestored = path.join(fixtureDirectory, '历史重试还原')
      await callDesktopBridge('extractArchive', retryArchive, retryRestored, password)
      assert.deepEqual(readFileSync(path.join(retryRestored, path.basename(folder), '数据.bin')), payload)
      assert.equal(readFileSync(path.join(retryRestored, path.basename(folder), '说明.txt'), 'utf8'), '中文内容与空格密码')
      assert.deepEqual(readdirSync(path.join(retryRestored, path.basename(folder), '空目录')), [])
      parts.forEach((file, index) => assert.deepEqual(readFileSync(file), snapshots[index]))
      writeFileSync(path.join(artifactDirectory, 'history-retry-result.json'), JSON.stringify({ originalId: saved.id, retryId: retried.id, passwordRequired: true, encryptFilenames: true, volumeCount: retryParts.length, payloadMatches: true, originalUnchanged: true }, null, 2))
      // Apply the imported profile through the actual template draft UI.
      await callDesktopBridge('clearCompressionWorkspace')
      await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
      await clickVisible(await waitForElement('[role="dialog"] [data-testid="manage-compression-profiles"]'))
      await callDesktopBridge('queueTaskTemplateDialogSelections', [[folder]])
      await clickVisible(await waitForElement(`[data-testid="create-template-draft-${importedProfileId}"]`))
      await waitForElement('[data-testid="template-draft-plan"]')
      await clickVisible(await waitForElement('[data-testid="confirm-template-draft"]'))
      await driver.wait(async () => (await driver.findElements(By.css('[role="dialog"]'))).length === 0, 10_000)
      const group = await waitForElement('[data-testid="compression-group-row"]')
      if (!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length) await group.click()
      const config = '[data-testid="compression-draft-config"]'
      const templatePassword = await waitForElement(`${config} [data-testid="compression-password-input"]`)
      assert.equal(await templatePassword.getAttribute('value'), '')
      assert.equal(await (await waitForElement(`${config} [data-testid="compression-encrypt-filenames"]`)).isSelected(), true)
      const templateOutput = path.join(fixtureDirectory, '导入模板输出')
      mkdirSync(templateOutput)
      const templateDestination = await waitForElement(`${config} [data-testid="compression-output-path"]`)
      await clickVisible(templateDestination)
      await templateDestination.clear()
      await templateDestination.sendKeys(templateOutput)
      const beforeTemplateHistory = await readHistory()
      await clickVisible(await waitForElement('[data-testid="start-compression"]'))
      await driver.wait(async () => (await driver.findElement(By.css('body')).getText()).includes('文件名加密仅支持 7Z，请先填写压缩密码'), 10_000)
      assert.deepEqual(readdirSync(templateOutput), [])
      assert.deepEqual(await readHistory(), beforeTemplateHistory, 'Missing template password must not create an execution')
      await clickVisible(templatePassword)
      await templatePassword.sendKeys(password)
      await clickVisible(await waitForElement('[data-testid="start-compression"]'))
      const templateRun = await driver.wait(async () => (await readHistory()).find(row => row.status === 'completed' && row.outputPath?.startsWith(templateOutput)), 60_000)
      assert.equal(templateRun.executionConfig.settings.encryptFilenames, true)
      assert.equal(templateRun.executionConfig.settings.password, undefined)
      assert.equal(templateRun.executionConfig.passwordRequired, true)
      assert.ok(!(await driver.findElement(By.css('body')).getText()).includes('文件名加密仅支持 7Z，请先填写压缩密码'), 'Resolved validation error must not remain beside successful output')
      const denied = spawnSync(bundledSevenZip, ['l', '-slt', '-pwrong-password', templateRun.outputPath], { encoding: 'utf8', timeout: 30_000 })
      assert.notEqual(denied.status, 0)
      assert.ok(!denied.stdout.includes('说明.txt'))
      const templateRestored = path.join(fixtureDirectory, '模板产物还原')
      await callDesktopBridge('extractArchive', templateRun.outputPath, templateRestored, password)
      assert.deepEqual(readFileSync(path.join(templateRestored, path.basename(folder), '数据.bin')), payload)
      assert.equal(readFileSync(path.join(templateRestored, path.basename(folder), '说明.txt'), 'utf8'), '中文内容与空格密码')
      assert.deepEqual(readdirSync(path.join(templateRestored, path.basename(folder), '空目录')), [])
      assert.deepEqual(readFileSync(path.join(folder, '数据.bin')), payload)
      writeFileSync(path.join(artifactDirectory, 'filename-template-execution.json'), JSON.stringify({ taskId: templateRun.id, importedProfileId, missingPasswordBlocked: true, encryptFilenames: true, payloadMatches: true, executionVerified: true }, null, 2))
      writeFileSync(path.join(artifactDirectory, 'filename-template-execution.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
    }
  }
  assert.ok(history.some(row => row.status === 'completed' && row.sourcePaths.includes(folder)))
  if (multipleSources) {
    const matching = history.filter(row => row.taskType === 'compression' && row.sourcePaths.includes(folder))
    assert.equal(matching.length, 1, 'One group must produce one compression history record')
    assert.equal(matching[0].status, 'completed')
    assert.deepEqual([...matching[0].sourcePaths].sort(), [folder, extraSource].sort())
  }
  writeFileSync(path.join(artifactDirectory, 'password-split-create.json'), JSON.stringify({
    format, encrypted: Boolean(password), encryptFilenames, solid, multipleSources, volumeCount: parts.length, task, buildKind: productionIpc ? 'installed-production' : 'isolated-desktop-e2e', nativePickerVerified: false, extractionViaProductionCommand: true,
    checks: ['UI password and split settings', ...(format === '7z' ? ['UI solid toggle', 'independent 7-Zip solid/encryption metadata'] : []), 'UI start compression', 'real directory and multiple files', '1 MiB numeric volumes', 'verified publication', 'extracted payload and empty directory', 'source and volumes preserved', 'completed compression history'],
  }, null, 2))
}

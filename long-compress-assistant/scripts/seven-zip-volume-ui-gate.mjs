import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { By } from 'selenium-webdriver'

export async function runSevenZipVolumeUiGate({ driver, callDesktopBridge, waitForElement, runFixtureCommand, bundledSevenZip, fixtureDirectory, artifactDirectory }) {
  const root = path.join(fixtureDirectory, '7z-ui')
  mkdirSync(root)
  const name = '用户资料.txt'
  const payload = '需要原样保留的中文文件\n'.repeat(100)
  writeFileSync(path.join(root, name), payload)
  const password = '页面密码 + !独立'
  const archive = path.join(root, '界面密码分卷.7z')
  runFixtureCommand(bundledSevenZip, ['a', '-t7z', '-y', '-mhe=on', `-p${password}`, '-v37b', archive, name], '7Z UI encrypted header', { cwd: root })
  const parts = readdirSync(root).filter(name => /\.7z\.\d+$/.test(name)).sort().map(name => path.join(root, name))
  assert.ok(parts.length > 2)
  const snapshots = parts.map(file => readFileSync(file))
  await callDesktopBridge('clearTasks')
  await (await waitForElement('[data-testid="nav-Decompress"]')).click()
  await waitForElement('[data-testid="decompression-header"]')
  // Native picker return is supplied; all following actions use the real page.
  await callDesktopBridge('queueDesktopDialogSelections', [[parts[1]]])
  await (await waitForElement('[data-testid="dropzone-file"]')).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="task-row"]'))).length === 1, 15_000)
  const recycle = await waitForElement('[data-testid="global-recycle-source-switch"]')
  if (await recycle.getAttribute('aria-checked') === 'true') await recycle.click()
  const start = () => driver.findElement(By.xpath('//header[@data-testid="decompression-header"]//button[contains(., "开始解压队列")]'))
  await (await start()).click()
  const original = await driver.wait(async () => (await callDesktopBridge('taskHistory')).find(record => record.sourcePaths.includes(parts[0])), 30_000)
  assert.equal(original.status, 'failed')
  const input = await waitForElement('.task-password-cell input')
  await input.sendKeys(password)
  assert.equal(await input.getAttribute('value'), password)
  await (await driver.findElement(By.css('.task-password-cell button:last-child'))).click()
  const panel = await waitForElement('[data-testid="failed-extraction-retry"]')
  const output = path.join(root, '用户输出')
  mkdirSync(output)
  writeFileSync(path.join(output, name), '已有内容不许覆盖')
  const outputInput = await panel.findElement(By.css('[data-testid="retry-output"]'))
  await outputInput.clear()
  await outputInput.sendKeys(output)
  const subfolder = await panel.findElement(By.css('[data-testid="retry-subfolder"]'))
  if (await subfolder.isSelected()) await subfolder.click()
  assert.equal(await (await panel.findElement(By.css('[data-testid="retry-password"]'))).getAttribute('value'), password)
  await (await waitForElement('[data-testid="create-retry-drafts"]')).click()
  await driver.wait(async () => (await panel.getText()).includes('已创建新任务'), 30_000)
  assert.deepEqual(readdirSync(output), [name])
  await (await driver.findElement(By.xpath('//button[contains(., "返回队列确认并启动")]'))).click()
  await (await start()).click()
  await waitForElement('[data-testid="conflict-rename"]')
  const layouts = []
  for (const size of [{ width: 920, height: 620 }, { width: 760, height: 520 }]) {
    await driver.manage().window().setRect(size)
    await driver.wait(async () => driver.executeScript(() => {
      const actions = document.querySelector('[data-testid="conflict-actions"]')
      return actions && actions.getAnimations({ subtree: true }).every(animation => animation.playState !== 'running')
    }), 10_000)
    const layout = await driver.executeScript(() => {
      const actions = ['rename', 'overwrite', 'skip'].map(action => {
        const button = document.querySelector(`[data-testid="conflict-${action}"]`)
        const r = button.getBoundingClientRect()
        return { action, visible: r.width > 0 && r.top >= 0 && r.bottom <= innerHeight && r.left >= 0 && r.right <= innerWidth,
          clickable: button.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)) }
      })
      return { actions, width: innerWidth, height: innerHeight }
    })
    assert.ok(layout.actions.every(action => action.visible && action.clickable), JSON.stringify(layout))
    layouts.push(layout)
    writeFileSync(path.join(artifactDirectory, `seven-zip-ui-conflict-${size.width}.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  await (await waitForElement('[data-testid="conflict-rename"]')).click()
  const retried = await driver.wait(async () => (await callDesktopBridge('taskHistory')).find(record => record.retryOfTaskId === original.id), 60_000)
  assert.equal(retried.status, 'completed', JSON.stringify(retried))
  assert.equal(readFileSync(path.join(output, name), 'utf8'), '已有内容不许覆盖')
  assert.equal(readFileSync(path.join(output, '用户资料 (1).txt'), 'utf8'), payload)
  parts.forEach((file, index) => assert.deepEqual(readFileSync(file), snapshots[index]))
  await driver.navigate().refresh()
  await waitForElement('[data-testid="nav-History"]')
  const reopened = await callDesktopBridge('taskHistory')
  assert.deepEqual(reopened.find(record => record.id === original.id), original)
  assert.deepEqual(reopened.find(record => record.id === retried.id), retried)
  await (await waitForElement('[data-testid="nav-History"]')).click()
  await (await waitForElement('[data-testid="history-search"]')).sendKeys('界面密码分卷.7z.001')
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="history-record-row"]'))).length === 2, 10_000)
  writeFileSync(path.join(artifactDirectory, 'seven-zip-ui-history.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  writeFileSync(path.join(artifactDirectory, 'seven-zip-volume-ui.json'), JSON.stringify({
    volumeCount: parts.length, importedNonFirstVolume: true, original, retried, layouts,
    nativePickerVerified: false, pageReloadVerified: true, processRestartVerified: false,
  }, null, 2))
  await driver.manage().window().setRect({ width: 920, height: 620 })
}

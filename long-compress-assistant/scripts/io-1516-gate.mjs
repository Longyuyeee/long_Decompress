import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, statSync, writeFileSync } from 'node:fs'
import { spawn } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runIo1516Gate({ driver, bridge, waitForElement, artifactDirectory, application, root }) {
  const base = path.join(root, 'test-results/io-1516'), manifest = JSON.parse(readFileSync(path.join(base, 'originals.json'), 'utf8'))
  const work = path.join(base, 'ui-' + randomUUID()); mkdirSync(work)
  const cases = [], screenshots = [], layouts = []
  const doneFaults = []
  const sha = p => createHash('sha256').update(readFileSync(p)).digest('hex')
  const binarySha256 = sha(application), checkpoint = path.join(base, 'desktop-progress.json')
  if (existsSync(checkpoint)) {
    const saved = JSON.parse(readFileSync(checkpoint, 'utf8'))
    if (saved.binarySha256 === binarySha256 && saved.screenshots.every(n => existsSync(path.join(artifactDirectory, n)))) {
      cases.push(...saved.cases); screenshots.push(...saved.screenshots); layouts.push(...saved.layouts); doneFaults.push(...saved.doneFaults)
    }
  }
  const save = () => writeFileSync(checkpoint, JSON.stringify({ binarySha256, cases, screenshots, layouts, doneFaults }, null, 2))
  const click = async selector => {
    for (const e of await driver.findElements(By.css('.toast-item button'))) { try { if (await e.isDisplayed()) await e.click() } catch {} }
    const el = typeof selector === 'string' ? await waitForElement(selector) : selector
    await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), el)
    await driver.wait(() => driver.executeScript(e => {
      e.scrollIntoView({ block: 'center', behavior: 'instant' })
      const r = e.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
      return !e.disabled && r.width > 0 && r.height > 0 && (hit === e || e.contains(hit))
    }, el), 10000); await el.click()
  }
  const fill = async (selector, value) => { const el = await waitForElement(selector); await el.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE); if (value) await el.sendKeys(value) }
  const sizes = async (name, anchor) => {
    for (const [width, height] of [[1100, 700], [760, 520]]) {
      await driver.manage().window().setRect({ width, height })
      await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), await waitForElement(anchor))
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
      const geometry = await driver.executeScript(() => ({ width: innerWidth, height: innerHeight, overflow: document.documentElement.scrollWidth - innerWidth }))
      assert.ok(geometry.overflow <= 1, JSON.stringify(geometry)); layouts.push({ name, ...geometry })
      const file = `${name}-${width}x${height}.png`; writeFileSync(path.join(artifactDirectory, file), Buffer.from(await driver.takeScreenshot(), 'base64')); screenshots.push(file)
    }
  }
  const history = id => driver.wait(async () => (await bridge('taskHistory')).find(h => h.id === id && ['completed', 'failed'].includes(h.status)) || false, 120000)
  async function importSource(source) {
    await driver.manage().window().setRect({ width: 1100, height: 700 })
    await bridge('clearTasks'); await click('[data-testid="nav-Decompress"]')
    await bridge('queueDesktopDialogSelections', [[source]]); await click('[data-testid="dropzone-file"]')
    const task = await driver.wait(async () => (await bridge('extractionRecoveryTasks'))[0] || false, 15000)
    await click(`[data-task-id="${task.id}"]`)
    await click('.task-output-actions button')
    const sub = await waitForElement('.task-subfolder-option')
    if (await sub.getAttribute('aria-checked') === 'true') await click(sub)
    await click(`[data-task-id="${task.id}"]`)
    return task
  }
  const staged = () => {
    for (const dir of readdirSync(work).filter(n => n.startsWith('.long-extract-'))) {
      const file = path.join(work, dir, 'payload.bin')
      if (existsSync(file)) return { file, bytes: statSync(file).size }
    }
    return null
  }
  const absent = target => assert.ok(!existsSync(target), 'Incomplete output must not be published: ' + target)
  await bridge('seedVaultPassword', 'IO 1516 isolated public test', manifest.password)
  async function retry(old, output, password, capture) {
    mkdirSync(output)
    await click('[data-testid="retry-batch-failures"]')
    await fill('[role="dialog"] [data-testid="retry-output"]', output)
    if (password) await fill('[role="dialog"] [data-testid="retry-password"]', password)
    const sub = await waitForElement('[role="dialog"] [data-testid="retry-subfolder"]'); if (await sub.isSelected()) await click(sub)
    if (capture) await sizes('io1516-retry-config', '[role="dialog"]')
    await click('[role="dialog"] [data-testid="create-retry-drafts"]')
    await driver.wait(async () => (await (await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'), 30000)
    assert.deepEqual(readdirSync(output), [])
    assert.deepEqual((await bridge('taskHistory')).find(h => h.id === old.id), old)
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(., "返回队列确认并启动")]')))
    const task = (await bridge('extractionRecoveryTasks')).find(t => t.retryOfTaskId === old.id)
    assert.ok(task && task.status === 'pending')
    await click(`[data-task-id="${old.id}"]`)
    await click('[data-testid="start-decompression-queue"]')
    const completed = await history(task.id); assert.equal(completed.status, 'completed', JSON.stringify(completed))
    assert.deepEqual((await bridge('taskHistory')).find(h => h.id === old.id), old)
    if (capture) { await click(`[data-task-id="${task.id}"]`); await sizes('io1516-retry-completed', `[data-task-id="${task.id}"]`) }
    return completed
  }
  for (const changed of [false, true]) {
    if (doneFaults.includes(changed)) { console.log('[io1516] reusing completed source fault', changed); continue }
    const folder = path.join(work, changed ? '原卷被替换' : '原卷被撤回'); mkdirSync(folder)
    for (const part of manifest.parts) copyFileSync(path.join(base, part.name), path.join(folder, part.name))
    const task = await importSource(path.join(folder, manifest.parts[0].name))
    await click('[data-testid="start-decompression-queue"]')
    await driver.wait(() => { const s = staged(); return s?.bytes > 1024 * 1024 && s.bytes < manifest.bytes ? s : false }, 90000, 'Wait for real written ZIP data before pause', 25)
    await click(`[data-testid="pause-archive-task-${task.id}"]`)
    await driver.wait(async () => await bridge('taskStatus', task.id) === 'paused', 10000, 'Wait for the real pause request to settle')
    await driver.sleep(250); const before = staged(); assert.ok(before && before.bytes < manifest.bytes)
    await driver.sleep(250); assert.equal(staged().bytes, before.bytes)
    const part = path.join(folder, manifest.parts.at(-2).name), backup = part + '.owned-original'
    assert.ok(part.startsWith(work + path.sep)); renameSync(part, backup)
    if (changed) writeFileSync(part, Buffer.from('a different volume is not the original archive'))
    await click(`[data-testid="resume-archive-task-${task.id}"]`)
    const failed = await history(task.id); assert.equal(failed.status, 'failed')
    assert.match(failed.errorMessage, changed ? /archive-source:Extracting:source-changed/ : /archive-source:Extracting:source-missing/)
    assert.equal(failed.failure?.category, changed ? 'source-changed' : 'source-missing')
    assert.ok(!failed.logs.some(l => /所有已知密码均无效/.test(l.message)))
    absent(path.join(folder, 'payload.bin')); assert.equal(staged(), null)
    await click(`[data-task-id="${task.id}"]`)
    await sizes(changed ? 'io1516-source-changed' : 'io1516-source-missing', '[data-testid="failed-extraction-retry"]')
    if (changed) {
      // Replace only our own injected file, then restore the original member.
      const { unlinkSync } = await import('node:fs'); unlinkSync(part)
    }
    renameSync(backup, part)
    for (const member of manifest.parts) assert.equal(sha(path.join(folder, member.name)), member.sha256)
    cases.push({ sourceFault: changed ? 'replaced-volume' : 'withdrawn-volume', actualWrittenBeforePause: before.bytes,
      pauseStoppedWriting: true, noPublishedOrStagedPartial: true, failure: failed.errorMessage, historyCategory: failed.failure?.category })
    if (!changed) {
      const output = path.join(work, '恢复原卷后的新任务')
      const completed = await retry(failed, output, manifest.password, true)
      assert.equal(sha(path.join(output, 'payload.bin')), manifest.sha256)
      cases.push({ restoredSourceNewTask: true, fullPayloadHash: manifest.sha256, previousFailureUnchanged: true, retryId: completed.id })
    }
    doneFaults.push(changed); save()
  }
  const folder = path.join(work, '目标被占用'); mkdirSync(folder)
  const small = path.join(folder, 'small.zip'); copyFileSync(manifest.small, small)
  const target = path.join(folder, 'payload.bin'); writeFileSync(target, 'original user result')
  const task = await importSource(small)
  const ready = path.join(work, 'lock-ready'), stop = path.join(work, 'lock-stop')
  const lock = spawn('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(root, 'scripts/hold-io-1516-file.ps1'), '-Path', target, '-Ready', ready, '-Stop', stop], { windowsHide: true, stdio: ['ignore', 'ignore', 'pipe'] })
  let lockError = ''; lock.stderr.on('data', b => { lockError += b.toString() })
  try {
    await driver.wait(() => {
      if (lock.exitCode !== null) throw new Error('Exclusive-file helper failed: ' + lockError)
      return existsSync(ready)
    }, 15000, 'Actual exclusive Windows file handle')
    await click('[data-testid="start-decompression-queue"]')
    await driver.wait(async () => (await driver.findElements(By.css('[data-testid="conflict-overwrite"]'))).length > 0 || await bridge('taskStatus', task.id) === 'failed', 30000)
    const conflict = await driver.findElements(By.css('[data-testid="conflict-overwrite"]')); if (conflict.length) await click(conflict[0])
    const failed = await history(task.id); assert.equal(failed.status, 'failed'); assert.match(failed.errorMessage, /archive-io:target-busy/)
    assert.equal(failed.failure?.category, 'target-busy')
    await click(`[data-task-id="${task.id}"]`); await sizes('io1516-target-busy', '[data-testid="failed-extraction-retry"]')
    cases.push({ actualExclusiveTargetFailure: true, historyCategory: failed.failure.category, originalError: failed.errorMessage })
    writeFileSync(stop, 'stop'); await new Promise(resolve => lock.once('close', resolve))
    assert.equal(readFileSync(target, 'utf8'), 'original user result')
    const output = path.join(work, '关闭占用后新位置')
    await retry(failed, output, '', false)
    assert.equal(sha(path.join(output, 'payload.bin')), manifest.smallPayloadSha256)
    assert.equal(readFileSync(target, 'utf8'), 'original user result')
    cases.push({ unlockedNewTask: true, newOutputFullHash: manifest.smallPayloadSha256, oldOutputAndFailurePreserved: true })
    await click('[data-testid="nav-History"]')
    const row = await driver.wait(async () => {
      for (const row of await driver.findElements(By.css('[data-testid="history-record-row"]'))) if ((await row.getText()).includes('small.zip') && (await row.getText()).includes('失败')) return row
      return false
    }, 15000)
    await click(row); assert.match(await (await waitForElement('[data-testid="history-detail"]')).getText(), /文件被占用/)
    await sizes('io1516-history-cause', '[data-testid="history-detail"]')
    cases.push({ actualHistoryIoGuidance: true })
  } finally { if (!existsSync(stop)) writeFileSync(stop, 'stop') }
  writeFileSync(path.join(artifactDirectory, 'io-1516-results.json'), JSON.stringify({ version: '1.5.16', completedAt: new Date().toISOString(),
    application, binarySha256: sha(application), work, cases, screenshots, layouts }, null, 2))
}

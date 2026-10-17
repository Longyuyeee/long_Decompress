import assert from 'node:assert/strict'
import { mkdirSync, copyFileSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

// Real controls -> production commands -> real archive bytes -> persisted history.
// The bridge supplies only file-dialog responses and read-only observations.
export async function runLegacyZipInput1416Gate({ driver, bridge, waitForElement: findElement, fixtureDirectory, artifactDirectory, root, application }) {
  const waitForElement = async selector => {
    try { return await findElement(selector) }
    catch (cause) { throw new Error(`UI element unavailable: ${selector}`, {cause}) }
  }
  const sourceRoot = path.join(root, 'tests/fixtures/archives/legacy-zip-password-1416')
  const work = path.join(fixtureDirectory, 'legacy-zip-input-1416'); mkdirSync(work)
  const evidence = path.join(artifactDirectory, 'legacy-zip-input-1416'); mkdirSync(evidence, { recursive: true })
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const payload = readFileSync(path.join(sourceRoot, 'payload.txt'))
  const screenshots = [], results = [], negativeCases = []
  const shot = async name => {
    await driver.wait(() => driver.executeScript(() => [...document.getAnimations()].filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 5000, 'finite UI animations must settle before visual evidence')
    const file = `${name}.png`; writeFileSync(path.join(evidence, file), Buffer.from(await driver.takeScreenshot(), 'base64')); screenshots.push(file)
  }
  const click = async selector => {
    const el = typeof selector === 'string' ? await waitForElement(selector) : selector
    let previous = '', stableSince = 0
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), el)
    await driver.wait(async () => {
      const state = await driver.executeScript(el => {
      el.scrollIntoView({ block: 'center', behavior: 'instant' })
      const r = el.getBoundingClientRect(); return { box:JSON.stringify(r.toJSON()), hit:!el.disabled && el.contains(document.elementFromPoint(r.x+r.width/2, r.y+r.height/2)) }
      }, el)
      if (!state.hit || previous !== state.box) { previous=state.box; stableSince=Date.now(); return false }
      return Date.now()-stableSince >= 300
    }, 10000, `control must be stable and reachable: ${typeof selector === 'string' ? selector : await el.getAttribute('data-testid')}`, 100)
    await el.click()
  }
  const fill = async (selector, value) => {
    const el = await waitForElement(selector); await el.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (value) await el.sendKeys(value)
  }
  const select = async (id, label) => {
    await click(`[data-testid="${id}"]`)
    console.log(`[legacy-zip-input-1416] select ${id}: expanded=${await (await waitForElement(`[data-testid="${id}"]`)).getAttribute('aria-expanded')}; options=${await driver.findElements(By.css('[role="option"]')).then(elements=>elements.length)}`)
    const option = await driver.wait(async () => {
      for (const el of await driver.findElements(By.css('[role="option"]'))) if ((await el.getText()).includes(label)) return el
      return false
    }, 10000, `encoding option must be visible: ${label}`)
    await click(option)
  }
  const tasks = () => bridge('extractionRecoveryTasks')
  const run = async id => {
    await click('[data-testid="start-decompression-queue"]')
    return driver.wait(async () => {
      const task = (await tasks()).find(task => task.id === id)
      return task && ['failed','completed','cancelled'].includes(task.status) ? task : false
    }, 60000)
  }
  const expand = async id => {
    for (const open of await driver.findElements(By.css('[data-testid^="toggle-archive-task-"][aria-expanded="true"]'))) {
      if (await open.getAttribute('data-testid') !== `toggle-archive-task-${id}`) await click(open)
    }
    const toggle = await waitForElement(`[data-testid="toggle-archive-task-${id}"]`)
    if (await toggle.getAttribute('aria-expanded') !== 'true') await click(toggle)
    await waitForElement('[data-testid="decompression-config-panel"]')
    await driver.wait(() => driver.executeScript(() => [...document.querySelectorAll('.details-drawer')].every(el => el.style.overflow !== 'hidden' && !el.getAnimations().some(a=>a.playState==='running'))), 10000, 'task drawer enter/leave transitions must finish')
  }
  for (const [code, password, label] of [
    ['cp932', '日本語パスワード+!1416', '日文（CP932）'],
    ['cp936', '简体密码测试+!1416', '简体中文（CP936）'],
    ['cp950', '繁體密碼測試+!1416', '繁体中文（CP950）'],
  ]) {
    const original = path.join(sourceRoot, `${code}.zip`), file = path.join(work, `${code}.zip`)
    copyFileSync(original, file); const before = hash(file)
    await bridge('queueDesktopDialogSelections', [[file]])
    await click('[data-testid="dropzone-file"]')
    const first = await driver.wait(async () => (await tasks()).find(t => t.source === file && t.status === 'pending'), 15000)
    await expand(first.id)
    await select('task-zip-password-encoding', label)
    if (code === 'cp932') {
      await click('[data-testid="task-select"]')
      for (const [width,height] of [[1100,700],[760,520]]) {
        await driver.manage().window().setRect({width,height})
        await click('.batch-config-bar button[aria-haspopup="dialog"]')
        await click('[data-testid="batch-zip-password-encoding"]')
        await shot(`batch-menu-${width}x${height}`)
        await driver.actions().sendKeys(Key.ESCAPE).perform()
        await select('batch-zip-password-encoding',label)
        await click('[data-testid="batch-config-apply"]')
        await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="batch-config-fields"]'))).length,10000)
      }
      await click('[data-testid="task-select"]')
      console.log('[legacy-zip-input-1416] batch checks complete')
      for (const [width, height] of [[1100,700], [760,520]]) {
        console.log(`[legacy-zip-input-1416] view input menu ${width}x${height}`)
        await driver.manage().window().setRect({ width, height })
        await click('[data-testid="task-zip-password-encoding"]')
        await driver.wait(async () => (await driver.findElements(By.css('[role="listbox"]'))).length > 0, 5000)
        const bounds = await driver.executeScript(() => {
          const s = document.querySelector('[data-testid="task-zip-password-encoding"]'), p = s.closest('[data-testid="decompression-config-panel"]'), m = document.querySelector('[role="listbox"]'); const r = m.getBoundingClientRect()
          return { panelWidth: p.clientWidth, panelScroll: p.scrollWidth, menu: r.toJSON(), width: innerWidth, height: innerHeight }
        })
        assert.ok(bounds.panelScroll <= bounds.panelWidth+1, JSON.stringify(bounds))
        assert.ok(bounds.menu.left >= 0 && bounds.menu.right <= bounds.width+1 && bounds.menu.top >= 0 && bounds.menu.bottom <= bounds.height+1, JSON.stringify(bounds))
        await shot(`input-menu-${width}x${height}`)
        if (width === 760 && process.env.LONG_ZIP1416_VIEW_PAUSE === '1') await new Promise(resolve => setTimeout(resolve,30000))
        await driver.actions().sendKeys(Key.ESCAPE).perform()
      }
      await driver.manage().window().setRect({ width:1100, height:700 })
    }
    assert.equal((await run(first.id)).status, 'failed', 'without a supplied password the original encrypted fixture must fail')
    await expand(first.id)
    await waitForElement('[data-testid="retry-zip-password-encoding"]')
    assert.ok((await (await waitForElement('[data-testid="retry-zip-password-encoding"]')).getText()).includes(label))
    const out = path.join(work, `${code}-output`); mkdirSync(out)
    await fill('[data-testid="retry-output"]', out)
    await fill('[data-testid="retry-password"]', code === 'cp932' ? 'wrong-password' : password)
    const subfolder = await waitForElement('[data-testid="retry-subfolder"]'); if (await subfolder.isSelected()) await click(subfolder)
    await click('[data-testid="create-retry-drafts"]')
    let corrected = await driver.wait(async () => (await tasks()).find(t => t.retryOfTaskId === first.id), 15000)
    let outcome = await run(corrected.id)
    if (code === 'cp932') {
      assert.equal(outcome.status, 'failed', 'wrong password must not succeed or become file damage')
      const wrongRecord = await driver.wait(async () => (await bridge('taskHistory')).find(t=>t.id===corrected.id),15000)
      assert.match(wrongRecord.errorMessage,/password|密码/i)
      assert.doesNotMatch(wrongRecord.errorMessage,/\[archive-inspection:damaged\]/)
      negativeCases.push({kind:'wrong-manual-password',taskId:corrected.id,category:wrongRecord.failure?.category})
      assert.equal(existsSync(path.join(out,'payload.txt')), false)
      await expand(corrected.id)
      assert.ok((await (await waitForElement('[data-testid="retry-zip-password-encoding"]')).getText()).includes(label))
      await fill('[data-testid="retry-password"]', '🔒cannot-map-to-cp932')
      await click('[data-testid="create-retry-drafts"]')
      corrected = await driver.wait(async () => (await tasks()).find(t => t.retryOfTaskId === corrected.id), 15000)
      assert.equal((await run(corrected.id)).status,'failed')
      const encodingFailure = await driver.wait(async () => (await bridge('taskHistory')).find(t=>t.id===corrected.id),15000)
      assert.match(encodingFailure.errorMessage,/\[archive-inspection:password-encoding\]/)
      assert.doesNotMatch(encodingFailure.errorMessage,/7Z/,'reading an existing ZIP must not advise changing its format')
      assert.equal(encodingFailure.failure.category,'password-encoding')
      assert.equal(existsSync(path.join(out,'payload.txt')),false)
      negativeCases.push({kind:'unrepresentable-manual-password',taskId:corrected.id,category:encodingFailure.failure.category})
      await expand(corrected.id)
      await fill('[data-testid="retry-password"]', password)
      for (const [width,height] of [[1100,700],[760,520]]) {
        await driver.manage().window().setRect({width,height})
        await click('[data-testid="retry-zip-password-encoding"]')
        await shot(`password-encoding-correction-${width}x${height}`)
        await driver.actions().sendKeys(Key.ESCAPE).perform()
      }
      await driver.manage().window().setRect({width:1100,height:700})
      await click('[data-testid="create-retry-drafts"]')
      corrected = await driver.wait(async () => (await tasks()).find(t => t.retryOfTaskId === corrected.id), 15000)
      outcome = await run(corrected.id)
    }
    assert.equal(outcome.status, 'completed', JSON.stringify(outcome))
    assert.deepEqual(readFileSync(path.join(out, 'payload.txt')), payload)
    const saved = await driver.wait(async () => (await bridge('taskHistory')).find(t => t.id === corrected.id), 15000)
    assert.equal(saved.executionConfig.zipPasswordEncoding, code)
    assert.equal((await bridge('taskHistory')).find(t=>t.id===first.id).status, 'failed')
    assert.equal(hash(file), before); assert.equal(hash(original), before)
    results.push({ code, firstTask: first.id, completedTask: corrected.id, outputHash: hash(path.join(out,'payload.txt')), sourceHash: before })
    await shot(`${code}-complete`)
  }
  // Re-execute a real completed history entry, not a seeded success record.
  await click('[data-testid="nav-History"]')
  const row = await driver.wait(async () => {
    for (const el of await driver.findElements(By.css('[data-testid="history-record-row"]'))) {
      const text = await el.getText(); if (text.includes('cp950.zip') && text.includes('已完成')) return el
    }
    return false
  }, 10000)
  await click(await row.findElement(By.css('[data-testid="history-retry"]')))
  await click('[data-testid="history-draft-zip-password-encoding"]')
  await driver.actions().sendKeys(Key.ESCAPE).perform()
  assert.ok((await (await waitForElement('[data-testid="history-draft-zip-password-encoding"]')).getText()).includes('CP950'))
  const replay = path.join(work,'history-replay'); mkdirSync(replay)
  await fill('[data-testid="history-draft-output"]', replay)
  await fill('[data-testid="history-draft-password"]', '繁體密碼測試+!1416')
  await driver.manage().window().setRect({ width:760, height:520 })
  await click('[data-testid="history-draft-zip-password-encoding"]'); await shot('history-narrow-menu')
  await driver.actions().sendKeys(Key.ESCAPE).perform()
  await click('[data-testid="history-draft-create"]')
  const again = await driver.wait(async () => (await tasks()).find(t=>t.output===replay),15000)
  await click('[data-testid="history-extraction-draft"] a')
  assert.equal((await run(again.id)).status,'completed')
  assert.deepEqual(readFileSync(path.join(replay,'payload.txt')),payload)
  writeFileSync(path.join(evidence,'results.json'),JSON.stringify({ verifiedAt:new Date().toISOString(), applicationSha256:hash(application), source:'unmodified pinned Info-ZIP originals', results, negativeCases, historyReplayTask:again.id, screenshots },null,2))
  console.log(`[legacy-zip-input-1416] real extraction and history workflows passed: ${evidence}`)
}

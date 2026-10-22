import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runAlzBzip2165Gate({ driver, bridge, waitForElement, application, root }) {
  const base = path.join(root, 'test-results/alz-bzip2-165')
  mkdirSync(base, { recursive: true })
  const sha = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const receipt = path.join(base, 'desktop.json')
  const state = existsSync(receipt) ? JSON.parse(readFileSync(receipt, 'utf8')) : {
    version: '1.6.5', binarySha256: sha(application), helperSha256: sha(path.join(root, 'src-tauri/resources/alz-engine/long-alz.exe')),
    work: path.join(base, randomUUID()), checks: [], layouts: [],
  }
  assert.equal(state.binarySha256, sha(application), 'Retain results only for the same candidate')
  assert.equal(state.helperSha256, sha(path.join(path.dirname(application), 'resources/alz-engine/long-alz.exe')))
  mkdirSync(state.work, { recursive: true })
  const save = () => writeFileSync(receipt, JSON.stringify(state, null, 2) + '\n')
  const passed = name => state.checks.some(c => c.name === name)
  const done = (name, values = {}) => { state.checks.push({ name, ...values }); save(); console.log('alz-bzip2-165:', name) }
  const invoke = (cmd, args) => driver.executeAsyncScript(`
    const [cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));
    const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
    window['_'+ids[0]]=value=>{clean();done({ok:true,value})};
    window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};
    window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`, cmd, args)
  const ipc = async (cmd, args) => { const r = await invoke(cmd, args); assert.equal(r.ok, true, r.error); return r.value }
  const find = waitForElement
  const click = async selector => {
    for (const e of await driver.findElements(By.css('.toast-item button'))) {
      try { if (await e.isDisplayed()) await e.click() } catch (error) { if (error.name !== 'StaleElementReferenceError') throw error }
    }
    const e = typeof selector === 'string' ? await find(selector) : selector
    await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), e)
    await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
    await driver.wait(() => driver.executeScript(e => { const r = e.getBoundingClientRect(); return !e.disabled && e.contains(document.elementFromPoint(r.x+r.width/2, r.y+r.height/2)) }, e), 15000)
    await e.click()
  }
  const fill = async (selector, value) => {
    const e = await find(selector)
    await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), e)
    await e.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (value) await e.sendKeys(value)
  }
  const shot = async (name, anchor) => {
    for (const [width, height] of [[1100, 700], [760, 520]]) {
      if (state.layouts.some(l => l.name === name && l.width === width)) continue
      await driver.manage().window().setRect({ width, height })
      await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), await find(anchor))
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
      const geometry = await driver.executeScript(() => ({ width: innerWidth, height: innerHeight, overflow: document.documentElement.scrollWidth-innerWidth }))
      assert.ok(geometry.overflow <= 1, JSON.stringify(geometry))
      const file = path.join(base, `${name}-${width}x${height}.png`)
      writeFileSync(file, Buffer.from(await driver.takeScreenshot(), 'base64'))
      state.layouts.push({ name, file, ...geometry }); save()
    }
    await driver.manage().window().setRect({ width: 1100, height: 700 })
  }
  const fixtures = path.join(root, 'tests/fixtures/archives/alz-bzip2-165')
  const manifest = JSON.parse(readFileSync(path.join(fixtures, 'manifest.json'), 'utf8'))
  const original = path.join(root, manifest.original)
  const opts = { preserve_paths: true, overwrite_existing: false, delete_after: false, preserve_timestamps: true,
    skip_corrupted: false, extract_only_newer: false, create_subdirectory: false, conflict_policy: 'rename',
    enable_bruteforce: false, bruteforce_wordlists: [], zip_password_encoding: null }
  const extractArgs = (file, output, password, selected = []) => ({ taskId: randomUUID(), filePath: file, outputPath: output, password, options: { ...opts, selected_entries: selected } })
  const tree = folder => !existsSync(folder) ? [] : readdirSync(folder).sort().map(name => ({ path: name, sha256: sha(path.join(folder, name)) }))
  const expected = manifest.expected.map(e => ({ path: e.path, sha256: e.sha256 })).sort((a,b) => a.path.localeCompare(b.path))

  if (!passed('original-content')) {
    await ipc('test_archive_integrity', { filePath: original })
    const out = path.join(state.work, '原件完整取回')
    await ipc('extract_file', extractArgs(original, out, null))
    assert.deepEqual(tree(out), expected)
    done('original-content', { hashes: true })
  }
  for (const name of ['plain-missing-end-single', 'key-trailing-volumes']) {
    if (passed(name)) continue
    const c = manifest.cases.find(c => c.name === name)
    const file = path.join(fixtures, c.controlledNativeFraming ? c.members[1].name : c.archive)
    const error = await invoke('test_archive_integrity', { filePath: file, password: c.password })
    assert.equal(error.ok, false); assert.match(error.error, /\[archive-inspection:damaged\]/)
    const diagnostic = await ipc('diagnose_archive', { diagnosticId: randomUUID(), filePath: file, password: c.password })
    assert.equal(diagnostic.status, 'damaged')
    const out = path.join(state.work, name), extraction = await invoke('extract_file', extractArgs(file, out, c.password))
    assert.equal(extraction.ok, false); assert.match(extraction.error, /重新获取完整原包/); assert.deepEqual(tree(out), [])
    const converted = out + '.zip'
    const conversion = await invoke('convert_archive', { taskId: randomUUID(), sourcePath: file, sourcePassword: c.password,
      sourceZipPasswordEncoding: null, outputPath: converted, options: { format: 'zip', level: 6, password: null,
        split_size: null, verify_after: true, preserve_paths: true, delete_after: false } })
    assert.equal(conversion.ok, false); assert.equal(existsSync(converted), false)
    const selected = out + '-所选完好文件'
    await ipc('extract_file', extractArgs(file, selected, null, ['b.txt']))
    assert.deepEqual(tree(selected), expected.filter(e => e.path === 'b.txt'))
    done(name, { diagnostic, extraction: extraction.error, conversion: conversion.error, noPartialResult: true, unrelatedSelectionHashes: true })
  }

  const expand = async id => { const e = await find(`[data-task-id="${id}"]`); if (await e.getAttribute('aria-expanded') !== 'true') await click(e) }
  const record = id => driver.wait(async () => (await bridge('taskHistory')).find(h => h.id === id) || false, 30000)
  if (!passed('failure-and-retry')) {
    const broken = path.join(state.work, '未完整结束.alz'), reacquired = path.join(state.work, '重新获取完整原包.alz')
    const failedOut = path.join(state.work, '旧失败结果'), retryOut = path.join(state.work, '新任务完整结果')
    state.ui ||= {}; save()
    if (!state.ui.failed) {
      if (!state.ui.taskId) {
        await bridge('clearTasks')
        copyFileSync(path.join(fixtures, 'plain-missing-end-single.alz'), broken); copyFileSync(original, reacquired)
        await click('[data-testid="nav-Decompress"]')
        await bridge('queueDesktopDialogSelections', [[broken]]); await click('[data-testid="dropzone-file"]')
        const task = await driver.wait(async () => (await bridge('extractionRecoveryTasks')).find(t => t.source === broken && t.status === 'pending') || false, 15000)
        state.ui.taskId = task.id; save()
      }
      await click('[data-testid="nav-Decompress"]'); await expand(state.ui.taskId)
      mkdirSync(failedOut, { recursive: true })
      // The task's legacy native folder chooser has no queue hook. Use the
      // existing file-manager recent-location route, without modifying state.
      await click('[data-testid="nav-ArchiveBrowser"]')
      await fill('[data-testid="file-manager-path-input-left"]', failedOut)
      await (await find('[data-testid="file-manager-path-input-left"]')).sendKeys(Key.ENTER)
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
      await click('[data-testid="nav-Decompress"]'); await expand(state.ui.taskId)
      await click('[data-testid="saved-output-select"]')
      const location = await driver.wait(async () => {
        for (const e of await driver.findElements(By.css('[role="menu"][aria-label="常用目录"] [role="menuitem"]'))) if (await e.getAttribute('title') === failedOut) return e
        return false
      }, 15000)
      await click(location)
      await driver.wait(async () => (await bridge('extractionRecoveryTasks')).find(t => t.id === state.ui.taskId)?.output === failedOut, 15000)
      const sub = await find('.task-subfolder-option'); if (await sub.getAttribute('aria-checked') === 'true') await click(sub)
      await click('[data-testid="start-decompression-queue"]')
      await driver.wait(async () => await bridge('taskStatus', state.ui.taskId) === 'failed', 30000)
      state.ui.failed = await record(state.ui.taskId)
      assert.match(state.ui.failed.errorMessage, /缺少结束标记/); assert.deepEqual(tree(failedOut), []); save()
    }
    await click('[data-testid="nav-Decompress"]'); await expand(state.ui.taskId)
    await shot('alz-bzip2-failure', '[data-testid="retry-source"]')
    if (!state.ui.nextId) {
      await fill('[data-testid="retry-source"]', reacquired)
      mkdirSync(retryOut, { recursive: true }); await fill('[data-testid="retry-output"]', retryOut)
      await click('[data-testid="create-retry-drafts"]')
      const next = await driver.wait(async () => (await bridge('extractionRecoveryTasks')).find(t => t.status === 'pending' && t.output === retryOut) || false, 15000)
      assert.equal(next.retryOfTaskId, state.ui.taskId)
      state.ui.nextId = next.id; save()
    }
    if (await bridge('taskStatus', state.ui.nextId) === 'pending') await click('[data-testid="start-decompression-queue"]')
    await driver.wait(async () => await bridge('taskStatus', state.ui.nextId) === 'completed', 30000)
    assert.deepEqual(tree(retryOut), expected); assert.deepEqual(await record(state.ui.taskId), state.ui.failed)
    done('failure-and-retry', { failed: state.ui.failed, next: await record(state.ui.nextId), oldFailurePreserved: true, hashes: true })
  }
  if (!passed('history-cause')) {
    const failed = state.checks.find(c => c.name === 'failure-and-retry').failed
    await click('[data-testid="nav-History"]'); await fill('[data-testid="history-search"]', failed.outputPath)
    const row = await driver.wait(async () => { const rows = await driver.findElements(By.css('[data-testid="history-record-row"]')); return rows.length === 1 ? rows[0] : false }, 15000)
    await click(row)
    await find('[data-testid="history-detail"]')
    assert.match(await (await find('[data-testid="history-detail"]')).getText(), /缺少结束标记/)
    await shot('alz-bzip2-history', '[data-testid="history-detail"]')
    done('history-cause', { actualDialog: true })
  }
  assert.equal(sha(original), manifest.originalSha256)
  for (const c of manifest.cases) for (const file of c.members) assert.equal(sha(path.join(fixtures, file.name)), file.sha256)
  state.completedAt = new Date().toISOString(); state.originalAndDerivativesUnchanged = true; save()
}

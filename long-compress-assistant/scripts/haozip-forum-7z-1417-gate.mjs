import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

const sha = file => createHash('sha256').update(readFileSync(file)).digest('hex')
const expected = [
  { path: '中文目录/原文件.bin', sha256: '47eb7c96a9dea6493faca0336c5225f1d83e0d3e987e96c2287e764b669002c6' },
  { path: '说明.txt', sha256: 'b70dfbde85c5ad302aae256db376bb19c84c991ca9c25de18696e93f1178910b' },
].sort((a, b) => a.path.localeCompare(b.path))
const tree = directory => !existsSync(directory) ? [] : readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
  const file = path.join(directory, entry.name)
  return entry.isDirectory() ? tree(file).map(item => ({ ...item, path: entry.name + '/' + item.path })) : [{ path: entry.name, sha256: sha(file) }]
}).sort((a, b) => a.path.localeCompare(b.path))
const verify = directory => {
  assert.deepEqual(tree(directory), expected)
  assert.deepEqual(readdirSync(path.join(directory, '中文目录/空目录')), [])
}

// Copy only genuine checked-in GUI output, with original names and bytes.
export async function runHaozipForum7z1417Gate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, application, bundledSevenZip, root }) {
  const original = path.join(root, 'tests/fixtures/archives/haozip-forum-7z-1417')
  const work = path.join(fixtureDirectory, '我收到的好压论坛7Z')
  mkdirSync(work)
  const password = 'ForumTest-1411', cases = [], screenshots = []
  const invoke = (cmd, args) => driver.executeAsyncScript(`
    const [cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));
    const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
    window['_'+ids[0]]=value=>{clean();done({ok:true,value})};
    window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};
    window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});
  `, cmd, args)
  const click = async selector => {
    const el = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), el)
    await driver.wait(() => driver.executeScript(el => { const r = el.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return !el.disabled && (hit === el || el.contains(hit)) }, el), 10000, `Control must be reachable: ${selector}`).catch(async error => {
      const hit = await driver.executeScript(el => { const r = el.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return { rect: r.toJSON(), disabled: el.disabled, hit: hit?.outerHTML.slice(0, 500) } }, el)
      throw new Error(`${error.message}; ${JSON.stringify(hit)}`, { cause: error })
    })
    await el.click()
  }
  const fill = async (selector, value) => { const el = await waitForElement(selector); await el.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE); if (value) await el.sendKeys(value) }
  const dismissToasts = async () => {
    for (const close of await driver.findElements(By.css('.toast-stack button[aria-label$="提示"]'))) await close.click()
    await driver.wait(async () => (await driver.findElements(By.css('.toast-stack [role="alert"]'))).length === 0, 10000)
  }
  const shot = async name => {
    // Ambient infinite status animation does not settle; await finite UI motion.
    await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
    writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64')); screenshots.push(name + '.png')
  }
  const groups = []
  for (const stem of ['论坛7Z内容密码', '论坛7Z文件名密码']) {
    console.log(`[forum-7z] native matrix: ${stem}`)
    const directory = path.join(work, stem); mkdirSync(directory)
    const members = [1, 2, 3].map(index => {
      const name = `${stem}.haozip${String(index).padStart(2, '0')}.7z`, file = path.join(directory, name)
      copyFileSync(path.join(original, name), file); return file
    })
    const snapshots = members.map(file => readFileSync(file))
    const records = []
    for (const [index, member] of members.entries()) {
      const group = await invoke('detect_split_archive', { path: member })
      assert.equal(group.ok, true, group.error); assert.equal(group.value.is_split, true)
      assert.equal(group.value.first_part, members[0]); assert.deepEqual(group.value.parts, members)
      assert.equal(group.value.base_name, stem); assert.equal(group.value.archive_format, '7z')
      const browse = await invoke('browse_archive', { filePath: member, password, browseId: `forum-${stem}-${index}` })
      assert.equal(browse.ok, true, browse.error); assert.equal(browse.value.totalFiles, 2)
      const output = path.join(directory, `从第${index + 1}卷解压`)
      const wrong = await failure('extractArchive', member, output, 'wrong')
      assert.match(wrong, /ambiguous-data|密码|password/i); assert.deepEqual(tree(output), [])
      await bridge('extractArchive', member, output, password); verify(output)
      const diagnosis = await bridge('diagnoseArchive', member, password)
      assert.equal(diagnosis.status, 'healthy', JSON.stringify(diagnosis)); assert.equal(diagnosis.integrityTested, true)
      const converted = path.join(directory, `从第${index + 1}卷转换.zip`)
      await bridge('convertArchive', member, password, converted, { format: 'zip', password: null })
      const independent = converted + '-独立回解'
      const decoded = spawnSync(bundledSevenZip, ['x', '-y', `-o${independent}`, converted], { windowsHide: true, timeout: 30000 })
      assert.ifError(decoded.error); assert.equal(decoded.status, 0, String(decoded.stderr)); verify(independent)
      const preview = await invoke('preview_extraction_output', { filePath: member, outputPath: directory, createSubdirectory: true })
      assert.equal(preview.ok, true, preview.error); assert.equal(preview.value, path.join(directory, stem))
      records.push({ member: path.basename(member), grouped: true, browsed: true, wrongRejected: true, extracted: true, integrityTested: true, convertedIndependently: true, cleanPreview: true })
    }
    const faults = []
    for (const fault of ['missing-first', 'missing-middle', 'missing-tail', 'truncated-tail', 'damaged-header']) {
      const index = fault === 'missing-middle' ? 1 : fault === 'missing-tail' || fault === 'truncated-tail' ? 2 : 0
      const missing = fault.startsWith('missing-'), hidden = members[index] + '.暂缺'
      if (missing) renameSync(members[index], hidden)
      else { const bytes = Buffer.from(snapshots[index]); if (fault === 'damaged-header') bytes[12] ^= 1; writeFileSync(members[index], fault === 'truncated-tail' ? bytes.subarray(0, -1) : bytes) }
      const selected = members[index === 0 ? 2 : 0]
      try {
        for (const supplied of [null, 'wrong', password]) {
          const output = path.join(directory, `${fault}-${supplied === null ? '未填' : supplied === password ? '正确' : '错误'}`)
          const error = await failure('extractArchive', selected, output, supplied)
          assert.ok(error.includes(`[archive-inspection:${fault === 'damaged-header' ? 'damaged' : 'incomplete'}]`), error)
          assert.doesNotMatch(error, /密码不正确|需要输入密码/); assert.deepEqual(tree(output), [])
          if (fault === 'missing-first' || fault === 'missing-middle') assert.ok(error.includes(path.basename(members[index])), error)
          const history = await driver.wait(async () => (await bridge('taskHistory')).find(item => item.outputPath === output), 10000)
          assert.equal(history.status, 'failed'); assert.ok(history.errorMessage.includes(fault === 'damaged-header' ? '归档头校验失败' : fault === 'missing-tail' || fault === 'truncated-tail' ? '可能缺少尾卷' : path.basename(members[index])), history.errorMessage)
          faults.push({ fault, supplied: supplied === null ? 'none' : supplied === password ? 'correct' : 'wrong', error: history.errorMessage })
        }
      } finally { if (missing) renameSync(hidden, members[index]); else writeFileSync(members[index], snapshots[index]) }
      const restored = await invoke('browse_archive', { filePath: members[2], password, browseId: `restored-${stem}-${fault}` })
      assert.equal(restored.ok, true, restored.error); assert.equal(restored.value.totalFiles, 2)
    }
    groups.push({ stem, members })
    cases.push({ stem, records, faults, originalVolumes: members.map(file => ({ name: path.basename(file), sha256: sha(file) })) })
  }
  writeFileSync(path.join(artifactDirectory, 'haozip-forum-7z-1417-matrix.json'), JSON.stringify({ application, binarySha256: sha(application), cases }, null, 2))
  // Real page workflow: header-encrypted non-first member, mistype, correct,
  // reload its directory. 7Z bounded content preview is not an existing feature.
  await click('[data-testid="nav-ArchiveBrowser"]')
  await bridge('queueDesktopDialogSelections', [groups[1].members[2]])
  await click('[data-testid="file-manager-open-archive"]'); await waitForElement('[data-testid="archive-browse-notice"]')
  await fill('.browser-toolbar input[type="password"]', 'wrong')
  await click('.browser-toolbar button[title="重新读取"]')
  await driver.wait(async () => /密码|加密/.test(await (await waitForElement('[data-testid="archive-browse-notice"]')).getText()), 15000)
  await fill('.browser-toolbar input[type="password"]', password)
  await click('.browser-toolbar button[title="重新读取"]'); await waitForElement('.browser-summary')
  assert.match(await (await waitForElement('.browser-page')).getText(), /说明.txt/)
  assert.equal((await driver.findElements(By.css('[data-testid="archive-browse-notice"]'))).length, 0)
  for (const close of await driver.findElements(By.css('.toast-stack button[aria-label$="提示"]'))) await close.click()
  await shot('haozip-forum-7z-1417-corrected-browse')
  console.log('[forum-7z] UI: unordered import and individual output')
  const uiOutput = path.join(work, '补卷之后仍解压到这里'); mkdirSync(uiOutput)
  // The task-row native folder dialog has no existing E2E selection queue.
  // Choose a real bookmark through the file manager instead of adding a bypass.
  await click('[data-testid="return-to-file-manager"]')
  await fill('[data-testid="file-manager-path-input-left"]', uiOutput)
  await (await waitForElement('[data-testid="file-manager-path-input-left"]')).sendKeys(Key.ENTER)
  await driver.wait(async () => await (await waitForElement('[data-testid="bookmark-current-left"]')).isEnabled(), 10000)
  await click('[data-testid="bookmark-current-left"]')
  await driver.wait(async () => await (await waitForElement('[data-testid="bookmark-current-left"]')).getAttribute('aria-pressed') === 'true', 10000)
  await bridge('clearTasks'); await click('[data-testid="nav-Decompress"]')
  await bridge('queueDesktopDialogSelections', [[groups[1].members[1], groups[1].members[0], groups[1].members[2]]])
  await click('.decompress-view [data-testid="dropzone-file"]')
  await driver.wait(async () => (await bridge('archiveBrowserTaskState')).length === 1, 10000)
  await click('[data-testid^="toggle-archive-task-"]')
  await click('[data-testid="decompression-config-panel"] [data-testid="saved-output-select"]')
  let savedOutput
  for (const item of await driver.findElements(By.css('.location-menu [role="menuitem"]'))) {
    if (await item.getAttribute('title') === uiOutput) { savedOutput = item; break }
  }
  assert.ok(savedOutput, 'the user-selected bookmark must be available'); await savedOutput.click()
  if (await (await waitForElement('.task-subfolder-option')).getAttribute('aria-checked') !== 'true') await click('.task-subfolder-option')
  await driver.wait(async () => (await (await waitForElement('[data-testid="extraction-output-path"]')).getAttribute('title')) === path.join(uiOutput, groups[1].stem), 10000, 'Output preview must match the individually selected directory and logical archive name').catch(async error => {
    throw new Error(`${error.message}; actual=${await (await waitForElement('[data-testid="extraction-output-path"]')).getAttribute('title')}; tasks=${JSON.stringify(await bridge('archiveBrowserTaskState'))}`, { cause: error })
  })
  await dismissToasts()
  for (const [width, height] of [[1100, 700], [760, 520]]) {
    await driver.manage().window().setRect({ width, height })
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), await waitForElement('[data-testid="extraction-output-preview"]'))
    await shot(`haozip-forum-7z-1417-output-${width}x${height}`)
    if (width === 760) {
      await driver.executeScript(el => el.scrollIntoView({ block: 'end', behavior: 'instant' }), await waitForElement('[data-testid="extraction-output-path"]'))
      await shot('haozip-forum-7z-1417-output-narrow-end')
    }
  }
  renameSync(groups[1].members[1], groups[1].members[1] + '.暂缺')
  let failed
  try {
    await click('[data-testid="start-decompression-queue"]')
    failed = await driver.wait(async () => { const task = (await bridge('archiveBrowserTaskState'))[0]; return task?.status === 'failed' ? task : false }, 15000)
    assert.ok(failed.error.includes(path.basename(groups[1].members[1])), failed.error)
    assert.doesNotMatch(failed.error, /密码不正确|需要输入密码/)
    await waitForElement('[data-testid="retry-missing-volumes"]')
    await dismissToasts(); await driver.manage().window().setRect({ width: 1100, height: 700 })
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), await waitForElement('[data-testid="retry-missing-volumes"]'))
    await shot('haozip-forum-7z-1417-missing-middle')
  } finally { renameSync(groups[1].members[1] + '.暂缺', groups[1].members[1]) }
  assert.equal(await (await waitForElement('[data-testid="retry-output"]')).getAttribute('value'), uiOutput)
  await fill('[data-testid="retry-password"]', password); await click('[data-testid="create-retry-drafts"]')
  await driver.wait(async () => (await bridge('archiveBrowserTaskState')).length === 2, 15000)
  await click('[data-testid="start-decompression-queue"]')
  const completed = await driver.wait(async () => { const task = (await bridge('archiveBrowserTaskState'))[1]; return ['completed', 'failed'].includes(task?.status) ? task : false }, 60000)
  assert.equal(completed.status, 'completed', JSON.stringify(completed)); verify(path.join(uiOutput, groups[1].stem))
  assert.equal((await bridge('archiveBrowserTaskState'))[0].error, failed.error)
  await dismissToasts()
  const finalRow = (await driver.findElements(By.css('[data-testid="task-row"]'))).at(-1)
  await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), finalRow)
  await driver.wait(async () => /完成|100%/.test(await finalRow.getText()), 10000)
  await shot('haozip-forum-7z-1417-restored-completed')
  cases.forEach((item, i) => item.originalVolumes.forEach((volume, j) => {
    assert.equal(sha(groups[i].members[j]), volume.sha256)
    assert.equal(sha(path.join(original, volume.name)), volume.sha256)
  }))
  writeFileSync(path.join(artifactDirectory, 'haozip-forum-7z-1417-results.json'), JSON.stringify({ checkedAt: new Date().toISOString(), application, binarySha256: sha(application), fixtureScope: 'Original HaoZip GUI 6.6.0.11316 forum 7Z content/header password groups, NOT renamed numeric volumes', cases, ui: { unorderedMembersDeduplicated: true, correctedHeaderPasswordBrowse: true, outputNameMatchesActual: true, missingMiddleBeforePassword: true, suppliedVolumeRetried: true, originalFailureRetained: true }, screenshots }, null, 2))
}

import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync, copyFileSync, renameSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'
import { gzipSync } from 'node:zlib'

export async function runMultistream1517Gate({ driver, bridge, waitForElement, artifactDirectory, application, root, bundledSevenZip }) {
  const base = path.join(root, 'tests/fixtures/archives/multistream-1517')
  const manifest = JSON.parse(readFileSync(path.join(base, 'manifest.json'), 'utf8'))
  const resultRoot = path.join(root, 'test-results/multistream-1517')
  const hash = bytes => createHash('sha256').update(bytes).digest('hex')
  const sha = file => hash(readFileSync(file)), binarySha256 = sha(application)
  const progress = path.join(resultRoot, 'desktop-progress.json')
  const saved = existsSync(progress) ? JSON.parse(readFileSync(progress, 'utf8')) : null
  const prior = saved?.binarySha256 === binarySha256 ? saved : null
  const work = prior?.work || path.join(resultRoot, 'ui-' + randomUUID())
  assert.ok(path.resolve(work).startsWith(path.resolve(resultRoot) + path.sep))
  mkdirSync(work, { recursive: true })
  const cases = prior?.cases || [], groups = prior?.groups || {}, screenshots = [], layouts = []
  const save = () => writeFileSync(progress, JSON.stringify({ binarySha256, work, cases, groups }, null, 2))
  const owned = p => { assert.ok(path.resolve(p).startsWith(path.resolve(work) + path.sep)); return p }
  const fresh = label => owned(path.join(work, label + '-' + randomUUID()))
  const opts = { preserve_paths: true, overwrite_existing: false, delete_after: false, preserve_timestamps: true,
    skip_corrupted: false, extract_only_newer: false, file_filter: null, create_subdirectory: false,
    conflict_policy: 'rename', enable_bruteforce: false, bruteforce_wordlists: [] }
  await driver.manage().setTimeouts({ script: 120000 })
  const invoke = (cmd, args) => driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});', cmd, args)
  const ipc = async (cmd, args) => { const r = await invoke(cmd, args); assert.equal(r.ok, true, r.error); return r.value }
  const click = async selector => {
    for (const e of await driver.findElements(By.css('.toast-item button'))) { try { if (await e.isDisplayed()) await e.click() } catch {} }
    const e = typeof selector === 'string' ? await waitForElement(selector) : selector
    await driver.wait(() => driver.executeScript(e => {
      e.scrollIntoView({ block: 'center', behavior: 'instant' })
      const r = e.getBoundingClientRect(), h = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
      return !e.disabled && r.width > 0 && r.height > 0 && (h === e || e.contains(h))
    }, e), 15000); await e.click()
  }
  const fill = async (selector, value) => { const e = await waitForElement(selector); await e.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE); if (value) await e.sendKeys(value) }
  const sizes = async (name, anchor) => {
    for (const [width, height] of [[1100, 700], [760, 520]]) {
      await driver.manage().window().setRect({ width, height })
      await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), await waitForElement(anchor))
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
      const geometry = await driver.executeScript(() => ({ width: innerWidth, height: innerHeight, overflow: document.documentElement.scrollWidth - innerWidth }))
      assert.ok(geometry.overflow <= 1, JSON.stringify(geometry)); layouts.push({ name, ...geometry })
      const file = name + '-' + width + 'x' + height + '.png'
      writeFileSync(path.join(artifactDirectory, file), Buffer.from(await driver.takeScreenshot(), 'base64')); screenshots.push(file)
    }
  }
  const checkOutput = (c, output, selected = false) => {
    if (c.kind === 'tar') {
      for (const f of manifest.expected.filter(f => !selected || f.path === '末尾.txt')) assert.equal(sha(path.join(output, f.path)), f.sha256)
      if (selected) assert.ok(!existsSync(path.join(output, '数据.bin')))
      else assert.ok(existsSync(path.join(output, manifest.emptyDirectory)))
    } else assert.equal(sha(path.join(output, '说明.txt')), c.decodedSha256)
  }
  for (const c of manifest.cases) {
    const source = path.join(base, c.archive), key = c.kind + '-' + c.codec
    assert.equal(sha(source), c.sha256)
    if (!cases.some(done => done.key === key && done.kind === 'original')) {
      console.log('[stream1517] full original', key)
      const browse = await ipc('browse_archive', { filePath: source, password: null })
      assert.equal(browse.totalFiles, c.kind === 'tar' ? 3 : 1)
      if (c.kind === 'plain') assert.equal(browse.totalUncompressedSize, c.decodedBytes)
      await ipc('test_archive_integrity', { filePath: source, password: null })
      const output = fresh('原件-' + key)
      await ipc('extract_file', { taskId: randomUUID(), filePath: source, outputPath: output, password: null, options: opts })
      checkOutput(c, output)
      cases.push({ key, kind: 'original', fullPayloadHashes: true, actualIntegrity: true, output }); save()
    }
    if (!cases.some(done => done.key === key && done.kind === 'bad-tail')) {
      const bad = owned(path.join(work, '损坏-' + key + (c.kind === 'tar' ? '.tar.' : '.txt.') + c.codec))
      copyFileSync(path.join(base, c.corrupt), bad)
      const output = fresh('不应发布-' + key)
      const result = await invoke('extract_file', { taskId: randomUUID(), filePath: bad, outputPath: output, password: 'irrelevant', options: opts })
      assert.equal(result.ok, false); assert.match(result.error, /\[archive-inspection:damaged\]/)
      assert.ok(!existsSync(output) || readdirSync(output).length === 0)
      assert.ok(!readdirSync(work).some(name => name.startsWith('.long-extract-')), 'Failed stream must not retain unpublished staging')
      cases.push({ key, kind: 'bad-tail', originalReason: result.error, noPublishedPartial: true, source: bad }); save()
    }
    if (!groups[key]) {
      const folder = fresh('数字卷-' + key); mkdirSync(folder)
      const raw = readFileSync(source), size = Math.min(8193, Math.floor(raw.length / 3)), parts = []
      for (let i = 0; i < raw.length; i += size) {
        const name = '原件' + (c.kind === 'tar' ? '.tar.' : '.txt.') + c.codec + '.' + String(parts.length + 1).padStart(3, '0')
        const file = owned(path.join(folder, name)); writeFileSync(file, raw.subarray(i, i + size)); parts.push(file)
      }
      groups[key] = parts; save()
    }
    if (!cases.some(done => done.key === key && done.kind === 'numeric')) {
      const parts = groups[key], member = parts[Math.floor(parts.length / 2)]
      const browse = await ipc('browse_archive', { filePath: member, password: null })
      if (c.kind === 'plain') assert.equal(browse.totalCompressedSize, c.bytes)
      const output = fresh('中卷读取-' + key)
      await ipc('extract_file', { taskId: randomUUID(), filePath: member, outputPath: output, password: null,
        options: { ...opts, selected_entries: c.kind === 'tar' ? ['末尾.txt'] : null } })
      checkOutput(c, output, c.kind === 'tar')
      cases.push({ key, kind: 'numeric', middleMember: member, fullSelectedHash: true, output }); save()
    }
  }
  if (!cases.some(c => c.kind === 'image-preview')) {
    const original = readFileSync(path.join(root, 'src-tauri/icons/32x32.png'))
    const source = owned(path.join(work, '图标.png.gz'))
    writeFileSync(source, Buffer.concat([gzipSync(original.subarray(0, 17)), gzipSync(original.subarray(17))]))
    const preview = await ipc('preview_archive_image', { filePath: source, entryPath: '图标.png', password: null })
    assert.equal(preview.width, 32); assert.equal(preview.height, 32)
    assert.equal(hash(Buffer.from(preview.dataUrl.split(',')[1], 'base64')), hash(original))
    cases.push({ kind: 'image-preview', nodeProducer: process.version, source, sourceSha256: sha(source), originalPngHash: hash(original) }); save()
  }
  // The following actions use real UI controls and real retained task records.
  await driver.manage().window().setRect({ width: 1100, height: 700 })
  await click('[data-testid="nav-ArchiveBrowser"]')
  await bridge('queueDesktopDialogSelections', [[path.join(base, '说明.txt.gz')]])
  await click('[data-testid="file-manager-open-archive"]')
  await waitForElement('[data-entry-path="说明.txt"] .preview-trigger')
  await click('[data-entry-path="说明.txt"] .preview-trigger')
  assert.match(await (await waitForElement('[data-testid="archive-text-preview"]')).getText(), /用户需要的末尾内容/)
  await sizes('stream1517-plain-preview', '[data-testid="archive-text-preview"]'); await click('.preview-close')
  await bridge('queueDesktopDialogSelections', [[groups['tar-xz'][1]]])
  await click('[data-testid="file-manager-open-archive"]')
  await click('[data-entry-path="末尾.txt"] .preview-trigger')
  assert.match(await (await waitForElement('[data-testid="archive-text-preview"]')).getText(), /跨越压缩流边界/)
  await sizes('stream1517-tar-preview', '[data-testid="archive-text-preview"]'); await click('.preview-close')
  const uiCases = [{ actualPlainTextPreview: true }, { actualNumericTarTailPreview: true }]

  await driver.manage().window().setRect({ width: 1100, height: 700 })
  await bridge('clearTasks'); await click('[data-testid="nav-Decompress"]')
  const parts = groups['tar-gz']
  await bridge('queueDesktopDialogSelections', [[parts[1]]]); await click('[data-testid="dropzone-file"]')
  const task = await driver.wait(async () => (await bridge('extractionRecoveryTasks'))[0] || false, 15000)
  const withdrawn = owned(parts[2]), backup = owned(withdrawn + '.owned-original')
  renameSync(withdrawn, backup)
  let failed
  try {
    await click('[data-testid="start-decompression-queue"]')
    failed = await driver.wait(async () => (await bridge('taskHistory')).find(h => h.id === task.id && h.status === 'failed') || false, 30000)
    assert.match(failed.errorMessage, /incomplete|缺少|分卷|source-missing/)
  } finally { if (existsSync(backup)) renameSync(backup, withdrawn) }
  const output = fresh('补卷后的新任务'); mkdirSync(output)
  await click('[data-testid="retry-batch-failures"]')
  await fill('[role="dialog"] [data-testid="retry-output"]', output)
  const sub = await waitForElement('[role="dialog"] [data-testid="retry-subfolder"]'); if (await sub.isSelected()) await click(sub)
  await sizes('stream1517-restored-batch', '[role="dialog"]')
  await click('[role="dialog"] [data-testid="create-retry-drafts"]')
  await driver.wait(async () => (await (await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'), 15000)
  assert.deepEqual(readdirSync(output), [])
  await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(., "返回队列确认并启动")]')))
  const retry = (await bridge('extractionRecoveryTasks')).find(t => t.retryOfTaskId === task.id)
  assert.ok(retry && retry.status === 'pending')
  await click('[data-testid="start-decompression-queue"]')
  const completed = await driver.wait(async () => (await bridge('taskHistory')).find(h => h.id === retry.id && ['completed','failed'].includes(h.status)) || false, 30000)
  assert.equal(completed.status, 'completed', JSON.stringify(completed))
  checkOutput(manifest.cases.find(c => c.kind === 'tar' && c.codec === 'gz'), output)
  assert.deepEqual((await bridge('taskHistory')).find(h => h.id === failed.id), failed)
  uiCases.push({ missingVolumeRestoredNewTask: true, fullHashes: true, oldFailureUnchanged: true, output })

  await driver.manage().window().setRect({ width: 1100, height: 700 })
  await click('[data-testid="nav-Compress"]'); await click('[data-testid="open-archive-conversion"]')
  await bridge('queueDesktopDialogSelections', [[groups['tar-xz'][1]]]); await click('[data-testid="conversion-choose-source"]')
  await waitForElement('[data-testid="conversion-item"]')
  if (!(await driver.findElements(By.css('[data-testid="conversion-details"]'))).length) await click('[data-testid="conversion-row"]')
  await click('[data-testid="conversion-format"]')
  let zipChosen = false
  for (const option of await driver.findElements(By.css('[role="option"]'))) {
    const label = await option.findElements(By.css('strong'))
    if ((await (label[0] || option).getText()).trim() === 'ZIP') { await click(option); zipChosen = true; break }
  }
  assert.ok(zipChosen)
  const password = 'Multistream output + !', converted = fresh('转换结果') + '.zip'
  await fill('[data-testid="conversion-password"]', password); await fill('[data-testid="conversion-output"]', converted)
  await click('[data-testid="conversion-start"]')
  const convertedTask = await driver.wait(async () => (await bridge('taskHistory')).find(h => h.outputPath.replaceAll('\\','/').toLowerCase() === converted.replaceAll('\\','/').toLowerCase() && ['completed','failed'].includes(h.status)) || false, 60000)
  assert.equal(convertedTask.status, 'completed', JSON.stringify(convertedTask))
  for (const f of manifest.expected) {
    const bytes = execFileSync(bundledSevenZip, ['x','-so','-spd','-p' + password, converted, f.path], { maxBuffer: 4 * 1024 * 1024, windowsHide: true })
    assert.equal(hash(bytes), f.sha256)
  }
  const listing = execFileSync(bundledSevenZip, ['l','-slt','-sccUTF-8','-p' + password, converted], { encoding: 'utf8', windowsHide: true })
  assert.match(listing, /Path = 空目录\/?\r?\n/)
  await sizes('stream1517-conversion-result', '[data-testid="conversion-item"]')
  uiCases.push({ actualPasswordZipConversion: true, numericXzSource: true, independentFullHashes: true, emptyDirectoryPreserved: true, output: converted })
  await click('[data-testid="nav-FileIntegrity"]'); await click('[data-testid="archive-diagnostic-mode"]')
  const bad = cases.find(c => c.kind === 'bad-tail' && c.key === 'tar-gz').source
  await bridge('queueDesktopDialogSelections', [bad]); await click('.archive-select')
  await click('[data-testid="start-archive-diagnosis"]')
  await driver.wait(async () => (await (await waitForElement('[data-testid="diagnostic-report"] h2')).getText()) === '归档损坏', 30000)
  const visible = await (await waitForElement('[data-testid="diagnostic-report"]')).getText()
  assert.match(visible, /TAR.*尾部|压缩流数据或尾部/)
  assert.ok(!visible.includes('[archive-inspection:'))
  await sizes('stream1517-later-damage', '[data-testid="diagnostic-report"] h2')
  uiCases.push({ actualLaterMemberDamagedReport: true })
  writeFileSync(path.join(artifactDirectory, 'multistream-1517-results.json'), JSON.stringify({
    version: '1.5.17', completedAt: new Date().toISOString(), application, binarySha256, work, cases, uiCases, screenshots, layouts,
  }, null, 2))
}

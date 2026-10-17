import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

const sha = file => createHash('sha256').update(readFileSync(file)).digest('hex')
const tree = directory => !existsSync(directory) ? [] : readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
  const file = path.join(directory, entry.name)
  return entry.isDirectory() ? tree(file).map(item => ({ ...item, path: entry.name + '/' + item.path })) : [{ path: entry.name, sha256: sha(file) }]
}).sort((a, b) => a.path.localeCompare(b.path))

export async function runStandardZipx1418Gate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, application, root }) {
  const original = path.join(root, 'tests/fixtures/archives/standard-zipx-1418')
  const manifest = JSON.parse(readFileSync(path.join(original, 'manifest.json'), 'utf8'))
  const expected = manifest.source.map(file => ({ path: file.name.replaceAll('\\', '/'), sha256: file.sha256 })).sort((a, b) => a.path.localeCompare(b.path))
  const verify = directory => { assert.deepEqual(tree(directory), expected); assert.deepEqual(readdirSync(path.join(directory, '中文目录/空目录')), []) }
  const uiOnly = process.env.LONG_ZIPX_1418_UI_ONLY === '1'
  const work = process.env.LONG_ZIPX_1418_REUSE_WORK || path.join(fixtureDirectory, '收到的ZIPX标准卷'); mkdirSync(work, { recursive: true })
  const password = manifest.publicPassword, cases = [], screenshots = [], layouts = []
  const tool = process.env.LONG_BANDIZIP || 'E:/Bandzip/Bandizip/bz.exe'
  assert.equal(sha(tool), manifest.toolSha256, 'Use the recorded independent decoder')
  const winrar = process.env.LONG_WINRAR || 'E:/ProjectSave/ClaudeProject/Long-Decompress/long-compress-assistant/test-results/archive-upgrade-tools/winrar/WinRAR.exe'
  const independent = (source, destination, secret, expectedTree = expected, original = false) => {
    const result = original
      ? spawnSync(tool, ['x', '-y', ...(secret ? ['-p:' + secret] : []), '-o:' + destination, source], { windowsHide: true, encoding: 'utf8', timeout: 30000 })
      : spawnSync(winrar, ['x', '-y', '-ibck', secret ? '-p' + secret : '-p-', source, destination + path.sep], { windowsHide: true, encoding: 'utf8', timeout: 30000 })
    assert.ifError(result.error); assert.equal(result.status, 0, result.stdout + result.stderr)
    assert.deepEqual(tree(destination), expectedTree)
  }
  const invoke = (cmd, args) => driver.executeAsyncScript(`
    const [cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));
    const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
    window['_'+ids[0]]=value=>{clean();done({ok:true,value})};
    window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};
    window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`, cmd, args)
  const ipc = async (cmd, args) => { const result = await invoke(cmd, args); assert.equal(result.ok, true, result.error); return result.value }
  const click = async selector => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await driver.wait(() => driver.executeScript(el => { const r = el.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return !el.disabled && (hit === el || el.contains(hit)) }, element), 10000, selector)
    await element.click()
  }
  const fill = async (selector, value) => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await element.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (String(value)) await element.sendKeys(String(value))
  }
  const select = async (selector, label) => {
    await click(selector); await waitForElement('[role="option"]')
    for (const option of await driver.findElements(By.css('[role="option"]'))) {
      if ((await option.findElement(By.css('strong')).getAttribute('textContent')).trim() === label) {
        await driver.executeScript(el => el.scrollIntoView({ block: 'nearest', behavior: 'instant' }), option); await option.click(); return
      }
    }
    throw new Error('Missing visible option: ' + label)
  }
  const shot = async name => {
    await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
    const layout = await driver.executeScript(() => ({ documentOverflow: document.documentElement.scrollWidth - innerWidth,
      panels: [...document.querySelectorAll('.compression-config-panel,.compression-execution-panel,[role="dialog"]')].map(el => ({ width: el.clientWidth, overflow: el.scrollWidth - el.clientWidth })),
      columns: [...document.querySelectorAll('.compression-detail-grid')].map(el => [...el.children].map(child => child.getBoundingClientRect().width)) }))
    assert.ok(layout.documentOverflow <= 1, JSON.stringify(layout)); layout.panels.forEach(panel => assert.ok(panel.overflow <= 2, JSON.stringify(layout)))
    layout.columns.forEach(columns => assert.ok(Math.abs(columns[0] - columns[1]) <= 1, JSON.stringify(layout)))
    layouts.push({ name, ...layout }); screenshots.push(name + '.png')
    writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  const sizes = async (name, anchor) => {
    for (const [width, height] of [[1100, 700], [760, 520]]) {
      await driver.manage().window().setRect({ width, height })
      await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), await waitForElement(anchor))
      await shot(`${name}-${width}x${height}`)
    }
    await driver.manage().window().setRect({ width: 1100, height: 700 })
  }
  const groups = []
  const source = path.join(work, '原始文件')
  const nativeReportPath = path.join(artifactDirectory, 'standard-zipx-1418-native.json')
  if (!uiOnly) {
  for (const mode of ['none', 'password']) {
    console.log('[zipx-1418] original volumes: ' + mode)
    const directory = path.join(work, mode); mkdirSync(directory)
    const members = ['zx01', 'zx02', 'zipx'].map(ext => {
      const file = path.join(directory, '真实原包.' + ext); copyFileSync(path.join(original, mode, path.basename(file)), file); return file
    })
    const snapshots = members.map(file => readFileSync(file)), secret = mode === 'password' ? password : null
    independent(members[2], path.join(directory, '独立原包回解'), secret, expected, true)
    const records = []
    for (const [index, member] of members.entries()) {
      const group = await ipc('detect_split_archive', { path: member })
      assert.equal(group.archive_format, 'zipx'); assert.equal(group.first_part, members[2]); assert.deepEqual(group.parts, members)
      const browse = await ipc('browse_archive', { filePath: member, password: secret, browseId: 'zipx-' + mode + index })
      assert.equal(browse.totalFiles, 2); assert.equal(browse.totalDirectories, 2)
      const output = path.join(directory, '从第' + index + '卷解压')
      if (secret) { assert.match(await failure('extractArchive', member, output, 'wrong'), /密码|password/i); assert.deepEqual(tree(output), []) }
      await bridge('extractArchive', member, output, secret); verify(output)
      const diagnostic = await bridge('diagnoseArchive', member, secret)
      assert.equal(diagnostic.status, 'healthy', JSON.stringify(diagnostic)); assert.equal(diagnostic.integrityTested, true)
      const preview = await ipc('preview_extraction_output', { filePath: member, outputPath: directory, createSubdirectory: true })
      assert.equal(preview, path.join(directory, '真实原包'))
      const converted = path.join(directory, '转换-' + index + '.zip')
      await bridge('convertArchive', member, secret, converted, { format: 'zip', password: null })
      independent(converted, converted + '-独立回解', null)
      records.push({ member: path.basename(member), grouped: true, browsed: true, wrongRejected: !!secret, extracted: true, integrityTested: true, convertedIndependently: true, cleanOutputName: true })
    }
    const faults = []
    for (const fault of ['missing-first', 'missing-middle', 'missing-tail', 'truncated-tail']) {
      const index = fault === 'missing-first' ? 0 : fault === 'missing-middle' ? 1 : 2
      if (fault.startsWith('missing')) renameSync(members[index], members[index] + '.暂缺')
      else writeFileSync(members[index], snapshots[index].subarray(0, -1))
      try {
        for (const supplied of [null, 'wrong', password]) {
          const message = await failure('extractArchive', members[index === 0 ? 2 : 0], path.join(directory, fault + '-' + String(supplied)), supplied)
          assert.match(message, fault.startsWith('missing') ? /缺少.*ZIP.*分卷|缺.*卷/ : /损坏|不完整/)
          if (fault.startsWith('missing')) assert.ok(message.includes(path.basename(members[index])), message)
          assert.doesNotMatch(message, /密码不正确|需要输入密码|尝试.*密码/)
          faults.push({ fault, supplied: supplied === null ? 'none' : supplied === password ? 'correct' : 'wrong', finalReason: message })
        }
      } finally {
        if (fault.startsWith('missing')) renameSync(members[index] + '.暂缺', members[index]); else writeFileSync(members[index], snapshots[index])
      }
      assert.equal((await ipc('browse_archive', { filePath: members[0], password: secret, browseId: 'restored-' + fault })).totalFiles, 2)
    }
    groups.push({ mode, members, secret }); cases.push({ mode, records, faults, volumes: members.map(file => ({ name: path.basename(file), sha256: sha(file) })) })
  }
  // Creation is verified and published by the normal command, not a test writer.
  mkdirSync(source)
  await bridge('extractArchive', groups[0].members[2], source, null); verify(source)
  for (const encryption of ['none', 'aes256', 'zipcrypto']) {
    const output = path.join(work, '发给朋友-' + encryption + '.zipx'), secret = encryption === 'none' ? null : password
    const published = await ipc('compress_files', { taskId: 'zipx-create-' + encryption, files: readdirSync(source).map(name => path.join(source, name)), outputPath: output,
      options: { format: 'zipx', level: 6, preserve_paths: true, password: secret, zip_encryption: encryption === 'none' ? 'aes256' : encryption, zip_split_mode: 'standard', split_size: 65536, verify_after: true, delete_after: false } })
    assert.equal(published.primaryPath, output); assert.equal(published.files.length, 3)
    assert.ok(published.files.some(file => file.path.endsWith('.zx01')))
    independent(output, output + '-独立回解', secret)
    const restored = output + '-应用回解'; await bridge('extractArchive', output.replace(/\.zipx$/, '.zx02'), restored, secret); verify(restored)
    cases.push({ creation: encryption, files: published.files.map(file => ({ ...file, sha256: sha(file.path) })), independentReadback: true, applicationReadback: true })
  }
  // Re-encrypt/revolume a genuine AES input, then merge and remove its password.
  const changed = path.join(work, '重新分卷改密.zipx'), changedPassword = 'Changed-1418'
  await bridge('convertArchive', groups[1].members[1], password, changed, { format: 'zipx', password: changedPassword, zip_encryption: 'zipcrypto', zip_split_mode: 'standard', split_size: 65536 })
  independent(changed, changed + '-独立回解', changedPassword)
  const merged = path.join(work, '合并取消密码.zipx')
  await bridge('convertArchive', changed.replace(/\.zipx$/, '.zx01'), changedPassword, merged, { format: 'zipx', password: null, split_size: null })
  independent(merged, merged + '-独立回解', null)
  cases.push({ reconfigure: 'AES original → new ZipCrypto standard ZIPX → merged unencrypted ZIPX', independentReadback: true })
  writeFileSync(nativeReportPath, JSON.stringify({ application, applicationSha256: sha(application), cases }, null, 2))
  } else {
    const native = JSON.parse(readFileSync(nativeReportPath, 'utf8'))
    cases.push(...native.cases)
    for (const mode of ['none', 'password']) groups.push({ mode, members: ['zx01', 'zx02', 'zipx'].map(ext => path.join(work, mode, '真实原包.' + ext)), secret: mode === 'password' ? password : null })
    console.log('[zipx-1418] reuse completed unchanged native/independent matrix; continue real UI')
  }

  // UI: a real file import, explicit standard ZIPX choice, normal start button.
  await bridge('clearTasks'); await bridge('clearCompressionWorkspace'); await click('[data-testid="nav-Compress"]')
  const ui = path.join(work, '用户操作-' + Date.now()); mkdirSync(ui)
  const uiFile = path.join(ui, '界面真实资料.bin')
  writeFileSync(uiFile, Buffer.concat(Array.from({ length: 10 }, () => readFileSync(path.join(source, '中文目录/原文件.bin')))))
  await bridge('queueDesktopDialogSelections', [[uiFile]]); await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
  await click('[data-testid="compression-draft-row"] [data-testid="compression-archive-name"]')
  await select('[data-testid="compression-format-select"]', 'ZIPX（XZ）')
  await fill('[data-testid="compression-password-input"]', password)
  await fill('[data-testid="compression-filename-input"]', '用户实际创建')
  await fill('[data-testid="compression-output-path"]', ui)
  await click('[data-testid="compression-split-toggle"]'); await fill('[data-testid="compression-split-size"]', '1')
  await select('[data-testid="compression-zip-split-mode"]', '标准 ZIPX · .zx01/.zipx')
  await sizes('zipx-create', '[data-testid="compression-zip-split-mode"]')
  await click('[data-testid="start-compression"]')
  const uiOutput = path.join(ui, '用户实际创建.zipx')
  const created = await driver.wait(async () => (await bridge('taskHistory')).find(item => item.outputPath === uiOutput && ['completed', 'failed'].includes(item.status)), 60000)
  assert.equal(created.status, 'completed', JSON.stringify(created)); assert.equal(created.executionConfig.settings.zipSplitMode, 'standard')
  assert.ok(created.archiveOutput.files.length > 1, 'The actual UI task must span multiple disks')
  independent(uiOutput, uiOutput + '-独立回解', password, [{ path: path.basename(uiFile), sha256: sha(uiFile) }])
  cases.push({ userCreated: true, historyPreservesStandardZipx: true, independentlyDecoded: true })

  // Actual page import of all three members deduplicates; missing middle fails
  // before passwords, restored original creates a retry draft and completes.
  await bridge('clearTasks'); await click('[data-testid="nav-Decompress"]')
  await bridge('queueDesktopDialogSelections', [[groups[1].members[1], groups[1].members[0], groups[1].members[2]]]); await click('.decompress-view [data-testid="dropzone-file"]')
  await driver.wait(async () => (await bridge('archiveBrowserTaskState')).length === 1, 10000)
  renameSync(groups[1].members[1], groups[1].members[1] + '.暂缺')
  let failed
  try {
    await click('[data-testid="start-decompression-queue"]')
    failed = await driver.wait(async () => { const task = (await bridge('archiveBrowserTaskState'))[0]; return task?.status === 'failed' ? task : false }, 15000)
    assert.ok(failed.error.includes('真实原包.zx02'), failed.error); assert.doesNotMatch(failed.error, /密码不正确|需要输入密码/)
    await waitForElement('[data-testid="retry-missing-volumes"]'); await sizes('zipx-missing-middle', '[data-testid="retry-missing-volumes"]')
  } finally { renameSync(groups[1].members[1] + '.暂缺', groups[1].members[1]) }
  await fill('[data-testid="retry-password"]', password); await click('[data-testid="create-retry-drafts"]')
  await driver.wait(async () => (await bridge('archiveBrowserTaskState')).length === 2, 10000)
  await click('[data-testid="start-decompression-queue"]')
  const completed = await driver.wait(async () => { const task = (await bridge('archiveBrowserTaskState'))[1]; return ['completed', 'failed'].includes(task?.status) ? task : false }, 60000)
  assert.equal(completed.status, 'completed', JSON.stringify(completed)); assert.equal((await bridge('archiveBrowserTaskState'))[0].error, failed.error)
  verify(path.join(path.dirname(groups[1].members[0]), '真实原包'))
  await shot('zipx-restored-completed')
  cases.forEach(item => { if (item.volumes) item.volumes.forEach(volume => assert.equal(sha(path.join(original, item.mode, volume.name)), volume.sha256)) })
  writeFileSync(path.join(artifactDirectory, 'standard-zipx-1418-results.json'), JSON.stringify({ checkedAt: new Date().toISOString(), application, applicationSha256: sha(application), nativeApplicationSha256: JSON.parse(readFileSync(nativeReportPath, 'utf8')).applicationSha256, originalToolSha256: sha(tool), independentOutputDecoder: winrar, independentOutputDecoderSha256: sha(winrar), cases, layouts, screenshots }, null, 2))
}

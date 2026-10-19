import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { spawn } from 'node:child_process'
import { createReadStream, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runZip641515Gate({ driver, bridge, waitForElement, artifactDirectory, application, root, bundledSevenZip }) {
  const work = path.join(root, 'test-results/zip64-1515')
  const manifest = JSON.parse(readFileSync(path.join(work, 'source.json'), 'utf8'))
  const native = JSON.parse(readFileSync(path.join(work, 'native-results.json'), 'utf8'))
  const original = JSON.parse(readFileSync(path.join(work, 'originals.json'), 'utf8'))
  const ui = path.join(work, 'ui-' + randomUUID()); mkdirSync(ui)
  const password = 'ZIP64-test-password + !', cases = [], screenshots = [], layouts = []
  const sha = async file => { const h = createHash('sha256'); for await (const b of createReadStream(file)) h.update(b); return h.digest('hex') }
  await driver.manage().setTimeouts({ script: 600000 })
  const invoke = (cmd, args) => driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});', cmd, args)
  const ipc = async (cmd, args) => { const r = await invoke(cmd, args); assert.equal(r.ok, true, r.error); return r.value }
  const click = async selector => {
    for (const toast of await driver.findElements(By.css('.toast-item button'))) if (await toast.isDisplayed()) await toast.click()
    const e = typeof selector === 'string' ? await waitForElement(selector) : selector
    await driver.wait(() => driver.executeScript(e => { e.scrollIntoView({ block: 'center', behavior: 'instant' }); const r = e.getBoundingClientRect(), h = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return r.width > 0 && r.height > 0 && !e.disabled && (h === e || e.contains(h)) }, e), 10000)
    await e.click()
  }
  const fill = async (selector, value) => { const e = await waitForElement(selector); await e.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE); if (value) await e.sendKeys(value) }
  const select = async (selector, label) => {
    await click(selector)
    for (const e of await driver.findElements(By.css('[role="option"]'))) {
      const strong = await e.findElements(By.css('strong'))
      if ((await (strong[0] || e).getAttribute('textContent')).trim() === label) { await click(e); return }
    }
    assert.fail('Missing option ' + label)
  }
  const sizes = async (name, anchor) => {
    for (const [width, height] of [[1100, 700], [760, 520]]) {
      await driver.manage().window().setRect({ width, height })
      await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), await waitForElement(anchor))
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
      const geometry = await driver.executeScript(() => ({ width: innerWidth, height: innerHeight, overflow: document.documentElement.scrollWidth - innerWidth }))
      assert.ok(geometry.overflow <= 1, JSON.stringify(geometry)); layouts.push({ name, ...geometry })
      const file = `${name}-${width}x${height}.png`
      writeFileSync(path.join(artifactDirectory, file), Buffer.from(await driver.takeScreenshot(), 'base64')); screenshots.push(file)
    }
  }
  const options = { preserve_paths: true, overwrite_existing: false, delete_after: false, preserve_timestamps: true, skip_corrupted: false, create_subdirectory: false, conflict_policy: 'rename', enable_bruteforce: false, bruteforce_wordlists: [] }
  const checkpoint = path.join(work, 'application-native.json')
  if (!existsSync(checkpoint)) {
    for (const c of original.cases) {
      console.log('[zip64-1515] application original', c.mode)
      assert.equal(await sha(c.file), c.sha256)
      const secret = c.mode.startsWith('plain') ? null : password
      const browse = await ipc('browse_archive', { filePath: c.file, password: secret })
      assert.equal(browse.entries.find(e => e.path === manifest.files[0].name).size, manifest.files[0].bytes)
      await ipc('test_archive_integrity', { filePath: c.file, password: secret })
      const out = path.join(ui, '原包所选-' + c.mode)
      await ipc('extract_file', { taskId: randomUUID(), filePath: c.file, outputPath: out, password: secret, options: { ...options, selected_entries: [manifest.files[1].name] } })
      assert.equal(await sha(path.join(out, manifest.files[1].name)), manifest.files[1].sha256)
      assert.ok(!existsSync(path.join(out, manifest.files[0].name)))
      cases.push({ original: c.mode, fullIntegrity: true, selectedTailHash: true, unselectedLargeFileNotWritten: true })
    }
    for (const c of native.filter(c => c.layout !== 'single')) {
      const member = c.parts[1]
      const browse = await ipc('browse_archive', { filePath: member, password: c.mode === 'plain' ? null : password })
      assert.equal(browse.entries.find(e => e.path === manifest.files[0].name).size, manifest.files[0].bytes)
      const out = path.join(ui, `${c.mode}-${c.layout}-末尾`)
      await ipc('extract_file', { taskId: randomUUID(), filePath: member, outputPath: out, password: c.mode === 'plain' ? null : password, options: { ...options, selected_entries: [manifest.files[1].name] } })
      assert.equal(await sha(path.join(out, manifest.files[1].name)), manifest.files[1].sha256)
      cases.push({ product: c.mode, layout: c.layout, anyMemberBrowse: true, selectedTailHash: true })
    }
    writeFileSync(checkpoint, JSON.stringify(cases, null, 2))
  } else { cases.push(...JSON.parse(readFileSync(checkpoint, 'utf8'))); console.log('[zip64-1515] reuse completed application command cases') }

  // Actual browser and text preview seek to a local header beyond 4 GiB.
  const standard = native.find(c => c.mode === 'plain' && c.layout === 'standard')
  await click('[data-testid="nav-ArchiveBrowser"]'); await bridge('queueDesktopDialogSelections', [[standard.source]])
  await click('[data-testid="file-manager-open-archive"]'); await waitForElement('.browser-summary')
  await sizes('zip641515-browser', '.browser-summary')
  await fill('input[placeholder="搜索文件名或路径"]', manifest.files[1].name)
  await click(`[data-entry-path="${manifest.files[1].name}"] .preview-trigger`)
  assert.match(await (await waitForElement('[data-testid="archive-text-preview"]')).getText(), /大文件后面的末尾说明不能丢失/)
  await sizes('zip641515-tail-preview', '[data-testid="archive-text-preview"]'); await click('.preview-close')
  cases.push({ actualBrowser: true, actualTextPreviewBeyond4GiB: true })

  // Actual conversion reads every byte, changes the password, splits, verifies
  // and publishes through the normal UI. No completion state is injected.
  await click('[data-testid="nav-Compress"]'); await click('[data-testid="open-archive-conversion"]')
  await bridge('queueDesktopDialogSelections', [[original.cases.find(c => c.mode === 'aes').file]])
  await click('[data-testid="conversion-choose-source"]'); await waitForElement('[data-testid="conversion-item"]')
  if (!(await driver.findElements(By.css('[data-testid="conversion-details"]'))).length) await click('[data-testid="conversion-row"]')
  await select('[data-testid="conversion-format"]', 'ZIP')
  await fill('[data-testid="conversion-source-password"]', password)
  const nextPassword = 'ZIP64-new-password + !', output = path.join(ui, '大文件改密标准分卷.zip')
  await fill('[data-testid="conversion-password"]', nextPassword)
  await fill('[data-testid="conversion-output"]', output)
  await fill('[data-testid="conversion-split"]', '16')
  await select('[data-testid="conversion-zip-split-mode"]', '标准 ZIP · .z01/.zip')
  await sizes('zip641515-conversion-config', '[data-testid="conversion-source-password"]')
  await click('[data-testid="conversion-start"]')
  const record = await driver.wait(async () => (await bridge('taskHistory')).find(h => h.output === output && ['completed', 'failed', 'cancelled'].includes(h.status)) || false, 600000)
  assert.equal(record.status, 'completed', JSON.stringify(record))
  for (const f of manifest.files) {
    const got = await new Promise((resolve, reject) => {
      const h = createHash('sha256'); let bytes = 0, stderr = ''
      const child = spawn(bundledSevenZip, ['x', '-so', '-spd', '-p' + nextPassword, output, f.name], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] })
      child.stdout.on('data', b => { bytes += b.length; h.update(b) }); child.stderr.on('data', b => { stderr += b.toString() })
      child.once('error', reject); child.once('close', code => code === 0 ? resolve({ bytes, sha256: h.digest('hex') }) : reject(new Error(stderr)))
    })
    assert.equal(got.bytes, f.bytes); assert.equal(got.sha256, f.sha256)
  }
  await sizes('zip641515-conversion-completed', '[data-testid="conversion-item"]')
  cases.push({ actualConversion: true, fullLargePayloadHash: true, changedPassword: true, standardVolumes: true, independentReadback: true, output })
  writeFileSync(path.join(artifactDirectory, 'zip64-1515-results.json'), JSON.stringify({ version: '1.5.15', application, binarySha256: await sha(application), work, ui, cases, screenshots, layouts }, null, 2))
}

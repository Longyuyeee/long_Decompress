import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By } from 'selenium-webdriver'

const sha = file => createHash('sha256').update(readFileSync(file)).digest('hex')

// Ordinary numeric volumes made by the original HaoZip CLI. Never rename them
// to .haozipNN.7z or count this browser regression as forum-format acceptance.
export async function runSevenZipBrowser1417Gate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, application, root }) {
  const directory = path.join(fixtureDirectory, '好压数字卷浏览缺尾卷')
  mkdirSync(directory)
  const source = path.join(directory, '内容.bin')
  const bytes = Buffer.alloc(163871)
  for (let offset = 0; offset < bytes.length; offset += 32) {
    createHash('sha256').update(`browser-1417-${offset}`).digest().copy(bytes, offset, 0, Math.min(32, bytes.length - offset))
  }
  writeFileSync(source, bytes)
  const encoder = path.join(root, 'test-results/haozip-tools/portable/HaoZipC.exe')
  const password = 'ForumTest-1411'
  const cases = []
  const invoke = (filePath, password) => driver.executeAsyncScript(`
    const [filePath,password,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));
    const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
    window['_'+ids[0]]=value=>{clean();done({ok:true,value})};
    window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};
    window.__TAURI_IPC__({cmd:'browse_archive',filePath,password,browseId:'browser-1417-'+crypto.randomUUID(),callback:ids[0],error:ids[1]});
  `, filePath, password)
  const screenshots = []
  for (const mode of ['plain', 'content', 'header']) {
    const archive = path.join(directory, `${mode}.7z`)
    const args = ['a', '-t7z', '-mx=0', '-v64k', '-y', ...(mode === 'plain' ? [] : [`-p${password}`, `-mhe=${mode === 'header' ? 'on' : 'off'}`]), archive, path.basename(source)]
    const result = spawnSync(encoder, args, { cwd: directory, windowsHide: true, timeout: 30000 })
    assert.ifError(result.error)
    assert.equal(result.status, 0, String(result.stderr))
    const members = readdirSync(directory).filter(name => name.startsWith(`${mode}.7z.`)).sort().map(name => path.join(directory, name))
    assert.equal(members.length, 3)
    const originalVolumes = members.map(file => ({ name: path.basename(file), sha256: sha(file) }))
    const correct = mode === 'plain' ? null : password
    const baseline = await invoke(members[0], correct)
    assert.equal(baseline.ok, true, baseline.error)
    assert.equal(baseline.value.totalFiles, 1)
    const tail = members.at(-1)
    renameSync(tail, tail + '.temporarily-missing')
    const failures = []
    try {
      for (const supplied of [null, 'wrong', password]) {
        const missing = await invoke(members[0], supplied)
        assert.equal(missing.ok, false)
        assert.match(missing.error, /^ARCHIVE_BROWSE_INCOMPLETE\|/)
        assert.match(missing.error, /可能缺少尾卷/)
        assert.doesNotMatch(missing.error, /ARCHIVE_BROWSE_PASSWORD/)
        failures.push({ supplied: supplied === null ? 'none' : supplied === password ? 'correct' : 'wrong', error: missing.error })
      }
      if (mode === 'header') {
        await (await waitForElement('[data-testid="nav-ArchiveBrowser"]')).click()
        await bridge('queueDesktopDialogSelections', [members[0]])
        await (await waitForElement('[data-testid="file-manager-open-archive"]')).click()
        await driver.wait(async () => (await waitForElement('.browser-toolbar input[type="password"]')).isEnabled(), 15000)
        const input = await waitForElement('.browser-toolbar input[type="password"]')
        await input.sendKeys(password)
        await (await waitForElement('.browser-toolbar button[title="重新读取"]')).click()
        await driver.wait(async () => /可能缺少尾卷/.test(await (await waitForElement('[data-testid="archive-browse-notice"]')).getText()), 15000)
        for (const [width, height] of [[1100, 700], [760, 520]]) {
          await driver.manage().window().setRect({ width, height })
          const notice = await waitForElement('[data-testid="archive-browse-notice"]')
          await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), notice)
          assert.match(await notice.getText(), /可能缺少尾卷/)
          assert.doesNotMatch(await notice.getText(), /密码不正确/)
          await driver.wait(() => driver.executeScript(() => document.getAnimations().every(a => a.playState !== 'running')), 10000)
          const contained = await driver.executeScript(() => {
            const card = document.querySelector('.browser-empty').getBoundingClientRect()
            const content = document.querySelector('.browser-empty > div').getBoundingClientRect()
            return content.top >= card.top && content.bottom <= card.bottom && content.left >= card.left && content.right <= card.right
          })
          assert.equal(contained, true, `failure explanation must remain inside its card at ${width}x${height}`)
          const name = `seven-zip-browser-1417-missing-tail-${width}x${height}.png`
          writeFileSync(path.join(artifactDirectory, name), Buffer.from(await driver.takeScreenshot(), 'base64'))
          screenshots.push(name)
        }
      }
    } finally { renameSync(tail + '.temporarily-missing', tail) }
    const originalFirst = readFileSync(members[0])
    const damagedFirst = Buffer.from(originalFirst)
    damagedFirst[12] ^= 1
    writeFileSync(members[0], damagedFirst)
    const headerFailures = []
    try {
      for (const supplied of [null, 'wrong', password]) {
        const damaged = await invoke(members[0], supplied)
        assert.equal(damaged.ok, false)
        assert.match(damaged.error, /^ARCHIVE_BROWSE_DAMAGED\|7Z 归档头校验失败/)
        headerFailures.push({ supplied: supplied === null ? 'none' : supplied === password ? 'correct' : 'wrong', error: damaged.error })
      }
      if (mode === 'header') {
        await driver.manage().window().setRect({ width: 1100, height: 700 })
        for (const close of await driver.findElements(By.css('.toast-item button[aria-label="关闭错误提示"]'))) await close.click()
        await (await waitForElement('.browser-toolbar button[title="重新读取"]')).click()
        await driver.wait(async () => /归档头校验失败/.test(await (await waitForElement('[data-testid="archive-browse-notice"]')).getText()), 15000)
        await driver.wait(() => driver.executeScript(() => document.getAnimations().every(a => a.playState !== 'running')), 10000)
        const name = 'seven-zip-browser-1417-damaged-header.png'
        writeFileSync(path.join(artifactDirectory, name), Buffer.from(await driver.takeScreenshot(), 'base64'))
        screenshots.push(name)
      }
    } finally { writeFileSync(members[0], originalFirst) }
    const restored = await invoke(members[0], correct)
    assert.equal(restored.ok, true, restored.error)
    assert.equal(restored.value.totalFiles, 1)
    if (mode === 'header') {
      await driver.manage().window().setRect({ width: 1100, height: 700 })
      await (await waitForElement('.browser-toolbar button[title="重新读取"]')).click()
      await driver.wait(async () => (await driver.findElements(By.css('.browser-row:not(.directory)'))).length === 1, 15000)
      assert.match(await (await waitForElement('.browser-page')).getText(), /内容.bin/)
      // Close the earlier transient alert through its real UI control. The
      // persistent browse notice must already have cleared after a good retry.
      assert.equal((await driver.findElements(By.css('[data-testid="archive-browse-notice"]'))).length, 0)
      for (const close of await driver.findElements(By.css('.toast-item button[aria-label="关闭错误提示"]'))) await close.click()
      await driver.wait(() => driver.executeScript(() => document.getAnimations().every(a => a.playState !== 'running')), 10000)
      const name = 'seven-zip-browser-1417-restored.png'
      writeFileSync(path.join(artifactDirectory, name), Buffer.from(await driver.takeScreenshot(), 'base64'))
      screenshots.push(name)
    }
    members.forEach((file, index) => assert.equal(sha(file), originalVolumes[index].sha256))
    cases.push({ mode, originalVolumes, failures, headerFailures, restoredBrowse: true })
  }
  assert.deepEqual(readFileSync(source), bytes)
  writeFileSync(path.join(artifactDirectory, 'seven-zip-browser-1417-results.json'), JSON.stringify({ checkedAt: new Date().toISOString(), application, binarySha256: sha(application), encoder, encoderSha256: sha(encoder), sourceBytes: bytes.length, sourceSha256: sha(source), fixtureScope: 'Original HaoZip CLI ordinary numeric volumes, NOT forum volumes', cases, screenshots }, null, 2))
}

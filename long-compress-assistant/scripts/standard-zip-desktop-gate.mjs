import assert from 'node:assert/strict'
import { readFileSync, writeFileSync, renameSync, existsSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'
import { makeStandardZipFixtures } from './standard-zip-fixtures.mjs'

export async function runStandardZipDesktopGate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, root }) {
  const winrar = path.join(root, 'test-results/archive-upgrade-tools/winrar/WinRAR.exe')
  const fixtures = makeStandardZipFixtures(path.join(fixtureDirectory, '收到的分卷资料'), winrar, bundledSevenZip)
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const before = fixtures.cases.flatMap(item => item.parts.map(file => ({ file, hash: hash(file) })))
  const records = []
  const invoke = async (command, args) => {
    const result = await driver.executeAsyncScript(`
      const [cmd,args,done]=arguments; const ids=crypto.getRandomValues(new Uint32Array(2));
      const cleanup=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
      window['_'+ids[0]]=value=>{cleanup();done({ok:true,value})};
      window['_'+ids[1]]=error=>{cleanup();done({ok:false,error:String(error)})};
      window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});
    `, command, args)
    assert.ok(result.ok, result.error); return result.value
  }
  const picture = async name => writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  for (const item of fixtures.cases) {
    const first = item.parts.find(file => /\.z01$/i.test(file))
    const metadata = await invoke('browse_archive', { filePath: first, password: item.password })
    assert.equal(metadata.totalFiles, 3); assert.equal(metadata.encrypted, !!item.password)
    const text = await invoke('preview_archive_text', { filePath: item.archive, entryPath: '说明.txt', password: item.password })
    assert.equal(text.content, fixtures.text)
    const image = await invoke('preview_archive_image', { filePath: first, entryPath: '像素.png', password: item.password })
    assert.equal(image.width, 1)
    if (item.password) assert.equal(await invoke('verify_archive_password', { taskId: 'verify-' + item.mode, filePath: first, password: 'wrong' }), false)
    const output = path.join(fixtureDirectory, '直接回解-' + item.mode)
    await bridge('extractArchive', first, output, item.password)
    assert.deepEqual(readFileSync(path.join(output, '中文目录/原文件.bin')), fixtures.payload)
    assert.ok(existsSync(path.join(output, '中文目录/空目录')))
    records.push({ mode: item.mode, importedMember: path.basename(first), files: metadata.totalFiles, payloadSha256: hash(path.join(output, '中文目录/原文件.bin')) })
  }

  // "我收到 ZIP 和 z01，先看看包里的说明。"
  await (await waitForElement('[data-testid="nav-ArchiveBrowser"]')).click()
  await bridge('queueDesktopDialogSelections', [fixtures.cases[0].parts.find(file => /\.z01$/.test(file))])
  await (await waitForElement('[data-testid="file-manager-open-archive"]')).click()
  await (await waitForElement('.browser-page > header .browser-primary')).click()
  await waitForElement('[data-entry-path="说明.txt"]')
  await (await driver.findElement(By.css('[data-entry-path="说明.txt"] .preview-trigger'))).click()
  await driver.wait(async () => (await (await waitForElement('[data-testid="archive-text-preview"]')).getText()).includes('我要先看看'), 15_000)
  const previewVisible = await driver.executeScript(() => {
    const preview = document.querySelector('[data-testid="archive-text-preview"]'), rect = preview.getBoundingClientRect()
    const dialog = document.querySelector('.preview-dialog'), header = dialog.querySelector('header').getBoundingClientRect()
    return preview.contains(document.elementFromPoint(rect.left + 10, rect.top + 10)) && dialog.contains(document.elementFromPoint(header.left + 15, header.top + 15))
  })
  assert.equal(previewVisible, true, 'Sidebar must not cover the preview text or header')
  await picture('standard-zip-text-preview')
  await (await waitForElement('[aria-label="关闭预览"]')).click()

  // "分卷都选进来，只处理一个包；默认格式是我自己选的。"
  await (await waitForElement('[data-testid="nav-Compress"]')).click()
  await (await waitForElement('[data-testid="open-archive-conversion"]')).click()
  await (await driver.findElement(By.xpath('//button[contains(., "默认配置")]'))).click()
  await (await waitForElement('[data-testid="conversion-batch-fields"] [data-testid="conversion-format"]')).click()
  await (await driver.findElement(By.xpath('//button[@role="option" and .//strong[contains(text(), "TAR.BR")]]'))).click()
  await (await waitForElement('[data-testid="conversion-batch-apply"]')).click()
  await bridge('queueDesktopDialogSelections', [fixtures.cases.flatMap(item => [item.archive, item.parts.find(file => /\.z01$/.test(file))])])
  await (await waitForElement('[data-testid="conversion-choose-source"]')).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="conversion-item"]'))).length === 4, 15_000)
  const drafts = await driver.findElements(By.css('[data-testid="conversion-item"]'))
  for (let index = 0; index < drafts.length; index++) {
    const item = fixtures.cases[index]
    const section = await driver.findElements(By.css('[data-testid="conversion-item"]')).then(items => items[index])
    const row = await section.findElement(By.css('[data-testid="conversion-row"]'))
    if (await row.getAttribute('aria-expanded') !== 'true') await row.click()
    const format = await section.findElement(By.css('[data-testid="conversion-format"]'))
    assert.match(await format.getText(), /TAR.BR/, 'Metadata refresh must preserve user default target')
    await format.click()
    await (await driver.findElement(By.xpath('//button[@role="option" and .//strong[text()="ZIP"]]'))).click()
    const fields = await section.findElements(By.css('[data-testid="conversion-output"]'))
    await driver.wait(async () => (await format.getAttribute('aria-expanded')) === 'false', 5000)
    await fields[0].click()
    await fields[0].sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    assert.equal(await fields[0].getAttribute('value'), '', 'Cleared output stays empty during editing')
    const destination = path.join(fixtureDirectory, `合卷-${item.mode}.zip`)
    await fields[0].sendKeys(destination)
    assert.equal(await fields[0].getAttribute('value'), destination, 'Replacement path must not append to the automatic filename')
    if (item.password) await (await section.findElement(By.css('[data-testid="conversion-source-password"]'))).sendKeys(item.password)
  }
  for (const size of [{ width: 1100, height: 760 }, { width: 760, height: 620 }]) {
    await driver.manage().window().setRect(size)
    const overflow = await driver.executeScript(() => {
      const el = document.querySelector('[data-testid="archive-conversion"]'); return { width: el.clientWidth, scroll: el.scrollWidth }
    })
    assert.ok(overflow.scroll <= overflow.width + 1, JSON.stringify(overflow))
    await picture('standard-zip-conversion-' + size.width)
  }
  await (await waitForElement('[data-testid="conversion-start"]')).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="conversion-result"]'))).length === 4, 90_000)
  for (const item of fixtures.cases) {
    const output = path.join(fixtureDirectory, '合卷回解-' + item.mode)
    await bridge('extractArchive', path.join(fixtureDirectory, `合卷-${item.mode}.zip`), output, null)
    assert.deepEqual(readFileSync(path.join(output, '中文目录/原文件.bin')), fixtures.payload)
    assert.equal(readFileSync(path.join(output, '说明.txt'), 'utf8'), fixtures.text)
    assert.ok(existsSync(path.join(output, '中文目录/空目录')))
  }
  await picture('standard-zip-conversion-completed')
  const highest = fixtures.cases[0].parts.filter(file => /\.z\d+$/.test(file)).sort().at(-1)
  renameSync(highest, highest + '.暂缺')
  try {
    const group = await invoke('detect_split_archive', { path: fixtures.cases[0].archive })
    assert.equal(group.is_complete, false); assert.ok(group.missing_parts.includes(highest))
  } finally { renameSync(highest + '.暂缺', highest) }
  before.forEach(record => assert.equal(hash(record.file), record.hash))
  writeFileSync(path.join(artifactDirectory, 'standard-zip-desktop-results.json'), JSON.stringify({ version: '1.4.3', binarySha256: hash(path.join(root, 'src-tauri/target/release/long-compress-assistant.exe')), nativePickerVerified: false, testKind: 'real-webview-real-engine-picker-response-substituted', records, sourceHashes: before, defaultFormatPreserved: true, volumeDedupeVerified: true, conversionResults: 4 }, null, 2))
  if (process.env.LONG_DESKTOP_REVIEW_HOLD_MS) await new Promise(resolve => setTimeout(resolve, Number(process.env.LONG_DESKTOP_REVIEW_HOLD_MS)))
}

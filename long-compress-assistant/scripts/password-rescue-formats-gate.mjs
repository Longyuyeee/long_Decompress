import assert from 'node:assert/strict'
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, copyFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'
import { prepareReadonlyFixtures, sha256 } from './prepare-readonly-format-fixtures.mjs'

export async function runPasswordRescueFormatsGate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application }) {
  const root = path.join(fixtureDirectory, 'password-rescue-formats'); mkdirSync(root)
  const source = path.join(root, '别人发来的资料'); mkdirSync(source)
  const good = Buffer.from('这份完整的中文说明应被救回，新包仍需密码。')
  writeFileSync(path.join(source, '完整说明.txt'), good)
  writeFileSync(path.join(source, 'broken.bin'), randomBytes(160_000))
  mkdirSync(path.join(source, '空目录'))
  const password = 'rescue-password + !'
  const sevenZip = (args, cwd = source, successful = true) => {
    const result = spawnSync(bundledSevenZip, args, { cwd, windowsHide: true, encoding: 'utf8' })
    assert.ifError(result.error)
    if (successful) assert.equal(result.status, 0, result.stdout + result.stderr)
    else assert.notEqual(result.status, 0)
    return result
  }
  const invoke = async (command, args) => {
    const reply = await driver.executeAsyncScript(`const [cmd,args,done]=arguments, ids=crypto.getRandomValues(new Uint32Array(2));
      const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
      window['_'+ids[0]]=value=>{clean();done({ok:true,value})};
      window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};
      window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`, command, args)
    assert.ok(reply.ok, reply.error); return reply.value
  }
  const flipBrokenCiphertext = file => {
    const bytes = readFileSync(file), end = bytes.length - 22
    assert.equal(bytes.readUInt32LE(end), 0x06054b50)
    let at = bytes.readUInt32LE(end + 16), found = false
    while (at < end && bytes.readUInt32LE(at) === 0x02014b50) {
      const name = bytes.subarray(at + 46, at + 46 + bytes.readUInt16LE(at + 28)).toString('utf8')
      if (name === 'broken.bin') {
        const local = bytes.readUInt32LE(at + 42), data = local + 30 + bytes.readUInt16LE(local + 26) + bytes.readUInt16LE(local + 28)
        bytes[data + 50] ^= 1; found = true; break
      }
      at += 46 + bytes.readUInt16LE(at + 28) + bytes.readUInt16LE(at + 30) + bytes.readUInt16LE(at + 32)
    }
    assert.ok(found); writeFileSync(file, bytes)
  }
  const rescueRecords = []
  for (const mode of ['AES256', 'ZipCrypto']) {
    const input = path.join(root, `部分损坏-${mode}.zip`)
    sevenZip(['a', '-tzip', '-mm=Copy', `-mem=${mode}`, `-p${password}`, input, '.\\*'])
    flipBrokenCiphertext(input)
    const before = sha256(input)
    const report = await bridge('diagnoseArchive', input, password)
    assert.ok(report.canRepair)
    const output = path.join(root, `救回-${mode}.zip`)
    assert.match(await failure('repairZip', input, output, 'wrong'), /密码|完整文件|complete file/i)
    assert.equal(existsSync(output), false)
    const result = await bridge('repairZip', input, output, password)
    assert.ok(result.verified && result.outputEncrypted && result.partial)
    assert.equal(result.recoveredFiles, 1)
    sevenZip(['t', output, `-p${password}`]); sevenZip(['t', output, '-pwrong'], source, false)
    const restored = path.join(root, '回解-' + mode)
    sevenZip(['x', output, `-p${password}`, `-o${restored}`, '-y'])
    assert.deepEqual(readFileSync(path.join(restored, '完整说明.txt')), good)
    assert.equal(existsSync(path.join(restored, 'broken.bin')), false)
    assert.ok(existsSync(path.join(restored, '空目录')))
    assert.equal(sha256(input), before)
    rescueRecords.push({ mode, sourceSha256: before, outputEncrypted: true, recoveredFiles: result.recoveredFiles, originalPreserved: true })
  }
  await (await waitForElement('[data-testid="nav-FileIntegrity"]')).click()
  await (await waitForElement('[data-testid="archive-diagnostic-mode"]')).click()
  const selected = path.join(root, '部分损坏-AES256.zip')
  await bridge('queueDesktopDialogSelections', [selected])
  await (await waitForElement('.archive-select')).click()
  const fillPassword = async value => {
    const input = await waitForElement('.archive-password')
    await input.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE, value)
  }
  await fillPassword('wrong')
  await (await waitForElement('.archive-primary')).click()
  await driver.wait(async () => (await (await waitForElement('[data-testid="diagnostic-report"]')).getText()).includes('密码错误'), 15_000)
  await fillPassword(password)
  assert.equal((await driver.findElements(By.css('[data-testid="diagnostic-report"]'))).length, 0)
  await (await waitForElement('.archive-primary')).click()
  await waitForElement('[data-testid="repair-password-note"]')
  const repairButton = () => driver.findElement(By.xpath('//button[contains(.,"选择位置并修复")]'))
  await bridge('queueDesktopDialogSelections', [null]); await (await repairButton()).click()
  const uiOutput = path.join(root, '页面救回的新包.zip')
  await bridge('queueDesktopDialogSelections', [uiOutput]); await (await repairButton()).click()
  const recovered = await waitForElement('[data-testid="repair-result"]')
  assert.match(await recovered.getText(), /部分恢复/)
  assert.match(await (await waitForElement('[data-testid="repair-output-encryption"]')).getText(), /AES-256/)
  assert.ok(!(await recovered.getText()).includes(password))
  for (const width of [1100, 760]) {
    await driver.manage().window().setRect({ width, height: 820 })
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', recovered)
    const bounds = await driver.executeScript(element => ({ client: element.clientWidth, scroll: element.scrollWidth, viewport: innerWidth }), recovered)
    assert.ok(bounds.scroll <= bounds.client + 1, JSON.stringify(bounds))
    writeFileSync(path.join(artifactDirectory, `password-rescue-${width}.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  const cancelSource = path.join(root, '取消来源'); mkdirSync(cancelSource)
  writeFileSync(path.join(cancelSource, '完整的大文件.bin'), randomBytes(96 * 1048576))
  writeFileSync(path.join(cancelSource, 'broken.bin'), randomBytes(4096))
  const cancelArchive = path.join(root, '选错的大密码包.zip')
  sevenZip(['a', '-tzip', '-mm=Copy', '-mem=AES256', `-p${password}`, cancelArchive, '.\\*'], cancelSource)
  flipBrokenCiphertext(cancelArchive)
  await bridge('queueDesktopDialogSelections', [cancelArchive])
  await (await waitForElement('.archive-select')).click()
  await (await waitForElement('.archive-primary')).click()
  await waitForElement('[data-testid="repair-password-note"]')
  const cancelOutput = path.join(root, '取消不应留下新包.zip')
  await bridge('queueDesktopDialogSelections', [cancelOutput]); await (await repairButton()).click()
  const cancel = await driver.wait(async () => (await driver.findElements(By.xpath('//button[contains(.,"取消修复")]')))[0] || false, 5000)
  assert.equal(await (await waitForElement('.archive-password')).isEnabled(), false)
  await cancel.click()
  await driver.wait(async () => (await driver.findElements(By.xpath('//button[contains(.,"取消修复")]'))).length === 0, 30000)
  assert.equal(existsSync(cancelOutput), false)
  const capabilities = await invoke('get_archive_engine_capabilities', {})
  for (const extension of ['arj', 'z', 'udf']) assert.ok(capabilities.browseExtensions.includes(extension))
  const formats = []
  for (const fixture of prepareReadonlyFixtures()) {
    const input = path.join(root, fixture.name); copyFileSync(fixture.file, input)
    const metadata = await invoke('browse_archive', { browseId: 'readonly-' + fixture.name, filePath: input, password: null })
    const entry = metadata.entries.find(entry => entry.path === fixture.entry)
    assert.ok(entry, JSON.stringify(metadata))
    assert.equal(metadata.totalFiles, 1)
    if (fixture.name.endsWith('.udf')) {
      assert.equal(metadata.totalDirectories, 1)
      assert.ok(metadata.entries.find(entry => entry.path === 't').isDir)
    }
    if (fixture.name.endsWith('.Z')) assert.equal(entry.sizeKnown, false)
    const destination = path.join(root, '直接解压-' + fixture.name)
    await bridge('extractArchive', input, destination, null)
    assert.deepEqual(readFileSync(path.join(destination, ...fixture.entry.split('/'))), Buffer.from(fixture.contents))
    const converted = path.join(root, '转换-' + fixture.name + '.zip')
    await bridge('convertArchive', input, null, converted, { format: 'zip', password: null })
    const output = path.join(root, '转换回解-' + fixture.name)
    await bridge('extractArchive', converted, output, null)
    assert.deepEqual(readFileSync(path.join(output, ...fixture.entry.split('/'))), Buffer.from(fixture.contents))
    assert.equal(sha256(input), fixture.sha256)
    formats.push({ name: fixture.name, sourceUrl: fixture.url, sourceSha256: fixture.sha256, payloadBytes: Buffer.byteLength(fixture.contents), browser: true, extract: true, convert: true })
  }
  // Actual browser page for the stream's unknown size (no mock file metadata).
  await (await waitForElement('[data-testid="nav-ArchiveBrowser"]')).click()
  await bridge('queueDesktopDialogSelections', [path.join(root, 't.txt.Z')])
  await (await waitForElement('[data-testid="file-manager-open-archive"]')).click()
  const row = await waitForElement('[data-entry-path="t.txt"]')
  assert.match(await row.getText(), /t.txt/)
  assert.match(await (await waitForElement('.browser-summary')).getText(), /大小未知/)
  writeFileSync(path.join(artifactDirectory, 'readonly-z-browser.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  for (const name of ['t.udf', 'compat_method1.arj']) {
    await bridge('queueDesktopDialogSelections', [path.join(root, name)])
    await (await waitForElement('.browser-page > header .browser-primary')).click()
    await driver.wait(async () => (await (await waitForElement('.browser-summary')).getText()).includes(name.endsWith('.udf') ? 'UDF' : 'ARJ'), 15000)
    writeFileSync(path.join(artifactDirectory, `readonly-${name.endsWith('.udf') ? 'udf' : 'arj'}-browser.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  assert.deepEqual(readdirSync(root).filter(name => name.startsWith('.long-repair')), [])
  writeFileSync(path.join(artifactDirectory, 'password-rescue-formats-results.json'), JSON.stringify({
    binarySha256: sha256(application), testKind: 'isolated-visible-windows-release-native-commands', nativePickerVerified: false,
    rescueRecords, wrongPasswordThenCorrection: true, partialAndEncryptedOutputVisible: true, cancelledSaveProducesNoOutput: true,
    runningEncryptedRecoveryCancellation: true, formats,
  }, null, 2))
}

import assert from 'node:assert/strict'
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'
import { makeStandardZipFixtures } from './standard-zip-fixtures.mjs'

// Real Windows UI and engine. The established test bridge supplies only the
// picker response; diagnosis and archive data are never mocked.
export async function runArchiveDiagnosisReliabilityGate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, root, application }) {
  const fixtures = makeStandardZipFixtures(path.join(fixtureDirectory, '诊断一致性'),
    path.join(root, 'test-results/archive-upgrade-tools/winrar/WinRAR.exe'), bundledSevenZip)
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const originals = fixtures.cases.flatMap(item => item.parts.map(file => ({ file, sha256: hash(file) })))
  const records = []
  const invoke = async (command, args) => {
    const result = await driver.executeAsyncScript(`
      const [cmd,args,done]=arguments, ids=crypto.getRandomValues(new Uint32Array(2));
      const cleanup=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
      window['_'+ids[0]]=value=>{cleanup();done({ok:true,value})};
      window['_'+ids[1]]=error=>{cleanup();done({ok:false,error:String(error)})};
      window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});
    `, command, args)
    assert.ok(result.ok, result.error)
    return result.value
  }
  const diagnose = (filePath, password = null) => invoke('diagnose_archive', {
    diagnosticId: 'reliability-' + Date.now(), filePath, password,
  })
  for (const item of fixtures.cases) {
    const selected = item.parts.find(file => /\.z01$/i.test(file))
    const report = await diagnose(selected, item.password)
    assert.equal(report.status, 'healthy', JSON.stringify(report))
    assert.equal(report.actualFormat, 'ZIP')
    assert.equal(report.totalFiles, 3)
    assert.equal(report.volumesFound, item.parts.length)
    assert.equal(report.integrityTested, true)
    records.push({ scenario: '收到标准分卷-' + item.mode, report })
  }

  const plain = fixtures.cases[0]
  const highest = plain.parts.filter(file => /\.z\d+$/i.test(file)).sort().at(-1)
  const incompleteDir = path.join(fixtureDirectory, '下载少了一卷')
  mkdirSync(incompleteDir)
  for (const file of plain.parts.filter(file => file !== highest)) copyFileSync(file, path.join(incompleteDir, path.basename(file)))
  const incomplete = path.join(incompleteDir, path.basename(plain.archive))
  const missing = await diagnose(incomplete)
  assert.equal(missing.status, 'missing_volume')
  assert.deepEqual(missing.missingVolumes, [path.basename(highest)])
  assert.equal(missing.integrityTested, false)
  records.push({ scenario: '缺少最后编号卷', report: missing })

  const aes = fixtures.cases.find(item => item.mode === 'aes')
  const corruptDir = path.join(fixtureDirectory, '密码正确但收到坏包')
  mkdirSync(corruptDir)
  for (const file of aes.parts) copyFileSync(file, path.join(corruptDir, path.basename(file)))
  const corruptFirst = path.join(corruptDir, path.basename(aes.parts.find(file => /\.z01$/i.test(file))))
  const damaged = readFileSync(corruptFirst)
  let payload = -1
  for (let offset = 0; offset + 30 < damaged.length; offset++) {
    if (damaged.readUInt32LE(offset) === 0x04034b50 && damaged.readUInt16LE(offset + 8) === 99) {
      payload = offset + 30 + damaged.readUInt16LE(offset + 26) + damaged.readUInt16LE(offset + 28)
      break
    }
  }
  assert.ok(payload >= 0 && payload + 25 < damaged.length, 'Fixture must contain an actual AES entry')
  damaged[payload + 25] ^= 1
  writeFileSync(corruptFirst, damaged)
  const corruptArchive = path.join(corruptDir, path.basename(aes.archive))
  const independentlyBad = spawnSync(bundledSevenZip, ['t', corruptArchive, '-p' + aes.password, '-y'], { windowsHide: true, encoding: 'utf8' })
  assert.notEqual(independentlyBad.status, 0, 'Independent engine must also reject the damaged content')
  const corruptBefore = hash(corruptFirst)
  const ambiguous = await diagnose(corruptArchive, aes.password)
  assert.equal(ambiguous.status, 'ambiguous_data', JSON.stringify(ambiguous))
  assert.equal(ambiguous.integrityTested, true)
  assert.equal(ambiguous.canRepair, false)
  assert.ok(ambiguous.issues.some(issue => issue.detail.includes('反复尝试密码')))
  records.push({ scenario: '正确密码但密文损坏', independentEngineRejected: true, report: ambiguous })

  await (await waitForElement('[data-testid="nav-FileIntegrity"]')).click()
  await (await waitForElement('[data-testid="archive-diagnostic-mode"]')).click()
  const select = async file => {
    await bridge('queueDesktopDialogSelections', [file])
    await (await waitForElement('.archive-select')).click()
    await driver.wait(async () => (await (await waitForElement('.archive-select')).getText()).includes(path.basename(file)), 5000)
  }
  const runPage = async (password, title) => {
    const field = await waitForElement('.archive-password')
    await field.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (password) await field.sendKeys(password)
    await (await waitForElement('.archive-primary')).click()
    await driver.wait(async () => (await driver.findElements(By.css('[data-testid="diagnostic-report"] h2'))).length > 0
      && (await driver.findElement(By.css('[data-testid="diagnostic-report"] h2')).getText()) === title, 30_000)
    return (await waitForElement('[data-testid="diagnostic-report"]')).getText()
  }
  const picture = async name => writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  // “少了一卷？告诉我是哪一个，我补回来再检查。”
  await select(incomplete)
  assert.ok((await runPage('', '缺少分卷')).includes(path.basename(highest)))
  await picture('diagnosis-missing-volume')
  copyFileSync(highest, path.join(incompleteDir, path.basename(highest)))
  await runPage('', '完整性正常')
  // “密码我确认是对的，不要只让我继续换密码。”
  await select(corruptArchive)
  assert.ok((await runPage(aes.password, '密码或加密内容异常')).includes('反复尝试密码不能修复损坏'))
  assert.equal((await driver.findElements(By.xpath('//button[contains(., "选择位置并修复")]'))).length, 0)
  for (const width of [1100, 760]) {
    await driver.manage().window().setRect({ width, height: 760 })
    const overflow = await driver.executeScript(() => {
      const panel = document.querySelector('[data-testid="archive-diagnostic-panel"]')
      const picker = panel.querySelector('.archive-select')
      return { width: panel.clientWidth, scroll: panel.scrollWidth,
        pickerHeight: picker.getBoundingClientRect().height,
        rootFont: parseFloat(getComputedStyle(document.documentElement).fontSize) }
    })
    assert.ok(overflow.scroll <= overflow.width + 1, JSON.stringify(overflow))
    assert.ok(overflow.pickerHeight <= overflow.rootFont * 6, 'File picker must remain a compact control: ' + JSON.stringify(overflow))
    await picture('diagnosis-encrypted-damage-' + width)
    await driver.executeScript(() => document.querySelector('[data-testid="diagnostic-report"]').scrollIntoView({ block: 'center' }))
    await picture('diagnosis-encrypted-detail-' + width)
  }
  // “拿到重新下载的完整包，同一个密码就能通过。”
  await select(aes.archive)
  await runPage(aes.password, '完整性正常')
  await runPage('wrong-password', '密码错误')
  await picture('diagnosis-wrong-password')
  assert.equal(hash(corruptFirst), corruptBefore)
  for (const item of originals) assert.equal(hash(item.file), item.sha256)
  writeFileSync(path.join(artifactDirectory, 'diagnosis-reliability-results.json'), JSON.stringify({
    version: 'development-after-1.4.3', binarySha256: hash(application),
    testKind: 'real-webview-real-engine-picker-response-substituted', nativePickerVerified: false,
    records, sourceHashes: originals, missingVolumeThenSupplementVerified: true, damagedThenReplacementVerified: true,
  }, null, 2))
  if (process.env.LONG_DESKTOP_REVIEW_HOLD_MS) await new Promise(resolve => setTimeout(resolve, Number(process.env.LONG_DESKTOP_REVIEW_HOLD_MS)))
}

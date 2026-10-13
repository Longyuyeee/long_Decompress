import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, existsSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { createHash, randomBytes } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { By } from 'selenium-webdriver'

export async function runZipRecoveryDesktopGate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application }) {
  const root = path.join(fixtureDirectory, 'zip-recovery'); mkdirSync(root)
  const folder = path.join(root, '别人发来的资料'); mkdirSync(folder)
  const good = Buffer.from('我确认这份说明完整，请不要因为另一个文件坏了就丢掉它。')
  writeFileSync(path.join(folder,'说明.txt'), good)
  writeFileSync(path.join(folder,'另一个完整文件.txt'), '这个文件也应保留')
  writeFileSync(path.join(folder,'损坏文件.bin'), randomBytes(90000))
  const original = path.join(root, '完整来源.zip')
  const encode = (output, source, password) => {
    const result = spawnSync(bundledSevenZip, ['a','-tzip','-mm=Copy',...(password ? [`-p${password}`,'-mem=AES256'] : []),output,'.\\*'], { cwd:source, encoding:'utf8', windowsHide:true })
    assert.equal(result.status,0,result.stdout+result.stderr)
  }
  encode(original,folder)
  const bytes = readFileSync(original), central = bytes.readUInt32LE(bytes.length-6)
  assert.equal(bytes.readUInt32LE(bytes.length-22),0x06054b50)
  const lost = Buffer.from(bytes.subarray(0,central))
  let badData
  for (let at=0;at<lost.length;) {
    assert.equal(lost.readUInt32LE(at),0x04034b50)
    assert.equal(lost.readUInt16LE(at+6)&9,0,'Independent plain source must have trusted local sizes')
    const nameLength=lost.readUInt16LE(at+26), extraLength=lost.readUInt16LE(at+28)
    const name=lost.subarray(at+30,at+30+nameLength).toString('utf8'), data=at+30+nameLength+extraLength
    if (name.endsWith('.bin') && lost.readUInt32LE(at+18) === 90000) badData=data
    at=data+lost.readUInt32LE(at+18)
  }
  assert.ok(badData !== undefined)
  lost[badData+17]^=1
  const damaged = path.join(root,'目录丢了而且一个文件坏了.zip'); writeFileSync(damaged,lost)
  const hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex')
  const before=hash(damaged)
  const report=await bridge('diagnoseArchive',damaged)
  assert.equal(report.canRepair,true)
  const encrypted=path.join(root,'密码目录丢失.zip'); encode(encrypted,folder,'password')
  const encryptedBytes=readFileSync(encrypted)
  writeFileSync(encrypted,encryptedBytes.subarray(0,encryptedBytes.readUInt32LE(encryptedBytes.length-6)))
  const encryptedReport=await bridge('diagnoseArchive',encrypted)
  assert.equal(encryptedReport.canRepair,false)
  const encryptedOutput=path.join(root,'不应出现明文.zip')
  assert.match(await failure('repairZip',encrypted,encryptedOutput),/密码 ZIP/)
  assert.equal(existsSync(encryptedOutput),false)
  await (await waitForElement('[data-testid="nav-FileIntegrity"]')).click()
  await (await waitForElement('[data-testid="archive-diagnostic-mode"]')).click()
  const select=async file=>{
    await bridge('queueDesktopDialogSelections',[file])
    await (await waitForElement('.archive-select')).click()
    await (await waitForElement('.archive-primary')).click()
    await waitForElement('[data-testid="diagnostic-report"]')
  }
  await select(damaged)
  const repairButton=()=>driver.findElement(By.xpath('//button[contains(.,"选择位置并修复")]'))
  // “先点了另存为又取消，别产生任何新文件。”
  await bridge('queueDesktopDialogSelections',[null])
  await (await repairButton()).click()
  assert.equal((await driver.findElements(By.css('[data-testid="repair-result"]'))).length,0)
  const output=path.join(root,'救回的完整文件.zip')
  await bridge('queueDesktopDialogSelections',[output])
  await (await repairButton()).click()
  const result=await waitForElement('[data-testid="repair-result"]')
  assert.match(await result.getText(),/部分恢复/)
  assert.match(await result.getText(),/原包总条目数无法确认/)
  assert.match(await result.getText(),/损坏文件/)
  const partialToast=await waitForElement('.toast-item:has(button[aria-label="关闭警告提示"])')
  await driver.wait(async()=>(await partialToast.getText()).includes('部分恢复'),5000)
  assert.match(await partialToast.getText(),/部分恢复/)
  for (const width of [1100,760]) {
    await driver.manage().window().setRect({width,height:760})
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})',result)
    const bounds=await driver.executeScript(element=>({width:element.clientWidth,scroll:element.scrollWidth,viewport:innerWidth}),result)
    assert.ok(bounds.scroll<=bounds.width+1,JSON.stringify(bounds))
    writeFileSync(path.join(artifactDirectory,`zip-recovery-${width}.png`),Buffer.from(await driver.takeScreenshot(),'base64'))
  }
  const restored=path.join(root,'打开恢复结果')
  const checked=spawnSync(bundledSevenZip,['x','-y',`-o${restored}`,output],{encoding:'utf8',windowsHide:true})
  assert.equal(checked.status,0,checked.stdout+checked.stderr)
  assert.deepEqual(readFileSync(path.join(restored,'说明.txt')),good)
  assert.equal(readFileSync(path.join(restored,'另一个完整文件.txt'),'utf8'),'这个文件也应保留')
  assert.equal(existsSync(path.join(restored,'损坏文件.bin')),false)
  assert.equal(hash(damaged),before)
  const healthy=await bridge('diagnoseArchive',output)
  assert.equal(healthy.status,'healthy')
  const existing=path.join(root,'用户已有文件.zip');writeFileSync(existing,'保留这个文件')
  assert.match(await failure('repairZip',damaged,existing),/already exists/)
  assert.equal(readFileSync(existing,'utf8'),'保留这个文件')
  // “选错了一个大包，处理中取消，不要留下半个恢复包。”
  const cancelSource=path.join(root,'cancel-source');mkdirSync(cancelSource)
  writeFileSync(path.join(cancelSource,'大文件.bin'),randomBytes(96*1048576))
  const cancelArchive=path.join(root,'大包中断.zip');encode(cancelArchive,cancelSource)
  const cancelBytes=readFileSync(cancelArchive)
  const cancelCentral=cancelBytes.readUInt32LE(cancelBytes.length-6)
  writeFileSync(cancelArchive,cancelBytes.subarray(0,cancelCentral))
  await select(cancelArchive)
  const cancelOutput=path.join(root,'取消不应发布.zip')
  await bridge('queueDesktopDialogSelections',[cancelOutput])
  await (await repairButton()).click()
  const cancel=await driver.wait(async()=>(await driver.findElements(By.xpath('//button[contains(.,"取消修复")]')))[0]||false,5000)
  await cancel.click()
  await driver.wait(async()=>(await driver.findElements(By.xpath('//button[contains(.,"取消修复")]'))).length===0,30000)
  assert.equal(existsSync(cancelOutput),false)
  assert.deepEqual(readdirSync(root).filter(name=>name.startsWith('.long-repair')),[])
  writeFileSync(path.join(artifactDirectory,'zip-recovery-results.json'),JSON.stringify({binarySha256:hash(application),testKind:'real-visible-release-desktop-native-engine-picker-response-substituted',nativePickerVerified:false,
    lostDirectoryAndCorruptEntryRecovered:true,onlyTwoIntactFilesPublished:true,partialResultVisible:true,unknownOriginalTotalVisible:true,independentSevenZipOpenedResult:true,
    sourcePreserved:true,passwordRecoveryNotClaimed:true,saveCancellationProducesNoOutput:true,runningCancellationProducesNoOutput:true,existingOutputPreserved:true},null,2))
}

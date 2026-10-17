import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runZipPartial1420Gate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, application, root, bundledSevenZip, forwardContextAction }) {
  const original = path.join(root, 'tests/fixtures/archives/haozip-forum-encrypted-1411')
  const manifest = JSON.parse(readFileSync(path.join(original, 'manifest.json'), 'utf8'))
  const sha = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const tree = dir => !existsSync(dir) ? [] : readdirSync(dir, { withFileTypes: true }).flatMap(e => e.isDirectory()
    ? tree(path.join(dir, e.name)).map(p => ({ ...p, path: e.name + '/' + p.path }))
    : [{ path: e.name, sha256: sha(path.join(dir, e.name)) }]).sort((a,b) => a.path.localeCompare(b.path))
  const expected = manifest.expectedFiles.filter(p => p.path === '说明.txt').map(p => ({ path: p.path, sha256: p.sha256 }))
  const fullExpected = manifest.expectedFiles.map(p => ({ path: p.path, sha256: p.sha256 })).sort((a,b) => a.path.localeCompare(b.path))
  const work = path.join(fixtureDirectory, 'ZIP真实缺卷抢救'); mkdirSync(work)
  const cases = [], screenshots = [], password = manifest.password
  const copy = (name, missing) => {
    const dir = path.join(work, name); mkdirSync(dir)
    manifest.volumes.forEach((v,i) => { assert.equal(sha(path.join(original,v.name)),v.sha256); if(i+1 !== missing) copyFileSync(path.join(original,v.name),path.join(dir,v.name)) })
    return dir
  }
  const independent = (source, output, expectedFiles = expected) => {
    const r = spawnSync(bundledSevenZip,['x','-y','-p'+password,'-o'+output,source],{windowsHide:true,encoding:'utf8',timeout:30000})
    assert.ifError(r.error); assert.equal(r.status,0,r.stdout+r.stderr); assert.deepEqual(tree(output),expectedFiles)
  }
  for(const missing of [1,2]) for(const selected of [1,2,3].filter(i=>i!==missing)) {
    const dir=copy(`缺${missing}卷-从${selected}卷打开`,missing), source=path.join(dir,manifest.volumes[selected-1].name), before=tree(dir)
    for(const supplied of [null,'wrong',password]) {
      const error=await failure('extractArchive',source,path.join(work,`不能普通解压-${missing}-${selected}-${String(supplied)}`),supplied)
      assert.ok(error.includes(manifest.volumes[missing-1].name),error); assert.doesNotMatch(error,/密码不正确|需要输入密码/)
    }
    const report=await bridge('diagnoseArchive',source,password); assert.equal(report.status,'partial_recovery'); assert.equal(report.canRepair,true); assert.equal(report.totalFiles,1)
    const wrong=path.join(work,`错误密码-${missing}-${selected}.zip`); await failure('repairZip',source,wrong,'wrong'); assert.equal(existsSync(wrong),false)
    const result=await bridge('repairZip',source,path.join(work,`抢救-${missing}-${selected}.zip`),password)
    assert.equal(result.verified,true); assert.equal(result.partial,true); assert.equal(result.outputEncrypted,true)
    assert.deepEqual(result.recoveredEntries,['说明.txt'])
    assert.ok(result.entryResults.some(e=>e.name==='说明.txt'&&e.status==='recovered'))
    assert.ok(result.entryResults.some(e=>e.name==='中文目录/原文件.bin'&&e.status==='missing'))
    independent(result.outputPath,result.outputPath+'-独立回解'); assert.deepEqual(tree(dir),before)
    cases.push({missing,selected,result,originalUnchanged:true,independentContentMatch:true})
  }
  const damagedDir=copy('完整卷组但内容损坏'), damaged=path.join(damagedDir,manifest.volumes[0].name)
  const bytes=readFileSync(damaged); bytes[3000]^=0x55; writeFileSync(damaged,bytes)
  const damagedBefore=tree(damagedDir), crc=await bridge('repairZip',damaged,path.join(work,'校验失败排除.zip'),password)
  assert.deepEqual(crc.recoveredEntries,['说明.txt']); assert.ok(crc.entryResults.some(e=>e.name==='中文目录/原文件.bin'&&e.status==='checksum_failed'))
  independent(crc.outputPath,crc.outputPath+'-独立回解'); assert.deepEqual(tree(damagedDir),damagedBefore)
  cases.push({kind:'content-check-failed',result:crc,originalUnchanged:true,independentContentMatch:true})
  for(const missing of [3]) {
    const dir=copy('缺终卷',missing), source=path.join(dir,manifest.volumes[0].name), output=path.join(work,'不能猜终卷.zip')
    await failure('repairZip',source,output,password); assert.equal(existsSync(output),false)
    cases.push({kind:'missing-terminal',rejectedWithoutOutput:true})
  }
  // Existing genuine ZIPX XZ/AES partial recovery must keep its bounds and status.
  for(const mode of ['none','password']) {
    const from=path.join(root,'tests/fixtures/archives/standard-zipx-1418',mode), dir=path.join(work,'ZIPX-'+mode); mkdirSync(dir)
    for(const file of readdirSync(from)) if(!file.endsWith('.zx02')) copyFileSync(path.join(from,file),path.join(dir,file))
    const source=path.join(dir,'真实原包.zipx'), secret=mode==='password'?'ZipxTest-1418':null
    const r=await bridge('repairZip',source,path.join(work,'ZIPX-'+mode+'.zip'),secret)
    assert.deepEqual(r.recoveredEntries,['说明.txt']); assert.ok(r.entryResults.some(e=>e.name==='中文目录/原文件.bin'&&e.status==='missing'))
    cases.push({kind:'ZIPX-regression',mode,result:r})
  }
  const click=async selector=>{
    const el=await waitForElement(selector); await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el)
    await driver.wait(()=>driver.executeScript(el=>{el.scrollIntoView({block:'center',behavior:'instant'}); const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2); return !el.disabled&&(el===hit||el.contains(hit))},el),10000)
    await el.click()
  }
  const fill=async(selector,value)=>{const el=await waitForElement(selector);await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE,value)}
  const sizes=async(name,anchor)=>{
    for(const [width,height] of [[1100,700],[760,520]]) {
      await driver.manage().window().setRect({width,height}); await driver.wait(()=>driver.executeScript(w=>Math.abs(innerWidth-w)<3,width),10000)
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const el=await waitForElement(anchor);await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el);await driver.sleep(200)
      await driver.executeAsyncScript('const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(done))')
      assert.ok(await driver.executeScript(()=>document.documentElement.scrollWidth-innerWidth)<=1)
      const file=`${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push(file)
    }
  }
  // A user encounters a missing disk, corrects the password, rescues a copy,
  // queues it with their chosen destination, and keeps the original failed task.
  await bridge('clearTasks')
  const dir=copy('用户收到的论坛卷',2), source=path.join(dir,manifest.volumes[0].name), before=tree(dir), output=path.join(work,'用户指定解压目录');mkdirSync(output)
  await failure('extractArchive',source,output,password)
  const [failed]=await bridge('extractionRecoveryTasks');assert.equal(failed.status,'failed')
  await click('[data-testid="nav-Decompress"]');await click(`[data-task-id="${failed.id}"]`)
  await fill('[data-testid="retry-password"]','wrong');await click('[data-testid="retry-rescue"]')
  await waitForElement('[data-testid="diagnostic-report"]')
  await fill('.archive-password',password);await click('[data-testid="start-archive-diagnosis"]');await waitForElement('[data-testid="start-archive-repair"]')
  await sizes('zip-partial-diagnosis','[data-testid="start-archive-repair"]')
  const rescued=path.join(work,'用户另存的抢救副本.zip');await bridge('queueDesktopDialogSelections',[rescued]);await click('[data-testid="start-archive-repair"]')
  const panel=await waitForElement('[data-testid="repair-result"]');assert.match(await panel.getText(),/部分恢复/)
  assert.equal((await panel.findElements(By.css('details[open]'))).length,0)
  await sizes('zip-partial-result-folded','[data-testid="repair-result"]')
  await click('[data-testid="recovery-outcome-missing"] summary');await click('[data-testid="recovery-outcome-recovered"] summary')
  await sizes('zip-partial-result-files','[data-testid="recovery-outcome-missing"]')
  independent(rescued,rescued+'-独立回解')
  await click('[data-testid="queue-recovered-archive"]')
  const queued=await driver.wait(async()=> (await bridge('extractionRecoveryTasks')).find(t=>t.status==='pending')||false,10000)
  assert.equal(queued.output,output);assert.equal(queued.retryOfTaskId,failed.id);assert.ok(queued.hasPassword)
  await click('[data-testid="nav-Decompress"]');await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async()=>await bridge('taskStatus',queued.id)==='completed',30000)
  assert.ok(tree(output).some(p=>p.path.endsWith('说明.txt')&&p.sha256===expected[0].sha256))
  assert.equal((await bridge('extractionRecoveryTasks')).find(t=>t.id===failed.id).status,'failed');assert.deepEqual(tree(dir),before)
  // Supplying the original missing disk restores ordinary complete extraction.
  copyFileSync(path.join(original,manifest.volumes[1].name),path.join(dir,manifest.volumes[1].name))
  const full=path.join(work,'补齐后完整解压');await bridge('extractArchive',source,full,password);assert.deepEqual(tree(full),fullExpected)
  // The actual batch page uses the same structured results, not a mocked mount.
  const batchDirs=[copy('批量-缺首',1),copy('批量-缺中',2)]
  forwardContextAction('--repair-archive',batchDirs.map(d=>path.join(d,manifest.volumes[2].name)))
  await waitForElement('[data-testid="archive-recovery-batch"]');await click('[data-testid="batch-recovery-config"]')
  const dialog=await waitForElement('[data-testid="archive-batch-config-dialog"]');await (await dialog.findElements(By.css('input[type="checkbox"]')))[0].click()
  await fill('[data-testid="archive-batch-shared-password"]',password)
  const batchOutput=path.join(work,'用户批量另存目录');mkdirSync(batchOutput);await fill('[data-testid="archive-batch-output"]',batchOutput);await click('[data-testid="archive-batch-config-apply"]')
  await click('[data-testid="batch-diagnose"]');await driver.wait(async()=>await (await waitForElement('[data-testid="batch-repair"]')).isEnabled(),30000)
  await click('[data-testid="batch-repair"]');await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-batch-result"]'))).length===2,30000)
  await click('[data-testid="archive-batch-result"] [data-testid="recovery-outcome-missing"] summary')
  await sizes('zip-partial-batch-files','[data-testid="archive-batch-result"] [data-testid="recovery-outcome-missing"]')
  for(const v of manifest.volumes) assert.equal(sha(path.join(original,v.name)),v.sha256)
  writeFileSync(path.join(artifactDirectory,'zip-partial-1420-results.json'),JSON.stringify({completedAt:new Date().toISOString(),application,binarySha256:sha(application),independentDecoder:bundledSevenZip,decoderSha256:sha(bundledSevenZip),cases,screenshots,realFailurePasswordCorrectionRescueQueueExtraction:true,originalFailureRetained:true,destinationAndPasswordRetained:true,originalVolumesUnchanged:true,restoredDiskFullExtraction:true,batchPageVerified:true},null,2))
}

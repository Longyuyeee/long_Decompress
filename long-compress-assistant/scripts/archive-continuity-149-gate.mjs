import assert from 'node:assert/strict'
import {mkdirSync,readFileSync,writeFileSync,readdirSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import {createHash,randomBytes} from 'node:crypto'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runArchiveContinuityGate({getDriver,restart,bridge,waitForElement,fixtureDirectory,artifactDirectory,bundledSevenZip,application,forwardContextAction}) {
  let driver=getDriver()
  const root=path.join(fixtureDirectory,'archive-continuity-149');mkdirSync(root)
  const source=path.join(root,'原始文件');mkdirSync(source)
  const payload=randomBytes(180000);writeFileSync(path.join(source,'资料.bin'),payload)
  const password='recovery + 149 !', saved=path.join(root,'恢复文件'), extracted=path.join(root,'再次解压')
  mkdirSync(saved);mkdirSync(extracted)
  const hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex')
  const command=args=>{const r=spawnSync(bundledSevenZip,args,{cwd:source,windowsHide:true,timeout:60000,encoding:'utf8'});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr)}
  const originals=[]
  for (const encrypted of [false,true]) {
    const file=path.join(root,encrypted?'带密码的客户资料.zip':'无密码的项目文件.zip')
    command(['a','-tzip',file,'.\\*',...(encrypted?[`-p${password}`,'-mem=AES256']:[])])
    command(['t',file,...(encrypted?[`-p${password}`]:[])])
    const bytes=readFileSync(file),end=bytes.length-22
    assert.equal(bytes.readUInt32LE(end),0x06054b50)
    // Remove the central directory from an independently created complete ZIP.
    writeFileSync(file,bytes.subarray(0,bytes.readUInt32LE(end+16)))
    originals.push({file,sha256:hash(file),encrypted})
  }
  const click=async selector=>{
    const el=await waitForElement(selector);await driver.wait(()=>el.isEnabled(),15000)
    await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el)
    let previous=''
    await driver.wait(async()=>{
      const state=await driver.executeScript(el=>{const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return {rect:[r.x,r.y,r.width,r.height],hit:hit===el||el.contains(hit)}},el)
      const rectangle=JSON.stringify(state.rect),stable=previous===rectangle;previous=rectangle
      return stable&&state.hit
    },10000,`button must settle and become clickable: ${selector}`,100)
    await el.click()
  }
  const fill=async(selector,value)=>{const el=await waitForElement(selector);await driver.executeScript(el=>el.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value)}
  const idle=()=>driver.wait(async()=>!await driver.findElement(By.css('[data-testid="batch-diagnose"]')).getAttribute('disabled'),30000)
  const rows=()=>driver.findElements(By.css('[data-testid="archive-batch-row"]'))
  const screenshots=[]
  async function shot(name,selector) {
    for(const close of await driver.findElements(By.css('.toast-item button'))){try{await close.click()}catch{}}
    await new Promise(resolve=>setTimeout(resolve,350))
    const el=await waitForElement(selector);await driver.executeScript(el=>el.scrollIntoView({block:'start'}),el)
    const bounds=await driver.executeScript(el=>({client:el.clientWidth,scroll:el.scrollWidth,viewport:innerWidth,right:document.querySelector('.main-container').getBoundingClientRect().right}),el)
    assert.ok(bounds.scroll<=bounds.client+1 && bounds.right<=bounds.viewport+1,name+' overflows: '+JSON.stringify(bounds))
    writeFileSync(path.join(artifactDirectory,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push({name:name+'.png',...bounds})
  }
  await bridge('clearTasks')
  forwardContextAction('--repair-archive',originals.map(item=>item.file))
  await waitForElement('[data-testid="archive-recovery-batch"]');await click('[data-testid="batch-recovery-config"]')
  const dialog=await waitForElement('[data-testid="archive-batch-config-dialog"]'),toggles=await dialog.findElements(By.css('input[type="checkbox"]'))
  await toggles[0].click();await fill('[data-testid="archive-batch-shared-password"]',password)
  await fill('[data-testid="archive-batch-output"]',saved);await toggles[1].click();await fill('input[placeholder="留空则使用各自原包目录"]',extracted)
  await click('[data-testid="archive-batch-config-apply"]');await click('[data-testid="batch-diagnose"]');await idle()
  const statuses=await Promise.all((await rows()).map(row=>row.findElement(By.css('[data-testid="archive-batch-status"]')).getText()))
  await click('[data-testid="nav-History"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-recovery-batch"]'))).length===0,10000)
  await click('[data-testid="nav-FileIntegrity"]');await waitForElement('[data-testid="archive-recovery-batch"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-batch-status"]'))).length===2 && (await Promise.all((await rows()).map(row=>row.findElement(By.css('[data-testid="archive-batch-status"]')).getText()))).every(Boolean),10000)
  assert.deepEqual(await Promise.all((await rows()).map(row=>row.findElement(By.css('[data-testid="archive-batch-status"]')).getText())),statuses,'navigation must retain reports')
  await driver.manage().window().setRect({width:1100,height:700});await shot('continuity-navigation-1100','[data-testid="archive-recovery-batch"]')
  await click('[data-testid="batch-repair"]');await idle()
  assert.equal((await driver.findElements(By.css('[data-testid="archive-batch-result"]'))).length,2)
  const snapshot=await driver.executeScript(()=>localStorage.getItem('long-archive-recovery-workspace-v1'))
  assert.ok(snapshot&&!snapshot.includes(password),'the actual persisted workspace must not contain the fixture password')
  await restart();driver=getDriver()
  await click('[data-testid="nav-FileIntegrity"]')
  await waitForElement('[data-testid="archive-recovery-resume"]')
  assert.equal((await driver.findElements(By.css('[data-testid="archive-batch-row"]'))).length,0,'restart must not automatically restore/run')
  await driver.manage().window().setRect({width:760,height:700});await shot('continuity-restore-offer-760','[data-testid="archive-recovery-resume"]')
  await click('[data-testid="archive-recovery-restore"]')
  assert.equal((await rows()).length,2)
  assert.equal(await driver.findElement(By.css('[data-testid="batch-repair"]')).getAttribute('disabled'),'true','old reports cannot authorize repair after restart')
  const passwords=await driver.findElements(By.css('[data-testid="archive-batch-password"]'))
  for(const input of passwords)assert.equal(await input.getAttribute('value'),'','source passwords must not persist')
  const resultPasswords=await driver.findElements(By.css('[data-testid="archive-batch-result-password"]'))
  assert.equal(resultPasswords.length,1);assert.equal(await resultPasswords[0].getAttribute('value'),'')
  await shot('continuity-result-password-760','[data-testid="archive-recovery-batch"]')
  await shot('continuity-password-field-760','[data-testid="archive-batch-result-password"]')
  await driver.manage().window().setRect({width:760,height:520});await driver.executeScript(()=>document.documentElement.style.fontSize='125%')
  await shot('continuity-result-small-125','[data-testid="archive-recovery-batch"]')
  await click('[data-testid="batch-recovery-config"]');await shot('continuity-config-small-125','[data-testid="archive-batch-config-dialog"]')
  await click('[data-testid="archive-batch-config-apply"]')
  await driver.executeScript(()=>document.documentElement.style.fontSize='');await driver.manage().window().setRect({width:1100,height:700})
  await click('[data-testid="batch-queue-recovered"]');await idle()
  let queued=await bridge('extractionRecoveryTasks');assert.equal(queued.length,1,'only the unencrypted recovered ZIP may queue without a password')
  assert.ok((await driver.findElement(By.css('[data-testid="archive-recovery-batch"]')).getText()).includes('请填写恢复结果密码'))
  await fill('[data-testid="archive-batch-result-password"]',password)
  await click('[data-testid="batch-queue-recovered"]');await idle()
  queued=await bridge('extractionRecoveryTasks');assert.equal(queued.length,2);assert.equal(queued.filter(t=>t.hasPassword).length,1)
  assert.ok(queued.every(t=>t.output===extracted),'independent output configuration must survive restart')
  assert.equal(await driver.findElement(By.css('[data-testid="batch-queue-recovered"]')).getAttribute('disabled'),'true')
  for(const task of queued)command(['t',task.source,...(task.hasPassword?[`-p${password}`]:[])])
  await click('[data-testid="nav-Decompress"]');await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async()=>{const tasks=await bridge('extractionRecoveryTasks');if(tasks.some(t=>t.status==='failed'))throw Error(JSON.stringify(tasks));return tasks.length===2&&tasks.every(t=>t.status==='completed')},60000)
  const walk=folder=>readdirSync(folder,{withFileTypes:true}).flatMap(entry=>entry.isDirectory()?walk(path.join(folder,entry.name)):[path.join(folder,entry.name)])
  const output=walk(extracted).filter(file=>path.basename(file)==='资料.bin');assert.equal(output.length,2);output.forEach(file=>assert.ok(readFileSync(file).equals(payload)))
  originals.forEach(item=>assert.equal(hash(item.file),item.sha256))
  await click('[data-testid="nav-FileIntegrity"]')
  await shot('continuity-completed-1100','[data-testid="archive-recovery-batch"]')
  if(process.env.LONG_CONTINUITY_UI_HOLD_MS)await new Promise(resolve=>setTimeout(resolve,Math.min(45000,Number(process.env.LONG_CONTINUITY_UI_HOLD_MS))))
  writeFileSync(path.join(artifactDirectory,'archive-continuity-149-results.json'),JSON.stringify({application,binarySha256:hash(application),navigationRetained:true,sameProfileNativeRestart:true,automaticWorkAfterRestart:false,passwordsPersisted:false,recoveredGroups:2,independentVerification:2,queuedAndExtracted:2,originalsUnchanged:true,nativePickerVerified:false,screenshots},null,2))
}

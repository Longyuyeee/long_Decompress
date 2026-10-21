import assert from 'node:assert/strict'
import { createHash, randomBytes, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runFailureRecovery1542Gate({ driver, bridge, waitForElement, artifactDirectory, application, root, bundledSevenZip }) {
  const base=path.join(root,'test-results/failure-recovery-1542'),progress=path.join(base,'desktop-progress.json')
  mkdirSync(base,{recursive:true})
  const hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex'),binarySha256=hash(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.42',binarySha256,work:path.join(base,randomUUID()),checks:[],layouts:[],screenshots:[]}
  if(state.binarySha256!==binarySha256){assert.equal(process.env.RECOVERY_1542_RESUME_FROM,state.binarySha256,'Acknowledge the previous evidence identity');state.previousIdentities=[...(state.previousIdentities||[]),state.binarySha256];state.binarySha256=binarySha256}
  const work=state.work;mkdirSync(work,{recursive:true})
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2)),passed=kind=>state.checks.some(row=>row.kind===kind)
  const done=(kind,evidence={})=>{state.checks.push({kind,binarySha256,...evidence});save();console.log('recovery1542: '+kind)}
  const find=selector=>driver.wait(async()=>{
    try{
      const dialogs=await driver.findElements(By.css('[role="dialog"]'))
      for(const dialog of dialogs.reverse()){
        if(!await dialog.isDisplayed())continue
        const matches=await dialog.findElements(By.css(selector))
        for(const e of matches.reverse())if(await e.isDisplayed())return e
        return false
      }
      const matches=await driver.findElements(By.css(selector))
      for(const e of matches.reverse())if(await e.isDisplayed())return e
      return false
    }catch(error){if(error.name==='StaleElementReferenceError')return false;throw error}
  },15000)
  const click=async selector=>{const e=typeof selector==='string'?await find(selector):selector;await driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});e.click()},e)}
  const fill=async(selector,value)=>{const e=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const historyFor=async id=>(await bridge('taskHistory')).filter(row=>row.id===id||row.retryOfTaskId===id)
  const complete=predicate=>driver.wait(async()=>(await bridge('taskHistory')).find(row=>predicate(row)&&['failed','completed','cancelled'].includes(row.status))||false,90000)
  async function screenshot(name,anchor){
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),await find(anchor))
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const layout=await driver.executeScript(selector=>{const e=document.querySelector(selector);const d=e?.closest('[data-testid="compression-draft-details"]');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,anchorOverflow:e?e.scrollWidth-e.clientWidth:0,columns:d?[...d.children].map(c=>c.getBoundingClientRect().width):[]}},anchor)
      assert.ok(layout.overflow<=1&&layout.anchorOverflow<=1,JSON.stringify(layout));if(layout.columns.length===2&&width===1100)assert.ok(Math.abs(layout.columns[0]-layout.columns[1])<=1,JSON.stringify(layout))
      const file=`recovery1542-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,binarySha256,...layout});state.screenshots.push(file);save()
    }
    await driver.manage().window().setRect({width:1100,height:700})
  }
  function independent(output,password,label){
    const destination=path.join(work,label)
    execFileSync(bundledSevenZip,['x','-y',...(password?['-p'+password]:[]),'-o'+destination,output.primaryPath],{windowsHide:true,maxBuffer:4*1024*1024})
    const walk=dir=>readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(dir,e.name)):[path.join(dir,e.name)])
    const files=walk(destination);assert.equal(files.length,1);assert.equal(hash(files[0]),hash(state.source))
    return{allHashes:true,primaryPath:output.primaryPath}
  }
  if(!state.source){
    state.source=path.join(work,'需要保留的源文件与长名称-'+ '资料'.repeat(8)+'.bin');writeFileSync(state.source,randomBytes(512*1024+4096))
    state.inputZip=path.join(work,'密码原包.zip');execFileSync(bundledSevenZip,['a','-tzip','-mem=AES256','-pSource1542!',state.inputZip,state.source],{windowsHide:true,maxBuffer:4*1024*1024})
    state.splitBase=path.join(work,'待补齐.7z');execFileSync(bundledSevenZip,['a','-t7z','-mx=0','-v256k',state.splitBase,state.source],{windowsHide:true,maxBuffer:4*1024*1024})
    state.parts=readdirSync(work).filter(name=>name.startsWith('待补齐.7z.')).sort().map(name=>path.join(work,name));assert.ok(state.parts.length>=3);save()
  }
  if(!passed('conversion-conflict-password-confirmation-and-linked-retry')){
    await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
    await bridge('queueDesktopDialogSelections',[[state.inputZip]]);await click('[data-testid="conversion-choose-source"]');const row=(await driver.findElements(By.css('[data-testid="conversion-row"]'))).at(-1);if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    await click('[data-testid="purpose-share"]');await click('[data-testid="purpose-apply"]')
    await fill('[data-testid="conversion-source-password"]','Source1542!');await fill('[data-testid="conversion-password"]','Target1542!')
    const occupied=path.join(work,'已有密码输出.zip');writeFileSync(occupied,'existing output must stay unchanged');const originalHash=hash(occupied)
    await fill('[data-testid="conversion-output"]',occupied);await click('[data-testid="conversion-start"]')
    const failed=await complete(row=>row.executionConfig?.kind==='archive-conversion');assert.equal(failed.status,'failed',JSON.stringify(failed));assert.equal(hash(occupied),originalHash)
    assert.match(await(await find('[data-testid="task-failure-hint"]')).getAttribute('textContent'),/输出未能提交/)
    await screenshot('conversion-failure','[data-testid="task-failure-hint"]')
    await click('[data-testid="conversion-prepare-retry"]');await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length===2,15000)
    assert.equal((await historyFor(failed.id)).length,1)
    await screenshot('conversion-password-confirm','[data-testid="conversion-retry-notice"]')
    await click('[data-testid="conversion-start"]');assert.match(await(await find('[data-testid="archive-conversion"]')).getAttribute('textContent'),/请重新填写原包密码/);assert.equal((await historyFor(failed.id)).length,1)
    await fill('[data-testid="conversion-source-password"]','Source1542!');await click('[data-testid="conversion-start"]');assert.match(await(await find('[data-testid="archive-conversion"]')).getAttribute('textContent'),/请重新填写新包密码/)
    await fill('[data-testid="conversion-password"]','Target1542!');await click('[data-testid="conversion-start"]')
    const completed=await complete(row=>row.retryOfTaskId===failed.id);assert.equal(completed.status,'completed',JSON.stringify(completed));assert.equal(hash(occupied),originalHash)
    assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed)
    assert.equal(completed.executionConfig.sourcePasswordRequired,true);assert.equal(completed.executionConfig.passwordRequired,true)
    done('conversion-conflict-password-confirmation-and-linked-retry',{oldFailurePreserved:true,occupiedFilePreserved:true,...independent(completed.archiveOutput,'Target1542!','converted-retry')})
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
  if(!passed('taskbar-missing-volume-correction-without-executing-old-task')){
    await bridge('clearTasks');await click('[data-testid="nav-Decompress"]');await bridge('queueDesktopDialogSelections',[[state.parts.at(-1)]]);await click('[data-testid="dropzone-file"]')
    const original=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000)
    const missing=state.parts[1],backup=missing+'.owned-original';renameSync(missing,backup)
    let failed
    try{await click('[data-testid="start-decompression-queue"]');failed=await complete(row=>row.id===original.id);assert.equal(failed.status,'failed')}
    finally{renameSync(backup,missing)}
    await click('[data-testid="global-progress-summary"]')
    await click(`[data-testid="global-task-prepare-retry"][data-retry-task-id="${original.id}"]`)
    await find('[data-testid="failed-extraction-retry"][data-inline="true"]')
    assert.equal((await historyFor(failed.id)).length,1);assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed)
    const destination=path.join(work,'补齐后提取');mkdirSync(destination,{recursive:true})
    await fill('[data-testid="retry-output"]',destination);const sub=await find('[data-testid="retry-subfolder"]');if(await sub.isSelected())await click(sub)
    await screenshot('extraction-correction','[data-testid="failed-extraction-retry"]')
    await click('[data-testid="create-retry-drafts"]')
    const pending=await driver.wait(async()=>(await bridge('extractionRecoveryTasks')).find(row=>row.retryOfTaskId===original.id)||false,15000);assert.equal(pending.status,'pending')
    await click('[data-testid="start-decompression-queue"]');const completed=await complete(row=>row.id===pending.id);assert.equal(completed.status,'completed',JSON.stringify(completed))
    const files=readdirSync(destination);assert.equal(files.length,1);assert.equal(hash(path.join(destination,files[0])),hash(state.source));assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed)
    done('taskbar-missing-volume-correction-without-executing-old-task',{oldFailurePreserved:true,newTaskWaitedForConfirmation:true,allHashes:true})
  }
  if(!passed('compression-conflict-uses-original-settings-and-new-draft')){
    await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[state.source]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]');await click('[data-testid="compression-archive-name"]')
    await fill('[data-testid="compression-output-path"]',work);await fill('[data-testid="compression-filename-input"]','已有压缩结果');await fill('[data-testid="compression-password-input"]','Compression1542!')
    const occupied=path.join(work,'已有压缩结果.zip');writeFileSync(occupied,'keep compression output');const originalHash=hash(occupied)
    await click('[data-testid="start-compression"]');const failed=await complete(row=>row.executionConfig?.kind==='archive-compression');assert.equal(failed.status,'failed',JSON.stringify(failed))
    await screenshot('compression-failure','[data-testid="task-failure-hint"]')
    await click('[data-testid="compression-prepare-retry"]');const group='[data-testid="compression-group-row"]'
    await find(group);assert.equal((await historyFor(failed.id)).length,1);assert.equal(hash(occupied),originalHash)
    await screenshot('compression-new-draft',group+' [data-testid="compression-format-select"]')
    await fill(group+' [data-testid="compression-password-input"]','Compression1542!')
    await click('[data-testid="start-compression"]');const completed=await complete(row=>row.retryOfTaskId===failed.id);assert.equal(completed.status,'completed',JSON.stringify(completed))
    assert.equal(hash(occupied),originalHash);assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed)
    done('compression-conflict-uses-original-settings-and-new-draft',{oldFailurePreserved:true,occupiedFilePreserved:true,...independent(completed.archiveOutput,'Compression1542!','compression-retry')})
  }
  save();console.log('recovery1542: complete '+progress)
}

import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runZipEdit1546Gate({ driver:initialDriver, getDriver, restart, bridge, artifactDirectory, application, root, bundledSevenZip, dataDirectory }) {
  let driver=initialDriver
  const base=path.join(root,'test-results/zip-edit-1546');mkdirSync(base,{recursive:true})
  const receipt=path.join(base,'desktop-progress.json'),hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex'),binarySha256=hash(application)
  const state=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.5.46',binarySha256,work:path.join(base,randomUUID()),checks:[],layouts:[],screenshots:[]}
  if(state.binarySha256!==binarySha256){assert.equal(process.env.ZIP_EDIT_1546_RESUME_FROM,state.binarySha256,'Acknowledge the earlier UI evidence');for(const row of [...state.checks,...state.layouts])row.binarySha256??=state.binarySha256;state.previousIdentities=[...(state.previousIdentities||[]),state.binarySha256];state.binarySha256=binarySha256}
  mkdirSync(state.work,{recursive:true});const save=()=>writeFileSync(receipt,JSON.stringify(state,null,2)),passed=kind=>state.checks.some(row=>row.kind===kind),done=(kind,evidence={})=>{state.checks.push({kind,binarySha256,...evidence});save();console.log('zipEdit1546: '+kind)}
  const find=selector=>driver.wait(async()=>{
    try{const dialogs=await driver.findElements(By.css('[role="dialog"]'));for(const dialog of dialogs.reverse())if(await dialog.isDisplayed()){const rows=await dialog.findElements(By.css(selector));for(const row of rows.reverse())if(await row.isDisplayed())return row;return false}
      const rows=await driver.findElements(By.css(selector));for(const row of rows)if(await row.isDisplayed())return row;return false
    }catch(error){if(error.name==='StaleElementReferenceError')return false;throw error}
  },15000,'Missing visible selector: '+selector)
  const click=async selector=>{const e=typeof selector==='string'?await find(selector):selector;await driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});e.click()},e)}
  const fill=async(selector,value)=>{const e=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const txt=async selector=>(await find(selector)).getAttribute('textContent')
  const same=(a,b)=>a.replace(/\\/g,'/').toLowerCase()===b.replace(/\\/g,'/').toLowerCase()
  const complete=predicate=>driver.wait(async()=>(await bridge('taskHistory')).find(row=>predicate(row)&&['completed','failed','cancelled'].includes(row.status))||false,90000)
  async function screenshot(name,anchor){
    if(state.layouts.filter(row=>row.name===name).length===2)return
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),await find(anchor))
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const layout=await driver.executeScript(selector=>{const e=[...document.querySelectorAll(selector)].find(e=>e.offsetHeight>0);const review=e?.closest('[data-testid="zip-edit-review-content"],[data-testid="zip-organize-content"]');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,anchorOverflow:e?e.scrollWidth-e.clientWidth:0,columns:review?[...review.children].map(c=>c.getBoundingClientRect().width):[]}},anchor)
      assert.ok(layout.overflow<=1&&layout.anchorOverflow<=1,JSON.stringify(layout));if(width===1100&&layout.columns.length===2)assert.ok(Math.abs(layout.columns[0]-layout.columns[1])<=1,JSON.stringify(layout))
      const file=`zip-edit1546-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,binarySha256,...layout});state.screenshots.push(file);save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  async function openArchive(file){
    for(const button of await driver.findElements(By.css('button[aria-label="关闭编辑清单"],button[aria-label="关闭整理窗口"]')))if(await button.isDisplayed())await click(button)
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[file]])
    const chooser=await driver.findElements(By.css('[data-testid="file-manager-open-archive"]'));if(chooser.length)await click(chooser[0]);else await click('.browser-page > header .browser-primary')
    await driver.wait(async()=>same(await(await find('.browser-toolbar button.browser-input')).getAttribute('title'),file),15000)
    await find('.browser-summary');await find('[data-testid="zip-edit-begin"]')
  }
  const closeReview=()=>click('button[aria-label="关闭编辑清单"]')
  async function selectChoice(name,choice){
    for(const row of await driver.findElements(By.css('[data-testid="zip-edit-change"]'))){if((await row.getAttribute('textContent')).includes(name)){await click(await row.findElement(By.css(`[data-testid="zip-edit-choice-${choice}"]`)));return}}
    throw new Error('Missing edit row '+name)
  }
  const closeOrganizer=()=>click('button[aria-label="关闭整理窗口"]')
  const rowSelector=path=>'[data-organize-path="'+path+'"]'
  const choose=path=>click(rowSelector(path)+' input[type="checkbox"]')
  const enter=path=>click(rowSelector(path)+' button')
  async function context(path,command){
    const row=await find('[data-entry-path="'+path+'"]');await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),row);await driver.actions().contextClick(row).perform();await click(command)
  }
  async function select(testId,label){
    await click('[data-testid="'+testId+'"]')
    const option=await driver.wait(async()=>{for(const item of await driver.findElements(By.css('[role="listbox"] [role="option"]')))if(await item.isDisplayed()&&(await item.getAttribute('textContent')).includes(label))return item;return false},10000)
    await click(option)
  }
  async function contextSingle(file,command){const checkbox=await find('[data-entry-path="'+file+'"] .browser-checkbox');if((await checkbox.getAttribute('aria-label')).startsWith('取消选择'))await click(checkbox);await context(file,command)}
  async function vault(target,name){await click('[data-testid="zip-edit-'+target+'-vault"]');await click('[data-vault-entry="'+name+'"]');await driver.wait(async()=>!(await driver.findElements(By.css('[data-vault-entry="'+name+'"]'))).length,10000)}
  async function persisted(predicate){return driver.wait(async()=>{try{const raw=readFileSync(path.join(dataDirectory,'zip-edit-drafts-v1.json'),'utf8'),value=JSON.parse(raw);return predicate(value)?value:false}catch{return false}},15000)}
  const sourcePassword='source-1546',targetPassword='target-1546',draftFile=path.join(dataDirectory,'zip-edit-drafts-v1.json')
  if(!state.source){
    const files=path.join(state.work,'原始资料');mkdirSync(path.join(files,'docs'),{recursive:true});mkdirSync(path.join(files,'empty'),{recursive:true});writeFileSync(path.join(files,'README.txt'),'original protected readme');writeFileSync(path.join(files,'docs','说明.txt'),'protected nested contents')
    state.source=path.join(state.work,'密码原包.zip');state.zipcrypto=path.join(state.work,'传统密码原包.zip')
    for(const[file,mode]of[[state.source,'AES256'],[state.zipcrypto,'ZipCrypto']])execFileSync(bundledSevenZip,['a','-tzip','-mem='+mode,'-p'+sourcePassword,file,'.'],{cwd:files,windowsHide:true,maxBuffer:1024*1024})
    state.sourceHash=hash(state.source);state.zipcryptoHash=hash(state.zipcrypto);state.addition=path.join(state.work,'补入.txt');writeFileSync(state.addition,'new encrypted addition');save()
  }
  if(!passed('password-edit-failure-vault-keep-and-new-result')){
    await bridge('clearTasks');await bridge('clearTaskHistory');await bridge('seedVaultPassword','编辑原包密码',sourcePassword);await bridge('seedVaultPassword','编辑新包密码',targetPassword);await openArchive(state.source)
    await contextSingle('README.txt','[data-testid="archive-context-edit-rename"]');await fill('[data-testid="zip-organize-single-path"]','说明-新版.txt');await click('[data-testid="zip-organize-apply"]');await click('[data-testid="zip-organize-review"]');await fill('[data-testid="zip-edit-source-password"]','incorrect-password')
    state.output=path.join(state.work,'保留密码-'+randomUUID().slice(0,8)+'.zip');save();await fill('[data-testid="zip-edit-output"]',state.output);await click('[data-testid="zip-edit-save"]');const failed=await complete(row=>same(row.outputPath,state.output));assert.equal(failed.status,'failed',JSON.stringify(failed));await screenshot('wrong-password','[data-testid="zip-edit-credentials"]')
    await vault('source','编辑原包密码');await screenshot('password-review','[data-testid="zip-edit-credentials"]');await click('[data-testid="zip-edit-save"]');const completed=await complete(row=>row.retryOfTaskId===failed.id);assert.equal(completed.status,'completed',JSON.stringify(completed));assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed);assert.equal(hash(state.source),state.sourceHash)
    const extracted=path.join(state.work,'保留密码独立回解');execFileSync(bundledSevenZip,['x','-y','-p'+sourcePassword,'-o'+extracted,state.output],{windowsHide:true,maxBuffer:1024*1024});assert.equal(readFileSync(path.join(extracted,'说明-新版.txt'),'utf8'),'original protected readme');assert.equal(readFileSync(path.join(extracted,'docs','说明.txt'),'utf8'),'protected nested contents');assert.ok(existsSync(path.join(extracted,'empty')))
    await click('[data-testid="zip-edit-browse-result"]');const row=await find('[data-entry-path="说明-新版.txt"]');await click(await row.findElement(By.css('.preview-trigger')));await driver.wait(async()=>(await txt('[data-testid="archive-text-preview"]')).includes('original protected readme'),15000);await screenshot('password-result-preview','[data-testid="archive-text-preview"]');await click('[aria-label="关闭预览"]')
    done('password-edit-failure-vault-keep-and-new-result',{originalHashPreserved:true,wrongPasswordRetainsEdits:true,originalFailurePreserved:true,vault:true,independentContents:true,resultCredentialContinuity:true})
  }
  if(!passed('zipcrypto-password-only-rekey-as-aes')){
    await openArchive(state.zipcrypto);await click('[data-testid="zip-edit-begin"]');await click('[data-testid="zip-edit-review"]');await vault('source','编辑原包密码');await select('zip-edit-output-mode','设置新密码 · AES-256');await vault('output','编辑新包密码')
    state.rekey=path.join(state.work,'新AES密码-'+randomUUID().slice(0,8)+'.zip');save();await fill('[data-testid="zip-edit-output"]',state.rekey);await screenshot('new-password-settings','[data-testid="zip-edit-credentials"]');await click('[data-testid="zip-edit-save"]');const completed=await complete(row=>same(row.outputPath,state.rekey));assert.equal(completed.status,'completed',JSON.stringify(completed));assert.equal(hash(state.zipcrypto),state.zipcryptoHash)
    const extracted=path.join(state.work,'新密码独立回解');execFileSync(bundledSevenZip,['x','-y','-p'+targetPassword,'-o'+extracted,state.rekey],{windowsHide:true,maxBuffer:1024*1024});assert.equal(readFileSync(path.join(extracted,'README.txt'),'utf8'),'original protected readme');const listing=execFileSync(bundledSevenZip,['l','-slt',state.rekey],{windowsHide:true,encoding:'utf8',maxBuffer:1024*1024});assert.match(listing,/AES-256/)
    await click('[data-testid="zip-edit-browse-result"]');await click('[data-testid="zip-edit-begin"]');await click('[data-testid="zip-edit-review"]');assert.equal(await(await find('[data-testid="zip-edit-source-password"]')).getAttribute('value'),targetPassword);await closeReview();done('zipcrypto-password-only-rekey-as-aes',{originalPreserved:true,noContentEditsRequired:true,newPasswordIndependentRead:true,newSourceCredentialContinuity:true})
  }
  if(!passed('real-restart-missing-changed-references-relink-and-corrected-save')){
    if(!state.recoveryPrepared){
    state.draftSource=path.join(state.work,'需要恢复的密码包.zip');copyFileSync(state.source,state.draftSource);state.folder=path.join(state.work,'外部引用目录');mkdirSync(state.folder);writeFileSync(path.join(state.folder,'file.txt'),'original incoming contents');await openArchive(state.draftSource);await click('[data-testid="zip-edit-begin"]');await bridge('queueDesktopDialogSelections',[[state.folder]]);await click('[data-testid="zip-edit-add-folder"]');await vault('source','编辑原包密码');await select('zip-edit-output-mode','设置新密码 · AES-256');await vault('output','编辑新包密码')
    await click('[data-testid="zip-edit-structure-preview"]');await click('[data-testid="zip-organize-search"]');await fill('[data-testid="zip-organize-search"]','外部引用目录/');for(const checkbox of await driver.findElements(By.css('[data-organize-path] input[type="checkbox"]')))if(await checkbox.isSelected())await click(checkbox)
    await choose('外部引用目录/');await fill('[data-testid="zip-organize-single-path"]','已整理资料');await click('[data-testid="zip-organize-apply"]');await click('[data-testid="zip-organize-review"]');state.recoveredOutput=path.join(state.work,'恢复编辑-'+randomUUID().slice(0,8)+'.zip');await fill('[data-testid="zip-edit-output"]',state.recoveredOutput)
    const saved=await persisted(value=>value.sessions.some(row=>same(row.sourcePath,state.draftSource)&&row.operations.length&&row.outputMode==='aes256'));const raw=JSON.stringify(saved);assert.ok(!raw.includes(sourcePassword)&&!raw.includes(targetPassword));await closeReview()
    state.movedSource=path.join(state.work,'重新定位的密码包.zip');state.movedFolder=path.join(state.work,'重新定位的外部目录');for(const target of[state.movedSource,state.movedFolder])assert.ok(path.resolve(target).startsWith(path.resolve(state.work)+path.sep));renameSync(state.draftSource,state.movedSource);renameSync(state.folder,state.movedFolder);writeFileSync(path.join(state.movedFolder,'file.txt'),'updated relocated contents');writeFileSync(path.join(state.movedFolder,'extra-not-selected.txt'),'do not add automatically')
    writeFileSync(path.join(state.work,'后来补入原包.txt'),'accepted new original content');execFileSync(bundledSevenZip,['a','-tzip','-mem=AES256','-p'+sourcePassword,state.movedSource,'后来补入原包.txt'],{cwd:state.work,windowsHide:true,maxBuffer:1024*1024});state.changedSourceHash=hash(state.movedSource);save()
    await bridge('setCloseToTray',false);await restart({preserveProfile:true,requestExit:async()=>{await click('.window-titlebar button[aria-label="关闭"]')}});driver=getDriver();state.recoveryPrepared=true;save();}
    await click('[data-testid="zip-edit-recovery-open"]');await screenshot('draft-recovery-list','[data-testid="zip-edit-recover-one"]');await click('[data-testid="zip-edit-recover-one"]');await click('[data-testid="zip-edit-review"]');await find('[data-testid="zip-edit-source-status"]');await screenshot('recovery-sources','[data-testid="zip-edit-source-status"]');assert.equal(await(await find('[data-testid="zip-edit-source-password"]')).getAttribute('value'),'');assert.equal(await(await find('[data-testid="zip-edit-output-password"]')).getAttribute('value'),'');assert.equal((await bridge('taskHistory')).filter(row=>same(row.outputPath,state.recoveredOutput)).length,0)
    const currentPath=await(await driver.findElement(By.css('.browser-toolbar button.browser-input'))).getAttribute('title')
    if(!same(currentPath,state.movedSource)){
      await bridge('queueDesktopDialogSelections',[[state.movedSource]]);await click('[data-testid="zip-edit-relocate-archive"]');await driver.wait(async()=>{const errors=await driver.findElements(By.css('[data-testid="zip-edit-source-status"] [role="alert"]'));for(const error of errors)if(await error.isDisplayed())throw new Error('Source relocation: '+await error.getAttribute('textContent'));const titles=await driver.findElements(By.css('.browser-toolbar button.browser-input'));return titles.length&&same(await titles[0].getAttribute('title'),state.movedSource)},15000);await click('[data-testid="zip-edit-review"]')
    }
    if((await driver.findElements(By.css('[data-testid="zip-edit-accept-source"]'))).length)await click('[data-testid="zip-edit-accept-source"]')
    const storedDraft=JSON.parse(readFileSync(draftFile,'utf8')).sessions.find(row=>same(row.sourcePath,state.movedSource)),folderId=storedDraft.drafts.find(row=>row.isDir).id
    await bridge('queueDesktopDialogSelections',[[state.movedFolder]]);await click(`[data-source-issue="${folderId}"] [data-testid="zip-edit-relink-input"]`);await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="zip-edit-source-status"]'))).length,15000);
    await vault('source','编辑原包密码');await vault('output','编辑新包密码');await screenshot('restored-password-review','[data-testid="zip-edit-credentials"]')
    const occupied=path.join(state.work,'不可覆盖的结果.zip');writeFileSync(occupied,'keep existing result');const occupiedHash=hash(occupied);await fill('[data-testid="zip-edit-output"]',occupied);await click('[data-testid="zip-edit-save"]');const failed=await complete(row=>same(row.outputPath,occupied));assert.equal(failed.status,'failed');await fill('[data-testid="zip-edit-output"]',state.recoveredOutput)
    const current=await persisted(value=>value.sessions.some(row=>same(row.sourcePath,state.movedSource)&&same(row.outputPath,state.recoveredOutput)));state.staleDraft=current.sessions.find(row=>same(row.sourcePath,state.movedSource));save();await click('[data-testid="zip-edit-save"]');const completed=await complete(row=>row.retryOfTaskId===failed.id);assert.equal(completed.status,'completed',JSON.stringify(completed));assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed);assert.equal(hash(occupied),occupiedHash);assert.equal(hash(state.movedSource),state.changedSourceHash)
    const extracted=path.join(state.work,'恢复后独立回解');execFileSync(bundledSevenZip,['x','-y','-p'+targetPassword,'-o'+extracted,state.recoveredOutput],{windowsHide:true,maxBuffer:1024*1024});assert.equal(readFileSync(path.join(extracted,'已整理资料','file.txt'),'utf8'),'updated relocated contents');assert.ok(!existsSync(path.join(extracted,'已整理资料','extra-not-selected.txt')));assert.equal(readFileSync(path.join(extracted,'后来补入原包.txt'),'utf8'),'accepted new original content')
    state.completedRecoveryId=completed.id;save();await screenshot('recovered-result','[data-testid="archive-output-summary"]');done('real-restart-missing-changed-references-relink-and-corrected-save',{realProfileRestart:true,passwordsOmitted:true,sourceRelocatedAndExplicitlyAccepted:true,folderRelinkPreservesReferences:true,extraFileExcluded:true,existingAndFailurePreserved:true,independentExtraction:true})
  }
  if(!passed('completed-receipt-prevents-replaying-a-stale-draft')){
    await closeReview();const before=(await bridge('taskHistory')).map(row=>row.id).sort();await driver.wait(async()=>!existsSync(draftFile),15000);writeFileSync(draftFile,JSON.stringify({version:1,sessions:[{...state.staleDraft,taskId:state.completedRecoveryId}]}))
    await restart({preserveProfile:true});driver=getDriver();await click('[data-testid="zip-edit-recovery-open"]');await click('[data-testid="zip-edit-recover-one"]');await driver.wait(async()=>same(await(await find('.browser-toolbar button.browser-input')).getAttribute('title'),state.recoveredOutput),15000);assert.deepEqual((await bridge('taskHistory')).map(row=>row.id).sort(),before);assert.ok(!existsSync(draftFile));assert.equal(hash(state.movedSource),state.changedSourceHash);done('completed-receipt-prevents-replaying-a-stale-draft',{persistedCompletedReceipt:true,noNewTasks:true,originalResultOpened:true})
  }
  console.log('zipEdit1546: complete '+receipt)
}

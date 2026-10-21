import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runZipEdit1545Gate({ driver, bridge, artifactDirectory, application, root, bundledSevenZip }) {
  const base=path.join(root,'test-results/zip-edit-1545');mkdirSync(base,{recursive:true})
  const receipt=path.join(base,'desktop-progress.json'),hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex'),binarySha256=hash(application)
  const state=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.5.45',binarySha256,work:path.join(base,randomUUID()),checks:[],layouts:[],screenshots:[]}
  if(state.binarySha256!==binarySha256){assert.equal(process.env.ZIP_EDIT_1545_RESUME_FROM,state.binarySha256,'Acknowledge the earlier UI evidence');for(const row of [...state.checks,...state.layouts])row.binarySha256??=state.binarySha256;state.previousIdentities=[...(state.previousIdentities||[]),state.binarySha256];state.binarySha256=binarySha256}
  mkdirSync(state.work,{recursive:true});const save=()=>writeFileSync(receipt,JSON.stringify(state,null,2)),passed=kind=>state.checks.some(row=>row.kind===kind),done=(kind,evidence={})=>{state.checks.push({kind,binarySha256,...evidence});save();console.log('zipEdit1545: '+kind)}
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
      const file=`zip-edit1545-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,binarySha256,...layout});state.screenshots.push(file);save()
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
  if(!state.source){
    state.source=path.join(state.work,'待整理原包.zip');state.addition=path.join(state.work,'新增资料.txt');state.replacement=path.join(state.work,'替换内容.txt')
    writeFileSync(state.addition,'added after directory move');writeFileSync(state.replacement,'replacement at the moved path')
    execFileSync('python',['-X','utf8','-c',`import sys,zipfile
with zipfile.ZipFile(sys.argv[1],'w') as z:
 z.comment='保留原包备注'.encode('utf-8')
 for name,content in [('README.txt','original readme'),('docs/a.txt','alpha'),('docs/b.txt','beta'),('docs/empty/',''),('obsolete/remove.txt','remove me'),('obsolete/sub/',''),('case.txt','case only content')]:
  i=zipfile.ZipInfo(name,(2020,2,3,4,5,6));i.compress_type=zipfile.ZIP_DEFLATED;z.writestr(i,content.encode('utf-8'))`,state.source],{windowsHide:true})
    state.sourceHash=hash(state.source);save()
  }
  if(!passed('mixed-directory-batch-move-delete-add-replace-save')){
    await bridge('clearTasks');await bridge('clearTaskHistory');await openArchive(state.source)
    await context('docs/','[data-testid="archive-context-edit-rename"]');await fill('[data-testid="zip-organize-single-path"]','交付资料');assert.match(await txt('[data-testid="zip-organize-preview"]'),/交付资料\/a.txt/)
    await screenshot('directory-rename','[data-testid="zip-organize-rules"]');await click('[data-testid="zip-organize-apply"]');await enter('交付资料/');await choose('交付资料/a.txt');await choose('交付资料/b.txt')
    await fill('[data-testid="zip-organize-prefix"]','交付-');await fill('[data-testid="zip-organize-suffix"]','-终版');await fill('[data-testid="zip-organize-destination"]','交付资料/文稿');assert.match(await txt('[data-testid="zip-organize-preview"]'),/交付资料\/文稿\/交付-a-终版.txt/)
    await screenshot('batch-move','[data-testid="zip-organize-rules"]');await click('[data-testid="zip-organize-apply"]');await click('[data-testid="zip-organize-root"]');await choose('obsolete/');await click('[data-testid="zip-organize-mode-delete"]');assert.match(await txt('[data-testid="zip-organize-preview"]'),/obsolete\/remove.txt/);await click('[data-testid="zip-organize-apply"]');await click('[data-testid="zip-organize-review"]')
    // Undo one staged delete, then stage it again from the projected tree.
    for(const row of await driver.findElements(By.css('[data-testid="zip-edit-change"]')))if((await row.getAttribute('textContent')).includes('obsolete/remove.txt')){await click(await row.findElement(By.css('[data-testid="zip-edit-undo-operation"]')));break}
    await click('[data-testid="zip-edit-structure-preview"]');await choose('obsolete/');await click('[data-testid="zip-organize-mode-delete"]');await click('[data-testid="zip-organize-apply"]')
    await click('[data-testid="zip-organize-mode-rename"]');await enter('交付资料/');await enter('交付资料/文稿/');await bridge('queueDesktopDialogSelections',[[state.addition]]);await click('[data-testid="zip-organize-add"]');assert.match(await txt('[data-testid="zip-edit-change-list"]'),/交付资料\/文稿\/新增资料.txt/);await closeReview()
    const docs=await find('[data-entry-path="docs/"]');await driver.actions().doubleClick(docs).perform();await bridge('queueDesktopDialogSelections',[[state.replacement]]);await context('docs/a.txt','[data-testid="archive-context-edit-replace"]');assert.match(await txt('[data-testid="zip-edit-change-list"]'),/交付资料\/文稿\/交付-a-终版.txt/)
    await fill('[data-testid="zip-edit-review-search"]','obsolete/remove.txt');assert.equal((await driver.findElements(By.css('[data-testid="zip-edit-change"]'))).length,1);await fill('[data-testid="zip-edit-review-search"]','')
    state.output=path.join(state.work,'整理交付-'+randomUUID().slice(0,8)+'.zip');save();await fill('[data-testid="zip-edit-output"]',state.output);await screenshot('unified-review','[data-testid="zip-edit-change-list"]');await click('[data-testid="zip-edit-save"]')
    const completed=await complete(row=>row.executionConfig?.kind==='archive-edit'&&same(row.outputPath,state.output));assert.equal(completed.status,'completed',JSON.stringify(completed));assert.equal(hash(state.source),state.sourceHash)
    const extracted=path.join(state.work,'独立回解');execFileSync(bundledSevenZip,['x','-y','-o'+extracted,state.output],{windowsHide:true,maxBuffer:1024*1024})
    assert.equal(readFileSync(path.join(extracted,'交付资料','文稿','交付-a-终版.txt'),'utf8'),'replacement at the moved path');assert.equal(readFileSync(path.join(extracted,'交付资料','文稿','交付-b-终版.txt'),'utf8'),'beta');assert.equal(hash(path.join(extracted,'交付资料','文稿','新增资料.txt')),hash(state.addition));assert.ok(existsSync(path.join(extracted,'交付资料','empty')));assert.ok(!existsSync(path.join(extracted,'docs'))&&!existsSync(path.join(extracted,'obsolete')));assert.equal(readFileSync(path.join(extracted,'README.txt'),'utf8'),'original readme')
    state.savedId=completed.id;save();await screenshot('organized-result','[data-testid="archive-output-summary"]');done('mixed-directory-batch-move-delete-add-replace-save',{originalHashPreserved:true,independentExtraction:true,emptyDirectoryRetained:true})
  }
  if(!passed('collision-case-only-failure-correction-result-preview-extraction')){
    await openArchive(state.source);const caseCheckbox=await find('[data-entry-path="case.txt"] .browser-checkbox');if((await caseCheckbox.getAttribute('aria-label')).startsWith('取消选择'))await click(caseCheckbox);await context('case.txt','[data-testid="archive-context-edit-rename"]');await fill('[data-testid="zip-organize-single-path"]','README.txt');await find('[data-testid="zip-organize-issue"]');assert.equal(await(await find('[data-testid="zip-organize-apply"]')).isEnabled(),false)
    await fill('[data-testid="zip-organize-single-path"]','CASE.txt');await click('[data-testid="zip-organize-apply"]');await click('[data-testid="zip-organize-review"]')
    const occupied=path.join(state.work,'已有结果.zip');writeFileSync(occupied,'keep existing file');const occupiedHash=hash(occupied);await fill('[data-testid="zip-edit-output"]',occupied);await click('[data-testid="zip-edit-save"]');const failed=await complete(row=>row.executionConfig?.kind==='archive-edit'&&same(row.outputPath,occupied));assert.equal(failed.status,'failed',JSON.stringify(failed));await screenshot('failed-organizing-save','[data-testid="zip-edit-save-panel"]');assert.equal(hash(occupied),occupiedHash)
    await closeReview();await click('[data-testid="nav-Compress"]');await click('[data-testid="global-progress-summary"]');await click(`[data-testid="global-task-prepare-retry"][data-retry-task-id="${failed.id}"]`);await click('[data-testid="zip-edit-review"]');assert.match(await txt('[data-testid="zip-edit-change-list"]'),/CASE.txt/)
    state.corrected=path.join(state.work,'纠正整理-'+randomUUID().slice(0,8)+'.zip');save();await fill('[data-testid="zip-edit-output"]',state.corrected);await click('[data-testid="zip-edit-save"]');const completed=await complete(row=>row.retryOfTaskId===failed.id);assert.equal(completed.status,'completed',JSON.stringify(completed));assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed);assert.equal(hash(occupied),occupiedHash);assert.equal(hash(state.source),state.sourceHash)
    await click('[data-testid="zip-edit-browse-result"]');await find('[data-entry-path="CASE.txt"]');const row=await find('[data-entry-path="CASE.txt"]');await click(await row.findElement(By.css('.preview-trigger')));await driver.wait(async()=>(await txt('[data-testid="archive-text-preview"]')).includes('case only content'),15000);await screenshot('renamed-preview','[data-testid="archive-text-preview"]');await click('[aria-label="关闭预览"]')
    const destination=path.join(state.work,'界面提取');mkdirSync(destination);await bridge('queueDesktopDialogSelections',[destination]);await click('.output-target');await click('.browser-page > footer .browser-primary');await driver.wait(async()=>existsSync(path.join(destination,'CASE.txt')),60000);assert.equal(readFileSync(path.join(destination,'CASE.txt'),'utf8'),'case only content')
    done('collision-case-only-failure-correction-result-preview-extraction',{collisionBlocked:true,originalFailurePreserved:true,existingOutputPreserved:true,caseOnlyRename:true,previewAndExtraction:true})
  }
  if(!passed('delete-all-saves-and-browses-an-empty-zip')){
    await openArchive(state.source);await click('[data-testid="zip-edit-begin"]');await click('[data-testid="zip-edit-organize"]');await click('[data-testid="zip-organize-select-all"]');await click('[data-testid="zip-organize-mode-delete"]');await click('[data-testid="zip-organize-apply"]');await click('[data-testid="zip-organize-review"]')
    state.empty=path.join(state.work,'清空后-'+randomUUID().slice(0,8)+'.zip');save();await fill('[data-testid="zip-edit-output"]',state.empty);await click('[data-testid="zip-edit-save"]');const completed=await complete(row=>same(row.outputPath,state.empty));assert.equal(completed.status,'completed',JSON.stringify(completed));await click('[data-testid="zip-edit-browse-result"]');await driver.wait(async()=>(await txt('.browser-summary')).includes('0'),15000);assert.equal((await driver.findElements(By.css('[data-entry-path]'))).length,0);assert.equal(hash(state.source),state.sourceHash);await screenshot('empty-zip','[data-testid="zip-edit-bar"]');done('delete-all-saves-and-browses-an-empty-zip',{originalPreserved:true,emptyArchiveReadable:true})
  }
  console.log('zipEdit1545: complete '+receipt)
}

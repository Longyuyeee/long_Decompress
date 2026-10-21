import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runZipEdit1544Gate({ driver, bridge, artifactDirectory, application, root, bundledSevenZip }) {
  const base=path.join(root,'test-results/zip-edit-1544');mkdirSync(base,{recursive:true})
  const receipt=path.join(base,'desktop-progress.json'),hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex'),binarySha256=hash(application)
  const state=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.5.44',binarySha256,work:path.join(base,randomUUID()),checks:[],layouts:[],screenshots:[]}
  if(state.binarySha256!==binarySha256){assert.equal(process.env.ZIP_EDIT_1544_RESUME_FROM,state.binarySha256,'Acknowledge the earlier UI evidence');for(const row of [...state.checks,...state.layouts])row.binarySha256??=state.binarySha256;state.previousIdentities=[...(state.previousIdentities||[]),state.binarySha256];state.binarySha256=binarySha256}
  mkdirSync(state.work,{recursive:true});const save=()=>writeFileSync(receipt,JSON.stringify(state,null,2)),passed=kind=>state.checks.some(row=>row.kind===kind),done=(kind,evidence={})=>{state.checks.push({kind,binarySha256,...evidence});save();console.log('zipEdit1544: '+kind)}
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
      const layout=await driver.executeScript(selector=>{const e=[...document.querySelectorAll(selector)].find(e=>e.offsetHeight>0);const review=e?.closest('[data-testid="zip-edit-review-content"]');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,anchorOverflow:e?e.scrollWidth-e.clientWidth:0,columns:review?[...review.children].map(c=>c.getBoundingClientRect().width):[]}},anchor)
      assert.ok(layout.overflow<=1&&layout.anchorOverflow<=1,JSON.stringify(layout));if(width===1100&&layout.columns.length===2)assert.ok(Math.abs(layout.columns[0]-layout.columns[1])<=1,JSON.stringify(layout))
      const file=`zip-edit1544-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,binarySha256,...layout});state.screenshots.push(file);save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  async function openArchive(file){
    for(const button of await driver.findElements(By.css('button[aria-label="关闭编辑清单"]')))if(await button.isDisplayed())await click(button)
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
  if(!state.source){
    state.source=path.join(state.work,'需要保留的原包.zip');state.incoming=path.join(state.work,'待补入资料');mkdirSync(state.incoming,{recursive:true})
    for(const[name,content]of Object.entries({'README.txt':'new readme content','原资料.txt':'keep both copy','原来保留.txt':'skip this replacement','新增资料-长名称-项目交付说明.txt':'new content'}))writeFileSync(path.join(state.incoming,name),content)
    state.folder=path.join(state.work,'补充目录');mkdirSync(path.join(state.folder,'子目录'),{recursive:true});mkdirSync(path.join(state.folder,'新空目录'),{recursive:true});writeFileSync(path.join(state.folder,'子目录','说明.txt'),'nested added content')
    state.drag=path.join(state.work,'拖入资料.txt');writeFileSync(state.drag,'added through drag and drop')
    execFileSync('python',['-X','utf8','-c',`import sys,zipfile
with zipfile.ZipFile(sys.argv[1],'w') as z:
 z.comment='原包备注'.encode('utf-8')
 for name,content in [('README.txt','old readme'),('原资料.txt','original material'),('原来保留.txt','keep this content'),('文档/中文说明.txt','原有中文内容'),('原空目录/','')]:
  i=zipfile.ZipInfo(name,(2020,2,3,4,5,6));i.compress_type=zipfile.ZIP_DEFLATED;z.writestr(i,content.encode('utf-8'))`,state.source],{windowsHide:true})
    state.sourceHash=hash(state.source);save()
  }
  if(!passed('save-as-add-replace-conflicts-folder-drag')){
    await bridge('clearTasks');await bridge('clearTaskHistory');await openArchive(state.source);await click('[data-testid="zip-edit-begin"]')
    const inputs=['README.txt','原资料.txt','原来保留.txt','新增资料-长名称-项目交付说明.txt'].map(name=>path.join(state.incoming,name))
    await bridge('queueDesktopDialogSelections',[inputs]);await click('[data-testid="zip-edit-add-files"]');assert.equal(await(await find('[data-testid="zip-edit-save"]')).isEnabled(),false)
    await selectChoice('README.txt','replace');await selectChoice('原资料.txt','keep-both');await selectChoice('原来保留.txt','skip')
    assert.match(await txt('[data-testid="zip-edit-change-list"]'),/原资料 \(2\)\.txt/)
    await screenshot('conflict-review','[data-testid="zip-edit-change-list"]');await closeReview()
    await bridge('queueDesktopDialogSelections',[[state.folder]]);await click('[data-testid="zip-edit-add-folder"]');await closeReview()
    await driver.executeScript(file=>{const transfer=new DataTransfer(),item=new File([''],'拖入资料.txt');Object.defineProperty(item,'path',{value:file});transfer.items.add(item);document.querySelector('.browser-page').dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:transfer}))},state.drag)
    await find('[data-testid="zip-edit-review-content"]')
    // Undo the dropped item once, then add it again through the same real drop handler.
    const dragRow=await driver.wait(async()=>{for(const row of await driver.findElements(By.css('[data-testid="zip-edit-change"]')))if((await row.getAttribute('textContent')).includes('拖入资料.txt'))return row;return false},15000)
    await click(await dragRow.findElement(By.css('[data-testid="zip-edit-undo"]')));await closeReview()
    await driver.executeScript(file=>{const transfer=new DataTransfer(),item=new File([''],'拖入资料.txt');Object.defineProperty(item,'path',{value:file});transfer.items.add(item);document.querySelector('.browser-page').dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:transfer}))},state.drag)
    state.output=path.join(state.work,'编辑后-'+randomUUID().slice(0,8)+'.zip');save();await fill('[data-testid="zip-edit-output"]',state.output);await screenshot('save-review','[data-testid="zip-edit-save-panel"]');await click('[data-testid="zip-edit-save"]')
    const result=await complete(row=>row.executionConfig?.kind==='archive-edit'&&same(row.outputPath,state.output));assert.equal(result.status,'completed',JSON.stringify(result));assert.equal(hash(state.source),state.sourceHash)
    state.savedId=result.id;save();await screenshot('save-result','[data-testid="archive-output-summary"]')
    const extracted=path.join(state.work,'独立回解');execFileSync(bundledSevenZip,['x','-y','-o'+extracted,state.output],{windowsHide:true,maxBuffer:1024*1024})
    assert.equal(readFileSync(path.join(extracted,'README.txt'),'utf8'),'new readme content');assert.equal(readFileSync(path.join(extracted,'原资料.txt'),'utf8'),'original material');assert.equal(readFileSync(path.join(extracted,'原资料 (2).txt'),'utf8'),'keep both copy');assert.equal(readFileSync(path.join(extracted,'原来保留.txt'),'utf8'),'keep this content');assert.equal(readFileSync(path.join(extracted,'文档','中文说明.txt'),'utf8'),'原有中文内容');assert.equal(hash(path.join(extracted,'拖入资料.txt')),hash(state.drag));assert.ok(existsSync(path.join(extracted,'原空目录'))&&existsSync(path.join(extracted,'补充目录','新空目录')))
    done('save-as-add-replace-conflicts-folder-drag',{originalHashPreserved:true,independentContents:true,emptyDirectories:true})
  }
  if(!passed('new-result-browse-preview-and-selected-extraction')){
    await openArchive(state.output);await driver.wait(async()=>(await txt('.browser-summary')).includes('ZIP'),10000)
    const row=await find('[data-entry-path="README.txt"]');await click(await row.findElement(By.css('.preview-trigger')));await driver.wait(async()=>(await txt('[data-testid="archive-text-preview"]')).includes('new readme content'),15000);await screenshot('result-preview','[data-testid="archive-text-preview"]');await click('[aria-label="关闭预览"]')
    const destination=path.join(state.work,'界面提取-'+randomUUID().slice(0,8));mkdirSync(destination)
    await bridge('queueDesktopDialogSelections',[destination]);await click('.output-target');await click('.browser-page > footer .browser-primary')
    await driver.wait(async()=>existsSync(path.join(destination,'README.txt')),60000);assert.equal(readFileSync(path.join(destination,'README.txt'),'utf8'),'new readme content');assert.equal(hash(path.join(destination,'拖入资料.txt')),hash(state.drag));assert.equal(hash(state.source),state.sourceHash)
    done('new-result-browse-preview-and-selected-extraction',{nativePreview:true,selectedExtraction:true})
  }
  if(!passed('failed-save-retains-edits-and-original-failure-on-corrected-save')){
    await openArchive(state.source);await click('[data-testid="zip-edit-begin"]');await bridge('queueDesktopDialogSelections',[[state.drag]]);await click('[data-testid="zip-edit-add-files"]')
    await closeReview();const originalRow=await find('[data-entry-path="README.txt"]');await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),originalRow);await driver.actions().contextClick(originalRow).perform();await bridge('queueDesktopDialogSelections',[[path.join(state.incoming,'README.txt')]]);await click('[data-testid="archive-context-edit-replace"]');await selectChoice('README.txt','skip');await selectChoice('README.txt','replace')
    const occupied=path.join(state.work,'已有文件.zip');writeFileSync(occupied,'user result must remain');const existingHash=hash(occupied);await fill('[data-testid="zip-edit-output"]',occupied);await click('[data-testid="zip-edit-save"]')
    const failed=await complete(row=>row.executionConfig?.kind==='archive-edit'&&same(row.outputPath,occupied));assert.equal(failed.status,'failed');await find('[data-testid="zip-edit-change"]');await screenshot('failed-save','[data-testid="zip-edit-save-panel"]');assert.equal(hash(occupied),existingHash)
    await closeReview();await click('[data-testid="nav-Compress"]');assert.equal((await driver.findElements(By.css('[data-testid="compression-draft-row"],[data-testid="compression-group-row"]'))).length,0)
    await click('[data-testid="global-progress-summary"]');await click(`[data-testid="global-task-prepare-retry"][data-retry-task-id="${failed.id}"]`);await find('[data-testid="zip-edit-review"]');await click('[data-testid="zip-edit-review"]');assert.equal((await driver.findElements(By.css('[data-testid="zip-edit-change"]'))).length,2)
    const corrected=path.join(state.work,'纠正另存-'+randomUUID().slice(0,8)+'.zip');await fill('[data-testid="zip-edit-output"]',corrected);await click('[data-testid="zip-edit-save"]');const completed=await complete(row=>row.retryOfTaskId===failed.id);assert.equal(completed.status,'completed',JSON.stringify(completed));assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed);assert.equal(hash(occupied),existingHash);assert.equal(hash(state.source),state.sourceHash)
    await screenshot('corrected-result','[data-testid="archive-output-summary"]');await closeReview();done('failed-save-retains-edits-and-original-failure-on-corrected-save',{existingFilePreserved:true,oldFailurePreserved:true})
  }
  console.log('zipEdit1544: complete '+receipt)
}

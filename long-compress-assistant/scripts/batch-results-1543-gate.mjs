import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { mkdirSync, readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runBatchResults1543Gate({ driver, bridge, artifactDirectory, application, root, bundledSevenZip }) {
  const base=path.join(root,'test-results/batch-results-1543');mkdirSync(base,{recursive:true})
  const progress=path.join(base,'desktop-progress.json'),hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex')
  const binarySha256=hash(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.43',binarySha256,work:path.join(base,randomUUID()),checks:[],layouts:[],screenshots:[]}
  if(state.binarySha256!==binarySha256){for(const row of [...state.checks,...state.layouts])row.binarySha256??=state.binarySha256;assert.equal(process.env.BATCH_1543_RESUME_FROM,state.binarySha256,'Acknowledge previous evidence identity');state.previousIdentities=[...(state.previousIdentities||[]),state.binarySha256];state.binarySha256=binarySha256}
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2)),done=(kind,details={})=>{state.checks.push({kind,binarySha256,...details});save();console.log('batch1543: '+kind)},passed=kind=>state.checks.some(row=>row.kind===kind)
  mkdirSync(state.work,{recursive:true})
  const find=selector=>driver.wait(async()=>{
    try {
      const dialogs=await driver.findElements(By.css('[role="dialog"]'))
      for(const dialog of dialogs.reverse())if(await dialog.isDisplayed()){
        const elements=await dialog.findElements(By.css(selector));for(const element of elements.reverse())if(await element.isDisplayed())return element
        return false
      }
      const elements=await driver.findElements(By.css(selector));for(const element of elements)if(await element.isDisplayed())return element
      return false
    }catch(error){if(error.name==='StaleElementReferenceError')return false;throw error}
  },15000)
  const click=async selector=>{const element=typeof selector==='string'?await find(selector):selector;await driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});e.click()},element)}
  const fill=async(selector,value)=>{const element=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),element);await element.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await element.sendKeys(value)}
  const same=(a,b)=>a.replace(/\\/g,'/').toLowerCase()===b.replace(/\\/g,'/').toLowerCase()
  const text=async selector=>(await find(selector)).getAttribute('textContent')
  const history=async(predicate,count)=>driver.wait(async()=>{const rows=(await bridge('taskHistory')).filter(predicate);return rows.length===count&&rows.every(row=>['completed','failed','cancelled'].includes(row.status))?rows:false},90000)
  async function screenshots(name,anchor){
    if(state.layouts.filter(layout=>layout.name===name).length===2)return
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),await find(anchor))
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const layout=await driver.executeScript(selector=>{const anchors=[...document.querySelectorAll(selector)].filter(e=>e.offsetHeight>0),e=anchors.at(-1),d=e?.closest('[data-testid="compression-draft-details"],[data-testid="decompression-task-details"]');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,anchorOverflow:e?e.scrollWidth-e.clientWidth:0,columns:d?[...d.children].map(e=>e.getBoundingClientRect().width):[]}},anchor)
      assert.ok(layout.overflow<=1&&layout.anchorOverflow<=1,JSON.stringify(layout))
      if(layout.columns.length===2&&width===1100)assert.ok(Math.abs(layout.columns[0]-layout.columns[1])<=1,JSON.stringify(layout))
      const file=`batch1543-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,binarySha256,...layout});state.screenshots.push(file);save()
    }
    await driver.manage().window().setRect({width:1100,height:700})
  }
  if(!state.sources){state.sources=['资料-甲-'+ '很长的文件名'.repeat(5)+'.txt','资料-乙-需要处理.txt'].map((name,index)=>{const file=path.join(state.work,name);writeFileSync(file,('batch1543-'+index+'-').repeat(8192));return file});save()}
  if(!passed('compression-batch-summary-mixed-results-and-filtering')){
    await bridge('clearCompressionWorkspace');await bridge('clearTasks');await bridge('clearTaskHistory');await click('[data-testid="nav-Compress"]')
    await bridge('queueDesktopDialogSelections',[state.sources]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    const rows=await driver.wait(async()=>{const rows=await driver.findElements(By.css('[data-testid="compression-draft-row"]'));return rows.length===2?rows:false},10000)
    for(let index=0;index<rows.length;index++){
      await click(await rows[index].findElement(By.css('[data-testid="compression-archive-name"]')))
      await fill('[data-testid="compression-output-path"]',state.work)
      await fill('[data-testid="compression-filename-input"]',index?'已有文件请保留':'成功结果长名称-'+ '项目资料'.repeat(6)+'-'+randomUUID().slice(0,8))
      await click(await rows[index].findElement(By.css('[data-testid="compression-archive-name"]')))
    }
    const occupied=path.join(state.work,'已有文件请保留.zip');writeFileSync(occupied,'original-file-1543');const originalHash=hash(occupied)
    for(const row of rows)await click(await row.findElement(By.css('[data-testid="compression-group-checkbox"]')))
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await click('[data-testid="purpose-share"]');await click('[data-testid="purpose-apply"]')
    await click('[data-testid="batch-selected-subjects"] summary');assert.match(await text('[data-testid="batch-config-impact"]'),/格式：ZIP/)
    await screenshots('compression-batch-preview','[data-testid="batch-config-impact"]');await click('[data-testid="batch-config-apply"]')
    await click('[data-testid="start-compression"]')
    const records=await history(row=>row.executionConfig?.kind==='archive-compression'&&row.sourcePaths.some(file=>state.sources.some(source=>same(source,file))),2)
    assert.equal(records.filter(row=>row.status==='completed').length,1);assert.equal(records.filter(row=>row.status==='failed').length,1);assert.equal(hash(occupied),originalHash)
    state.compressed=records.find(row=>row.status==='completed').archiveOutput.primaryPath;save()
    await click('[data-testid="task-scope-completed"]');assert.equal((await driver.findElements(By.css('[data-testid="compression-draft-row"]'))).length,1)
    const success=await find('[data-testid="compression-archive-name"]');await click(success);await screenshots('compression-success','[data-testid="archive-output-summary"]')
    await click('[data-testid="task-scope-attention"]');assert.equal((await driver.findElements(By.css('[data-testid="compression-draft-row"]'))).length,1)
    await click('[data-testid="compression-archive-name"]');assert.match(await text('[data-testid="task-failure-hint"]'),/同名|已存在|保存/)
    await click('[data-testid="compression-prepare-retry"]');assert.equal(await(await find('[data-testid="task-scope-pending"]')).getAttribute('aria-pressed'),'true');assert.equal((await driver.findElements(By.css('[data-testid="compression-group-row"]'))).length,1)
    done('compression-batch-summary-mixed-results-and-filtering',{occupiedFilePreserved:true,originalFailurePreserved:(await bridge('taskHistory')).some(row=>row.id===records.find(row=>row.status==='failed').id)})
  }
  if(!passed('conversion-batch-preview-and-result-scope')){
    await bridge('clearCompressionWorkspace');await bridge('clearTasks');await bridge('clearTaskHistory');await click('[data-testid="nav-Compress"]')
    const other=path.join(state.work,'原包乙.zip');if(!existsSync(other))execFileSync(bundledSevenZip,['a','-tzip',other,state.sources[1]],{windowsHide:true,maxBuffer:1024*1024})
    await click('[data-testid="open-archive-conversion"]');await bridge('queueDesktopDialogSelections',[[state.compressed,other]]);await click('[data-testid="conversion-choose-source"]')
    await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length===2,10000)
    await click('[data-testid="conversion-batch-config"]');await click('[data-testid="purpose-share"]');await click('[data-testid="purpose-apply"]');await click('[data-testid="batch-selected-subjects"] summary')
    await screenshots('conversion-batch-preview','[data-testid="batch-config-impact"]');await click('[data-testid="conversion-batch-apply"]')
    const rows=await driver.findElements(By.css('[data-testid="conversion-item"]'))
    state.converted=[]
    for(let index=0;index<rows.length;index++){
      const row=await rows[index].findElement(By.css('[data-testid="conversion-row"]'));if(await row.getAttribute('aria-expanded')!=='true')await click(row)
      const output=path.join(state.work,`批量转换-${index}-${randomUUID().slice(0,8)}.zip`);state.converted.push(output);await fill('[data-testid="conversion-output"]',output);await click(row)
    }
    save();await click('[data-testid="conversion-start"]')
    const records=await history(row=>state.converted.some(output=>same(output,row.outputPath)),2);assert.ok(records.every(row=>row.status==='completed'),JSON.stringify(records))
    await click('[data-testid="task-scope-completed"]');assert.equal((await driver.findElements(By.css('[data-testid="conversion-item"]'))).length,2)
    const row=await find('[data-testid="conversion-row"]');if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    await screenshots('conversion-success','[data-testid="conversion-result"]')
    await click('[data-testid="task-scope-attention"]');await find('[data-testid="task-scope-empty"]');await click('[data-testid="task-scope-all"]')
    await click('button[aria-label="Close dialog"]')
    done('conversion-batch-preview-and-result-scope')
  }
  if(!passed('extraction-batch-preview-results-and-content') || process.env.BATCH_1543_EXTRACTION_LAYOUT==='1'){
    const layoutRefresh=process.env.BATCH_1543_EXTRACTION_LAYOUT==='1'
    if(layoutRefresh)state.converted=state.converted.map((file,index)=>{const next=path.join(state.work,`显示接续-${index}-${randomUUID().slice(0,8)}.zip`);writeFileSync(next,readFileSync(file));return next})
    await bridge('clearTasks');await bridge('clearTaskHistory');await click('[data-testid="nav-Decompress"]');await bridge('queueDesktopDialogSelections',[state.converted]);await click('[data-testid="dropzone-file"]')
    await driver.wait(async()=>(await bridge('extractionRecoveryTasks')).length===2,15000)
    for(const checkbox of await driver.findElements(By.css('[data-testid="task-select"]')))await click(checkbox)
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await click('[data-testid="batch-subfolder"]');const choices=await driver.findElements(By.css('[role="option"]'));let choice
    for(const option of choices)if((await option.getAttribute('textContent')).includes('创建同名子文件夹'))choice=option
    assert.ok(choice);await click(choice)
    await click('[data-testid="batch-selected-subjects"] summary');await screenshots('extraction-batch-preview','[data-testid="batch-config-impact"]');await click('[data-testid="batch-config-apply"]');await click('[data-testid="start-decompression-queue"]');assert.equal(await(await find('[data-testid="task-scope-all"]')).getAttribute('aria-pressed'),'true')
    const records=await history(row=>row.taskType==='decompression'&&row.sourcePaths.some(file=>state.converted.some(source=>same(source,file))),2);assert.ok(records.every(row=>row.status==='completed'),JSON.stringify(records))
    const walk=directory=>readdirSync(directory,{withFileTypes:true}).flatMap(file=>file.isDirectory()?walk(path.join(directory,file.name)):[path.join(directory,file.name)])
    for(const record of records){const files=walk(record.outputPath);assert.equal(files.length,1);const sourceIndex=state.converted.findIndex(source=>record.sourcePaths.some(file=>same(source,file)));assert.ok(sourceIndex>=0);assert.equal(hash(state.sources[sourceIndex]),hash(files[0]))}
    await click('[data-testid="task-scope-completed"]');await find('[data-testid="batch-result-actions"]');await click('[data-testid="task-row"]');await screenshots(layoutRefresh?'extraction-success-final':'extraction-success','[data-testid="decompression-task-details"]')
    done(layoutRefresh?'extraction-final-layout-and-content':'extraction-batch-preview-results-and-content',{contentHashes:true,binarySha256})
  }
  console.log('batch1543: complete '+progress)
}

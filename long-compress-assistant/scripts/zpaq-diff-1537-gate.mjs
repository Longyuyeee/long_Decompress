import assert from 'node:assert/strict'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync,readdirSync,renameSync,copyFileSync,unlinkSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runZpaqDiff1537Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const base=path.join(root,'test-results/zpaq-diff-1537'),progress=path.join(base,'desktop-progress.json'),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p)),binarySha256=sha(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.37',binarySha256,sourceCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim(),work:path.join(base,'ui-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  if(state.binarySha256!==binarySha256){
    assert.equal(process.env.ZPAQ_1537_RESUME_FROM,state.binarySha256,'Acknowledge the earlier completed chains before continuing affected drafts only')
    state.layouts=state.layouts.map(row=>({...row,binarySha256:row.binarySha256||state.binarySha256}))
    state.previousIdentities=[...(state.previousIdentities||[]),{binarySha256:state.binarySha256,sourceCommit:state.sourceCommit}]
    state.binarySha256=binarySha256;state.sourceCommit=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim()
  }
  const work=state.work;mkdirSync(work,{recursive:true})
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const done=(group,kind,data={})=>{state[group].push({kind,binarySha256,...data});save();console.log('zpaq-diff-1537: '+group+' '+kind)}
  const passed=(group,kind)=>state[group].some(row=>row.kind===kind)
  const tree=(directory,prefix='')=>Object.fromEntries(readdirSync(directory,{withFileTypes:true}).flatMap(entry=>{const name=prefix+entry.name,file=path.join(directory,entry.name);return entry.isDirectory()?Object.entries(tree(file,name+'/')):[[name,sha(file)]]}))
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const result=await invoke(cmd,args);assert.equal(result.ok,true,result.error);return result.value}
  const click=async target=>{
    for(const toast of await driver.findElements(By.css('.toast-item button'))){try{if(await toast.isDisplayed())await toast.click()}catch{}}
    const element=typeof target==='string'?await waitForElement(target):target
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&r.height>0&&(hit===e||e.contains(hit))},element),15000);await driver.executeScript(e=>e.click(),element)
  }
  const fill=async(selector,value)=>{const e=typeof selector==='string'?await waitForElement(selector):selector;await driver.wait(async()=>await e.isEnabled()&&await e.isDisplayed(),35000);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const choose=async(selector,label)=>{await click(selector);for(const option of await driver.findElements(By.css('[role="option"]'))){const strong=await option.findElements(By.css('strong')),text=(await(strong[0]||option).getAttribute('textContent')).trim();if(text===label||text.startsWith(label+'（')){await click(option);return}}throw Error('Missing option '+label)}
  const sizes=async(name,anchor)=>{
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),(typeof anchor==='string'?await waitForElement(anchor):anchor))
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('[role="dialog"],.browser-page')].map(e=>e.scrollWidth-e.clientWidth)}))
      assert.ok(layout.overflow<=1&&layout.panels.every(v=>v<=1),JSON.stringify(layout))
      const file='zpaq1537-'+name+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots=[...new Set([...state.screenshots,file])];state.layouts=state.layouts.filter(row=>!(row.name===name&&row.width===layout.width&&row.height===layout.height&&row.binarySha256===binarySha256));state.layouts.push({name,binarySha256,...layout});save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  const fixtures=path.join(root,'tests/fixtures/archives/zpaq-snapshots-1530'),manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8')),password=manifest.password,original=manifest.cases.find(c=>c.encrypted)
  const snap=n=>({version:n,date:original.snapshots[n-1].date})
  if(!state.parts){const bytes=readFileSync(path.join(fixtures,original.file));assert.equal(hash(bytes),original.sha256);const parts=[];for(let offset=0,n=1;offset<bytes.length;n++,offset+=65536){const file=owned(path.join(work,'资料备份.zpaq.'+String(n).padStart(3,'0')));writeFileSync(file,bytes.subarray(offset,offset+65536));parts.push(file)}state.parts=parts;save()}
  const member=state.parts[1],select=async(selector,value)=>{const element=await waitForElement(selector);await driver.executeScript((e,v)=>{e.value=String(v);e.dispatchEvent(new Event('change',{bubbles:true}))},element,value)}
  const completed=async id=>driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.id===id&&['completed','failed','cancelled'].includes(r.status))||false,60000)
  const savedDrafts=()=>driver.executeScript(()=>{try{return JSON.parse(localStorage.getItem('long-pending-archive-drafts-v1'))?.extraction?.items||[]}catch{return[]}})
  const expected=record=>{const files=original.snapshots[record.executionConfig.zpaqSnapshot.version-1].expected;return Object.fromEntries(record.executionConfig.selectedEntries.filter(p=>Object.hasOwn(files,p)).map(p=>[p,files[p]]))}
  const verifyOutput=record=>{assert.deepEqual(tree(record.outputPath),expected(record));const snapshot=original.snapshots[record.executionConfig.zpaqSnapshot.version-1];for(const p of record.executionConfig.selectedEntries)if(!Object.hasOwn(snapshot.expected,p)){assert.ok(snapshot.directories.includes(p));assert.ok(existsSync(path.join(record.outputPath,p)))}assert.match(record.executionConfig.zpaqCatalogId,/^[0-9a-f]{64}$/);assert.ok(!JSON.stringify(record).includes(password))}
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  if(!passed('native','public-catalog-cancellation-and-missing-volume')){
    const args={filePath:member,password,browseId:randomUUID(),before:snap(1),after:snap(2),verifyPaths:[],expectedRevision:null}
    const result=await ipc('compare_zpaq_snapshots',args);assert.ok(result.entries.some(e=>e.kind==='candidate'&&!e.verified));assert.ok(result.entries.some(e=>e.kind==='removed'))
    const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const cancelled=await invoke('compare_zpaq_snapshots',{...args,browseId:id});assert.equal(cancelled.ok,false);assert.match(cancelled.error,/停止|取消/)
    const wrong=await invoke('compare_zpaq_snapshots',{...args,browseId:randomUUID(),password:'wrong'});assert.equal(wrong.ok,false)
    const missing=state.parts[2],backup=owned(missing+'.owned-original');renameSync(missing,backup);try{const incomplete=await invoke('compare_zpaq_snapshots',{...args,browseId:randomUUID()});assert.equal(incomplete.ok,false);assert.match(incomplete.error,/分卷|缺失|incomplete/)}finally{renameSync(backup,missing)}
    done('native','public-catalog-cancellation-and-missing-volume',{referenceOnly:true,cancelledBeforeRegistration:cancelled,wrongPasswordRejected:true,missingMemberRejected:true})
  }
  if(!passed('ui','compare-exact-and-queue-by-version')){
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[member]]);await click('[data-testid="file-manager-open-archive"],.browser-header-actions .browser-primary');await fill('.browser-toolbar input[type="password"]',password);await click('.browser-toolbar button[title="重新读取"]');await waitForElement('[data-testid="zpaq-snapshot-diff"]');await click('[data-testid="zpaq-snapshot-diff"]');await waitForElement('[data-testid="zpaq-diff"]');await select('[data-testid="zpaq-diff-before"]',1);await select('[data-testid="zpaq-diff-after"]',2);await click('[data-testid="zpaq-diff-compare"]');await waitForElement('[data-testid="zpaq-diff-entries"]');await sizes('catalog-comparison','[data-testid="zpaq-diff-counts"]')
    await fill('[data-testid="zpaq-diff-search"]','后来删除');assert.equal((await driver.findElements(By.css('[data-diff-path]'))).length,1);assert.equal(await(await waitForElement('[data-diff-path]')).getAttribute('data-diff-kind'),'removed');await fill('[data-testid="zpaq-diff-search"]','')
    await click('[data-testid="zpaq-diff-verify"]');await driver.wait(async()=>/已按实际内容核对/.test(await(await waitForElement('[data-testid="zpaq-diff-notice"]')).getText()),60000);assert.ok((await(await waitForElement('[data-testid="zpaq-diff-entries"]')).getText()).includes('SHA256'))
    const same=await waitForElement('[data-diff-path="资料/说明.txt"] [data-diff-side="before"]');if(!await same.isSelected())await click(same);await fill('[data-testid="zpaq-diff-output"]',work);await sizes('verified-and-selection','[data-diff-path="资料/说明.txt"]');await click('[data-testid="zpaq-diff-create"]');await waitForElement('[data-testid="zpaq-diff-review"]');await sizes('pending-confirmation','[data-testid="zpaq-diff-notice"]');await click('[data-testid="zpaq-diff-review"]')
    const drafts=await driver.wait(async()=>{const d=await savedDrafts();return d.length===2?d:false},15000);assert.deepEqual(drafts.map(d=>d.zpaqSnapshot.version).sort(),[1,2]);assert.ok(drafts.every(d=>d.zpaqCatalogId&&d.selectedEntries.length&&d.draftPasswordRequired));assert.ok(!JSON.stringify(drafts).includes(password));assert.ok(drafts.every(d=>!existsSync(d.outputPath)));done('ui','compare-exact-and-queue-by-version',{drafts,notAutomaticallyStarted:true})
  }
  if(!passed('ui','restored-selection-missing-volume-retry')){
    await driver.navigate().refresh();await waitForElement('[data-testid="pending-draft-recovery"]');await click(await driver.findElement(By.xpath('//*[@data-testid="pending-draft-recovery"]//button[contains(.,"恢复任务")]')));await click('[data-testid="nav-Decompress"]')
    const restored=await driver.wait(async()=>{const r=await bridge('extractionRecoveryTasks');return r.length===2?r:false},15000)
    for(const task of restored){const row=await waitForElement('[data-testid="task-row"][data-task-id="'+task.id+'"]');if(await row.getAttribute('aria-expanded')!=='true')await click(row);const notice=await row.findElement(By.css('[data-testid="draft-recovery-notice"]'));await fill(await notice.findElement(By.css('input[type="password"]')),password);await click(await notice.findElement(By.xpath('.//button[contains(.,"确认密码设置")]')));await click(await waitForElement('[data-testid="toggle-archive-task-'+task.id+'"]'))}
    await sizes('restored-pending','[data-testid="task-row"]');const missing=state.parts[2],backup=owned(missing+'.owned-original');renameSync(missing,backup);let failed
    try{await click('[data-testid="start-decompression-queue"]');failed=await Promise.all(restored.map(r=>completed(r.id)));assert.ok(failed.every(r=>r.status==='failed'),JSON.stringify(failed))}finally{renameSync(backup,missing)}
    assert.ok(failed.every(r=>r.executionConfig?.zpaqCatalogId&&r.executionConfig?.selectedEntries?.length),JSON.stringify(failed));await click('[data-testid="retry-batch-failures"]');for(const field of await driver.findElements(By.css('[data-testid="retry-password"]')))await fill(field,password);await sizes('guarded-missing-volume-retry','[data-testid="failed-extraction-retry"]');await click('[data-testid="create-retry-drafts"]')
    await driver.wait(async()=>/已创建/.test(await(await waitForElement('[data-testid="failed-extraction-retry"]')).getText()),20000);await click(await driver.findElement(By.xpath('//button[contains(.,"返回队列确认并启动")]')))
    const retries=(await bridge('extractionRecoveryTasks')).filter(r=>failed.some(f=>r.retryOfTaskId===f.id));assert.equal(retries.length,2);assert.ok(retries.every(r=>r.status==='pending'));await click('[data-testid="start-decompression-queue"]');const records=await Promise.all(retries.map(r=>completed(r.id)));assert.ok(records.every(r=>r.status==='completed'),JSON.stringify(records));for(const record of records)verifyOutput(record);for(const old of failed)assert.deepEqual((await bridge('taskHistory')).find(r=>r.id===old.id),old)
    const row=await waitForElement('[data-testid="task-row"][data-task-id="'+retries[0].id+'"]');if(await row.getAttribute('aria-expanded')!=='true')await click(row);await sizes('version-result','[data-testid="decompression-task-details"]');const columns=await driver.executeScript(()=>[...document.querySelector('[data-testid="decompression-task-details"] .task-detail-layout').children].map(e=>e.getBoundingClientRect().width));assert.ok(Math.abs(columns[0]-columns[1])<=1,JSON.stringify(columns));state.records=records;done('ui','restored-selection-missing-volume-retry',{failed,records,completeIndependentRangeHashes:true,columns})
  }
  async function historyDraft(previous,output){
    await click('[data-testid="nav-History"]');const row=await driver.wait(async()=>{for(const row of await driver.findElements(By.css('[data-testid="history-record-row"]')))if((await row.getText()).includes(previous.outputPath))return row;return false},15000);await click(await row.findElement(By.css('[data-testid="history-retry"]')));await waitForElement('[data-testid="history-extraction-draft"]');assert.match(await(await waitForElement('[data-testid="history-extraction-draft"]')).getText(),/保留所选/);assert.equal(await(await waitForElement('[data-testid="history-draft-password"]')).getAttribute('value'),'');await fill('[data-testid="history-draft-password"]',password);await fill('[data-testid="history-draft-output"]',output);const sub=await waitForElement('[data-testid="history-draft-subfolder"]');if(await sub.isSelected())await click(sub);await sizes('history-guarded-selection','[data-testid="history-extraction-draft"]');await click('[data-testid="history-draft-create"]');await click('[data-testid="history-extraction-draft"] a');return await driver.wait(async()=>(await bridge('extractionRecoveryTasks')).find(r=>r.retryOfTaskId===previous.id&&r.status==='pending'&&r.output===output)||false,15000)
  }
  if(!passed('ui','history-redo-exact-range')){
    const previous=state.records.find(r=>r.executionConfig.zpaqSnapshot.version===1),output=owned(path.join(work,'历史所选-'+randomUUID()));mkdirSync(output);const task=await historyDraft(previous,output);await click('[data-testid="start-decompression-queue"]');const record=await completed(task.id);assert.equal(record.status,'completed',JSON.stringify(record));assert.deepEqual(record.executionConfig.selectedEntries,previous.executionConfig.selectedEntries);assert.equal(record.executionConfig.zpaqCatalogId,previous.executionConfig.zpaqCatalogId);verifyOutput(record);done('ui','history-redo-exact-range',{record,rangeNotExpanded:true})
  }
  if(!passed('ui','changed-source-does-not-read-another-version')){
    const previous=state.records.find(r=>r.executionConfig.zpaqSnapshot.version===2),output=owned(path.join(work,'变化后拒绝-'+randomUUID()));mkdirSync(output);const task=await historyDraft(previous,output),source=state.parts[0],backup=owned(source+'.owned-original');renameSync(source,backup);writeFileSync(source,readFileSync(backup));let record
    try{await click('[data-testid="start-decompression-queue"]');record=await completed(task.id);assert.equal(record.status,'failed',JSON.stringify(record));assert.match(record.errorMessage,/变化|重新对比|snapshot/);assert.deepEqual(tree(output),{})}finally{unlinkSync(owned(source));renameSync(backup,source)}
    done('ui','changed-source-does-not-read-another-version',{record,noWrongVersionOutput:true})
  }
  assert.equal(hash(Buffer.concat(state.parts.map(p=>readFileSync(p)))),original.sha256);state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'zpaq-diff-1537-results.json'),JSON.stringify(state,null,2))
}

import assert from 'node:assert/strict'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync,readdirSync,renameSync,copyFileSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runZpaqSnapshots1530Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const base=path.join(root,'test-results/zpaq-snapshots-1530'),progress=path.join(base,'desktop-progress.json'),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p)),binarySha256=sha(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.30',binarySha256,sourceCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim(),work:path.join(base,'ui-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  if(state.binarySha256!==binarySha256){
    assert.equal(process.env.ZPAQ_1530_RESUME_FROM,state.binarySha256,'Explicitly acknowledge prior passed evidence identity')
    state.previousIdentities=[...(state.previousIdentities||[]),{binarySha256:state.binarySha256,sourceCommit:state.sourceCommit}]
    state.binarySha256=binarySha256;state.sourceCommit=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim()
  }
  const work=state.work;mkdirSync(work,{recursive:true})
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const done=(group,kind,data={})=>{state[group].push({kind,binarySha256,...data});save();console.log('zpaq-1530: '+group+' '+kind)}
  const passed=(group,kind)=>state[group].some(row=>row.kind===kind)
  const tree=(directory,prefix='')=>Object.fromEntries(readdirSync(directory,{withFileTypes:true}).flatMap(entry=>{const name=prefix+entry.name,file=path.join(directory,entry.name);return entry.isDirectory()?Object.entries(tree(file,name+'/')):[[name,sha(file)]]}))
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const result=await invoke(cmd,args);assert.equal(result.ok,true,result.error);return result.value}
  const click=async target=>{
    for(const toast of await driver.findElements(By.css('.toast-item button'))){try{if(await toast.isDisplayed())await toast.click()}catch{}}
    const element=typeof target==='string'?await waitForElement(target):target
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&r.height>0&&(hit===e||e.contains(hit))},element),15000);await driver.executeScript(e=>e.click(),element)
  }
  const closeRow=async row=>{await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')));await driver.wait(async()=>(await row.findElements(By.css('[data-testid="compression-draft-details"]'))).length===0,10000)}
  const fill=async(selector,value)=>{const e=await waitForElement(selector);await driver.wait(async()=>await e.isEnabled()&&await e.isDisplayed(),35000);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const choose=async(selector,label)=>{await click(selector);for(const option of await driver.findElements(By.css('[role="option"]'))){const strong=await option.findElements(By.css('strong')),text=(await(strong[0]||option).getAttribute('textContent')).trim();if(text===label||text.startsWith(label+'（')){await click(option);return}}throw Error('Missing option '+label)}
  const sizes=async(name,anchor)=>{
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),(typeof anchor==='string'?await waitForElement(anchor):anchor))
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const layout=await driver.executeScript(()=>{const d=document.querySelector('[data-testid="compression-draft-details"]');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,columns:d?[...d.children].map(e=>e.getBoundingClientRect().width):[],panels:[...document.querySelectorAll('[role="dialog"],.browser-page')].map(e=>e.scrollWidth-e.clientWidth)}})
      assert.ok(layout.overflow<=1&&layout.panels.every(value=>value<=1),JSON.stringify(layout));if(name==='inline-config'&&width===1100)assert.ok(Math.abs(layout.columns[0]-layout.columns[1])<=1)
      const file='zpaq1530-'+name+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots=state.screenshots.filter(value=>value!==file);state.screenshots.push(file);state.layouts=state.layouts.filter(value=>!(value.name===name&&value.width===layout.width&&value.height===layout.height&&value.binarySha256===binarySha256));state.layouts.push({name,binarySha256,...layout});save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  const fixtures=path.join(root,'tests/fixtures/archives/zpaq-snapshots-1530'),manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,file_filter:null,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],selected_entries:[]}
  const snap=n=>({version:n,date:manifest.cases[0].snapshots[n-1].date})
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  for(const original of manifest.cases){
    const kind='original-'+original.file;if(passed('native',kind))continue
    const source=path.join(fixtures,original.file);assert.equal(sha(source),original.sha256)
    const folder=owned(path.join(work,kind));mkdirSync(folder,{recursive:true});const data=readFileSync(source),files=[]
    for(let offset=0,n=1;offset<data.length;n++){const length=n===1?7:n===2?13:32749,file=owned(path.join(folder,original.file+'.'+String(n).padStart(3,'0')));writeFileSync(file,data.subarray(offset,offset+length));files.push(file);offset+=length}
    const password=original.encrypted?manifest.password:null,member=files[Math.floor(files.length/2)],args={filePath:member,password,zpaqSnapshot:snap(1)}
    const browse=await ipc('browse_archive',args);assert.equal(browse.zpaqHistory.snapshots.length,3);assert.deepEqual(browse.zpaqHistory.selected,snap(1))
    await ipc('test_archive_integrity',args);const diagnosis=await ipc('diagnose_archive',{...args,diagnosticId:randomUUID()});assert.equal(diagnosis.status,'healthy')
    const output=owned(path.join(work,kind+'-完整提取'));mkdirSync(output,{recursive:true});await ipc('extract_file',{taskId:randomUUID(),filePath:member,outputPath:output,password,options:{...options,zpaq_snapshot:snap(1)}})
    assert.deepEqual(tree(output),original.snapshots[0].expected);for(const directory of original.snapshots[0].directories)assert.ok(existsSync(path.join(output,directory)))
    assert.equal(hash(Buffer.concat(files.map(file=>readFileSync(file)))),original.sha256);done('native',kind,{files,output,fullCatalog:true,independentHashes:true,noJoinedArchive:readdirSync(folder).length===files.length})
  }
  const original=manifest.cases.find(row=>row.encrypted),parts=state.native.find(row=>row.kind==='original-'+original.file).files,member=parts[Math.floor(parts.length/2)]
  async function openBrowser(version){
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[member]]);await click('[data-testid="file-manager-open-archive"],.browser-header-actions .browser-primary')
    await fill('.browser-toolbar input[type="password"]',manifest.password);await click('.browser-toolbar button[title="重新读取"]');await waitForElement('[data-testid="zpaq-snapshot-select"]');await selectSnapshot(version)
  }
  async function selectSnapshot(version){
    const selector='.browser-page [data-testid="zpaq-snapshot-select"]';await driver.wait(async()=>await(await waitForElement(selector)).isEnabled(),30000)
    await driver.executeScript((e,v)=>{e.value=String(v);e.dispatchEvent(new Event('change',{bubbles:true}))},await waitForElement(selector),version)
    await driver.wait(async()=>{const e=await waitForElement(selector);return await e.isEnabled()&&(await e.getAttribute('value'))===String(version)},30000)
  }
  async function closeConversion(){await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))}
  if(process.env.ZPAQ_1530_POLISH_ONLY==='1'){
    assert.equal(state.ui.length,5,'Retain the prior completed functional chains')
    await openBrowser(3);await sizes('snapshot-browser-polish','[data-testid="zpaq-snapshot-picker"]');await click('[data-testid="zpaq-snapshot-convert"]')
    await choose('[data-testid="conversion-format"]','ZPAQ');await fill('[data-testid="conversion-split"]','0.0625');await(await waitForElement('[data-testid="conversion-split"]')).sendKeys(Key.TAB)
    assert.equal(await(await waitForElement('[data-testid="conversion-split"]')).getAttribute('value'),'0.0625');await click('[data-testid="conversion-load-snapshots"]');await waitForElement('[data-testid="archive-conversion"] [data-testid="zpaq-snapshot-select"]')
    await sizes('snapshot-conversion-polish','[data-testid="conversion-zpaq-history"]');assert.equal(await(await waitForElement('[data-testid="conversion-split"]')).getAttribute('value'),'0.0625')
    state.visualPolish={binarySha256,sourceCommit:state.sourceCommit,fractionalMiBTyped:true,noImplicitStart:true,completedAt:new Date().toISOString()};save();writeFileSync(path.join(artifactDirectory,'zpaq-snapshots-1530-results.json'),JSON.stringify(state,null,2));return
  }
  if(!passed('ui','old-previews-selected-extraction')){
    await openBrowser(1);await sizes('snapshot-browser','[data-testid="zpaq-snapshot-picker"]')
    const folder=await waitForElement('[data-entry-path="资料/"],[data-entry-path="资料"]');await folder.sendKeys(Key.ENTER)
    for(const[kind,name,selector]of[['text','说明.txt','[data-testid="archive-text-preview"]'],['image','图标.png','[data-testid="archive-image-preview"]']]){
      await click('[data-entry-path="资料/'+name+'"] .preview-trigger');await waitForElement(selector)
      if(kind==='text')assert.match(await(await waitForElement(selector)).getText(),/历史快照 1/)
      else {const image=await waitForElement(selector+' img');assert.equal(await driver.executeScript(e=>e.naturalWidth,image),32)}
      await sizes('old-'+kind+'-preview',selector);await click('.preview-close')
    }
    await click('[data-testid="archive-nav-refresh"]');assert.equal(await(await waitForElement('.browser-page [data-testid="zpaq-snapshot-select"]')).getAttribute('value'),'1')
    await fill('.browser-search input','资料/后来删除.txt');await click('[data-entry-path="资料/后来删除.txt"] .browser-checkbox')
    const output=owned(path.join(work,'旧快照所选-'+randomUUID()));mkdirSync(output);await bridge('queueDesktopDialogSelections',[output]);await click('.output-target');await click('footer .browser-primary')
    const record=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.outputPath===output&&['completed','failed'].includes(row.status))||false,30000);assert.equal(record.status,'completed',JSON.stringify(record));assert.deepEqual(record.executionConfig.zpaqSnapshot,snap(1));assert.deepEqual(tree(output),{'资料/后来删除.txt':original.snapshots[0].expected['资料/后来删除.txt']})
    done('ui','old-previews-selected-extraction',{record,independentHash:true,oldImageWidth:32})
  }
  if(!passed('ui','history-missing-volume-recovery')){
    await bridge('clearTasks');await click('[data-testid="nav-History"]');const previous=state.ui.find(row=>row.kind==='old-previews-selected-extraction').record
    const row=await driver.wait(async()=>{for(const r of await driver.findElements(By.css('[data-testid="history-record-row"]')))if((await r.getText()).includes(path.basename(previous.outputPath)))return r;return false},15000)
    await click(await row.findElement(By.css('[data-testid="history-retry"]')));await driver.wait(async()=>/快照 1/.test(await(await waitForElement('[data-testid="history-zpaq-snapshot"]')).getText()),10000)
    assert.equal(await(await waitForElement('[data-testid="history-draft-password"]')).getAttribute('value'),'');await fill('[data-testid="history-draft-password"]',manifest.password)
    const first=owned(path.join(work,'历史待执行-'+randomUUID()));mkdirSync(first);await fill('[data-testid="history-draft-output"]',first);const sub=await waitForElement('[data-testid="history-draft-subfolder"]');if(await sub.isSelected())await click(sub)
    await click('[data-testid="history-draft-create"]');await click('[data-testid="history-extraction-draft"] a')
    const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000);assert.equal(task.status,'pending');assert.deepEqual(readdirSync(first),[])
    const missing=parts[2],backup=owned(missing+'.owned-original');renameSync(missing,backup);let failure
    try{await click('[data-testid="start-decompression-queue"]');failure=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.id===task.id&&r.status==='failed')||false,30000)}finally{renameSync(backup,missing)}
    assert.ok(failure.errorMessage.includes(path.basename(missing)),JSON.stringify(failure));assert.deepEqual(failure.executionConfig.zpaqSnapshot,snap(1))
    await click('[data-testid="retry-batch-failures"]');const output=owned(path.join(work,'补齐后快照一-'+randomUUID()));mkdirSync(output);await fill('[data-testid="retry-output"]',output);const retrySub=await waitForElement('[data-testid="retry-subfolder"]');if(await retrySub.isSelected())await click(retrySub)
    await fill('[data-testid="retry-password"]',manifest.password);await sizes('retry-snapshot','[role="dialog"]');await click('[data-testid="create-retry-drafts"]');await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'),20000);await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(.,"返回队列确认并启动")]')))
    const retry=(await bridge('extractionRecoveryTasks')).find(r=>r.retryOfTaskId===task.id);assert.equal(retry.status,'pending');await click('[data-testid="start-decompression-queue"]')
    const record=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.id===retry.id&&['completed','failed'].includes(r.status))||false,30000);assert.equal(record.status,'completed',JSON.stringify(record));assert.deepEqual(record.executionConfig.zpaqSnapshot,snap(1));assert.deepEqual(tree(output),original.snapshots[0].expected);assert.deepEqual((await bridge('taskHistory')).find(r=>r.id===failure.id),failure)
    const taskRow=await waitForElement('[data-testid="task-row"][data-task-id="'+retry.id+'"]');if(await taskRow.getAttribute('aria-expanded')!=='true')await click(taskRow);await sizes('extraction-result','[data-testid="decompression-task-details"]')
    const columns=await driver.executeScript(()=>[...document.querySelector('[data-testid="decompression-task-details"] .task-detail-layout').children].map(e=>e.getBoundingClientRect().width));assert.ok(Math.abs(columns[0]-columns[1])<=1,JSON.stringify(columns))
    done('ui','history-missing-volume-recovery',{record,columns,newPendingTask:true,oldFailurePreserved:true,independentHashes:true})
  }
  for(const[version,target]of[[1,'zip'],[2,'zpaq']]){
    const kind='snapshot-'+version+'-to-'+target;if(passed('ui',kind))continue
    await openBrowser(version);await click('[data-testid="zpaq-snapshot-convert"]');const item=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).at(-1);await waitForElement('[data-testid="conversion-zpaq-history"]')
    await choose('[data-testid="conversion-format"]',target.toUpperCase());const password='新包快照密码1530';await fill('[data-testid="conversion-password"]',password);if(target==='zpaq'){await fill('[data-testid="conversion-split"]','0.0625');await(await waitForElement('[data-testid="conversion-split"]')).sendKeys(Key.TAB);assert.equal(await(await waitForElement('[data-testid="conversion-split"]')).getAttribute('value'),'0.0625')}
    const output=owned(path.join(work,'快照'+version+'转换-'+randomUUID()+'.'+target));await fill('[data-testid="conversion-output"]',output);await sizes('conversion-'+target+'-config',item);await click('[data-testid="conversion-start"]')
    const requested=output.replaceAll('\\','/').toLowerCase(),record=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>[requested,requested+'.001'].includes(r.outputPath?.replaceAll('\\','/').toLowerCase())&&['completed','failed'].includes(r.status))||false,60000);assert.equal(record.status,'completed',JSON.stringify(record));assert.deepEqual(record.executionConfig.sourceZpaqSnapshot,snap(version))
    if(target==='zip'){
      const request=owned(path.join(work,'zip-request.json')),result=owned(path.join(work,'zip-result.json'));writeFileSync(request,JSON.stringify({type:'zip',file:output,password,expectedFiles:original.snapshots[version-1].expected,emptyDirectory:'资料/空目录'}));execFileSync('python',[path.join(root,'scripts/verify-tar-lzma-1524.py'),request,result],{windowsHide:true,timeout:30000,maxBuffer:4*1024*1024})
    }else{
      const volumes=readdirSync(work).filter(name=>name.startsWith(path.basename(output)+'.')).sort().map(name=>path.join(work,name));assert.ok(volumes.length>1);const joined=owned(path.join(work,'oracle-only-joined.zpaq'));writeFileSync(joined,Buffer.concat(volumes.map(p=>readFileSync(p))))
      const tool=path.join(root,'test-results/zpaq-volumes-1526/upstream/zpaq64.exe');assert.equal(sha(tool),manifest.toolSha256);const decoded=owned(path.join(work,'独立回解快照二'));mkdirSync(decoded);execFileSync(tool,['x',joined,'-to',decoded,'-key',password],{windowsHide:true,timeout:30000,maxBuffer:4*1024*1024});assert.deepEqual(tree(decoded),original.snapshots[version-1].expected);assert.ok(existsSync(path.join(decoded,'资料/空目录')))
    }
    await sizes('conversion-'+target+'-result',item);done('ui',kind,{record,independentDecodedHashes:true});await closeConversion()
  }
  if(!passed('ui','bulk-and-draft-snapshots')){
    await bridge('clearTasks')
    for(const version of[1,3]){await openBrowser(version);await click('[data-testid="zpaq-snapshot-convert"]');await closeConversion()}
    await click('[data-testid="zpaq-snapshot-convert"]');await click('[data-testid="conversion-batch-config"]');await choose('[data-testid="conversion-batch-format"]','ZIP');await sizes('batch-snapshots','[data-testid="conversion-batch-fields"]');await click('[data-testid="conversion-batch-apply"]');await closeConversion()
    const saved=await driver.wait(()=>driver.executeScript(()=>{for(const key of Object.keys(localStorage)){try{const v=JSON.parse(localStorage.getItem(key));if(v?.conversion?.items?.length===2)return v}catch{}}return null}),15000)
    assert.deepEqual(saved.conversion.items.map(i=>i.settings.sourceZpaqSnapshot.version).sort(),[1,3]);assert.ok(saved.conversion.items.every(i=>!i.settings.sourcePassword&&!i.settings.password))
    await driver.navigate().refresh();await waitForElement('[data-testid="pending-draft-recovery"]');await click(await driver.findElement(By.xpath('//*[@data-testid="pending-draft-recovery"]//button[contains(.,"恢复任务")]')));await waitForElement('[data-testid="archive-conversion"]')
    const rows=await driver.findElements(By.css('[data-testid="conversion-item"]'));assert.equal(rows.length,2)
    const versions=[];for(const item of rows){const row=await item.findElement(By.css('[data-testid="conversion-row"]'));if(await row.getAttribute('aria-expanded')!=='true')await click(row);const text=await item.getText();versions.push(Number(text.match(/读取快照 (\d+)/)?.[1]));assert.match(text,/待处理/);assert.equal(await(await item.findElement(By.css('[data-testid="conversion-source-password"]'))).getAttribute('value'),'');await click(row)}
    assert.deepEqual(versions.sort(),[1,3]);await sizes('restored-snapshots','[data-testid="archive-conversion"]');done('ui','bulk-and-draft-snapshots',{versions,noImplicitStart:true,passwordsNotPersisted:true})
  }
  assert.equal(hash(Buffer.concat(parts.map(p=>readFileSync(p)))),original.sha256)
  state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'zpaq-snapshots-1530-results.json'),JSON.stringify(state,null,2))
}

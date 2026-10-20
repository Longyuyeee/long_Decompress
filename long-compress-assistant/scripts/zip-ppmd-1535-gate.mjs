import assert from 'node:assert/strict'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync,readdirSync,renameSync,copyFileSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runZipPpmd1535Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const base=path.join(root,'test-results/zip-ppmd-1535'),progress=path.join(base,'desktop-progress.json'),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p)),binarySha256=sha(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.35',binarySha256,sourceCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim(),work:path.join(base,'ui-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  if(state.binarySha256!==binarySha256){
    assert.equal(process.env.ZIP_1535_RESUME_FROM,state.binarySha256,'Explicitly acknowledge prior passed evidence identity')
    state.previousIdentities=[...(state.previousIdentities||[]),{binarySha256:state.binarySha256,sourceCommit:state.sourceCommit}]
    state.binarySha256=binarySha256;state.sourceCommit=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim()
  }
  const work=state.work;mkdirSync(work,{recursive:true})
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const done=(group,kind,data={})=>{state[group].push({kind,binarySha256,...data});save();console.log('zip-ppmd-1535: '+group+' '+kind)}
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
      const file='zip1535-'+name+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots=state.screenshots.filter(value=>value!==file);state.screenshots.push(file);state.layouts=state.layouts.filter(value=>!(value.name===name&&value.width===layout.width&&value.height===layout.height&&value.binarySha256===binarySha256));state.layouts.push({name,binarySha256,...layout});save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,file_filter:null,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],selected_entries:[]}
  const password='SourceKey1535!',newPassword='新的 ZIP 密码1535'
  if(!state.source){
    const source=owned(path.join(work,'资料'));mkdirSync(path.join(source,'项目/空目录'),{recursive:true});writeFileSync(path.join(source,'说明.txt'),'PPMd 界面创建，完整保留目录与内容。\n');writeFileSync(path.join(source,'空文件.txt'),'');writeFileSync(path.join(source,'图标.png'),readFileSync(path.join(root,'src-tauri/icons/32x32.png')));writeFileSync(path.join(source,'pagefile.sys'),'普通用户选择的文件也要保留')
    const chunks=[];for(let c=0;c<128;c++){for(let n=0;n<512;n++)chunks.push(createHash('sha256').update(c+':'+n).digest());chunks.push(Buffer.alloc(16384,65+c%26))}writeFileSync(path.join(source,'项目/数据.bin'),Buffer.concat(chunks))
    state.source=source;state.expected=Object.fromEntries(Object.entries(tree(source)).map(([name,digest])=>['资料/'+name,digest]));state.inputZip=owned(path.join(work,'独立密码原包.zip'));execFileSync(bundledSevenZip,['a','-tzip','-mem=AES256','-p'+password,state.inputZip,'资料'],{cwd:work,windowsHide:true,maxBuffer:4*1024*1024});save()
  }

  const independent=(output,label,secret='')=>{
    let file=output.primaryPath
    if(output.split&&output.files.every(f=>/\.\d{3,}$/.test(f.path))){file=owned(path.join(work,'oracle-'+label+'-'+randomUUID()+'.zip'));writeFileSync(file,Buffer.concat(output.files.map(f=>readFileSync(f.path))))}
    if(output.split&&output.files.every(f=>/\.\d{3,}$/.test(f.path))&&secret){
      const request=owned(path.join(work,'request-'+randomUUID()+'.json')),result=owned(path.join(work,'result-'+randomUUID()+'.json'));writeFileSync(request,JSON.stringify({file,password:secret,expectedFiles:state.expected,emptyDirectory:'资料/项目/空目录'}));execFileSync('python',[path.join(root,'scripts/verify-zip-ppmd-1535.py'),request,result],{windowsHide:true,maxBuffer:4*1024*1024});return JSON.parse(readFileSync(result,'utf8'))
    }
    if(output.files.some(f=>/\.z\d+$/i.test(f.path)))file=output.files.find(f=>/\.zip$/i.test(f.path))?.path||file
    const listing=execFileSync(bundledSevenZip,['l','-slt','-sccUTF-8','-p'+(secret||'-'),file],{windowsHide:true,encoding:'utf8',maxBuffer:4*1024*1024});assert.match(listing,/Method = .*PPMD/i)
    const destination=owned(path.join(work,'oracle-'+label+'-'+randomUUID()));execFileSync(bundledSevenZip,['x','-y','-p'+(secret||'-'),file,'-o'+destination],{windowsHide:true,maxBuffer:4*1024*1024});assert.deepEqual(tree(destination),state.expected);assert.ok(existsSync(path.join(destination,'资料/项目/空目录')));return{tool:'independent 7-Zip PPMd decoder',ppmdMethod:true,allIndependentHashes:true,emptyDirectory:true,outputFiles:output.files.length}
  }

  const completed=async predicate=>driver.wait(async()=>(await bridge('taskHistory')).find(r=>predicate(r)&&['completed','failed'].includes(r.status))||false,60000)
  const closeConversion=async()=>click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  if(!passed('native','pause-resume-public')){
    const began=Date.now(),taskId=randomUUID(),output=owned(path.join(work,'暂停恢复-'+randomUUID()+'.zip'))
    await driver.executeScript((taskId,files,outputPath)=>{
      const ids=crypto.getRandomValues(new Uint32Array(2));window.__zip1535PauseResult=null
      const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id))
      window['_'+ids[0]]=value=>{clean();window.__zip1535PauseResult={ok:true,value}}
      window['_'+ids[1]]=error=>{clean();window.__zip1535PauseResult={ok:false,error:String(error)}}
      window.__TAURI_IPC__({cmd:'compress_files',taskId,files,outputPath,options:{format:'zip',zip_method:'ppmd',level:9,preserve_paths:true,verify_after:true,delete_after:false},callback:ids[0],error:ids[1]})
    },taskId,[state.source],output)
    await ipc('pause_archive_task',{taskId})
    try{
      await new Promise(resolve=>setTimeout(resolve,250))
      assert.equal(await driver.executeScript(()=>window.__zip1535PauseResult),null,'Paused PPMd task must remain pending')
    }finally{await ipc('resume_archive_task',{taskId})}
    const result=await driver.wait(()=>driver.executeScript(()=>window.__zip1535PauseResult),60000);assert.equal(result.ok,true,result.error)
    const resourceScript="Get-CimInstance Win32_Process -Filter \"Name='long-compress-assistant.exe'\" | Where-Object { $_.ExecutablePath -eq '"+application.replaceAll("'","''")+"' } | ForEach-Object { $p=Get-Process -Id $_.ProcessId; [ordered]@{pid=$p.Id;peakWorkingSetBytes=$p.PeakWorkingSet64;peakVirtualMemoryBytes=$p.PeakVirtualMemorySize64} } | ConvertTo-Json -Compress"
    const resources=JSON.parse(execFileSync('pwsh',['-NoProfile','-Command',resourceScript],{windowsHide:true,encoding:'utf8'}));assert.ok(resources.peakWorkingSetBytes>0)
    done('native','pause-resume-public',{level:9,dictionaryMiB:256,inputBytes:4196108,elapsedMilliseconds:Date.now()-began,pauseHeldMilliseconds:250,resources,published:result.value,evidence:independent(result.value,'pause-resume')})
  }
  for(const [kind,encryption,split,secret]of[['aes-numeric-public','aes256','numeric','原生 PPMd 密码1535'],['zipcrypto-standard-public','zipcrypto','standard','NativeKey1535!']]){
    if(passed('native',kind))continue
    const output=owned(path.join(work,'公共-'+kind+'-'+randomUUID()+'.zip')),published=await ipc('compress_files',{taskId:randomUUID(),files:[state.source],outputPath:output,options:{format:'zip',zip_method:'ppmd',level:6,password:secret,zip_encryption:encryption,zip_password_encoding:'utf8',zip_split_mode:split,split_size:65536,preserve_paths:true,verify_after:true,delete_after:false}})
    const evidence=independent(published,kind,secret),member=published.files[Math.floor(published.files.length/2)].path;assert.ok(published.files.length>2)
    await ipc('browse_archive',{filePath:member,password:secret});await ipc('test_archive_integrity',{filePath:member,password:secret});const wrong=await invoke('test_archive_integrity',{filePath:member,password:'wrong'});assert.equal(wrong.ok,false)
    const directory=owned(path.join(work,'公共提取-'+kind));mkdirSync(directory);await ipc('extract_file',{taskId:randomUUID(),filePath:member,outputPath:directory,password:secret,options});assert.deepEqual(tree(directory),state.expected);assert.ok(existsSync(path.join(directory,'资料/项目/空目录')));done('native',kind,{published,evidence,directory,wrongPasswordRejected:true})
  }
  if(!passed('ui','create-ppmd-standard-and-analysis')){
    await bridge('clearCompressionWorkspace');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[state.source]]);await click('[data-testid="compression-center"] [data-testid="dropzone-folder"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-archive-name"]')
    await choose('[data-testid="compression-format-select"]','ZIP');await choose('[data-testid="compression-zip-method"]','PPMd · 文本归档');assert.match(await(await waitForElement('[data-testid="zip-ppmd-memory"]')).getText(),/32 MiB/);await click('[data-testid="compression-analysis"] .analysis-button');await driver.wait(async()=>(await driver.findElements(By.css('.analysis-metrics'))).length>0,30000);const analysis=await(await waitForElement('.analysis-metrics')).getText();assert.match(analysis,/预计体积/)
    const split=await waitForElement('[data-testid="compression-split-toggle"]');if(!await split.isSelected())await click(split);await fill('[data-testid="compression-split-size"]','1');await choose('[data-testid="compression-zip-split-mode"]','标准 ZIP · .z01/.zip');await fill('[data-testid="compression-output-path"]',work);const name='界面PPMd-'+randomUUID();await fill('[data-testid="compression-filename-input"]',name)
    await sizes('inline-config','[data-testid="compression-zip-method"]');await click('[data-testid="start-compression"]');const record=await completed(r=>r.outputPath?.includes(name));assert.equal(record.status,'completed',JSON.stringify(record));assert.equal(record.executionConfig.settings.zipMethod,'ppmd');assert.ok(record.processedBytes>=4194304&&record.totalBytes>=4194304);const evidence=independent(record.archiveOutput,'ui-standard');await sizes('creation-result','[data-testid="compression-draft-details"]');done('ui','create-ppmd-standard-and-analysis',{record,evidence,analysis})
  }
  const published=state.ui.find(r=>r.kind==='create-ppmd-standard-and-analysis').record.archiveOutput,parts=published.files.map(f=>f.path)
  if(!passed('ui','new-archive-previews-and-selection')){
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[parts[1]]]);await click('[data-testid="file-manager-open-archive"],.browser-header-actions .browser-primary');await(await waitForElement('[data-entry-path="资料"],[data-entry-path="资料/"]')).sendKeys(Key.ENTER)
    for(const[kind,name,selector]of[['text','说明.txt','[data-testid="archive-text-preview"]'],['image','图标.png','[data-testid="archive-image-preview"]']]){await click('[data-entry-path="资料/'+name+'"] .preview-trigger');await waitForElement(selector);if(kind==='text')assert.match(await(await waitForElement(selector)).getText(),/PPMd/);await sizes(kind+'-preview',selector);await click('.preview-close')}
    await fill('.browser-search input','.');const all=await waitForElement('.browser-table-head .browser-checkbox');if((await all.getAttribute('class')).includes('checked'))await click(all);await fill('.browser-search input','资料/说明.txt');await click('[data-entry-path="资料/说明.txt"] .browser-checkbox')
    const output=owned(path.join(work,'界面所选-'+randomUUID()));mkdirSync(output);await bridge('queueDesktopDialogSelections',[output]);await click('.output-target');await click('footer .browser-primary');const record=await completed(r=>r.outputPath===output);assert.equal(record.status,'completed',JSON.stringify(record));assert.deepEqual(tree(output),{'资料/说明.txt':state.expected['资料/说明.txt']});done('ui','new-archive-previews-and-selection',{record,independentHash:true})
  }
  if(!passed('ui','new-volume-missing-recovery')){
    await bridge('clearTasks');await click('[data-testid="nav-Decompress"]');await bridge('queueDesktopDialogSelections',[[parts[1]]]);await click('[data-testid="dropzone-file"]');const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000),missing=parts[1],backup=owned(missing+'.owned-original');renameSync(missing,backup);let failed
    try{await click('[data-testid="start-decompression-queue"]');failed=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.id===task.id&&r.status==='failed')||false,30000)}finally{renameSync(backup,missing)}
    assert.match(failed.errorMessage,/分卷|incomplete|missing/);await click('[data-testid="retry-batch-failures"]');const output=owned(path.join(work,'补齐PPMd-'+randomUUID()));mkdirSync(output);await fill('[data-testid="retry-output"]',output);const sub=await waitForElement('[data-testid="retry-subfolder"]');if(await sub.isSelected())await click(sub);await sizes('missing-recovery','[role="dialog"]');await click('[data-testid="create-retry-drafts"]');await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'),15000);await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(.,"返回队列确认并启动")]')))
    const retry=(await bridge('extractionRecoveryTasks')).find(r=>r.retryOfTaskId===task.id);assert.equal(retry.status,'pending');await click('[data-testid="start-decompression-queue"]');const record=await completed(r=>r.id===retry.id);assert.equal(record.status,'completed',JSON.stringify(record));assert.deepEqual(tree(output),state.expected);assert.deepEqual((await bridge('taskHistory')).find(r=>r.id===failed.id),failed);done('ui','new-volume-missing-recovery',{record,oldFailurePreserved:true})
  }

  for(const target of['zip','7z']){
    const kind='conversion-'+target;if(passed('ui',kind))continue
    const previous=target==='7z'?state.ui.find(r=>r.kind==='conversion-zip').record.archiveOutput:null
    const source=previous?previous.files.at(-1).path:state.inputZip
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]');await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="conversion-choose-source"]');const item=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).at(-1),row=await item.findElement(By.css('[data-testid="conversion-row"]'));if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    await choose('[data-testid="conversion-format"]',target.toUpperCase());await fill('[data-testid="conversion-source-password"]',target==='zip'?password:newPassword);await fill('[data-testid="conversion-password"]',target==='zip'?newPassword:'SevenKey1535!')
    if(target==='zip'){await choose('[data-testid="conversion-zip-method"]','PPMd · 文本归档');assert.match(await(await waitForElement('[data-testid="zip-ppmd-memory"]')).getText(),/32 MiB/);await fill('[data-testid="conversion-split"]','1');await(await waitForElement('[data-testid="conversion-split"]')).sendKeys(Key.TAB)}
    const output=owned(path.join(work,'转换-'+randomUUID()+'.'+target));await fill('[data-testid="conversion-output"]',output);await sizes('conversion-'+target+'-config',target==='zip'?'[data-testid="conversion-zip-method"]':item);await click('[data-testid="conversion-start"]');const wanted=output.replaceAll('\\','/').toLowerCase(),record=await completed(r=>[wanted,wanted+'.001'].includes(r.outputPath?.replaceAll('\\','/').toLowerCase()));assert.equal(record.status,'completed',JSON.stringify(record))
    let evidence;if(target==='zip'){assert.equal(record.executionConfig.zipMethod,'ppmd');evidence=independent(record.archiveOutput,'converted-aes',newPassword)}else{const destination=owned(path.join(work,'独立7z-'+randomUUID()));execFileSync(bundledSevenZip,['x','-y','-pSevenKey1535!',record.archiveOutput.primaryPath,'-o'+destination],{windowsHide:true,maxBuffer:4*1024*1024});assert.deepEqual(tree(destination),state.expected);assert.ok(existsSync(path.join(destination,'资料/项目/空目录')));evidence={independentHashes:true}}
    await sizes('conversion-'+target+'-result',item);done('ui',kind,{record,evidence});await closeConversion()
  }
  if(!passed('ui','bulk-method-and-draft-recovery')){
    await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[path.join(state.source,'项目/数据.bin'),path.join(state.source,'说明.txt')]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]');for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]')))await click(box)
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','ZIP');await choose('[data-testid="batch-zip-method"]','PPMd · 文本归档');await sizes('batch-method','[role="dialog"]');await click('[data-testid="batch-config-apply"]');await driver.wait(async()=>(await driver.findElements(By.css('[role="dialog"]'))).length===0,10000)
    const row=(await driver.findElements(By.css('[data-testid="compression-draft-row"]')))[0];if(!(await row.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')));await choose('[data-testid="compression-zip-method"]','Zstandard');await closeRow(row)
    const saved=await driver.wait(()=>driver.executeScript(()=>{try{const s=JSON.parse(localStorage.getItem('long-pending-archive-drafts-v1'));return s?.compression.files.length===2?s:null}catch{return null}}),15000);assert.deepEqual(saved.compression.files.map(f=>f.settings.zipMethod).sort(),['ppmd','zstd']);await driver.navigate().refresh();await waitForElement('[data-testid="pending-draft-recovery"]');await click(await driver.findElement(By.xpath('//*[@data-testid="pending-draft-recovery"]//button[contains(.,"恢复任务")]')))
    const rows=await driver.findElements(By.css('[data-testid="compression-draft-row"]'));assert.equal(rows.length,2);const methods=[];for(const item of rows){const open=(await item.findElements(By.css('[data-testid="compression-draft-details"]'))).length;if(!open)await click(await item.findElement(By.css('[data-testid="compression-archive-name"]')));methods.push(await driver.wait(async()=>{const text=await(await waitForElement('[data-testid="compression-zip-method"]')).getText();return /PPMd|Zstandard/.test(text)?text:false},10000));await closeRow(item)}assert.ok(methods.some(m=>m.includes('PPMd'))&&methods.some(m=>m.includes('Zstandard')));await sizes('restored-drafts','[data-testid="compression-center"]');done('ui','bulk-method-and-draft-recovery',{methods,noImplicitStart:true})
  }
  state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'zip-ppmd-1535-results.json'),JSON.stringify(state,null,2))
}

import assert from 'node:assert/strict'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync,readdirSync,renameSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runZpaqVolumes1526Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const base=path.join(root,'test-results/zpaq-volumes-1526'),progress=path.join(base,'desktop-progress.json'),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p)),binarySha256=sha(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.26',binarySha256,sourceCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim(),work:path.join(base,'ui-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  if(state.binarySha256!==binarySha256){
    assert.equal(process.env.ZPAQ_1526_RESUME_FROM,state.binarySha256,'Explicitly acknowledge prior passed evidence identity')
    state.previousIdentities=[...(state.previousIdentities||[]),{binarySha256:state.binarySha256,sourceCommit:state.sourceCommit}]
    state.binarySha256=binarySha256;state.sourceCommit=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim()
  }
  const work=state.work;mkdirSync(work,{recursive:true})
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const done=(group,kind,data={})=>{state[group].push({kind,binarySha256,...data});save();console.log('zpaq-1526: '+group+' '+kind)}
  const passed=(group,kind)=>state[group].some(row=>row.kind===kind)
  const tree=(directory,prefix='')=>Object.fromEntries(readdirSync(directory,{withFileTypes:true}).flatMap(entry=>{const name=prefix+entry.name,file=path.join(directory,entry.name);return entry.isDirectory()?Object.entries(tree(file,name+'/')):[[name,sha(file)]]}))
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const result=await invoke(cmd,args);assert.equal(result.ok,true,result.error);return result.value}
  const click=async target=>{
    for(const toast of await driver.findElements(By.css('.toast-item button'))){try{if(await toast.isDisplayed())await toast.click()}catch{}}
    const element=typeof target==='string'?await waitForElement(target):target
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&r.height>0&&(hit===e||e.contains(hit))},element),15000);await element.click()
  }
  const fill=async(selector,value)=>{const e=await waitForElement(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const choose=async(selector,label)=>{await click(selector);for(const option of await driver.findElements(By.css('[role="option"]'))){const strong=await option.findElements(By.css('strong')),text=(await(strong[0]||option).getAttribute('textContent')).trim();if(text===label||text.startsWith(label+'（')){await click(option);return}}throw Error('Missing option '+label)}
  const sizes=async(name,anchor)=>{
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement(anchor))
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const layout=await driver.executeScript(()=>{const d=document.querySelector('[data-testid="compression-draft-details"]');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,columns:d?[...d.children].map(e=>e.getBoundingClientRect().width):[],panels:[...document.querySelectorAll('[role="dialog"],.browser-page')].map(e=>e.scrollWidth-e.clientWidth)}})
      assert.ok(layout.overflow<=1&&layout.panels.every(value=>value<=1),JSON.stringify(layout));if(name==='inline-config'&&width===1100)assert.ok(Math.abs(layout.columns[0]-layout.columns[1])<=1)
      const file='zpaq1526-'+name+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots.push(file);state.layouts.push({name,binarySha256,...layout});save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  const original=path.join(root,'src-tauri/tests/fixtures/zpaq'),manifest=JSON.parse(readFileSync(path.join(original,'manifest.json'),'utf8')),originalPassword=manifest.password,secret='数字分卷 密码1526'
  const originalExpected=Object.fromEntries(Object.entries(manifest.payload).map(([name,sha256])=>['项目 文档/'+name,sha256]))
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,file_filter:null,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],selected_entries:[]}
  const split=(source,label)=>{const folder=owned(path.join(work,label));mkdirSync(folder,{recursive:true});const bytes=readFileSync(source),files=[];for(let offset=0;offset<bytes.length;){const size=files.length===0?7:files.length===1?13:4096,end=Math.min(offset+size,bytes.length),file=owned(path.join(folder,'资料.zpaq.'+String(files.length+1).padStart(3,'0')));writeFileSync(file,bytes.subarray(offset,end));files.push(file);offset=end}return files}
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  for(const name of ['journal.zpaq','journal-key.zpaq']){
    const kind='original-'+name;if(passed('native',kind))continue
    assert.equal(sha(path.join(original,name)),manifest.hashes[name]);const files=split(path.join(original,name),kind),output=owned(path.join(work,kind+'-提取'));mkdirSync(output)
    await ipc('extract_file',{taskId:randomUUID(),filePath:files[1],outputPath:output,password:name.includes('-key')?originalPassword:null,options})
    assert.deepEqual(tree(output),originalExpected);done('native',kind,{files,output,allIndependentHashes:true})
  }
  if(!state.source){
    const source=owned(path.join(work,'资料'));mkdirSync(source)
    writeFileSync(path.join(source,'说明.txt'),'ZPAQ 数字分卷可从任意成员打开。\n');writeFileSync(path.join(source,'数据.bin'),randomBytes(2*1024*1024+257));writeFileSync(path.join(source,'空文件.txt'),'');mkdirSync(path.join(source,'空目录'));writeFileSync(path.join(source,'图标.png'),readFileSync(path.join(root,'src-tauri/icons/32x32.png')))
    state.source=source;state.expected=Object.fromEntries(Object.entries(tree(source)).map(([name,digest])=>['资料/'+name,digest]));save()
  }
  const independent=async(published,password,label)=>{
    const joined=owned(path.join(work,label+'-完整.zpaq'));writeFileSync(joined,Buffer.concat(published.files.map(file=>readFileSync(file.path))))
    const output=owned(path.join(work,label+'-独立回解'));mkdirSync(output)
    const tool=path.join(base,'upstream/zpaq64.exe');assert.equal(sha(tool),'7a94b4f1d6323a758c7b0b6344036f166bff0fd44f1c3c86f05b3688023496cb')
    execFileSync(tool,['x',joined,...(password?['-key',password]:[]),'-to',output,'-threads','2','-noattributes'],{windowsHide:true,timeout:120000,maxBuffer:4*1024*1024})
    assert.deepEqual(tree(output),state.expected);assert.ok(existsSync(path.join(output,'资料/空目录')));return{joined,output,hashes:true,emptyDirectory:true,toolSha256:sha(tool)}
  }
  for(const encrypted of [false,true]){
    const kind=encrypted?'created-key':'created-plain';if(passed('native',kind))continue
    const output=owned(path.join(work,kind+'.zpaq')),published=await ipc('compress_files',{taskId:randomUUID(),files:[state.source],outputPath:output,options:{format:'zpaq',level:6,password:encrypted?secret:null,split_size:65537,preserve_paths:true,verify_after:true,delete_after:false}})
    assert.ok(published.split&&published.files.length>2);assert.ok(published.files.every(file=>file.bytes<=65537))
    const recovered=owned(path.join(work,kind+'-中卷提取'));mkdirSync(recovered)
    await ipc('extract_file',{taskId:randomUUID(),filePath:published.files[Math.floor(published.files.length/2)].path,outputPath:recovered,password:encrypted?secret:null,options})
    assert.deepEqual(tree(recovered),state.expected);assert.ok(existsSync(path.join(recovered,'资料/空目录')))
    done('native',kind,{published,recovered,independent:await independent(published,encrypted?secret:null,kind)})
  }
  if(!passed('ui','creation-and-result-checksums')){
    await bridge('clearCompressionWorkspace');await click('[data-testid="nav-Compress"]')
    await bridge('queueDesktopDialogSelections',[[path.join(state.source,'数据.bin')]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-draft-row"]')
    await choose('[data-testid="compression-format-select"]','ZPAQ');await fill('[data-testid="compression-password-input"]',secret)
    const split=await waitForElement('[data-testid="compression-split-toggle"]');assert.equal(await split.isEnabled(),true);if(!await split.isSelected())await click(split)
    await fill('[data-testid="compression-split-size"]','1');await fill('[data-testid="compression-output-path"]',work);await fill('[data-testid="compression-filename-input"]','界面分享')
    await sizes('inline-config','[data-testid="zpaq-volume-hint"]');await click('[data-testid="start-compression"]')
    const record=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.outputPath?.includes('界面分享')&&['completed','failed'].includes(row.status))||false,60000);assert.equal(record.status,'completed',JSON.stringify(record))
    const files=record.archiveOutput.files;assert.equal(files.length,3)
    await click('[data-testid="compression-draft-details"] [data-testid="result-generate-checksums"]');assert.match(await(await waitForElement('[data-testid="checksum-source-summary"]')).getText(),/整组 3 个文件/)
    await click('[data-testid="checksum-calculate-start"]');await driver.wait(async()=>await driver.executeScript(()=>document.querySelectorAll('[data-testid="checksum-calculation-entry"][data-status="success"]').length===3),15000)
    assert.deepEqual(await driver.executeScript(()=>[...document.querySelectorAll('.checksum-main>code')].map(e=>e.textContent)),files.map(file=>sha(file.path)))
    await sizes('result-checksums','.checksum-results');done('ui','creation-and-result-checksums',{record,allVolumes:true,independentDigests:true})
  }
  if(!passed('ui','compression-batch')){
    await bridge('clearCompressionWorkspace');await click('[data-testid="nav-Compress"]')
    await bridge('queueDesktopDialogSelections',[[path.join(state.source,'数据.bin'),path.join(state.source,'说明.txt')]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    for(let index=0;index<2;index++){
      const row=(await driver.findElements(By.css('[data-testid="compression-draft-row"]')))[index];if(!(await row.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
      await choose('[data-testid="compression-format-select"]','ZPAQ');await fill('[data-testid="compression-password-input"]','各自密码-'+index)
      const split=await waitForElement('[data-testid="compression-split-toggle"]');if(!await split.isSelected())await click(split);await fill('[data-testid="compression-split-size"]',String(index+1))
      if(index===0)await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
    }
    for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]')))if(!await box.isSelected())await click(box)
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','ZPAQ');await sizes('compression-batch','[role="dialog"]');const before=(await bridge('taskHistory')).length
    await click('[data-testid="batch-config-apply"]');await driver.wait(async()=>(await driver.findElements(By.css('[role="dialog"]'))).length===0,10000)
    assert.equal((await bridge('taskHistory')).length,before)
    for(let index=0;index<2;index++){
      const row=(await driver.findElements(By.css('[data-testid="compression-draft-row"]')))[index];if(!(await driver.findElements(By.css('[data-testid="compression-password-input"]'))).length||await(await waitForElement('[data-testid="compression-password-input"]')).getAttribute('value')!=='各自密码-'+index)await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
      assert.equal(await(await waitForElement('[data-testid="compression-password-input"]')).getAttribute('value'),'各自密码-'+index);assert.equal(await(await waitForElement('[data-testid="compression-split-size"]')).getAttribute('value'),String(index+1));await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
    }
    done('ui','compression-batch',{originalModal:true,perRowPasswordAndSplitPreserved:true,noImplicitStart:true})
  }
  const created=state.native.find(row=>row.kind==='created-key').published
  if(!passed('ui','bounded-previews')){
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[created.files[1].path]]);await click('[data-testid="file-manager-open-archive"]')
    await fill('.browser-toolbar input[type="password"]',secret);await(await waitForElement('.browser-toolbar input[type="password"]')).sendKeys(Key.ENTER)
    const folder=await waitForElement('[data-entry-path="资料"]');await driver.actions().doubleClick(folder).perform()
    await click('[data-entry-path="资料/说明.txt"] .preview-trigger');assert.match(await(await waitForElement('[data-testid="archive-text-preview"]')).getText(),/数字分卷可从任意成员打开/);await sizes('text-preview','[data-testid="archive-text-preview"]');await click('.preview-close')
    await click('[data-entry-path="资料/图标.png"] .preview-trigger');await waitForElement('[data-testid="archive-image-preview"]');await sizes('image-preview','[data-testid="archive-image-preview"]');await click('.preview-close')
    done('ui','bounded-previews',{middleMember:true,nativePassword:true,text:true,image:true})
  }
  if(!passed('ui','missing-volume-recovery')){
    const originalCase=state.native.find(row=>row.kind==='original-journal-key.zpaq'),files=originalCase.files
    await bridge('clearTasks');await click('[data-testid="nav-Decompress"]');await bridge('queueDesktopDialogSelections',[[files[1]]]);await click('[data-testid="dropzone-file"]')
    const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000),backup=owned(files[1]+'.owned-original');renameSync(files[1],backup)
    let failed
    try{await click('[data-testid="start-decompression-queue"]');failed=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.id===task.id&&row.status==='failed')||false,30000)}finally{renameSync(backup,files[1])}
    assert.match(failed.errorMessage,/incomplete|缺少|分卷|source-missing/)
    await click('[data-testid="retry-batch-failures"]');const output=owned(path.join(work,'补齐后新任务'));mkdirSync(output)
    await fill('[role="dialog"] [data-testid="retry-output"]',output);await fill('[role="dialog"] [data-testid="retry-password"]',originalPassword)
    const sub=await waitForElement('[data-testid="retry-subfolder"]');if(await sub.isSelected())await click(sub);await sizes('missing-recovery','[role="dialog"]')
    await click('[data-testid="create-retry-drafts"]');await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'),15000)
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(.,"返回队列确认并启动")]')))
    const retry=(await bridge('extractionRecoveryTasks')).find(row=>row.retryOfTaskId===task.id);assert.equal(retry.status,'pending');assert.deepEqual(readdirSync(output),[])
    await click('[data-testid="start-decompression-queue"]');const completed=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.id===retry.id&&['completed','failed'].includes(row.status))||false,30000);assert.equal(completed.status,'completed',JSON.stringify(completed));assert.deepEqual(tree(output),originalExpected)
    assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed);done('ui','missing-volume-recovery',{originalFailurePreserved:true,completeIndependentHashes:true,newTask:true})
  }
  for(const target of ['zip','zpaq']){
    const kind='conversion-'+target;if(passed('ui',kind))continue
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
    const source=target==='zip'?created.files[1].path:state.ui.find(row=>row.kind==='conversion-zip').output,inputPassword=target==='zip'?secret:'新的目标密码1526',outputPassword=target==='zip'?'新的目标密码1526':'再换密码1526'
    const before=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length
    await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="conversion-choose-source"]');await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length>before,15000)
    const item=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).at(-1),row=await item.findElement(By.css('[data-testid="conversion-row"]'));if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    await fill('[data-testid="conversion-source-password"]',inputPassword);await choose('[data-testid="conversion-format"]',target.toUpperCase());await fill('[data-testid="conversion-password"]',outputPassword)
    const output=owned(path.join(work,'互转-'+target+'.'+target));await fill('[data-testid="conversion-output"]',output);if(target==='zpaq')await fill('[data-testid="conversion-split"]','1')
    await click('[data-testid="conversion-start"]');const requested=output.replaceAll('\\','/').toLowerCase(),record=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>[requested,requested+'.001'].includes(row.outputPath?.replaceAll('\\','/').toLowerCase())&&['completed','failed'].includes(row.status))||false,60000);assert.equal(record.status,'completed',JSON.stringify(record))
    if(target==='zip'){
      const recovered=owned(path.join(work,'密码ZIP独立回解'));mkdirSync(recovered);execFileSync(bundledSevenZip,['x','-y','-p'+outputPassword,'-o'+recovered,output],{windowsHide:true,timeout:30000,maxBuffer:4*1024*1024});assert.deepEqual(tree(recovered),state.expected)
    }else assert.ok((await independent(record.archiveOutput,outputPassword,'转换结果')).hashes)
    await sizes('conversion-'+target,item);done('ui',kind,{output,record,independentDecodedHashes:true});await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
  state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'zpaq-volumes-1526-results.json'),JSON.stringify(state,null,2))
}

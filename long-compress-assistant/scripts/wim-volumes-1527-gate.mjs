import assert from 'node:assert/strict'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync,readdirSync,renameSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runWimVolumes1527Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const base=path.join(root,'test-results/wim-volumes-1527'),progress=path.join(base,'desktop-progress.json'),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p)),binarySha256=sha(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.27',binarySha256,sourceCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim(),work:path.join(base,'ui-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  if(state.binarySha256!==binarySha256){
    assert.equal(process.env.WIM_1527_RESUME_FROM,state.binarySha256,'Explicitly acknowledge prior passed evidence identity')
    state.previousIdentities=[...(state.previousIdentities||[]),{binarySha256:state.binarySha256,sourceCommit:state.sourceCommit}]
    state.binarySha256=binarySha256;state.sourceCommit=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim()
  }
  const work=state.work;mkdirSync(work,{recursive:true})
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const done=(group,kind,data={})=>{state[group].push({kind,binarySha256,...data});save();console.log('wim-1527: '+group+' '+kind)}
  const passed=(group,kind)=>state[group].some(row=>row.kind===kind)
  const tree=(directory,prefix='')=>Object.fromEntries(readdirSync(directory,{withFileTypes:true}).flatMap(entry=>{const name=prefix+entry.name,file=path.join(directory,entry.name);return entry.isDirectory()?Object.entries(tree(file,name+'/')):[[name,sha(file)]]}))
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const result=await invoke(cmd,args);assert.equal(result.ok,true,result.error);return result.value}
  const click=async target=>{
    for(const toast of await driver.findElements(By.css('.toast-item button'))){try{if(await toast.isDisplayed())await toast.click()}catch{}}
    const element=typeof target==='string'?await waitForElement(target):target
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&r.height>0&&(hit===e||e.contains(hit))},element),15000);await element.click()
  }
  const fill=async(selector,value)=>{const e=await waitForElement(selector);await driver.wait(async()=>await e.isEnabled()&&await e.isDisplayed(),35000);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const choose=async(selector,label)=>{await click(selector);for(const option of await driver.findElements(By.css('[role="option"]'))){const strong=await option.findElements(By.css('strong')),text=(await(strong[0]||option).getAttribute('textContent')).trim();if(text===label||text.startsWith(label+'（')){await click(option);return}}throw Error('Missing option '+label)}
  const sizes=async(name,anchor)=>{
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),(typeof anchor==='string'?await waitForElement(anchor):anchor))
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const layout=await driver.executeScript(()=>{const d=document.querySelector('[data-testid="compression-draft-details"]');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,columns:d?[...d.children].map(e=>e.getBoundingClientRect().width):[],panels:[...document.querySelectorAll('[role="dialog"],.browser-page')].map(e=>e.scrollWidth-e.clientWidth)}})
      assert.ok(layout.overflow<=1&&layout.panels.every(value=>value<=1),JSON.stringify(layout));if(name==='inline-config'&&width===1100)assert.ok(Math.abs(layout.columns[0]-layout.columns[1])<=1)
      const file='wim1527-'+name+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots.push(file);state.layouts.push({name,binarySha256,...layout});save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  const fixtures=path.join(root,'tests/fixtures/archives/wim-volumes-1527'),manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
  const tool=path.join(base,'wimlib/wimlib-imagex.exe');assert.equal(sha(tool),manifest.toolSha256)
  const independent=(file,expected,label,{emptyDirectory,level}={})=>{
    const output=owned(path.join(work,label+'-wimlib-'+randomUUID()));mkdirSync(output)
    execFileSync(tool,['verify',file],{windowsHide:true,timeout:120000,maxBuffer:4*1024*1024})
    execFileSync(tool,['apply',file,'1',output,'--no-acls'],{windowsHide:true,timeout:120000,maxBuffer:4*1024*1024})
    assert.deepEqual(tree(output),expected);if(emptyDirectory)assert.ok(existsSync(path.join(output,emptyDirectory)))
    if(level!==undefined)assert.equal(Boolean(readFileSync(file).readUInt32LE(16)&2),level!==0)
    return{output,toolSha256:sha(tool),allIndependentHashes:true,emptyDirectory:!!emptyDirectory}
  }
  const mergeForOracle=(published,label)=>{const file=owned(path.join(work,label+'-独立合并-'+randomUUID()+'.wim'));writeFileSync(file,Buffer.concat(published.files.map(value=>readFileSync(value.path))));return file}
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,file_filter:null,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],selected_entries:[]}
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  for(const original of manifest.cases){
    const kind='original-'+original.method;if(passed('native',kind))continue
    const file=path.join(fixtures,original.file);assert.equal(sha(file),original.sha256)
    const folder=owned(path.join(work,kind));mkdirSync(folder)
    const bytes=readFileSync(file),files=[];for(let offset=0;offset<bytes.length;){const size=files.length===0?7:files.length===1?13:4093,end=Math.min(offset+size,bytes.length),member=owned(path.join(folder,'镜像.wim.'+String(files.length+1).padStart(3,'0')));writeFileSync(member,bytes.subarray(offset,end));files.push(member);offset=end}
    const output=owned(path.join(work,kind+'-实际提取'));mkdirSync(output)
    await ipc('extract_file',{taskId:randomUUID(),filePath:files[Math.floor(files.length/2)],outputPath:output,password:null,options});assert.deepEqual(tree(output),original.expected)
    for(const directory of original.directories)assert.ok(existsSync(path.join(output,directory)))
    done('native',kind,{files,output,allIndependentHashes:true})
  }
  if(!state.source){
    const source=owned(path.join(work,'资料'));mkdirSync(source);mkdirSync(path.join(source,'空目录'))
    writeFileSync(path.join(source,'说明.txt'),'WIM 数字分卷，直接浏览和提取。\n');writeFileSync(path.join(source,'空文件.txt'),'');writeFileSync(path.join(source,'数据.bin'),randomBytes(2*1024*1024+257));writeFileSync(path.join(source,'图标.png'),readFileSync(path.join(root,'src-tauri/icons/32x32.png')))
    state.source=source;state.expected=Object.fromEntries(Object.entries(tree(source)).map(([name,digest])=>['资料/'+name,digest]));save()
  }
  for(const level of [0,6]){
    const kind='created-level-'+level;if(passed('native',kind))continue
    const output=owned(path.join(work,kind+'.wim')),published=await ipc('compress_files',{taskId:randomUUID(),files:[state.source],outputPath:output,options:{format:'wim',level,split_size:65537,preserve_paths:true,verify_after:true,delete_after:false}})
    assert.ok(published.split&&published.files.length>2);assert.ok(published.files.every(file=>file.bytes<=65537))
    const recovered=owned(path.join(work,kind+'-中卷提取'));mkdirSync(recovered)
    await ipc('extract_file',{taskId:randomUUID(),filePath:published.files[Math.floor(published.files.length/2)].path,outputPath:recovered,password:null,options});assert.deepEqual(tree(recovered),state.expected)
    done('native',kind,{published,recovered,independent:independent(mergeForOracle(published,kind),state.expected,kind,{emptyDirectory:'资料/空目录',level})})
  }
  if(!passed('ui','creation-and-result-checksums')){
    await bridge('clearCompressionWorkspace');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[path.join(state.source,'数据.bin')]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-draft-row"] [data-testid="compression-archive-name"]')
    await choose('[data-testid="compression-format-select"]','WIM');assert.equal(await(await waitForElement('[data-testid="compression-password-input"]')).isEnabled(),false)
    const split=await waitForElement('[data-testid="compression-split-toggle"]');assert.equal(await split.isEnabled(),true);if(!await split.isSelected())await click(split)
    await fill('[data-testid="compression-split-size"]','1');await fill('[data-testid="compression-output-path"]',work);await fill('[data-testid="compression-filename-input"]','界面镜像')
    await sizes('inline-config','[data-testid="wim-volume-hint"]');await click('[data-testid="start-compression"]')
    const record=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.outputPath?.includes('界面镜像')&&['completed','failed'].includes(row.status))||false,60000);assert.equal(record.status,'completed',JSON.stringify(record));const files=record.archiveOutput.files;assert.equal(files.length,3)
    independent(mergeForOracle(record.archiveOutput,'界面镜像'),{'数据.bin':sha(path.join(state.source,'数据.bin'))},'界面创建')
    await click('[data-testid="compression-draft-details"] [data-testid="result-generate-checksums"]');assert.match(await(await waitForElement('[data-testid="checksum-source-summary"]')).getText(),/整组 3 个文件/)
    await click('[data-testid="checksum-calculate-start"]');await driver.wait(async()=>await driver.executeScript(()=>document.querySelectorAll('[data-testid="checksum-calculation-entry"][data-status="success"]').length===3),15000)
    assert.deepEqual(await driver.executeScript(()=>[...document.querySelectorAll('.checksum-main>code')].map(e=>e.textContent)),files.map(file=>sha(file.path)));await sizes('result-checksums','.checksum-results');done('ui','creation-and-result-checksums',{record,allVolumes:true,independentDigests:true})
  }
  if(!passed('ui','compression-batch')){
    await bridge('clearCompressionWorkspace');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[path.join(state.source,'数据.bin'),path.join(state.source,'说明.txt')]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    for(let index=0;index<2;index++){
      const row=(await driver.findElements(By.css('[data-testid="compression-draft-row"]')))[index];await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
      await choose('[data-testid="compression-format-select"]','7Z');if(index===0)await fill('[data-testid="compression-password-input"]','保留这条密码')
      const split=await waitForElement('[data-testid="compression-split-toggle"]');if(!await split.isSelected())await click(split);await fill('[data-testid="compression-split-size"]',String(index+1));await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
    }
    for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]')))await click(box)
    const before=(await bridge('taskHistory')).length
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','WIM');await click('[data-testid="batch-config-apply"]')
    await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('WIM 不支持原生密码'),15000);await sizes('batch-password-kept','[role="dialog"]');await click('[role="dialog"] button[aria-label="关闭批量配置"]')
    const row=(await driver.findElements(By.css('[data-testid="compression-draft-row"]')))[0];await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')));assert.equal(await(await waitForElement('[data-testid="compression-password-input"]')).getAttribute('value'),'保留这条密码');await fill('[data-testid="compression-password-input"]','');await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','WIM');await sizes('compression-batch','[role="dialog"]');await click('[data-testid="batch-config-apply"]');await driver.wait(async()=>(await driver.findElements(By.css('[role="dialog"]'))).length===0,10000);assert.equal((await bridge('taskHistory')).length,before)
    for(let index=0;index<2;index++){
      const row=(await driver.findElements(By.css('[data-testid="compression-draft-row"]')))[index];await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')));assert.equal(await(await waitForElement('[data-testid="compression-split-size"]')).getAttribute('value'),String(index+1));assert.equal(await(await waitForElement('[data-testid="compression-password-input"]')).isEnabled(),false);await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
    }
    done('ui','compression-batch',{originalModal:true,perRowSplitRetained:true,passwordRejectedAtomically:true,noImplicitStart:true})
  }
  const original=manifest.cases.find(row=>row.method==='lzx'),parts=state.native.find(row=>row.kind==='original-lzx').files
  if(!passed('ui','images-previews-selected-extraction')){
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[parts[Math.floor(parts.length/2)]]]);await click('[data-testid="file-manager-open-archive"]')
    const folder=await waitForElement('[data-entry-path="2/"],[data-entry-path="2"]');await folder.sendKeys(Key.ENTER);await waitForElement('[data-entry-path="2/说明.txt"]')
    for(const [kind,name,selector]of[['text','说明.txt','[data-testid="archive-text-preview"]'],['image','图标.png','[data-testid="archive-image-preview"]']]){
      await driver.executeScript(e=>e.click(),await waitForElement('[data-entry-path="2/'+name+'"] .preview-trigger'));await waitForElement(selector);if(kind==='text')assert.match(await(await waitForElement(selector)).getText(),/独立 WIM 镜像 2/);await sizes(kind+'-preview',selector);await click('.preview-close')
    }
    // Start with no selected files, then choose exactly one from image 2.
    const all=await waitForElement('.browser-table-head .browser-checkbox');if((await all.getAttribute('class')).includes('checked'))await click(all)
    await click('[data-entry-path="2/说明.txt"] .browser-checkbox');const output=owned(path.join(work,'只取镜像二的说明'));mkdirSync(output)
    await bridge('queueDesktopDialogSelections',[[output]]);await click('.output-target');await sizes('selected-image','footer .browser-primary');await click('footer .browser-primary')
    await driver.wait(()=>existsSync(path.join(output,'2/说明.txt')),30000);assert.deepEqual(tree(output),{'2/说明.txt':original.expected['2/说明.txt']});done('ui','images-previews-selected-extraction',{middleMember:true,image2Only:true,output,independentHash:true})
  }
  if(!passed('ui','missing-volume-recovery')){
    await bridge('clearTasks');await click('[data-testid="nav-Decompress"]');await bridge('queueDesktopDialogSelections',[[parts[1]]]);await click('[data-testid="dropzone-file"]')
    const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000),backup=owned(parts[1]+'.owned-original');renameSync(parts[1],backup)
    let failed;try{await click('[data-testid="start-decompression-queue"]');failed=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.id===task.id&&row.status==='failed')||false,30000)}finally{renameSync(backup,parts[1])}
    assert.match(failed.errorMessage,/incomplete|缺少|分卷|source-missing/);await click('[data-testid="retry-batch-failures"]');const output=owned(path.join(work,'补齐后的镜像'));mkdirSync(output);await fill('[role="dialog"] [data-testid="retry-output"]',output)
    const sub=await waitForElement('[data-testid="retry-subfolder"]');if(await sub.isSelected())await click(sub);await sizes('missing-recovery','[role="dialog"]');await click('[data-testid="create-retry-drafts"]');await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'),15000);await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(.,"返回队列确认并启动")]')))
    const retry=(await bridge('extractionRecoveryTasks')).find(row=>row.retryOfTaskId===task.id);assert.equal(retry.status,'pending');assert.deepEqual(readdirSync(output),[]);await click('[data-testid="start-decompression-queue"]')
    const completed=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.id===retry.id&&['completed','failed'].includes(row.status))||false,30000);assert.equal(completed.status,'completed',JSON.stringify(completed));assert.deepEqual(tree(output),original.expected);assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed);done('ui','missing-volume-recovery',{oldFailurePreserved:true,newPendingTask:true,independentHashes:true})
  }
  for(const target of ['zip','wim']){
    const kind='conversion-'+target;if(passed('ui',kind))continue
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
    const source=target==='zip'?parts[parts.length-1]:state.ui.find(row=>row.kind==='conversion-zip').output,password='新的 ZIP 密码1527',before=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length
    await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="conversion-choose-source"]');await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length>before,15000)
    const item=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).at(-1),row=await item.findElement(By.css('[data-testid="conversion-row"]'));if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    if(target==='wim')await fill('[data-testid="conversion-source-password"]',password);await choose('[data-testid="conversion-format"]',target.toUpperCase());if(target==='zip')await fill('[data-testid="conversion-password"]',password);else{assert.equal(await(await waitForElement('[data-testid="conversion-password"]')).isEnabled(),false);assert.equal(await(await waitForElement('[data-testid="conversion-source-password"]')).getAttribute('value'),password);await fill('[data-testid="conversion-split"]','1')}
    const output=owned(path.join(work,'互转-'+target+'-'+randomUUID()+'.'+target));await fill('[data-testid="conversion-output"]',output);await click('[data-testid="conversion-start"]')
    const requested=output.replaceAll('\\','/').toLowerCase(),record=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>[requested,requested+'.001'].includes(row.outputPath?.replaceAll('\\','/').toLowerCase())&&['completed','failed'].includes(row.status))||false,60000);assert.equal(record.status,'completed',JSON.stringify(record))
    if(target==='zip'){
      const request=owned(path.join(work,'zip-independent-request.json')),result=owned(path.join(work,'zip-independent-result.json'));writeFileSync(request,JSON.stringify({type:'zip',file:output,password,expectedFiles:original.expected,emptyDirectory:'2/空目录'}));execFileSync('python',[path.join(root,'scripts/verify-tar-lzma-1524.py'),request,result],{windowsHide:true,timeout:30000,maxBuffer:4*1024*1024});state.zipIndependent=JSON.parse(readFileSync(result,'utf8'))
    }else independent(mergeForOracle(record.archiveOutput,'转换结果'),original.expected,'转换结果',{emptyDirectory:'2/空目录'})
    await sizes('conversion-'+target,item);done('ui',kind,{output,record,independentDecodedHashes:true,preservedImageDirectories:true});await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
  state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'wim-volumes-1527-results.json'),JSON.stringify(state,null,2))
}

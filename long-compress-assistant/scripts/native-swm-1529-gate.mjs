import assert from 'node:assert/strict'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync,readdirSync,renameSync,copyFileSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runSwm1529Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const base=path.join(root,'test-results/native-swm-1529'),progress=path.join(base,'desktop-progress.json'),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p)),binarySha256=sha(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.29',binarySha256,sourceCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim(),work:path.join(base,'ui-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  if(state.binarySha256!==binarySha256){
    assert.equal(process.env.SWM_1529_RESUME_FROM,state.binarySha256,'Explicitly acknowledge prior passed evidence identity')
    state.previousIdentities=[...(state.previousIdentities||[]),{binarySha256:state.binarySha256,sourceCommit:state.sourceCommit}]
    state.binarySha256=binarySha256;state.sourceCommit=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim()
  }
  const work=state.work;mkdirSync(work,{recursive:true})
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const done=(group,kind,data={})=>{state[group].push({kind,binarySha256,...data});save();console.log('swm-1529: '+group+' '+kind)}
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
      const file='swm1529-'+name+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots=state.screenshots.filter(value=>value!==file);state.screenshots.push(file);state.layouts=state.layouts.filter(value=>!(value.name===name&&value.width===layout.width&&value.height===layout.height&&value.binarySha256===binarySha256));state.layouts.push({name,binarySha256,...layout});save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  const fixtures=path.join(root,'tests/fixtures/archives/native-swm-1529'),manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,file_filter:null,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],selected_entries:[]}
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  if(!passed('native','menu-isolation')){
    for(const cmd of ['register_context_menu','unregister_context_menu']){const result=await invoke(cmd,{});assert.equal(result.ok,false);assert.match(result.error,/不修改系统右键菜单/)}
    assert.equal(await ipc('is_context_menu_registered',{}),false);done('native','menu-isolation',{bothMutationsRejected:true})
  }
  for(const original of manifest.cases){
    const kind='original-'+original.method;if(passed('native',kind))continue
    const folder=owned(path.join(work,kind));mkdirSync(folder)
    const files=original.members.map(part=>{const source=path.join(fixtures,part.file);assert.equal(sha(source),part.sha256);const target=owned(path.join(folder,path.basename(source)));copyFileSync(source,target);return target})
    const output=owned(path.join(work,kind+'-完整提取'));mkdirSync(output)
    await ipc('extract_file',{taskId:randomUUID(),filePath:files[Math.floor(files.length/2)],outputPath:output,password:null,options});assert.deepEqual(tree(output),original.expected)
    for(const directory of original.directories)assert.ok(existsSync(path.join(output,directory)))
    done('native',kind,{files,output,allIndependentHashes:true,noJoinedWim:readdirSync(folder).every(name=>name.endsWith('.swm'))})
  }
  const original=manifest.cases.find(c=>c.method==='lzx'),parts=state.native.find(c=>c.kind==='original-lzx').files
  if(!passed('ui','images-previews-selected-extraction')){
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[parts[Math.floor(parts.length/2)]]]);await click('[data-testid="file-manager-open-archive"]')
    const folder=await waitForElement('[data-entry-path="2/"],[data-entry-path="2"]');await folder.sendKeys(Key.ENTER);await waitForElement('[data-entry-path="2/说明.txt"]');assert.match(await(await waitForElement('[data-testid="wim-image-hint"]')).getText(),/原生 SWM/)
    if(!state.previewCompleted)for(const[kind,name,selector]of[['text','说明.txt','[data-testid="archive-text-preview"]'],['image','图标.png','[data-testid="archive-image-preview"]']]){
      await driver.executeScript(e=>e.click(),await waitForElement('[data-entry-path="2/'+name+'"] .preview-trigger'));await waitForElement(selector);if(kind==='text')assert.match(await(await waitForElement(selector)).getText(),/原生 SWM 独立镜像 2/);await sizes(kind+'-preview',selector);await click('.preview-close')
    }
    state.previewCompleted={binarySha256,completedAt:new Date().toISOString()};save()
    await fill('.browser-search input','.');const all=await waitForElement('.browser-table-head .browser-checkbox');if((await all.getAttribute('class')).includes('checked'))await click(all)
    await fill('.browser-search input','2/说明.txt');await click('[data-entry-path="2/说明.txt"] .browser-checkbox');await fill('.browser-search input','')
    const output=owned(path.join(work,'只取镜像二-'+randomUUID()));mkdirSync(output);await bridge('queueDesktopDialogSelections',[output]);await click('.output-target');await driver.wait(async()=>(await(await waitForElement('.output-target')).getAttribute('title'))===output,5000)
    await sizes('selected-image','footer .browser-primary');await click('footer .browser-primary');await driver.wait(()=>existsSync(path.join(output,'2/说明.txt')),30000);assert.deepEqual(tree(output),{'2/说明.txt':original.expected['2/说明.txt']})
    done('ui','images-previews-selected-extraction',{middleMember:true,image2Only:true,output,independentHash:true})
  }
  if(!passed('ui','missing-tail-recovery')){
    await bridge('clearTasks');await click('[data-testid="nav-Decompress"]');await bridge('queueDesktopDialogSelections',[[parts[1]]]);await click('[data-testid="dropzone-file"]')
    const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000),last=parts.at(-1),backup=owned(last+'.owned-original');renameSync(last,backup)
    let failed;try{await click('[data-testid="start-decompression-queue"]');failed=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.id===task.id&&row.status==='failed')||false,30000)}finally{renameSync(backup,last)}
    assert.ok(failed.errorMessage.includes(path.basename(last)),JSON.stringify(failed));await click('[data-testid="retry-batch-failures"]');const output=owned(path.join(work,'补齐后的原生镜像'));mkdirSync(output);await fill('[role="dialog"] [data-testid="retry-output"]',output)
    const sub=await waitForElement('[data-testid="retry-subfolder"]');if(await sub.isSelected())await click(sub);await sizes('missing-tail-recovery','[role="dialog"]');await click('[data-testid="create-retry-drafts"]');await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'),15000);await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(.,"返回队列确认并启动")]')))
    const retry=(await bridge('extractionRecoveryTasks')).find(row=>row.retryOfTaskId===task.id);assert.equal(retry.status,'pending');assert.deepEqual(readdirSync(output),[]);await click('[data-testid="start-decompression-queue"]')
    const completed=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.id===retry.id&&['completed','failed'].includes(row.status))||false,30000);assert.equal(completed.status,'completed',JSON.stringify(completed));assert.deepEqual(tree(output),original.expected);assert.deepEqual((await bridge('taskHistory')).find(row=>row.id===failed.id),failed)
    await sizes('extraction-result','[data-testid="decompression-center"]');done('ui','missing-tail-recovery',{oldFailurePreserved:true,newPendingTask:true,independentHashes:true,record:completed})
  }
  for(const target of ['zip','wim']){
    const kind='conversion-'+target;if(passed('ui',kind))continue
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]');const before=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length
    await bridge('queueDesktopDialogSelections',[[parts.at(-1)]]);await click('[data-testid="conversion-choose-source"]');await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length>before,15000)
    const item=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).at(-1),row=await item.findElement(By.css('[data-testid="conversion-row"]'));if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    await choose('[data-testid="conversion-format"]',target.toUpperCase());const password='新的 ZIP 密码1529';if(target==='zip')await fill('[data-testid="conversion-password"]',password);else assert.equal(await(await waitForElement('[data-testid="conversion-password"]')).isEnabled(),false)
    const output=owned(path.join(work,'原生SWM转换-'+target+'-'+randomUUID()+'.'+target));await fill('[data-testid="conversion-output"]',output);await click('[data-testid="conversion-start"]')
    const requested=output.replaceAll('\\','/').toLowerCase(),record=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>requested===row.outputPath?.replaceAll('\\','/').toLowerCase()&&['completed','failed'].includes(row.status))||false,60000);assert.equal(record.status,'completed',JSON.stringify(record))
    if(target==='zip'){
      const request=owned(path.join(work,'zip-independent-request.json')),result=owned(path.join(work,'zip-independent-result.json'));writeFileSync(request,JSON.stringify({type:'zip',file:output,password,expectedFiles:original.expected,emptyDirectory:'2/空目录'}));execFileSync('python',[path.join(root,'scripts/verify-tar-lzma-1524.py'),request,result],{windowsHide:true,timeout:30000,maxBuffer:4*1024*1024});state.zipIndependent=JSON.parse(readFileSync(result,'utf8'))
    }else{
      const tool=path.join(root,'test-results/wim-volumes-1527/wimlib/wimlib-imagex.exe');assert.equal(sha(tool),manifest.toolSha256);const recovered=owned(path.join(work,'转换WIM独立回解'));mkdirSync(recovered)
      execFileSync(tool,['apply',output,'1',recovered,'--no-acls'],{windowsHide:true,timeout:30000,maxBuffer:4*1024*1024});assert.deepEqual(tree(recovered),original.expected);assert.ok(existsSync(path.join(recovered,'2/空目录')))
    }
    await sizes('conversion-'+target,item);done('ui',kind,{output,record,independentDecodedHashes:true,preservedImageDirectories:true});await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
  state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'native-swm-1529-results.json'),JSON.stringify(state,null,2))
}

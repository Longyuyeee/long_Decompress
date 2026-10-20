import assert from 'node:assert/strict'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,existsSync,renameSync} from 'node:fs'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runAes1521Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p))
  const resultRoot=path.join(root,'test-results/aes-1521'),progress=path.join(resultRoot,'desktop-progress.json'),binarySha256=sha(application)
  const saved=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):null
  if(saved)assert.equal(saved.binarySha256,binarySha256,'Different program: do not reuse earlier results automatically')
  const state=saved||{version:'1.5.21',binarySha256,work:path.join(resultRoot,'ui-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  const work=state.work;assert.ok(path.resolve(work).startsWith(path.resolve(resultRoot)+path.sep));mkdirSync(work,{recursive:true})
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const fresh=name=>owned(path.join(work,name+'-'+randomUUID()))
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const password='Long 新内层-1521',base=path.join(root,'tests/fixtures/archives/aes-codecs-1521'),manifest=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8'))
  const old={expected:Object.entries(manifest.expectedFiles).map(([path,sha256])=>({path,sha256})),cases:manifest.cases}
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,file_filter:null,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],selected_entries:[]}
  await driver.manage().setTimeouts({script:180000})
  await driver.manage().window().setRect({width:1100,height:700})
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
  const click=async selector=>{
    for(const t of await driver.findElements(By.css('.toast-item button'))){try{if(await t.isDisplayed())await t.click()}catch{}}
    const e=typeof selector==='string'?await waitForElement(selector):selector
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&r.height>0&&(h===e||e.contains(h))},e),15000)
    await e.click()
  }
  const fill=async(selector,value)=>{const e=await waitForElement(selector);await driver.wait(async()=>await e.isDisplayed()&&await e.isEnabled(),15000);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const choose=async(selector,label)=>{
    await click(selector);let found=false
    for(const option of await driver.findElements(By.css('[role="option"]'))){
      const strong=await option.findElements(By.css('strong')),text=(await(strong[0]||option).getText()).trim()
      if(text===label||text.startsWith(label+' ')||text.startsWith(label+'（')){await click(option);found=true;break}
    }assert.ok(found,label+' option missing')
  }
  const sizes=async(name,anchor)=>{
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height})
      await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),typeof anchor==='string'?await waitForElement(anchor):anchor)
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const geometry=await driver.executeScript(()=>{const d=document.querySelector('[data-testid="compression-draft-details"]');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,columns:d?[...d.children].map(c=>c.getBoundingClientRect().width):[],detailOverflow:d?d.scrollWidth-d.clientWidth:0}})
      assert.ok(geometry.overflow<=1&&geometry.detailOverflow<=1,JSON.stringify(geometry));if(name==='inline-config'&&width===1100)assert.ok(Math.abs(geometry.columns[0]-geometry.columns[1])<=1,JSON.stringify(geometry))
      const namePng='aes1521-'+name+'-'+width+'x'+height+'.png'
      writeFileSync(path.join(artifactDirectory,namePng),Buffer.from(await driver.takeScreenshot(),'base64'))
      state.screenshots.push(namePng);state.layouts.push({name,...geometry});save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  const split=(source,label)=>{
    const folder=fresh(label);mkdirSync(folder);const bytes=readFileSync(source),parts=[],size=Math.max(1,Math.min(8191,Math.floor(bytes.length/3)))
    for(let off=0;off<bytes.length;off+=size){const p=owned(path.join(folder,path.basename(source)+'.'+String(parts.length+1).padStart(3,'0')));writeFileSync(p,bytes.subarray(off,off+size));parts.push(p)}return parts
  }
  const fullOriginal=(c,out)=>{
    if(c.kind==='tar'){
      for(const f of old.expected)assert.equal(sha(path.join(out,f.path)),f.sha256)
      assert.ok(existsSync(path.join(out,manifest.emptyDirectory)))
    }else assert.equal(sha(path.join(out,c.archive.replace(/\.aes$/,'').replace(/\.(gz|bz2|xz|zst|lz4|br|lz)$/,''))),manifest.expectedFiles['大文本.txt'])
  }
  for(const c of manifest.cases){
    if(state.native.some(r=>r.original===c.archive))continue
    const original=path.join(base,c.archive);assert.equal(sha(original),c.sha256)
    const parts=split(original,'原件卷组'),out=fresh('原件完整解压');mkdirSync(out)
    await ipc('extract_file',{taskId:randomUUID(),filePath:parts[Math.floor(parts.length/2)],outputPath:out,password,options})
    fullOriginal(c,out)
    state.native.push({original:c.archive,parts,output:out,completeHashes:true,emptyDirectory:c.kind==='tar'});save()
  }
  if(!state.source){
    const folder=owned(path.join(work,'创建来源'));mkdirSync(folder)
    writeFileSync(path.join(folder,'开头.txt'),'真正的开头文件\n');writeFileSync(path.join(folder,'数据.bin'),randomBytes(2*1024*1024+37))
    writeFileSync(path.join(folder,'末尾.txt'),'真实末尾，不能遗漏。\n');mkdirSync(path.join(folder,'空目录'))
    state.source=folder;state.expectedFiles=Object.fromEntries(['开头.txt','数据.bin','末尾.txt'].map(f=>[f,sha(path.join(folder,f))]));save()
  }
  const createOptions=format=>({format,level:6,password,split_size:65521,preserve_paths:true,verify_after:true,delete_after:false})
  for(const format of ['lz4.aes','br.aes','lz.aes','tar.lz4.aes','tar.br.aes','tar.lz.aes']){
    if(state.native.some(r=>r.created===format))continue
    const output=fresh('创建')+'.'+format,files=format.startsWith('tar')?readdirSync(state.source).map(f=>path.join(state.source,f)):[path.join(state.source,'数据.bin')]
    const published=await ipc('compress_files',{taskId:randomUUID(),files,outputPath:output,options:createOptions(format)})
    assert.ok(published.split&&published.files.length>1,JSON.stringify(published))
    const out=fresh('创建后完整解压');mkdirSync(out)
    await ipc('extract_file',{taskId:randomUUID(),filePath:published.files[Math.floor(published.files.length/2)].path,outputPath:out,password,options})
    if(format.startsWith('tar')){for(const[name,expected]of Object.entries(state.expectedFiles))assert.equal(sha(path.join(out,name)),expected);assert.ok(existsSync(path.join(out,'空目录')))}
    else assert.equal(sha(path.join(out,path.basename(output).replace(/\.aes$/,'').replace(/\.(gz|bz2|xz|zst|lz4|br|lz)$/,''))),state.expectedFiles['数据.bin'])
    state.native.push({created:format,published,output:out,completeHashes:true});save()
  }
  const original=state.native.find(r=>r.original==='资料.tar.lz.aes')
  if(!state.native.some(r=>r.kind==='image-preview')){
    const input=owned(path.join(work,'图标.png')),output=owned(path.join(work,'图标.png.lz4.aes'))
    writeFileSync(input,readFileSync(path.join(root,'src-tauri/icons/32x32.png')))
    const published=await ipc('compress_files',{taskId:randomUUID(),files:[input],outputPath:output,options:{...createOptions('lz4.aes'),split_size:37}})
    const preview=await ipc('preview_archive_image',{filePath:published.files.at(-1).path,entryPath:'图标.png',password})
    assert.equal(hash(Buffer.from(preview.dataUrl.split(',')[1],'base64')),sha(input));assert.equal(preview.width,32)
    state.native.push({kind:'image-preview',lastMember:true,fullPngHashMatches:true});save()
  }
  for(const format of ['tar.lz4.aes','tar.lz.aes']){
    if(state.native.some(r=>r.kind==='selected'&&r.format===format))continue
    const created=state.native.find(r=>r.created===format),out=fresh('所选文件');mkdirSync(out)
    await ipc('extract_file',{taskId:randomUUID(),filePath:created.published.files[1].path,outputPath:out,password,options:{...options,selected_entries:['末尾.txt']}})
    assert.deepEqual(readdirSync(out),['末尾.txt']);assert.equal(sha(path.join(out,'末尾.txt')),state.expectedFiles['末尾.txt'])
    state.native.push({kind:'selected',format,onlyRequestedFile:true,fullContentHash:true});save()
  }
  if(!state.native.some(r=>r.kind==='actual-late-failure')){
    const parts=split(path.join(base,'资料.tar.lz.aes'),'认证失败'),last=parts.at(-1),bytes=readFileSync(last);bytes[bytes.length-1]^=1;writeFileSync(last,bytes)
    const out=fresh('认证失败不得发布');mkdirSync(out)
    const failed=await invoke('extract_file',{taskId:randomUUID(),filePath:parts[1],outputPath:out,password,options})
    assert.equal(failed.ok,false);assert.match(failed.error,/damaged|后续分块/);assert.deepEqual(readdirSync(out),[])
    assert.ok(!readdirSync(work).some(f=>f.startsWith('.long-extract-')))
    state.native.push({kind:'actual-late-failure',reason:failed.error,noPublishedPartial:true});save()
  }
  if(!state.native.some(r=>r.converted==='single-stream')){
    const source=state.native.find(r=>r.created==='lz4.aes').published.files.at(-1).path,output=fresh('单文件改格式')+'.lz.aes'
    await bridge('convertArchive',source,password,output,{format:'lz.aes',password,split_size:65521})
    const prefix=path.basename(output),parts=readdirSync(work).filter(f=>f.startsWith(prefix+'.')&&/^\d{3,}$/.test(f.slice(prefix.length+1))).sort().map(f=>path.join(work,f));assert.ok(parts.length>1)
    state.native.push({converted:'single-stream',format:'lz.aes',parts});save()
  }
  if(!state.ui.some(r=>r.kind==='creation')){
    await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[path.join(state.source,'数据.bin')]]);await click('[data-testid="dropzone-file"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-draft-row"]')
    await choose('[data-testid="compression-format-select"]','LZ4.AES')
    await fill('[data-testid="compression-password-input"]',password)
    const toggle=await waitForElement('[data-testid="compression-split-toggle"]');assert.equal(await toggle.isEnabled(),true);if(!await toggle.isSelected())await click(toggle)
    await fill('[data-testid="compression-split-size"]','1');await fill('[data-testid="compression-output-path"]',work);await fill('[data-testid="compression-filename-input"]','界面创建')
    assert.match(await(await waitForElement('[data-testid="aes-volume-hint"]')).getText(),/Long 自有/)
    await sizes('inline-config','[data-testid="aes-volume-hint"]')
    await click('[data-testid="start-compression"]')
    const record=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.outputPath?.includes('界面创建')&&['completed','failed'].includes(h.status))||false,60000)
    assert.equal(record.status,'completed',JSON.stringify(record))
    state.ui.push({kind:'creation',history:record,realInlineConfiguration:true});save()
  }
  if(!state.ui.some(r=>r.kind==='preview')){
    await click('[data-testid="nav-ArchiveBrowser"]')
    await bridge('queueDesktopDialogSelections',[[original.parts[1]]]);await click('[data-testid="file-manager-open-archive"]')
    await fill('.browser-toolbar input[type="password"]',password)
    await (await waitForElement('.browser-toolbar input[type="password"]')).sendKeys(Key.ENTER)
    await click('[data-entry-path="说明.txt"] .preview-trigger')
    assert.match(await(await waitForElement('[data-testid="archive-text-preview"]')).getText(),/先填写密码/)
    await sizes('middle-preview','[data-testid="archive-text-preview"]');await click('.preview-close')
    state.ui.push({kind:'preview',actualMiddleMemberText:true});save()
  }
  if(!state.ui.some(r=>r.kind==='recovery')){
    await bridge('clearTasks');await click('[data-testid="nav-Decompress"]')
    await bridge('queueDesktopDialogSelections',[[original.parts[1]]]);await click('[data-testid="dropzone-file"]')
    const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000)
    const withdrawn=owned(original.parts[2]),backup=owned(withdrawn+'.owned-original');renameSync(withdrawn,backup)
    let failed
    try{await click('[data-testid="start-decompression-queue"]');failed=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.id===task.id&&h.status==='failed')||false,30000)}
    finally{if(existsSync(backup))renameSync(backup,withdrawn)}
    assert.match(failed.errorMessage,/incomplete|缺少|分卷|source-missing/)
    await click('[data-testid="retry-batch-failures"]')
    const out=fresh('补卷新任务');mkdirSync(out);await fill('[role="dialog"] [data-testid="retry-output"]',out)
    await fill('[role="dialog"] [data-testid="retry-password"]',password)
    const sub=await waitForElement('[role="dialog"] [data-testid="retry-subfolder"]');if(await sub.isSelected())await click(sub)
    await sizes('batch-recovery','[role="dialog"]')
    await click('[role="dialog"] [data-testid="create-retry-drafts"]')
    await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'),15000);assert.deepEqual(readdirSync(out),[])
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(.,"返回队列确认并启动")]')))
    const retry=(await bridge('extractionRecoveryTasks')).find(t=>t.retryOfTaskId===task.id);assert.equal(retry.status,'pending')
    await click('[data-testid="start-decompression-queue"]')
    const completed=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.id===retry.id&&['completed','failed'].includes(h.status))||false,30000);assert.equal(completed.status,'completed',JSON.stringify(completed))
    fullOriginal({kind:'tar'},out);assert.deepEqual((await bridge('taskHistory')).find(h=>h.id===failed.id),failed)
    state.ui.push({kind:'recovery',missingVolumeRestored:true,completeHashes:true,oldFailureUnchanged:true,output:out});save()
  }
  for(const target of ['zip','tar.br.aes']){
    if(state.ui.some(r=>r.kind==='conversion'&&r.target===target))continue
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
    const source=target==='zip'?state.native.find(r=>r.created==='tar.lz4.aes').published.primaryPath:state.ui.find(r=>r.kind==='conversion'&&r.target==='zip').output
    const before=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length
    await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="conversion-choose-source"]')
    await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length>before,15000)
    const currentItem=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).at(-1)
    const row=await currentItem.findElement(By.css('[data-testid="conversion-row"]'))
    if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    await fill('[data-testid="conversion-source-password"]',password)
    await choose('[data-testid="conversion-format"]',target==='zip'?'ZIP':'TAR.BR.AES')
    const output=fresh('转换')+'.'+target
    await fill('[data-testid="conversion-password"]',password);await fill('[data-testid="conversion-output"]',output)
    if(target!=='zip')await fill('[data-testid="conversion-split"]','1')
    await click('[data-testid="conversion-start"]')
    const requested=output.replaceAll('\\','/').toLowerCase()
    const history=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>[requested,requested+'.001'].includes(h.outputPath?.replaceAll('\\','/').toLowerCase())&&['completed','failed'].includes(h.status))||false,60000);assert.equal(history.status,'completed',JSON.stringify(history))
    if(target==='zip'){
      const request=owned(path.join(work,'zip-independent-request.json')),result=owned(path.join(work,'zip-independent-result.json'))
      writeFileSync(request,JSON.stringify({type:'zip',file:output,password,expectedFiles:state.expectedFiles,emptyDirectory:'空目录'}))
      execFileSync('python',[path.join(root,'scripts/verify-aes-1521.py'),request,result],{timeout:30000,windowsHide:true})
      state.zipIndependent=JSON.parse(readFileSync(result,'utf8'))
    }
    await sizes('conversion-'+(target==='zip'?'zip':'aes'),currentItem)
    state.ui.push({kind:'conversion',target,output,history,independentZipHashes:target==='zip'});save()
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
  if(!state.ui.some(r=>r.kind==='batch-config')){
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
    const sources=state.native.filter(r=>r.original).slice(0,2).map(r=>r.parts[1])
    await bridge('queueDesktopDialogSelections',[sources]);await click('[data-testid="conversion-choose-source"]')
    await driver.wait(async()=>{const checkboxes=await driver.findElements(By.css('[data-testid="conversion-checkbox"]'));let enabled=0;for(const c of checkboxes)if(await c.isEnabled())enabled++;return enabled===2},15000)
    for(const c of await driver.findElements(By.css('[data-testid="conversion-checkbox"]')))if(await c.isEnabled()&&!await c.isSelected())await click(c)
    await click('[data-testid="conversion-batch-config"]');await choose('[data-testid="conversion-batch-format"]','TAR.LZIP.AES')
    await click(await driver.findElement(By.xpath('//*[@data-testid="conversion-batch-fields"]//label[contains(.,"修改新包密码")]//input')));await fill('[data-testid="conversion-batch-fields"] input[autocomplete="new-password"]',password)
    await click(await driver.findElement(By.xpath('//*[@data-testid="conversion-batch-fields"]//label[contains(.,"填写共用原包密码")]//input')));await fill('[data-testid="conversion-batch-fields"] input[autocomplete="off"]',password)
    await click(await driver.findElement(By.xpath('//*[@data-testid="conversion-batch-fields"]//label[contains(.,"修改分卷大小")]//input')));await fill('[data-testid="conversion-batch-fields"] input[type="number"]','1')
    await sizes('batch-config','[data-testid="conversion-batch-fields"]');const before=(await bridge('taskHistory')).length
    await click('[data-testid="conversion-batch-apply"]');await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="conversion-batch-fields"]'))).length===0,10000)
    assert.equal((await bridge('taskHistory')).length,before,'Applying configuration must not start conversion')
    state.ui.push({kind:'batch-config',twoDrafts:true,encryptedTarAndVolumes:true,noImplicitStart:true});save()
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
  const requests=state.native.filter(c=>c.created).map(c=>({format:c.created,parts:c.published.files.map(f=>f.path),innerPath:owned(path.join(work,'independent-'+c.created.replace(/\.aes$/,'')))}))
  const single=state.native.find(c=>c.converted==='single-stream');requests.push({format:'lz.aes',parts:single.parts,innerPath:owned(path.join(work,'independent-single.lz'))})
  const converted=state.ui.find(c=>c.kind==='conversion'&&c.target==='tar.br.aes'),prefix=path.basename(converted.output)
  const convertedParts=readdirSync(work).filter(f=>f.startsWith(prefix+'.')&&/^\d{3,}$/.test(f.slice(prefix.length+1))).sort().map(f=>path.join(work,f));assert.ok(convertedParts.length>1)
  requests.push({format:'tar.br.aes',parts:convertedParts,innerPath:owned(path.join(work,'independent-conversion.tar.br'))})
  const uiParts=readdirSync(work).filter(f=>/^界面创建\.lz4\.aes\.\d{3,}$/.test(f)).sort().map(f=>path.join(work,f));assert.ok(uiParts.length>1);requests.push({format:'lz4.aes',parts:uiParts,innerPath:owned(path.join(work,'independent-ui.lz4'))})
  const request=owned(path.join(work,'independent-request.json')),result=owned(path.join(work,'independent-result.json'))
  writeFileSync(request,JSON.stringify({password,sevenZip:bundledSevenZip,expectedFiles:state.expectedFiles,emptyDirectory:'空目录',plainSha256:state.expectedFiles['数据.bin'],cases:requests},null,2))
  execFileSync('python',[path.join(root,'scripts/verify-aes-1521.py'),request,result],{timeout:120000,windowsHide:true})
  state.independent=JSON.parse(readFileSync(result,'utf8'));state.completedAt=new Date().toISOString();save()
  writeFileSync(path.join(artifactDirectory,'aes-1521-results.json'),JSON.stringify(state,null,2))
}

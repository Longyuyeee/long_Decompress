import assert from 'node:assert/strict'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,existsSync,renameSync} from 'node:fs'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runLzma1523Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p))
  const base=path.join(root,'test-results/lzma-1523');mkdirSync(base,{recursive:true})
  const progress=path.join(base,'desktop-progress.json'),binarySha256=sha(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.23',binarySha256,work:path.join(base,'work-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  if(state.binarySha256!==binarySha256 && state.currentBinarySha256!==binarySha256){
    assert.ok(process.argv.includes('--lzma-1523-continue-only'),'Different program requires explicitly scoped continuation')
    assert.equal(state.native.length,6);assert.deepEqual(state.ui.map(r=>r.kind),['creation','batch']);assert.equal(state.screenshots.length,4)
    for(const r of [...state.native,...state.ui,...state.layouts])r.binarySha256??=state.binarySha256
    state.currentBinarySha256=binarySha256;state.continuedScope='Only advertised LZMA preview capability, then remaining preview/recovery/conversion UI; previous native/create/batch retained with original program identity'
  }
  state.identities??=[];if(!state.identities.some(i=>i.binarySha256===binarySha256))state.identities.push({binarySha256,source:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim()})
  const work=state.work;mkdirSync(work,{recursive:true});const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p},fresh=n=>owned(path.join(work,n+'-'+randomUUID()))
  const save=()=>{for(const r of [...state.native,...state.ui,...state.layouts])r.binarySha256??=binarySha256;writeFileSync(progress,JSON.stringify(state,null,2))}
  const fixtures=path.join(root,'tests/fixtures/archives/lzma-1523'),manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,file_filter:null,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],selected_entries:[]}
  const seven=(args,cwd)=>execFileSync(bundledSevenZip,args,{cwd,windowsHide:true,encoding:'utf8',timeout:120000})
  const split=(file,label)=>{const folder=fresh(label);mkdirSync(folder);const bytes=readFileSync(file),parts=[],size=Math.max(1,Math.floor(bytes.length/3));for(let off=0;off<bytes.length;off+=size){const p=path.join(folder,path.basename(file)+'.'+String(parts.length+1).padStart(3,'0'));writeFileSync(p,bytes.subarray(off,off+size));parts.push(p)}return parts}
  const independent=(parts,expected)=>{
    const merged=fresh('合并')+'.lzma';writeFileSync(merged,Buffer.concat(parts.map(p=>readFileSync(p))))
    const digest=execFileSync('python',['-c','import sys,lzma,hashlib; print(hashlib.sha256(lzma.decompress(open(sys.argv[1],"rb").read(),format=lzma.FORMAT_ALONE)).hexdigest())',merged],{windowsHide:true,encoding:'utf8'}).trim();assert.equal(digest,expected)
  }
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
  const click=async target=>{
    for(const t of await driver.findElements(By.css('.toast-item button'))){try{if(await t.isDisplayed())await t.click()}catch{}}
    const e=typeof target==='string'?await waitForElement(target):target
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&(h===e||e.contains(h))},e),15000);await e.click()
  }
  const fill=async(selector,value)=>{const e=await waitForElement(selector);await driver.wait(async()=>await e.isDisplayed()&&await e.isEnabled(),10000);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const choose=async(selector,label)=>{await click(selector);for(const e of await driver.findElements(By.css('[role="option"]'))){const strong=await e.findElements(By.css('strong')),text=(await(strong[0]||e).getAttribute('textContent')).trim();if(text===label||text.startsWith(label+'（')){await click(e);return}}assert.fail('Option missing: '+label)}
  const sizes=async(name,anchor)=>{for(const[width,height]of[[1100,700],[760,520]]){
    await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),typeof anchor==='string'?await waitForElement(anchor):anchor)
    await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
    const geometry=await driver.executeScript(()=>{const d=document.querySelector('[data-testid="compression-draft-details"]'),cols=d?[...d.children].map(c=>c.getBoundingClientRect().width):[];return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,columns:cols,detailOverflow:d?d.scrollWidth-d.clientWidth:0}})
    assert.ok(geometry.overflow<=1&&geometry.detailOverflow<=1,JSON.stringify(geometry));if(name==='inline-config'&&width===1100)assert.ok(Math.abs(geometry.columns[0]-geometry.columns[1])<=1,JSON.stringify(geometry));state.layouts.push({name,...geometry})
    const png=`lzma1523-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,png),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots.push(png);save()
  }await driver.manage().window().setRect({width:1100,height:700})}
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  if(!state.source){state.source=owned(path.join(work,'随机来源.bin'));writeFileSync(state.source,randomBytes(2*1024*1024+73));state.expected=sha(state.source);state.encrypted=owned(path.join(work,'密码来源.zip'));seven(['a','-tzip','-mem=AES256','-pLong1523',state.encrypted,path.basename(state.source)],work);save()}
  if(!state.native.some(r=>r.kind==='advertised-preview')){
    const caps=await ipc('get_archive_engine_capabilities',{});assert.ok(caps.boundedPreviewFormats.includes('LZMA'));assert.ok(caps.browseExtensions.includes('lzma'));assert.ok(caps.nestedExtensions.includes('lzma'))
    const outer=fresh('嵌套原包')+'.zip';seven(['a','-tzip',outer,'说明.txt.lzma'],fixtures)
    const nested=await ipc('materialize_nested_archive',{filePath:outer,entryPath:'说明.txt.lzma',password:null,targetDepth:2,ancestorHashes:[]})
    const nestedPath=nested.cachePath;assert.ok(nestedPath,JSON.stringify(nested))
    const listing=await ipc('browse_archive',{filePath:nestedPath,password:null,browseId:randomUUID()});assert.equal(listing.format,'LZMA')
    const preview=await ipc('preview_archive_text',{filePath:nestedPath,entryPath:listing.entries[0].path,password:null});assert.match(preview.content,/收到 LZMA/)
    state.native.push({kind:'advertised-preview',boundedPreviewEnabled:true,nestedRealPreview:true});save()
  }
  for(const name of ['说明.txt','空文件.txt','其他属性.txt']){
    if(state.native.some(r=>r.original===name))continue
    const c=manifest.cases.find(c=>c.name===name),parts=split(path.join(fixtures,c.archive),'独立卷组')
    for(const member of [parts[0],parts[1],parts.at(-1)]){
      const browse=await ipc('browse_archive',{filePath:member,password:null,browseId:randomUUID()});assert.equal(browse.format,'LZMA');assert.equal(browse.entries[0].path,name)
      const report=await bridge('diagnoseArchive',member,null);assert.equal(report.status,'healthy',JSON.stringify(report));assert.equal(report.integrityTested,true)
      const preview=await ipc('preview_archive_text',{filePath:member,entryPath:name,password:null});if(name!=='空文件.txt')assert.match(preview.content,/收到 LZMA/)
    }
    const out=fresh('完整提取');mkdirSync(out);await ipc('extract_file',{taskId:randomUUID(),filePath:parts[1],outputPath:out,password:null,options:{...options,selected_entries:[name]}});assert.equal(sha(path.join(out,name)),c.sha256)
    state.native.push({original:name,parts,firstMiddleTail:true,publicDiagnosis:true,preview:true,fullHash:true});save()
  }
  if(!state.native.some(r=>r.kind==='image-preview')){
    const parts=split(path.join(fixtures,'图标.png.lzma'),'图片卷组'),preview=await ipc('preview_archive_image',{filePath:parts.at(-1),entryPath:'图标.png',password:null});assert.equal(hash(Buffer.from(preview.dataUrl.split(',')[1],'base64')),sha(path.join(root,'src-tauri/icons/32x32.png')))
    state.native.push({kind:'image-preview',fullImageHash:true});save()
  }
  if(!state.native.some(r=>r.kind==='creation')){
    const output=fresh('原生创建')+'.lzma',published=await ipc('compress_files',{taskId:randomUUID(),files:[state.source],outputPath:output,options:{format:'lzma',level:1,password:null,split_size:65521,preserve_paths:true,verify_after:true,delete_after:false}})
    assert.ok(published.split&&published.files.length>1);independent(published.files.map(f=>f.path),state.expected)
    const out=fresh('回解创建');mkdirSync(out);await ipc('extract_file',{taskId:randomUUID(),filePath:published.files.at(-1).path,outputPath:out,password:null,options});assert.equal(sha(path.join(out,path.basename(output,'.lzma'))),state.expected)
    state.native.push({kind:'creation',published,pythonFullHash:true,applicationFullHash:true});save()
  }
  if(!state.native.some(r=>r.kind==='damaged-rollback')){
    const bytes=readFileSync(path.join(fixtures,'大文本.txt.lzma')),bad=fresh('后段截断.txt')+'.lzma';writeFileSync(bad,bytes.subarray(0,bytes.length-6))
    const browse=await ipc('browse_archive',{filePath:bad,password:null,browseId:randomUUID()});assert.equal(browse.entries[0].sizeKnown,false)
    const report=await bridge('diagnoseArchive',bad,null);assert.equal(report.status,'damaged',JSON.stringify(report))
    const out=fresh('损坏不发布');mkdirSync(out);const result=await invoke('extract_file',{taskId:randomUUID(),filePath:bad,outputPath:out,password:null,options});assert.equal(result.ok,false);assert.match(result.error,/damaged|尾部|eof/);assert.deepEqual(readdirSync(out),[])
    state.native.push({kind:'damaged-rollback',boundedBrowseNotFullIntegrity:true,report,unpublishedOnFailure:true});save()
  }
  if(!state.ui.some(r=>r.kind==='creation')){
    await bridge('clearCompressionWorkspace');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[state.source]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-draft-row"]')
    await choose('[data-testid="compression-format-select"]','LZMA');const toggle=await waitForElement('[data-testid="compression-split-toggle"]');assert.equal(await toggle.isEnabled(),true);if(!await toggle.isSelected())await click(toggle)
    await fill('[data-testid="compression-split-size"]','1');await fill('[data-testid="compression-output-path"]',work);await fill('[data-testid="compression-filename-input"]','界面创建');await sizes('inline-config','[data-testid="lzma-volume-hint"]')
    await click('[data-testid="start-compression"]');const h=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.outputPath?.includes('界面创建')&&['completed','failed'].includes(h.status))||false,90000);assert.equal(h.status,'completed',JSON.stringify(h))
    const parts=readdirSync(work).filter(f=>/^界面创建\.lzma\.\d{3,}$/.test(f)).sort().map(f=>path.join(work,f));assert.ok(parts.length>1);independent(parts,state.expected)
    state.ui.push({kind:'creation',history:h,pythonFullHash:true});save()
  }
  if(!state.ui.some(r=>r.kind==='batch')){
    await bridge('clearCompressionWorkspace');await bridge('queueDesktopDialogSelections',[[state.source,path.join(root,'src-tauri/icons/32x32.png')]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-draft-row"]')
    const toggle=await waitForElement('[data-testid="compression-split-toggle"]');if(!await toggle.isSelected())await click(toggle);await fill('[data-testid="compression-split-size"]','1')
    for(const c of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]')))if(!await c.isSelected())await c.click()
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','LZMA');await sizes('batch-config','[role="dialog"]');await click('[data-testid="batch-config-apply"]')
    const rows=await driver.findElements(By.css('[data-testid="compression-draft-row"]'));assert.equal(rows.length,2);for(const row of rows)assert.match(await row.getText(),/LZMA/)
    await driver.wait(async()=>(await driver.findElements(By.css('[role="dialog"]'))).length===0,10000);state.ui.push({kind:'batch',twoDrafts:true,modalOnly:true});save()
  }
  const original=state.native.find(r=>r.original==='说明.txt')
  if(!state.ui.some(r=>r.kind==='preview')){
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[original.parts.at(-1)]]);await click((await driver.findElements(By.css('.browser-header-actions .browser-primary'))).length?'.browser-header-actions .browser-primary':'[data-testid="file-manager-open-archive"]');await click('[data-entry-path="说明.txt"] .preview-trigger')
    assert.match(await(await waitForElement('[data-testid="archive-text-preview"]')).getText(),/收到 LZMA/);await sizes('tail-preview','[data-testid="archive-text-preview"]');await click('.preview-close');state.ui.push({kind:'preview',tailMember:true});save()
  }
  if(!state.ui.some(r=>r.kind==='recovery')){
    await bridge('clearTasks');await click('[data-testid="nav-Decompress"]');await bridge('queueDesktopDialogSelections',[[original.parts[1]]]);await click('[data-testid="dropzone-file"]')
    const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000),withdrawn=owned(original.parts[1]),backup=owned(withdrawn+'.owned-original');renameSync(withdrawn,backup)
    let failed;try{await click('[data-testid="start-decompression-queue"]');failed=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.id===task.id&&h.status==='failed')||false,30000)}finally{if(existsSync(backup))renameSync(backup,withdrawn)}
    assert.match(failed.errorMessage,/incomplete|缺少|分卷|source-missing/);await click('[data-testid="retry-batch-failures"]')
    const out=fresh('补卷新任务');mkdirSync(out);await fill('[role="dialog"] [data-testid="retry-output"]',out);const sub=await waitForElement('[role="dialog"] [data-testid="retry-subfolder"]');if(await sub.isSelected())await click(sub)
    await sizes('retry-modal','[role="dialog"]');await click('[role="dialog"] [data-testid="create-retry-drafts"]');await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'),15000);assert.deepEqual(readdirSync(out),[])
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(.,"返回队列确认并启动")]')))
    const retry=(await bridge('extractionRecoveryTasks')).find(t=>t.retryOfTaskId===task.id);assert.equal(retry.status,'pending');await click('[data-testid="start-decompression-queue"]')
    const done=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.id===retry.id&&['completed','failed'].includes(h.status))||false,30000);assert.equal(done.status,'completed',JSON.stringify(done));assert.equal(sha(path.join(out,'说明.txt')),manifest.cases.find(c=>c.name==='说明.txt').sha256);assert.deepEqual((await bridge('taskHistory')).find(h=>h.id===failed.id),failed)
    state.ui.push({kind:'recovery',failed,completed:done,oldFailureUnchanged:true});save()
  }
  for(const target of ['lzma','zip']){
    if(state.ui.some(r=>r.kind==='conversion'&&r.target===target))continue
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]');const source=target==='lzma'?state.encrypted:state.ui.find(r=>r.kind==='conversion'&&r.target==='lzma').parts.at(-1)
    await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="conversion-choose-source"]')
    const item=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).at(-1),row=await item.findElement(By.css('[data-testid="conversion-row"]'));if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    if(target==='lzma')await fill('[data-testid="conversion-source-password"]','Long1523')
    await choose('[data-testid="conversion-format"]',target==='lzma'?'LZMA':'ZIP');const output=fresh('转换')+'.'+target
    if(target==='zip')await fill('[data-testid="conversion-password"]','Long1523')
    await fill('[data-testid="conversion-output"]',output);await fill('[data-testid="conversion-split"]','1');await click('[data-testid="conversion-start"]')
    const normalized=output.replaceAll('\\','/').toLowerCase(),h=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>[normalized,normalized+'.001'].includes(h.outputPath?.replaceAll('\\','/').toLowerCase())&&['completed','failed'].includes(h.status))||false,90000);assert.equal(h.status,'completed',JSON.stringify(h))
    const prefix=path.basename(output),parts=readdirSync(work).filter(f=>f.startsWith(prefix+'.')&&/^\d{3,}$/.test(f.slice(prefix.length+1))).sort().map(f=>path.join(work,f));assert.ok(parts.length>1)
    if(target==='lzma')independent(parts,state.expected);else{const merged=fresh('独立密码')+'.zip';writeFileSync(merged,Buffer.concat(parts.map(f=>readFileSync(f))));const out=fresh('密码ZIP回解');seven(['x','-y','-pLong1523','-o'+out,merged]);const outputs=readdirSync(out);assert.equal(outputs.length,1);assert.equal(sha(path.join(out,outputs[0])),state.expected)}
    await sizes('conversion-'+target,item);state.ui.push({kind:'conversion',target,parts,history:h,independentFullHashes:true});save()
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
  state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'lzma-1523-results.json'),JSON.stringify(state,null,2))
}

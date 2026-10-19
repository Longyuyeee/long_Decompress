import assert from 'node:assert/strict'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,existsSync,renameSync} from 'node:fs'
import {createHash,randomUUID,randomBytes} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'
import {makeLzipFixtures,lzipTools,linuxPath} from './prepare-lzip-fixtures.mjs'

export async function runLzip1520Gate({driver,bridge,waitForElement,artifactDirectory,application,root,bundledSevenZip}) {
  const sha=b=>createHash('sha256').update(b).digest('hex'),hash=p=>sha(readFileSync(p))
  const base=path.join(root,'test-results/lzip-1520');mkdirSync(base,{recursive:true})
  const progress=path.join(base,'desktop-progress.json'),binarySha256=hash(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{work:path.join(base,'work-'+randomUUID()),native:[],ui:[],screenshots:[],layouts:[]}
  mkdirSync(state.work,{recursive:true});const work=state.work,tools=lzipTools(root)
  state.identities??=[];if(!state.identities.some(i=>i.binarySha256===binarySha256))state.identities.push({binarySha256,source:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),startedAt:new Date().toISOString()})
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const fresh=n=>owned(path.join(work,n+'-'+randomUUID()))
  const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(r=>({...r,name:e.name+'/'+r.name})):[{name:e.name,sha256:hash(path.join(p,e.name))}]).sort((a,b)=>a.name.localeCompare(b.name))
  const sameTree=(actual,source)=>{assert.deepEqual(tree(actual),tree(source));assert.ok(existsSync(path.join(actual,'中文目录/空目录')))}
  const seven=(args,cwd)=>execFileSync(bundledSevenZip,args,{cwd,windowsHide:true,encoding:'utf8',timeout:120000})
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,file_filter:null,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],selected_entries:[]}
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
  const click=async selector=>{
    for(const t of await driver.findElements(By.css('.toast-item button'))){try{if(await t.isDisplayed())await t.click()}catch{}}
    const e=typeof selector==='string'?await waitForElement(selector):selector
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&r.height>0&&(h===e||e.contains(h))},e),15000);await e.click()
  }
  const fill=async(selector,value)=>{const e=await waitForElement(selector);await driver.wait(async()=>await e.isDisplayed()&&await e.isEnabled(),10000);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const choose=async(selector,label)=>{await click(selector);for(const e of await driver.findElements(By.css('[role="option"]'))){const strong=await e.findElements(By.css('strong'));if((await(strong[0]||e).getAttribute('textContent')).trim().startsWith(label)){await driver.executeScript(e=>e.scrollIntoView({block:'nearest',behavior:'instant'}),e);await e.click();return}}assert.fail('Option missing: '+label)}
  const sizes=async(name,anchor)=>{for(const[width,height]of[[1100,700],[760,520]]){
    await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement(anchor))
    await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
    const geometry=await driver.executeScript(()=>{const d=document.querySelector('[data-testid="compression-draft-details"]'),cols=d?[...d.children].map(c=>c.getBoundingClientRect().width):[];return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,columns:cols,detailOverflow:d?d.scrollWidth-d.clientWidth:0}})
    assert.ok(geometry.overflow<=1&&geometry.detailOverflow<=1,JSON.stringify(geometry));if(name==='inline-config'&&width===1100)assert.ok(Math.abs(geometry.columns[0]-geometry.columns[1])<=1,JSON.stringify(geometry));state.layouts.push({name,...geometry,binarySha256})
    const png='lzip1520-'+name+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,png),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots.push(png);save()
  }await driver.manage().window().setRect({width:1100,height:700})}
  const split=(file,label)=>{const folder=fresh(label);mkdirSync(folder);const bytes=readFileSync(file),parts=[],size=Math.max(1,Math.floor(bytes.length/3));for(let off=0;off<bytes.length;off+=size){const p=path.join(folder,path.basename(file)+'.'+String(parts.length+1).padStart(3,'0'));writeFileSync(p,bytes.subarray(off,off+size));parts.push(p)}return parts}
  const join=parts=>{const file=fresh('独立合并')+'.lz';writeFileSync(file,Buffer.concat(parts.map(p=>readFileSync(p))));tools.lzip(['-t',linuxPath(file)]);return file}
  const independent=(parts,source,isTar)=>{const file=join(parts);if(!isTar){assert.equal(sha(tools.lzip(['-d','-c',linuxPath(file)])),hash(source));return}
    const tar=fresh('独立回解')+'.tar',out=fresh('独立目录');writeFileSync(tar,tools.lzip(['-d','-c',linuxPath(file)]));mkdirSync(out);tools.run('/usr/bin/tar',['-xf',linuxPath(tar),'-C',linuxPath(out)]);sameTree(out,source)
  }
  if(!state.source){
    const f=makeLzipFixtures(path.join(work,'GNU原件'),root);state.small={source:f.source,single:f.single,tar:f.tarLzip,alias:f.alias,multi:f.multi,manifest:f.manifest}
    const source=path.join(work,'创建来源');mkdirSync(path.join(source,'中文目录/空目录'),{recursive:true})
    writeFileSync(path.join(source,'数据.bin'),randomBytes(2*1024*1024+37));writeFileSync(path.join(source,'说明.txt'),'用户可以从任意一卷浏览和预览，再选择解压。\n');writeFileSync(path.join(source,'空文件.txt'),'')
    writeFileSync(path.join(source,'中文目录/末尾.txt'),'请保留末尾内容和空目录。\n');state.source=source;state.expected=tree(source)
    const archive=path.join(work,'密码来源.zip');seven(['a','-tzip','-mem=AES256','-pLong1520',archive,'.\\*'],source);state.encrypted=archive;save()
  }
  for(const key of ['single','tar','alias','multi']){
    if(state.native.some(r=>r.original===key))continue
    const file=state.small[key],parts=split(file,'GNU卷组'),tar=key==='tar'||key==='alias',expected=await ipc('browse_archive',{filePath:file,password:null,browseId:randomUUID()})
    for(const member of [parts[0],parts[1],parts.at(-1)]){
      const browse=await ipc('browse_archive',{filePath:member,password:null,browseId:randomUUID()});assert.equal(browse.format,expected.format);assert.equal(browse.totalUncompressedSize,expected.totalUncompressedSize)
      const report=await bridge('diagnoseArchive',member,null);assert.equal(report.status,'healthy',JSON.stringify(report));assert.equal(report.integrityTested,true)
      if(tar){const preview=await ipc('preview_archive_text',{filePath:member,entryPath:'说明.txt',password:null});assert.match(preview.content,/收到 LZIP/)}
    }
    const out=fresh('GNU完整提取');mkdirSync(out);await ipc('extract_file',{taskId:randomUUID(),filePath:parts[1],outputPath:out,password:null,options})
    if(tar)sameTree(out,state.small.source);else assert.equal(hash(path.join(out,path.basename(file,'.lz'))),sha(tools.lzip(['-d','-c',linuxPath(file)])))
    if(tar){const selected=fresh('所选提取');mkdirSync(selected);await ipc('extract_file',{taskId:randomUUID(),filePath:parts.at(-1),outputPath:selected,password:null,options:{...options,selected_entries:['说明.txt']}});assert.deepEqual(tree(selected),[{name:'说明.txt',sha256:hash(path.join(state.small.source,'说明.txt'))}])}
    state.native.push({original:key,parts,completeHashes:true,firstMiddleTail:true,browse:true,integrity:true,selected:tar,binarySha256});save()
  }
  for(const format of ['lz','tar.lz']){
    if(state.native.some(r=>r.created===format))continue
    const output=fresh('应用创建')+'.'+format,files=format==='lz'?[path.join(state.source,'数据.bin')]:readdirSync(state.source).map(f=>path.join(state.source,f))
    const published=await ipc('compress_files',{taskId:randomUUID(),files,outputPath:output,options:{format,level:1,password:null,split_size:65521,preserve_paths:true,verify_after:true,delete_after:false}})
    assert.ok(published.split&&published.files.length>1,JSON.stringify(published));independent(published.files.map(f=>f.path),format==='lz'?files[0]:state.source,format!=='lz')
    const out=fresh('应用分卷完整解压');mkdirSync(out);await ipc('extract_file',{taskId:randomUUID(),filePath:published.files.at(-1).path,outputPath:out,password:null,options})
    if(format==='lz')assert.equal(hash(path.join(out,path.basename(output,'.lz'))),hash(files[0]));else sameTree(out,state.source)
    state.native.push({created:format,published,fullHashes:true,gnuReadback:true,binarySha256});save()
  }
  if(!state.ui.some(r=>r.kind==='creation')){
    await bridge('clearCompressionWorkspace');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[path.join(state.source,'数据.bin')]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-draft-row"]')
    await choose('[data-testid="compression-format-select"]','LZIP');const toggle=await waitForElement('[data-testid="compression-split-toggle"]');assert.equal(await toggle.isEnabled(),true);if(!await toggle.isSelected())await click(toggle)
    await fill('[data-testid="compression-split-size"]','1');await fill('[data-testid="compression-output-path"]',work);await fill('[data-testid="compression-filename-input"]','界面创建')
    assert.equal(await(await waitForElement('[data-testid="compression-password-input"]')).isEnabled(),false);await sizes('inline-config','[data-testid="lzip-volume-hint"]')
    await click('[data-testid="start-compression"]');const h=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.outputPath?.includes('界面创建')&&['completed','failed'].includes(h.status))||false,90000);assert.equal(h.status,'completed',JSON.stringify(h))
    const parts=readdirSync(work).filter(f=>/^界面创建\.lz\.\d{3,}$/.test(f)).sort().map(f=>path.join(work,f));assert.ok(parts.length>1);independent(parts,path.join(state.source,'数据.bin'),false)
    state.ui.push({kind:'creation',history:h,binarySha256,independentFullHash:true});save()
  }
  if(!state.ui.some(r=>r.kind==='batch')){
    await bridge('clearCompressionWorkspace');await bridge('queueDesktopDialogSelections',[[path.join(state.source,'说明.txt'),path.join(state.source,'空文件.txt')]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-draft-row"]')
    const toggle=await waitForElement('[data-testid="compression-split-toggle"]');if(!await toggle.isSelected())await click(toggle)
    await fill('[data-testid="compression-split-size"]','1')
    for(const c of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]')))if(!await c.isSelected())await c.click()
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','TAR.LZIP');await sizes('batch-config','[role="dialog"]');await click('[data-testid="batch-config-apply"]')
    const rows=await driver.findElements(By.css('[data-testid="compression-draft-row"]'));assert.equal(rows.length,2)
    for(const row of rows)assert.match(await row.getText(),/TAR\.LZ/i)
    state.ui.push({kind:'batch',twoDrafts:true,modalOnly:true,binarySha256});save()
  }
  const original=state.native.find(r=>r.original==='tar')
  if(!state.ui.some(r=>r.kind==='preview')){
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[original.parts.at(-1)]]);await click('[data-testid="file-manager-open-archive"]');await click('[data-entry-path="说明.txt"] .preview-trigger')
    assert.match(await(await waitForElement('[data-testid="archive-text-preview"]')).getText(),/收到 LZIP/);await sizes('tail-preview','[data-testid="archive-text-preview"]');await click('.preview-close')
    state.ui.push({kind:'preview',tailMember:true,binarySha256});save()
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
    const done=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.id===retry.id&&['completed','failed'].includes(h.status))||false,30000);assert.equal(done.status,'completed',JSON.stringify(done));sameTree(out,state.small.source);assert.deepEqual((await bridge('taskHistory')).find(h=>h.id===failed.id),failed)
    state.ui.push({kind:'recovery',failed,completed:done,oldFailureUnchanged:true,binarySha256});save()
  }
  for(const target of ['tar.lz','zip']){
    if(state.ui.some(r=>r.kind==='conversion'&&r.target===target))continue
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]');const source=target==='tar.lz'?state.encrypted:state.ui.find(r=>r.kind==='conversion'&&r.target==='tar.lz').parts.at(-1)
    await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="conversion-choose-source"]')
    const item=(await driver.findElements(By.css('[data-testid="conversion-item"]'))).at(-1),row=await item.findElement(By.css('[data-testid="conversion-row"]'));if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    if(target==='tar.lz')await fill('[data-testid="conversion-source-password"]','Long1520')
    await choose('[data-testid="conversion-format"]',target==='tar.lz'?'TAR.LZIP':'ZIP');const output=fresh('转换')+'.'+target
    if(target==='zip')await fill('[data-testid="conversion-password"]','Long1520')
    await fill('[data-testid="conversion-output"]',output);await fill('[data-testid="conversion-split"]','1');await click('[data-testid="conversion-start"]')
    const normalized=output.replaceAll('\\','/').toLowerCase(),h=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>[normalized,normalized+'.001'].includes(h.outputPath?.replaceAll('\\','/').toLowerCase())&&['completed','failed'].includes(h.status))||false,90000);assert.equal(h.status,'completed',JSON.stringify(h))
    const prefix=path.basename(output),parts=readdirSync(work).filter(f=>f.startsWith(prefix+'.')&&/^\d{3,}$/.test(f.slice(prefix.length+1))).sort().map(f=>path.join(work,f));assert.ok(parts.length>1)
    if(target==='tar.lz')independent(parts,state.source,true);else{const merged=fresh('独立密码')+'.zip';writeFileSync(merged,Buffer.concat(parts.map(f=>readFileSync(f))));const out=fresh('密码ZIP回解');seven(['x','-y','-pLong1520','-o'+out,merged]);sameTree(out,state.source)}
    await sizes('conversion-'+target.replace('.','-'),'[data-testid="conversion-item"]');state.ui.push({kind:'conversion',target,parts,history:h,independentFullHashes:true,binarySha256});save()
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
  state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'lzip-1520-results.json'),JSON.stringify(state,null,2))
}

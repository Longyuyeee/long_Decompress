import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By } from 'selenium-webdriver'

export async function runAlzBounded168Gate({driver,bridge,application,root,bundledSevenZip}) {
  const base=path.join(root,'test-results/alz-bounded-168'), receipt=path.join(base,'desktop.json')
  const sha=file=>createHash('sha256').update(readFileSync(file)).digest('hex')
  const helper=path.join(path.dirname(application),'resources/alz-engine/long-alz.exe')
  assert.equal(sha(helper),sha(path.join(root,'src-tauri/resources/alz-engine/long-alz.exe')))
  const report=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.6.8',applicationSha256:sha(application),helperSha256:sha(helper),work:path.join(base,'desktop-'+randomUUID()),checks:[],layouts:[]}
  assert.equal(report.applicationSha256,sha(application));assert.equal(report.helperSha256,sha(helper))
  mkdirSync(report.work,{recursive:true})
  const save=()=>writeFileSync(receipt,JSON.stringify(report,null,2)+'\n')
  const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;const detail=await fn();report.checks.push({name,...detail});save();console.log('[alz-bounded-168]',name)}
  const manifest=JSON.parse(readFileSync(path.join(base,'fixtures/manifest.json'),'utf8'))
  const large=manifest.cases.find(c=>c.name==='large'),key=manifest.cases.find(c=>c.name==='key-volumes')
  const source=c=>path.join(root,c.archive)
  const invoke=(cmd,args)=>driver.executeAsyncScript(`const [cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
  const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,create_subdirectory:false,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[],zip_password_encoding:null}
  const extraction=(c,output,selected=[],password=c.password)=>({taskId:randomUUID(),filePath:source(c),outputPath:output,password,options:{...options,selected_entries:selected}})
  const hashes=folder=>!existsSync(folder)?[]:readdirSync(folder,{withFileTypes:true}).flatMap(e=>e.isDirectory()?hashes(path.join(folder,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,sha256:sha(path.join(folder,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
  const expected=(c,selected=[])=>c.expected.filter(e=>!selected.length||selected.includes(e.path)).map(({path,sha256})=>({path,sha256})).sort((a,b)=>a.path.localeCompare(b.path))
  const find=selector=>driver.wait(async()=>{for(const e of await driver.findElements(By.css(selector)))if(await e.isDisplayed())return e;return false},15000,selector)
  const click=async selector=>{const e=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),e);await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000);await e.click()}

  await step('large archive stays in dual-pane browser with correct entries and size',async()=>{
    await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[source(large)]]);await click('[data-testid="file-manager-open-archive"]')
    await find('[data-file-pane="left"] [data-entry-path="note.txt"]')
    await find('[data-file-pane="left"] [data-entry-path="payload.bin"]')
    assert.equal((await driver.findElements(By.css('.file-pane'))).length,2)
    const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width)}))
    assert.ok(layout.overflow<=1);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2)
    writeFileSync(path.join(base,'large-browser.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name:'large-browser',...layout})
    return {controlled:true,compressedBytes:large.compressedBytes,entries:['payload.bin','note.txt'],layout}
  })
  await step('large selected extraction publishes exact payload without unrelated note',async()=>{
    const out=path.join(report.work,'large-selected');await ipc('extract_file',extraction(large,out,['payload.bin']))
    assert.deepEqual(hashes(out),expected(large,['payload.bin']));return {hashes:hashes(out)}
  })
  await step('password and middle native member retain cipher state; plain selection needs no key',async()=>{
    const bad=path.join(report.work,'wrong-password'),failure=await invoke('extract_file',extraction(key,bad,[],'wrong key'))
    assert.equal(failure.ok,false);assert.deepEqual(hashes(bad),[])
    const out=path.join(report.work,'key-volumes');await ipc('extract_file',extraction(key,out));assert.deepEqual(hashes(out),expected(key))
    const selected=path.join(report.work,'plain-note');await ipc('extract_file',extraction(key,selected,['note.txt'],null));assert.deepEqual(hashes(selected),expected(key,['note.txt']))
    return {controlled:true,wrongPassword:failure.error,hashes:hashes(out),plainSelection:true}
  })
  await step('large ALZ converts to verified ZIP and independently extracts exact files',async()=>{
    const zip=path.join(report.work,'large-converted.zip')
    await ipc('convert_archive',{taskId:randomUUID(),sourcePath:source(large),sourcePassword:null,sourceZipPasswordEncoding:null,outputPath:zip,options:{format:'zip',level:0,password:null,split_size:null,verify_after:true,preserve_paths:true,delete_after:false}})
    const out=path.join(report.work,'converted-independent'),r=spawnSync(bundledSevenZip,['x','-y','-o'+out,zip],{windowsHide:true,timeout:60000})
    assert.ifError(r.error);assert.equal(r.status,0,String(r.stdout)+String(r.stderr));assert.deepEqual(hashes(out),expected(large));return {hashes:hashes(out)}
  })
  await step('active large extraction pauses, resumes, cancels cleanly and new selection succeeds',async()=>{
    const out=path.join(report.work,'cancelled'),args=extraction(large,out),token=randomUUID()
    await driver.executeScript(`const [args,token]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));window.__alz168??={};window.__alz168[token]={pending:true};const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{window.__alz168[token]={ok:true,value};clean()};window['_'+ids[1]]=error=>{window.__alz168[token]={ok:false,error:String(error)};clean()};window.__TAURI_IPC__({cmd:'extract_file',...args,callback:ids[0],error:ids[1]});`,args,token)
    await driver.sleep(1200);await ipc('pause_archive_task',{taskId:args.taskId});await driver.sleep(350)
    assert.equal(await driver.executeScript(t=>window.__alz168[t].pending,token),true)
    await ipc('resume_archive_task',{taskId:args.taskId});await driver.sleep(350);await ipc('cancel_compression',{taskId:args.taskId})
    const result=await driver.wait(()=>driver.executeScript(t=>window.__alz168[t].pending?null:window.__alz168[t],token),15000)
    assert.equal(result.ok,false);assert.match(result.error,/cancel|取消/i);assert.deepEqual(hashes(out),[])
    const retry=path.join(report.work,'new-selection');await ipc('extract_file',extraction(large,retry,['note.txt']));assert.deepEqual(hashes(retry),expected(large,['note.txt']))
    return {cancelled:result.error,noPublishedPartialFiles:true,newTaskHashes:hashes(retry)}
  })
  report.passed=true;save()
}

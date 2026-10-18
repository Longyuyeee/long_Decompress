import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { readFileSync, writeFileSync, mkdirSync, readdirSync, existsSync, copyFileSync, renameSync, statSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runAlz152Gate({driver,bridge,failure,waitForElement,fixtureDirectory,artifactDirectory,application,root,bundledSevenZip}) {
  const fixtures=path.join(root,'tests/fixtures/archives/alz-152'),cn=path.join(fixtures,'original-alzip')
  const manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8')),cnManifest=JSON.parse(readFileSync(path.join(cn,'cp936-provenance.json'),'utf8'))
  const work=path.join(fixtureDirectory,'ALZ真实用户场景');mkdirSync(work)
  const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
  const tree=dir=>!existsSync(dir)?[]:readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(dir,e.name)).map(v=>({...v,path:e.name+'/'+v.path})):[{path:e.name,sha256:sha(path.join(dir,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
  const expected=c=>c.map(e=>({path:e.path,sha256:e.sha256})).sort((a,b)=>a.path.localeCompare(b.path))
  const cases=[],screenshots=[],layouts=[]
  const invoke=(cmd,args)=>driver.executeAsyncScript(`const [cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
  const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,create_subdirectory:false,file_filter:null,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[]}
  const extract=(source,output,password,extra={})=>ipc('extract_file',{taskId:'alz-read-'+crypto.randomUUID(),filePath:source,outputPath:output,password,options:{...options,...extra}})
  const diagnose=(source,password,encoding)=>ipc('diagnose_archive',{diagnosticId:'alz-check-'+crypto.randomUUID(),filePath:source,password,zipPasswordEncoding:encoding})
  const click=async selector=>{const el=typeof selector==='string'?await waitForElement(selector):selector;await driver.wait(()=>driver.executeScript(el=>{el.scrollIntoView({block:'center',behavior:'instant'});const r=el.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return r.width>0&&r.height>0&&!el.disabled&&(h===el||el.contains(h))},el),10000,'User control must actually be clickable: '+selector);await el.click()}
  const fill=async(selector,value)=>{const el=await waitForElement(selector);await driver.wait(()=>el.isEnabled(),10000);await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value)}
  const select=async(selector,label)=>{await click(selector);await waitForElement('[role="option"]');await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.getAttribute('textContent')).trim()===label){await click(el);await driver.wait(async()=>(await(await waitForElement(selector)).getText()).trim()===label,10000);return}assert.fail('Missing '+label)}
  const sizes=async(name,anchor)=>{
    for(const [width,height] of [[1100,700],[760,520]]) {
      await driver.manage().window().setRect({width,height});await driver.wait(()=>driver.executeScript(w=>Math.abs(innerWidth-w)<=2,width),10000)
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement(anchor));await driver.sleep(250)
      if(name==='alz-read-help') {
        const trigger=await waitForElement('[aria-label="ALZ 读取范围"]')
        if(await trigger.getAttribute('aria-expanded')!=='true')await click(trigger)
        await driver.wait(async()=>(await driver.findElements(By.css('[role="note"][aria-label="ALZ 读取范围"]'))).length===1,10000)
      }
      const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('.browser-page,[role="note"],[role="dialog"]')].map(el=>({overflow:el.scrollWidth-el.clientWidth,left:el.getBoundingClientRect().left,right:el.getBoundingClientRect().right}))}))
      assert.ok(layout.overflow<=1,JSON.stringify(layout));layout.panels.forEach(p=>assert.ok(p.overflow<=2&&p.left>=-1&&p.right<=width+1,JSON.stringify(layout)))
      layouts.push({name,...layout});const file=`${name}-${width}x${height}.png`;screenshots.push(file);writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'))
    }
  }
  // The native application, original Korean fixtures, independently fixed hashes.
  for(const c of manifest.cases) {
    console.log('[alz-152] original',c.name)
    const source=path.join(fixtures,c.archive),before=sha(source),key=c.password||null
    const directory=await ipc('browse_archive',{filePath:source,password:key});assert.equal(directory.format,'ALZ')
    await ipc('test_archive_integrity',{filePath:source,password:key})
    const report=await diagnose(source,key);assert.equal(report.status,'healthy',JSON.stringify(report));assert.equal(report.integrityTested,true)
    const out=path.join(work,c.name+'-完整解压');await extract(source,out,key);assert.deepEqual(tree(out),expected(c.expected))
    assert.equal(sha(source),before);cases.push({original:c.name,wholeNativeExtraction:true,integrity:true,diagnostic:true,payloadHashes:true})
  }
  for(const c of cnManifest.cases) {
    const source=path.join(cn,c.archive),out=path.join(work,c.archive+'-中文'),encoding='cp936'
    const report=await diagnose(source,c.password,encoding);assert.equal(report.status,'healthy')
    await extract(source,out,c.password,{zip_password_encoding:encoding});assert.deepEqual(tree(out),expected(cnManifest.sourceFiles));assert.ok(existsSync(path.join(out,'空目录')))
    const converted=path.join(work,c.archive+'.zip')
    await ipc('convert_archive',{taskId:'alz-convert-'+crypto.randomUUID(),sourcePath:source,sourcePassword:c.password,sourceZipPasswordEncoding:encoding,outputPath:converted,options:{format:'zip',level:6,password:null,split_size:null,verify_after:true,preserve_paths:true,delete_after:false}})
    const r=spawnSync(bundledSevenZip,['x','-y','-o'+converted+'-独立回解',converted],{windowsHide:true,encoding:'utf8',timeout:30000});assert.equal(r.status,0,r.stdout+r.stderr);assert.deepEqual(tree(converted+'-独立回解'),expected(cnManifest.sourceFiles))
    cases.push({original:c.archive,chineseNameAndPassword:true,nativeVolumes:c.split,convertedZipIndependentlyDecoded:true,emptyDirectory:true})
  }
  // Real pause/resume/kill of the decoder, not an E2E simulated task.
  const controlDir=path.join(root,'test-results/alz-152-control'),control=JSON.parse(readFileSync(path.join(controlDir,'manifest.json'),'utf8')),large=path.join(controlDir,'control.alz')
  assert.equal(sha(large),control.archiveSha256)
  for(const alias of ['¥','‾']) {
    const r=await invoke('verify_archive_password',{taskId:'alz-alias-'+crypto.randomUUID(),filePath:large,password:alias,zipPasswordEncoding:'cp932'})
    assert.equal(r.ok,false);assert.match(r.error,/\[alz-password-encoding\]/);assert.match(r.error,/无损/)
  }
  cases.push({actualCp932AliasesRejectedBeforePasswordClaim:true})
  const controlOut=path.join(work,'取消不能发布'),taskId='alz-control-'+Date.now()
  await driver.executeScript(`const [taskId,filePath,outputPath,password,options]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));window.__alzControl=null;const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=()=>{clean();window.__alzControl={ok:true}};window['_'+ids[1]]=error=>{clean();window.__alzControl={ok:false,error:String(error)}};window.__TAURI_IPC__({cmd:'extract_file',taskId,filePath,outputPath,password,options,callback:ids[0],error:ids[1]});`,taskId,large,controlOut,control.password,{...options,delete_after:true})
  const children=()=>{const r=spawnSync('powershell.exe',['-NoProfile','-Command',"Get-Process -Name long-alz -ErrorAction SilentlyContinue | Select-Object Id,CPU | ConvertTo-Json -Compress"],{windowsHide:true,encoding:'utf8',timeout:10000});return r.stdout.trim()?JSON.parse(r.stdout):null}
  await driver.wait(()=>children(),15000);await ipc('pause_archive_task',{taskId});await driver.sleep(200)
  const beforePause=children();assert.ok(beforePause);await driver.sleep(400);const afterPause=children();assert.ok(afterPause)
  const processArray=p=>Array.isArray(p)?p:[p]
  for(const p of processArray(beforePause)){const after=processArray(afterPause).find(q=>q.Id===p.Id);assert.ok(after);assert.equal(after.CPU,p.CPU)}
  await ipc('resume_archive_task',{taskId});await driver.sleep(100);await ipc('pause_archive_task',{taskId});await ipc('cancel_compression',{taskId})
  const stopped=await driver.wait(()=>driver.executeScript(()=>window.__alzControl),30000);assert.equal(stopped.ok,false);assert.match(stopped.error,/cancel|取消/i);assert.equal(sha(large),control.archiveSha256);assert.deepEqual(tree(controlOut),[]);assert.ok(!readdirSync(work).some(n=>n.startsWith('.long-extract')))
  cases.push({genuineControlArchive:control,actualPauseNoCpuProgress:true,resume:true,cancelWhilePaused:true,noPublishedOutput:true,sourceRetainedWithDeleteAfter:true})
  // User opens a middle native member from the real file manager.
  await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',cn);await(await waitForElement('[data-testid="file-manager-path-input-left"]')).sendKeys(Key.ENTER)
  const middle=path.join(cn,'native-cn-key-volumes.a01')
  const row=await driver.wait(async()=>{for(const row of await driver.findElements(By.css('.file-row')))if(await row.getAttribute('data-path')===middle)return row;return false},10000)
  assert.equal((await row.findElements(By.css('.pi-box'))).length,1);await driver.actions().doubleClick(row).perform()
  await waitForElement('[data-testid="browser-zip-password-encoding"]');await select('[data-testid="browser-zip-password-encoding"]','简体中文（CP936）');await fill('.browser-toolbar input[type="password"]','中文密码123!');await click('.browser-toolbar button[title="重新读取"]')
  await waitForElement('[data-testid="alz-read-view"]');await waitForElement('[data-entry-path="说明 文件.txt"]');await sizes('alz-native-member-browser','.browser-summary')
  await click('[aria-label="ALZ 读取范围"]');await waitForElement('[role="note"]');await sizes('alz-read-help','.browser-summary');await driver.actions().sendKeys(Key.ESCAPE).perform()
  await click('.browser-table-head .browser-checkbox');await click('[data-entry-path="说明 文件.txt"] .browser-checkbox')
  const selectedOutput=path.join(work,'用户只取中文说明');mkdirSync(selectedOutput);await bridge('queueDesktopDialogSelections',[selectedOutput]);await click('.output-target');await click('footer .browser-primary')
  await driver.wait(()=>existsSync(path.join(selectedOutput,'说明 文件.txt')),30000);assert.deepEqual(tree(selectedOutput),expected(cnManifest.sourceFiles.filter(e=>e.path==='说明 文件.txt')))
  cases.push({actualFileManagerMiddleMember:true,actualExplicitCodepage:true,actualSelectedExtraction:true})
  const saved=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.status==='completed'&&r.sourcePaths.some(p=>p.replaceAll('\\','/')===middle.replaceAll('\\','/')))||false,15000)
  assert.equal(saved.executionConfig.zipPasswordEncoding,'cp936');assert.equal(saved.executionConfig.password,undefined)
  await click('[data-testid="nav-History"]')
  const savedRow=await driver.wait(async()=>{for(const el of await driver.findElements(By.css('[data-testid="history-record-row"]')))if((await el.getText()).includes('native-cn-key-volumes'))return el;return false},10000)
  await click(await savedRow.findElement(By.css('[data-testid="history-retry"]')))
  await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement('[data-testid="history-draft-zip-password-encoding"]'))
  await driver.wait(async()=>/CP936/.test(await(await waitForElement('[data-testid="history-draft-zip-password-encoding"]')).getText()),10000,'Wait for the actual restored codepage in the scrolled history dialog')
  assert.match(await(await waitForElement('[data-testid="history-draft-zip-password-encoding"]')).getText(),/CP936/)
  assert.match(await(await waitForElement('[data-testid="history-extraction-draft"]')).getText(),/ALZ 文件名与密码编码/)
  assert.equal(await(await waitForElement('[data-testid="history-draft-password"]')).getAttribute('value'),'')
  const historyOutput=path.join(work,'历史再次解压中文密码包');mkdirSync(historyOutput)
  await fill('[data-testid="history-draft-output"]',historyOutput);await fill('[data-testid="history-draft-password"]','中文密码123!')
  const historySubfolder=await waitForElement('[data-testid="history-draft-subfolder"]');if(await historySubfolder.isSelected())await click(historySubfolder)
  await sizes('alz-history-codepage','[data-testid="history-draft-zip-password-encoding"]');await click('[data-testid="history-draft-create"]')
  await click('[data-testid="history-extraction-draft"] a');await click('[data-testid="start-decompression-queue"]')
  const rerun=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.retryOfTaskId===saved.id&&['completed','failed'].includes(r.status))||false,30000)
  assert.equal(rerun.status,'completed',JSON.stringify(rerun));assert.equal(rerun.executionConfig.zipPasswordEncoding,'cp936');assert.deepEqual(tree(historyOutput),expected(cnManifest.sourceFiles));assert.deepEqual((await bridge('taskHistory')).find(r=>r.id===saved.id),saved)
  cases.push({actualHistoryRerun:true,cp936Restored:true,passwordNotPersisted:true,actualWholeExtractionAfterPartialHistory:true,oldRecordUnchanged:true})
  // Failed task -> supplement exactly the missing original member -> new draft.
  await bridge('clearTasks');const brokenDir=path.join(work,'朋友发来的分卷');mkdirSync(brokenDir)
  for(const n of ['split_store.alz','split_store.a00','split_store.a01'])copyFileSync(path.join(fixtures,n),path.join(brokenDir,n))
  const absent=path.join(brokenDir,'split_store.a00');renameSync(absent,absent+'.saved')
  const retryOut=path.join(work,'补齐原卷后的输出');mkdirSync(retryOut)
  const detail=await failure('extractArchive',path.join(brokenDir,'split_store.a01'),retryOut,null);assert.match(detail,/split_store\.a00/);assert.match(detail,/不必反复尝试密码/);assert.deepEqual(tree(retryOut),[])
  const [failed]=await bridge('extractionRecoveryTasks');assert.equal(failed.status,'failed');renameSync(absent+'.saved',absent)
  await click('[data-testid="nav-Decompress"]');await click(`[data-task-id="${failed.id}"]`);await fill('[data-testid="retry-output"]',retryOut)
  const subfolder=await waitForElement('[data-testid="retry-subfolder"]');if(await subfolder.isSelected())await click(subfolder)
  await sizes('alz-missing-volume-retry','[data-testid="create-retry-drafts"]');await click('[data-testid="create-retry-drafts"]')
  const queued=await driver.wait(async()=>(await bridge('extractionRecoveryTasks')).find(t=>t.status==='pending')||false,15000);await click('[data-testid="start-decompression-queue"]');await driver.wait(async()=>await bridge('taskStatus',queued.id)==='completed',30000)
  assert.deepEqual(tree(retryOut),expected(manifest.cases.find(c=>c.name==='split_store').expected));assert.equal((await bridge('extractionRecoveryTasks')).find(t=>t.id===failed.id).status,'failed')
  cases.push({actualMissingMiddleFinalCause:true,originalMemberRestored:true,actualNewRetryTask:true,oldFailureRetained:true})
  // Importing multiple native members must not duplicate one archive.
  await bridge('clearTasks');await bridge('queueDesktopDialogSelections',[[middle,path.join(cn,'native-cn-key-volumes.alz'),path.join(cn,'native-cn-key-volumes.a00')]])
  await click('[data-testid="dropzone-file"]')
  await driver.wait(async()=>(await bridge('extractionRecoveryTasks')).length>0,10000);assert.equal((await bridge('extractionRecoveryTasks')).length,1)
  const pending=(await bridge('extractionRecoveryTasks'))[0];await click(`[data-task-id="${pending.id}"]`);await select('[data-testid="task-zip-password-encoding"]','简体中文（CP936）')
  await sizes('alz-pending-task-encoding','[data-testid="task-zip-password-encoding"]');cases.push({actualPickerMultiMemberDedup:true})
  await click(`[data-task-id="${pending.id}"] [data-testid="task-select"]`)
  await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await sizes('alz-batch-task-encoding','[data-testid="batch-zip-password-encoding"]');await click('[data-testid="batch-config-cancel"]')
  console.log('[alz-152] actual conversion controls')
  await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
  await bridge('queueDesktopDialogSelections',[[path.join(cn,'native-cn-key.alz')]]);await click('[data-testid="conversion-choose-source"]');await waitForElement('[data-testid="conversion-item"]')
  if(!(await driver.findElements(By.css('[data-testid="conversion-details"]'))).length)await click('[data-testid="conversion-row"]')
  await select('[data-testid="conversion-format"]','ZIP')
  await click('[data-testid="conversion-batch-config"]');await sizes('alz-batch-conversion-encoding','[data-testid="conversion-batch-source-encoding"]');await driver.actions().sendKeys(Key.ESCAPE).perform()
  await select('[data-testid="conversion-source-zip-password-encoding"]','简体中文（CP936）');await fill('[data-testid="conversion-source-password"]','中文密码123!')
  const uiConverted=path.join(work,'用户转成标准ZIP.zip');await fill('[data-testid="conversion-output"]',uiConverted)
  await sizes('alz-conversion-source-codepage','[data-testid="conversion-source-zip-password-encoding"]');await click('[data-testid="conversion-start"]');await driver.wait(()=>existsSync(uiConverted),30000);await waitForElement('[data-testid="conversion-result"]')
  const readback=spawnSync(bundledSevenZip,['x','-y','-o'+uiConverted+'-独立回解',uiConverted],{windowsHide:true,encoding:'utf8',timeout:30000});assert.equal(readback.status,0);assert.deepEqual(tree(uiConverted+'-独立回解'),expected(cnManifest.sourceFiles))
  cases.push({actualConversionControls:true,explicitCp936Source:true,independentZipReadback:true});await driver.actions().sendKeys(Key.ESCAPE).perform()
  console.log('[alz-152] actual diagnosis controls')
  await click('[data-testid="nav-FileIntegrity"]');await click('[data-testid="archive-diagnostic-mode"]');await bridge('queueDesktopDialogSelections',[[path.join(cn,'native-cn-key.alz')]]);await click('.archive-select')
  await select('[data-testid="diagnosis-zip-password-encoding"]','简体中文（CP936）');await fill('.archive-password','中文密码123!');await click('[data-testid="start-archive-diagnosis"]')
  await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement('[data-testid="diagnostic-report"]'))
  await driver.wait(async()=>/正常/.test(await(await waitForElement('[data-testid="diagnostic-report"]')).getText()),15000)
  assert.equal((await driver.findElements(By.css('[data-testid="start-archive-repair"]'))).length,0)
  await sizes('alz-integrity-report','[data-testid="diagnostic-report"]');cases.push({actualDiagnosticControls:true,noFakeAlzRepair:true})
  await bridge('queueDesktopDialogSelections',[[path.join(cn,'native-cn-key.alz'),path.join(cn,'native-cn-key-volumes.alz')]]);await click('.archive-select');await waitForElement('[data-testid="archive-recovery-batch"]');await click('[data-testid="batch-recovery-config"]')
  await sizes('alz-batch-diagnosis-encoding','[data-testid="recovery-batch-zip-password-encoding"]');await driver.actions().sendKeys(Key.ESCAPE).perform()
  console.log('[alz-152] settings')
  await click('[data-testid="nav-Settings"]');await sizes('alz-settings-readonly','[data-testid="settings-alz-read"]')
  await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement('[data-testid="settings-password-extraction-formats"]'))
  await driver.wait(async()=>/ALZ（只读）/.test(await(await waitForElement('[data-testid="settings-password-extraction-formats"]')).getText()),10000)
  assert.match(await(await waitForElement('[data-testid="settings-password-extraction-formats"]')).getText(),/ALZ（只读）/)
  for(const m of cnManifest.members)assert.equal(sha(path.join(cn,m.name)),m.sha256)
  writeFileSync(path.join(artifactDirectory,'alz-152-results.json'),JSON.stringify({completedAt:new Date().toISOString(),application,binarySha256:sha(application),helperSha256:sha(path.join(root,'src-tauri/resources/alz-engine/long-alz.exe')),cases,screenshots,layouts},null,2))
}

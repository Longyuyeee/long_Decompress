import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, openSync, writeSync, closeSync, createReadStream, unlinkSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runLzopVolumes173Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/lzop-numeric-create-173'),file=path.join(base,'desktop.json'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.7.3',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[lzop173]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){assert.notEqual(await el.getAttribute('aria-disabled'),'true');await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value,enter=false)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value);if(enter)await el.sendKeys(Key.ENTER)}
 const shot=async(name,width,height,focus)=>{resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();if(focus){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),await find(focus));await settle()}const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);if(layout.panes.length){assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1))}for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}
 const native=JSON.parse(readFileSync(path.join(base,'generated/manifest.json'),'utf8'))
 const source=path.join(path.dirname(native[0].archive),'source'),folder=path.join(source,'资料目录'),seven=path.join(root,'src-tauri/resources/archive-engine/7z.exe')
 const selected=[path.join(folder,'说明.txt'),path.join(source,'extra/说明.txt'),...['random.bin','图标.png','空文件.txt'].map(n=>path.join(folder,n))]
 const multiple=native.find(c=>c.case==='multiple').entries,tar=native.find(c=>c.case==='directory').entries
 const outputsFile=path.join(base,'desktop-outputs.json'),outputs=existsSync(outputsFile)?JSON.parse(readFileSync(outputsFile,'utf8')):[]
 const record=(snapshot,format,entries,size)=>{
  assert.ok(snapshot.split);assert.match(snapshot.primaryPath,/\.001$/);assert.ok(snapshot.files.length>1)
  const parts=snapshot.files.map(f=>f.path),data=parts.map(p=>readFileSync(p));assert.ok(data.every(b=>b.length>0&&b.length<=size));assert.equal(data.reduce((n,b)=>n+b.length,0),snapshot.totalBytes)
  const archive=snapshot.primaryPath.replace(/\.001$/,'');assert.ok(!existsSync(archive))
  if(!outputs.some(o=>o.archive===archive)){outputs.push({case:path.basename(archive),format,archive,parts,splitSize:size,streamSha256:createHash('sha256').update(Buffer.concat(data)).digest('hex'),entries});writeFileSync(outputsFile,JSON.stringify(outputs,null,2)+'\n')}
 }
 const check=(dir,entries)=>{for(const e of entries){const p=path.join(dir,e.path);assert.ok(existsSync(p),e.path);if(!e.directory)assert.equal(sha(p),e.sha256,e.path)}}
 const historyJobs=async sources=>driver.wait(async()=>{const rows=await bridge('taskHistory'),jobs=sources.map(s=>rows.find(r=>r.sourcePaths?.includes(s)));for(const row of jobs.filter(Boolean))assert.notEqual(row.status,'failed',row.errorMessage);return jobs.every(r=>r?.status==='completed')?jobs:false},120000)
 const closeConversion=async()=>{for(const e of await driver.findElements(By.css('[role="dialog"] .conversion-button')))if(await e.isDisplayed()&&(await e.getText())==='关闭'){await e.click();return}throw Error('Missing conversion close')}
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio))
 for(const prompt of await driver.findElements(By.css('[aria-label="关闭默认打开提醒"]')))if(await prompt.isDisplayed())await prompt.click()
 await step('inline and multi-select create three real numeric groups without changing inherited global configuration',async()=>{
  await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]')
  await bridge('queueDesktopDialogSelections',[selected]);await click('[data-testid="dropzone-file"]')
  await click('[data-testid="open-global-compression-settings"]');await choose('[role="dialog"] [data-testid="compression-format-select"]','ZIP');await fill('[role="dialog"] [data-testid="compression-output-path"]',path.join(report.work,'created'))
  const globalSplit=await find('[role="dialog"] [data-testid="compression-split-toggle"]');if(await globalSplit.isSelected())await globalSplit.click();await click('[data-testid="save-global-compression-settings"]')
  for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),box);await box.click()}
  await click('[data-testid="compression-grouping-actions"] button');await find('[data-testid="compression-group-row"]')
  if(!(await driver.findElements(By.css('[data-testid="compression-draft-config"]'))).length)await click('[data-testid="compression-group-row"] .compression-job-row')
  await choose('[data-testid="compression-draft-config"] [data-testid="compression-format-select"]','LZOP');await click('[data-testid="compression-group-row"] .compression-job-row')
  await bridge('queueDesktopDialogSelections',[[folder]]);await click('[data-testid="dropzone-folder"]')
  const folderRow='[data-testid="compression-draft-row"]:has([aria-label="选择 资料目录"])'
  await click(folderRow+' [data-testid="compression-archive-name"]');await choose('[data-testid="compression-draft-config"] [data-testid="compression-format-select"]','TAR.LZO');await click(folderRow+' [data-testid="compression-archive-name"]')
  const single=path.join(report.work,'单文件.bin');writeFileSync(single,readFileSync(path.join(folder,'random.bin')))
  await bridge('queueDesktopDialogSelections',[[single]]);await click('[data-testid="dropzone-file"]')
  await click('[data-testid="compression-draft-row"]:has([aria-label="选择 单文件.bin"]) [data-testid="compression-archive-name"]');await choose('[data-testid="compression-draft-config"] [data-testid="compression-format-select"]','LZOP')
  await click('[data-testid="compression-draft-group-checkbox"]');for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),box);await box.click()}
  await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-lzop-volume-mode"]','统一每卷大小');await fill('[data-testid="batch-lzop-volume-size"]','1');await shot('batch-narrow',760,520);await click('[data-testid="batch-config-apply"]')
  await click('[data-testid="open-global-compression-settings"]');assert.match(await(await find('[role="dialog"] [data-testid="compression-format-select"]')).getText(),/ZIP/);assert.equal(await(await find('[role="dialog"] [data-testid="compression-split-toggle"]')).isSelected(),false);await click('[data-testid="save-global-compression-settings"]')
  assert.equal(await(await find('[data-testid="compression-draft-config"] [data-testid="compression-split-toggle"]')).isSelected(),true)
  assert.equal((await driver.findElements(By.css('[data-testid="compression-draft-config"] [data-testid="compression-level-control"],[data-testid="compression-draft-config"] [data-testid="compression-password-input"]'))).length,0)
  await shot('inline-wide',1100,650,'[data-testid="compression-split-settings"]');await shot('inline-narrow',760,520,'[data-testid="compression-split-settings"]');await click('[data-testid="start-compression"]')
  const jobs=await historyJobs([selected[0],folder,single]);report.created={}
  for(let i=0;i<jobs.length;i++){const key=['multi','tar','single'][i],format=i===1?'tar.lzo':'lzo',entries=i===0?multiple:i===1?tar:[{path:path.basename(single),directory:false,bytes:readFileSync(single).length,sha256:sha(single)}];assert.equal(jobs[i].executionConfig.settings.format,format);assert.equal(jobs[i].executionConfig.settings.splitArchive,true);record(jobs[i].archiveOutput,format,entries,1048576);report.created[key]=jobs[i].archiveOutput}
  save();await click('[data-testid="archive-output-summary"] button[aria-expanded]');await shot('result-wide',1100,650,'[data-testid="archive-output-summary"]');await shot('result-narrow',760,520,'[data-testid="archive-output-summary"]')
  return{groups:3,parts:jobs.map(j=>j.archiveOutput.files.length),singleExpand:true,multiSelectModal:true,globalZipPreserved:true,historyComplete:true}
 })
 await step('nonfirst volumes stay in equal browser panes and support text image preview and selected empty-directory extraction',async()=>{
  const snapshot=report.created.tar
  await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',path.dirname(snapshot.primaryPath),true);await fill('[data-testid="file-manager-path-input-right"]',report.work,true)
  await click(`[data-file-pane="left"] .file-row[data-path$=${JSON.stringify(path.basename(snapshot.files.at(-1).path))}]`,true)
  await find('[data-file-pane="left"] [data-entry-path="资料目录/"]');await click('[data-file-pane="left"] [data-entry-path="资料目录/"]',true)
  await find('[data-file-pane="left"] [data-entry-path="资料目录/说明.txt"]');await shot('browser-wide',1100,650);await shot('browser-narrow',760,520)
  await click('[data-file-pane="left"] [data-entry-path="资料目录/说明.txt"] [data-testid="file-manager-preview"]');await find('[data-testid="archive-text-preview"]');await shot('text-narrow',760,520);await click('.preview-close')
  await click('[data-file-pane="left"] [data-entry-path="资料目录/图标.png"] [data-testid="file-manager-preview"]');await find('[data-testid="archive-image-preview"]');await shot('image-narrow',760,520);await click('.preview-close')
  const out=path.join(report.work,'selected');await bridge('extractArchive',snapshot.files.at(-1).path,out,null,{selected_entries:['资料目录/空目录','资料目录/说明.txt']});check(out,tar.filter(e=>e.path.startsWith('资料目录/空目录')||e.path==='资料目录/说明.txt'));assert.ok(!existsSync(path.join(out,'资料目录/random.bin')))
  return{nonfirstVolume:true,equalPanes:true,textPreview:true,imagePreview:true,selectiveExtraction:true,emptyDirectoriesRetained:true}
 })
 await step('password ZIP UI conversion keeps split size and reads back through a password-protected target',async()=>{
  await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
  const zip=path.join(report.work,'encrypted.zip'),names=['说明.txt','random.bin','图标.png','空文件.txt'],entries=multiple.filter(e=>names.includes(e.path));report.encryptedZip=zip
  if(!existsSync(zip)){const r=spawnSync(seven,['a','-tzip','-psource173','-mem=AES256',zip,...names],{cwd:folder,windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr)}const original=sha(zip)
  await bridge('queueDesktopDialogSelections',[[zip]]);await click('[data-testid="conversion-choose-source"]');await find('[data-testid="conversion-item"]')
  if(!(await driver.findElements(By.css('[data-testid="conversion-details"]'))).length)await click('[data-testid="conversion-row"]')
  await choose('[data-testid="conversion-format"]','LZOP（多文件，无目录）');await fill('[data-testid="conversion-source-password"]','source173');await fill('[data-testid="conversion-split"]','0.0625');await fill('[data-testid="conversion-output"]',path.join(report.work,'converted.lzo'));await shot('conversion-narrow',760,520,'[data-testid="conversion-split"]');await click('[data-testid="conversion-start"]')
  const [job]=await historyJobs([zip]);report.converted=job.archiveOutput;record(report.converted,'lzo',entries,65536);save();await closeConversion()
  const restored=path.join(report.work,'converted-restored');await bridge('extractArchive',report.converted.files.at(-1).path,restored,null);check(restored,entries)
  const back=await bridge('convertArchive',report.converted.files[1].path,null,path.join(report.work,'back.7z'),{format:'7z',level:1,password:'target173',encrypt_filenames:true})
  const out=path.join(report.work,'back-restored'),r=spawnSync(seven,['x','-y','-ptarget173','-o'+out,back.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr);check(out,entries);assert.equal(sha(zip),original)
  return{sourcePasswordOnly:true,numericMiB:0.0625,parts:report.converted.files.length,nonfirstVolumeInput:true,passwordRoundtrip:true,sourcePreserved:true}
 })
 await step('missing middle fails without loss and restored volumes retry; real cutting cancellation cleans all staged pieces',async()=>{
  const copy=path.join(report.work,'missing');mkdirSync(copy,{recursive:true});const parts=report.created.multi.files.map(f=>{const p=path.join(copy,path.basename(f.path));writeFileSync(p,readFileSync(f.path));return p}),missing=parts[1],bytes=readFileSync(missing);unlinkSync(missing)
  const out=path.join(report.work,'missing-restored');mkdirSync(out,{recursive:true});writeFileSync(path.join(out,'sentinel'),'keep');let error='';try{await bridge('extractArchive',parts.at(-1),out,null)}catch(e){error=String(e)}assert.ok(error);assert.deepEqual(readdirSync(out),['sentinel']);writeFileSync(missing,bytes);await bridge('extractArchive',parts.at(-1),out,null);check(out,multiple);assert.equal(readFileSync(path.join(out,'sentinel'),'utf8'),'keep')
  const first=path.join(report.work,'first.txt'),later=path.join(report.work,'later.bin'),target=path.join(report.work,'cancel.lzo');writeFileSync(first,'complete first member');if(!existsSync(later)){const chunk=readFileSync(path.join(folder,'random.bin')),fd=openSync(later,'w');try{for(let i=0;i<32;i++)writeSync(fd,chunk)}finally{closeSync(fd)}}
  const digest=async p=>{const h=createHash('sha256');for await(const chunk of createReadStream(p))h.update(chunk);return h.digest('hex')},before=await digest(later)
  const id=await bridge('startSevenZipCompression',[first,later],target,'lzo',65536,true);await driver.wait(async()=>{const status=await bridge('taskStatus',id);assert.notEqual(status,'failed');assert.notEqual(status,'completed');return status==='cancelled'},90000)
  const cut=await bridge('taskSplitCancellation',id);assert.ok(cut?.processedBytes>0&&cut.processedBytes<cut.totalBytes,JSON.stringify(cut));await driver.wait(()=>!readdirSync(report.work).some(n=>n.includes('cancel.lzo')),20000);assert.equal(await digest(later),before)
  const retry=await bridge('convertArchive',report.encryptedZip,'source173',path.join(report.work,'retry.lzo'),{format:'lzo',level:1,split_size:65536});record(retry,'lzo',multiple.filter(e=>['说明.txt','random.bin','图标.png','空文件.txt'].includes(e.path)),65536)
  return{missingMiddleError:error,restoreRetry:true,cutStage:cut,publishedPiecesAfterCancellation:0,sourceUnchanged:true,nextCreationSucceeded:true}
 })
 report.passed=true;save()
}

export async function runLzopVolumesUiFinal173({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/lzop-numeric-create-173'),file=path.join(base,'ui-final.json'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.7.3',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[lzop173]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){assert.notEqual(await el.getAttribute('aria-disabled'),'true');await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value,enter=false)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value);if(enter)await el.sendKeys(Key.ENTER)}
 const shot=async(name,width,height,focus)=>{resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();if(focus){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),await find(focus));await settle()}const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);if(layout.panes.length){assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1))}for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}
 const native=JSON.parse(readFileSync(path.join(base,'generated/manifest.json'),'utf8')),folder=path.join(path.dirname(native[0].archive),'source/资料目录')
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio))
 for(const prompt of await driver.findElements(By.css('[aria-label="关闭默认打开提醒"]')))if(await prompt.isDisplayed())await prompt.click()
 await step('final batch modal shows only applicable controls for existing LZOP tasks and preserves other configuration',async()=>{
  await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[path.join(folder,'说明.txt'),path.join(folder,'random.bin')]]);await click('[data-testid="dropzone-file"]')
  for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),box);await box.click()}
  await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','LZOP（多文件，无加密）');await choose('[data-testid="batch-lzop-volume-mode"]','统一每卷大小');await fill('[data-testid="batch-lzop-volume-size"]','1');await click('[data-testid="batch-config-apply"]')
  await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]')
  for(const id of ['batch-level','batch-zip-password-encoding','batch-wim-method','batch-sevenz-method','batch-zip-method','batch-rar-recovery','batch-rar-recovery-volumes'])assert.equal((await driver.findElements(By.css(`[data-testid="${id}"]`))).length,0,id)
  assert.match(await(await find('[data-testid="batch-format"]')).getText(),/保持各自设置/);await choose('[data-testid="batch-lzop-volume-mode"]','统一每卷大小');await fill('[data-testid="batch-lzop-volume-size"]','2');await shot('batch-final-wide',1100,650,'[data-testid="batch-lzop-volume-size"]');await shot('batch-final-narrow',760,520,'[data-testid="batch-lzop-volume-size"]');await click('[data-testid="batch-config-apply"]')
  await click('[data-testid="compression-draft-row"]:has([aria-label="选择 说明.txt"]) [data-testid="compression-archive-name"]');assert.match(await(await find('[data-testid="compression-draft-config"] [data-testid="compression-format-select"]')).getText(),/LZOP/);assert.equal(await(await find('[data-testid="compression-draft-config"] [data-testid="compression-split-size"]')).getAttribute('value'),'2')
  return{noUnrelatedControls:true,formatRetained:true,sizeOnlyApplied:true,nativeFlowsReusedFrom:'desktop.json',onlyChange:'batch modal hides irrelevant controls when all selected formats are LZOP family'}
 })
 report.passed=true;save()
}

function resizeHost(application,width,height,scale) {
  const escaped=application.replaceAll("'","''")
  const result=spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-Command',`Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class TarLzoImageWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hwnd,int command);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}


'@
[void][TarLzoImageWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
$taskHwnd=(Get-Process -Id $taskApp.ProcessId).MainWindowHandle
if ([TarLzoImageWindow]::IsIconic($taskHwnd)) { [void][TarLzoImageWindow]::ShowWindow($taskHwnd,9) }
if (-not [TarLzoImageWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}


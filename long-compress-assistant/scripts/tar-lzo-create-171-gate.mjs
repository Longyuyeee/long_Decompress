import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync,readdirSync,openSync,writeSync,closeSync,createReadStream} from 'node:fs'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runTarLzoCreate171Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/tar-lzo-create-171'),file=path.join(base,'desktop.json'),generated=path.join(base,'generated'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.7.1',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 report.applicationSha256 ??= sha(application);assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true});const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[tarlzo171]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value,enter=false)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value);if(enter)await el.sendKeys(Key.ENTER)}
 const ipc=async(cmd,args)=>{const r=await driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args);assert.ok(r.ok,r.error);return r.value}
 const shot=async(name,width,height,focus)=>{if(report.layouts.some(l=>l.name===name))return;resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();if(focus){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),await find(focus));await settle()}const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('[data-testid="compression-draft-config"],.compression-execution-panel')].map(e=>e.getBoundingClientRect().toJSON()),panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);if(layout.panes.length){assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1))}for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}
 const sources=path.join(generated,'source'),folder=path.join(sources,'资料目录'),extra=path.join(sources,'单独说明.md')
 const official=path.join(root,'test-results/lzop-read-convert-1657/official-lzop/lzop103w/lzop.exe'),seven=path.join(root,'src-tauri/resources/archive-engine/7z.exe')
 const verify=(archive,hasFolder=true)=>{
  const tested=spawnSync(official,['-t','-P',archive],{windowsHide:true});assert.equal(tested.status,0,tested.stderr?.toString())
  const decoded=spawnSync(official,['-d','-c',archive],{windowsHide:true,maxBuffer:32*1024*1024});assert.equal(decoded.status,0,decoded.stderr?.toString())
  const tar=archive+'.tar',destination=path.join(report.work,'independent-'+randomUUID());writeFileSync(tar,decoded.stdout);mkdirSync(destination,{recursive:true})
  const result=spawnSync('tar.exe',['-xf',tar,'-C',destination],{windowsHide:true});assert.equal(result.status,0,result.stderr?.toString())
  if(hasFolder){for(const n of ['说明.txt','空文件.txt','random.bin','图标.png'])assert.equal(sha(path.join(destination,'资料目录',n)),sha(path.join(folder,n)));assert.ok(existsSync(path.join(destination,'资料目录/空目录/下一级')))}
  return{archive,archiveSha256:sha(archive),officialReadback:true,independentTar:true,destination}
 }
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript(()=>innerWidth===1100&&innerHeight===650),10000)
 for(const prompt of await driver.findElements(By.css('[aria-label="关闭默认打开提醒"]')))if(await prompt.isDisplayed())await prompt.click()
 await step('folder inline and selected batch modal create real TAR.LZO outputs with results and history',async()=>{
  if(!report.created){
   await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]')
   await bridge('queueDesktopDialogSelections',[[folder]]);await click('[data-testid="dropzone-folder"]')
   await bridge('queueDesktopDialogSelections',[[extra]]);await click('[data-testid="dropzone-file"]')
   await click('[data-testid="open-global-compression-settings"]');await choose('[role="dialog"] [data-testid="compression-format-select"]','ZIP');await fill('[role="dialog"] [data-testid="compression-output-path"]',path.join(report.work,'created'));await click('[data-testid="save-global-compression-settings"]')
   await click('[data-testid="compression-group-row"],[data-testid="compression-draft-row"]');await choose('[data-testid="compression-draft-config"] [data-testid="compression-format-select"]','TAR.LZO')
   assert.equal((await driver.findElements(By.css('[data-testid="compression-level-control"],[data-testid="compression-password-input"],[role="dialog"]'))).length,0)
   await click('[data-testid="compression-analysis"] .analysis-button');await find('.analysis-metrics')
   await shot('inline-wide',1100,650,'[data-testid="compression-draft-config"]');await shot('inline-narrow',760,520,'[data-testid="compression-draft-config"]')
   for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"],[data-testid="compression-draft-group-checkbox"]'))){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),box);await box.click()}
   await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','TAR.LZO（目录，无加密）');assert.equal((await driver.findElements(By.css('[data-testid="batch-level"],[data-testid="batch-rar-recovery"]'))).length,0);await shot('batch-narrow',760,520);await click('[data-testid="batch-config-apply"]')
   await click('[data-testid="open-global-compression-settings"]');assert.match(await(await find('[role="dialog"] [data-testid="compression-format-select"]')).getText(),/ZIP/);await click('[data-testid="save-global-compression-settings"]')
   await click('[data-testid="start-compression"]')
   report.created=await driver.wait(async()=>{const rows=await bridge('taskHistory');const found=[folder,extra].map(p=>rows.find(row=>row.status==='completed'&&row.taskType==='compression'&&row.sourcePaths.includes(p)));return found.every(Boolean)?found:false},60000);save()
  }
  const proof=[];for(const task of report.created){assert.equal(task.executionConfig.settings.format,'tar.lzo');proof.push(verify(task.archiveOutput?.primaryPath||task.outputPath,task.sourcePaths.includes(folder)))}
  await shot('results-narrow',760,520)
  const archive=report.created.find(task=>task.sourcePaths.includes(folder));report.browserArchive=archive.archiveOutput?.primaryPath||archive.outputPath;save()
  return{inlinePreserved:true,batchModalPreserved:true,globalZipUnchanged:true,realAnalysis:true,outputs:proof}
 })
 await step('new TAR.LZO stays in equal dual panes with text and image previews',async()=>{
  await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',path.dirname(report.browserArchive),true);await fill('[data-testid="file-manager-path-input-right"]',report.work,true)
  await click(`[data-file-pane="left"] .file-row[data-path$=${JSON.stringify(path.basename(report.browserArchive))}]`,true)
  await find('[data-file-pane="left"] [data-entry-path="资料目录/"]');await click('[data-file-pane="left"] [data-entry-path="资料目录/"]',true)
  await find('[data-file-pane="left"] [data-entry-path="资料目录/说明.txt"]');await shot('browser-wide',1100,650);await shot('browser-narrow',760,520)
  await click('[data-file-pane="left"] [data-entry-path="资料目录/说明.txt"]',true);await find('[data-testid="archive-text-preview"]');await shot('text-narrow',760,520);await click('.preview-close')
  await click('[data-file-pane="left"] [data-entry-path="资料目录/图标.png"]',true);await find('[data-testid="archive-image-preview"]');await click('.preview-close')
  return{dualPanePreserved:true,textAndImagePreview:true}
 })
 await step('password ZIP converts all folders and empty entries to TAR.LZO and back without changing its source',async()=>{
  const input=path.join(report.work,'source-encrypted.zip'),target=path.join(report.work,'converted.tar.lzo')
  if(!existsSync(input)){const result=spawnSync(seven,['a','-tzip','-pOrigin171','-mem=AES256',input,'资料目录','单独说明.md'],{cwd:sources,windowsHide:true,encoding:'utf8'});assert.equal(result.status,0,result.stdout+result.stderr)}
  const before=sha(input)
  if(!report.converted){report.converted=await bridge('convertArchive',input,'Origin171',target,{format:'tar.lzo',level:1});save()}
  const proof=verify(report.converted.primaryPath);assert.equal(sha(path.join(proof.destination,'单独说明.md')),sha(extra));assert.equal(sha(input),before)
  if(!report.back){report.back=await bridge('convertArchive',target,null,path.join(report.work,'back.7z'),{format:'7z',level:1});save()}
  const output=path.join(report.work,'back-restored'),result=spawnSync(seven,['x','-y','-o'+output,report.back.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(result.status,0,result.stdout+result.stderr)
  assert.equal(sha(path.join(output,'资料目录/说明.txt')),sha(path.join(folder,'说明.txt')));assert.equal(sha(path.join(output,'单独说明.md')),sha(extra));assert.ok(existsSync(path.join(output,'资料目录/空目录/下一级')))
  return{proof,sourcePasswordReadOnly:true,sourceUnchanged:true,roundtripTo7z:true}
 })
 await step('active TAR.LZO cancellation removes unfinished output and a subsequent creation succeeds',async()=>{
  const source=path.join(report.work,'cancel.bin'),out=path.join(report.work,'cancel.tar.lzo')
  if(!existsSync(source)){const seed=readFileSync(path.join(folder,'random.bin')),fd=openSync(source,'w');try{for(let i=0;i<192;i++)writeSync(fd,seed)}finally{closeSync(fd)}}
  const digest=async p=>{const h=createHash('sha256');for await(const part of createReadStream(p))h.update(part);return h.digest('hex')},before=await digest(source)
  if(!report.cancel){const id=await bridge('startSevenZipCompression',source,out,'tar.lzo');await driver.wait(async()=>await bridge('taskStatus',id)==='compressing'&&await bridge('taskProgress',id)>0,20000);await bridge('cancelTask',id);await driver.wait(async()=>await bridge('taskStatus',id)==='cancelled',20000);await driver.wait(()=>!existsSync(out),5000);report.cancel={taskId:id,activeProgress:true,noPublishedOutput:true};save()}
  assert.equal(await digest(source),before)
  if(!report.retry){report.retry=await bridge('convertArchive',report.browserArchive,null,path.join(report.work,'after-cancel.tar.lzo'),{format:'tar.lzo',level:1});save()}
  return{cancel:report.cancel,sourceUnchanged:true,subsequentCreation:verify(report.retry.primaryPath)}
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

export async function runTarLzoUiFinal171({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/tar-lzo-create-171'),file=path.join(base,'desktop-ui-final.json'),generated=path.join(base,'generated'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.7.1',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 report.applicationSha256 ??= sha(application);assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true});const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[tarlzo171]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value,enter=false)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value);if(enter)await el.sendKeys(Key.ENTER)}
 const ipc=async(cmd,args)=>{const r=await driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args);assert.ok(r.ok,r.error);return r.value}
 const shot=async(name,width,height,focus)=>{if(report.layouts.some(l=>l.name===name))return;resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();if(focus){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),await find(focus));await settle()}const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('[data-testid="compression-draft-config"],.compression-execution-panel')].map(e=>e.getBoundingClientRect().toJSON()),panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);if(layout.panes.length){assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1))}for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}

 const original=JSON.parse(readFileSync(path.join(base,'desktop.json'),'utf8'))
 for(const prompt of await driver.findElements(By.css('[aria-label="关闭默认打开提醒"]')))if(await prompt.isDisplayed())await prompt.click()
 await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',path.dirname(original.browserArchive),true);await fill('[data-testid="file-manager-path-input-right"]',path.join(generated,'source'),true)
 await click(`[data-file-pane="left"] .file-row[data-path$=${JSON.stringify(path.basename(original.browserArchive))}]`,true)
 await find('[data-file-pane="left"] [data-entry-path="资料目录/"]');await click('[data-file-pane="left"] [data-entry-path="资料目录/"]',true)
 await shot('dates-wide',1100,650)
 const dates=await driver.findElements(By.css('[data-file-pane="right"] .file-row>span:last-child'))
 assert.ok(dates.length>0)
 for(const cell of dates){const text=await cell.getAttribute('textContent');assert.match(text,/20\d{2}/);assert.ok(!text.includes('[object Object]'))}
 await shot('dates-narrow',760,520)
 report.checks.push({name:'real native file timestamps render readable dates instead of objects',passed:true});report.passed=true;save()
}

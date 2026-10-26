import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync,readdirSync,openSync,writeSync,closeSync,createReadStream} from 'node:fs'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runLzopCreate1662Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/lzop-create-1662'),file=path.join(base,'desktop.json'),generated=path.join(base,'generated'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.62',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 report.applicationSha256 ??= sha(application);assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true});const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[lzop1662]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value,enter=false)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value);if(enter)await el.sendKeys(Key.ENTER)}
 const ipc=async(cmd,args)=>{const r=await driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args);assert.ok(r.ok,r.error);return r.value}
 const shot=async(name,width,height,focus)=>{if(report.layouts.some(l=>l.name===name))return;resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();if(focus){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),await find(focus));await settle()}const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('[data-testid="compression-draft-config"],.compression-execution-panel')].map(e=>e.getBoundingClientRect().toJSON()),panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);if(layout.panes.length){assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1))}for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}
 const seven=path.join(root,'src-tauri/resources/archive-engine/7z.exe'),official=path.join(root,'test-results/lzop-read-convert-1657/official-lzop/lzop103w/lzop.exe')
 const checkOfficial=(archive,source)=>{const t=spawnSync(official,['-t','-P',archive],{windowsHide:true});assert.equal(t.status,0,t.stderr?.toString());const d=spawnSync(official,['-d','-c',archive],{windowsHide:true,maxBuffer:400*1024*1024});assert.equal(d.status,0,d.stderr?.toString());assert.equal(createHash('sha256').update(d.stdout).digest('hex'),sha(source));return{archive,sourceSha256:sha(source),archiveSha256:sha(archive),officialTestExit:t.status,officialReadbackExit:d.status}}
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript(()=>innerWidth===1100&&innerHeight===650),10000)
 await step('single row expands inline and independent batch modal configures LZOP while global ZIP remains',async()=>{
  await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]')
  for(const name of ['资料.txt','cover.png']){await bridge('queueDesktopDialogSelections',[[path.join(generated,name)]]);await click('[data-testid="dropzone-file"]')}
  await click('[data-testid="open-global-compression-settings"]');await choose('[role="dialog"] [data-testid="compression-format-select"]','ZIP');await fill('[role="dialog"] [data-testid="compression-output-path"]',path.join(report.work,'created'));await click('[data-testid="save-global-compression-settings"]')
  await click('[data-testid="compression-draft-row"]');await choose('[data-testid="compression-draft-config"] [data-testid="compression-format-select"]','LZOP')
  assert.equal((await driver.findElements(By.css('[data-testid="compression-level-control"],[data-testid="compression-password-input"],[role="dialog"]'))).length,0);assert.equal(await(await find('[data-testid="compression-split-toggle"]')).isEnabled(),false)
  await click('[data-testid="compression-analysis"] .analysis-button');await find('.analysis-metrics');await shot('inline-wide',1100,650,'[data-testid="compression-draft-config"]');await shot('inline-narrow',760,520,'[data-testid="compression-draft-config"]');await shot('controls-narrow',760,520,'[data-testid="compression-draft-config"] [data-testid="compression-format-select"]')
  for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),box);await box.click()}
  await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','LZOP（单文件，无加密）');assert.equal((await driver.findElements(By.css('[data-testid="batch-level"]'))).length,0);await shot('batch-narrow',760,520);await click('[data-testid="batch-config-apply"]')
  await click('[data-testid="open-global-compression-settings"]');assert.match(await(await find('[role="dialog"] [data-testid="compression-format-select"]')).getText(),/ZIP/);await click('[data-testid="save-global-compression-settings"]');report.reseedCreation=false;return{inlinePreserved:true,batchModalPreserved:true,globalZipPreserved:true,realAnalysis:true,invalidControlsHidden:true}
 })
 await step('UI creation publishes verified results history and reusable text image previews in equal dual panes',async()=>{
  await click('[data-testid="nav-Compress"]');const recover=await driver.findElements(By.css('[data-testid="pending-draft-recovery"] button'));for(const button of recover)if((await button.getText()).includes('恢复')){await button.click();break}
  if(report.reseedCreation&&!report.created){await bridge('clearCompressionWorkspace');await bridge('clearTasks');for(const n of ['资料.txt','cover.png']){await bridge('queueDesktopDialogSelections',[[path.join(generated,n)]]);await click('[data-testid="dropzone-file"]')}for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),box);await box.click()}await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','LZOP（单文件，无加密）');await click('[data-testid="batch-config-apply"]')}
  if(!report.created){await click('[data-testid="start-compression"]');report.created=await driver.wait(async()=>{const rows=await bridge('taskHistory');const result=['资料.txt','cover.png'].map(n=>rows.find(r=>r.status==='completed'&&r.taskType==='compression'&&r.sourcePaths.includes(path.join(generated,n))));return result.every(Boolean)?result:false},60000);save()}
  const proof=[];for(const task of report.created){assert.equal(task.status,'completed',task.errorMessage);assert.equal(task.executionConfig.settings.format,'lzo');const archive=task.archiveOutput?.primaryPath||task.outputPath,source=task.sourcePaths[0],entry=path.basename(source);proof.push(checkOfficial(archive,source));const out=path.join(report.work,'restored-'+entry);await bridge('extractArchive',archive,out,null);assert.equal(sha(path.join(out,entry)),sha(source));const request={filePath:archive,entryPath:entry,password:null,previewId:randomUUID()};if(entry.endsWith('.txt')){const p=await ipc('preview_archive_text',request);assert.equal(createHash('sha256').update(p.content).digest('hex'),sha(source))}else{const p=await ipc('preview_archive_image',request);assert.equal(createHash('sha256').update(Buffer.from(p.dataUrl.split(',')[1],'base64')).digest('hex'),sha(source))}}
  await shot('result-wide',1100,650);await shot('result-narrow',760,520)
  await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',path.join(report.work,'created'),true);await fill('[data-testid="file-manager-path-input-right"]',report.work,true);const textTask=report.created.find(t=>t.sourcePaths[0].endsWith('资料.txt')),textName=path.basename(textTask.archiveOutput?.primaryPath||textTask.outputPath);await click(`[data-file-pane="left"] .file-row[data-path$=${JSON.stringify(textName)}]`,true);await find('[data-file-pane="left"] [data-entry-path="资料.txt"]');await shot('browser-wide',1100,650);await shot('browser-narrow',760,520);await click('[data-file-pane="left"] [data-entry-path="资料.txt"]',true);await find('[data-testid="archive-text-preview"]');await shot('text-narrow',760,520);await click('.preview-close');return{outputs:proof,historyFormat:'lzo',previewHashesMatch:true,sourcePreserved:true}
 })
 await step('encrypted ZIP single-file conversion keeps source filename and multi-file or folder input rejects without loss then corrects target',async()=>{
  const source=path.join(generated,'资料.txt'),zip=path.join(report.work,'encrypted.zip')
  if(!existsSync(zip)){const a=spawnSync(seven,['a','-tzip','-psource1662','-mem=AES256',zip,path.basename(source)],{cwd:generated,windowsHide:true,encoding:'utf8'});assert.equal(a.status,0,a.stdout+a.stderr)}
  if(!report.converted){report.converted=await bridge('convertArchive',zip,'source1662',path.join(report.work,'renamed-output.lzo'),{format:'lzo',level:1});save()}
  const proof=checkOfficial(report.converted.primaryPath,source),out=path.join(report.work,'converted-restored');await bridge('extractArchive',report.converted.primaryPath,out,null);assert.equal(sha(path.join(out,'资料.txt')),sha(source));const sourceSha=sha(zip)
  const failure=[];for(const kind of ['multiple','folder']){const input=path.join(report.work,kind+'.zip');if(!existsSync(input)){let args;if(kind==='multiple')args=['a','-tzip',input,path.join(generated,'notes.txt'),path.join(generated,'cover.png')];else{const folder=path.join(report.work,'only-folder');mkdirSync(folder,{recursive:true});writeFileSync(path.join(folder,'a.txt'),'folder content');args=['a','-tzip',input,folder]}const a=spawnSync(seven,args,{windowsHide:true,encoding:'utf8'});assert.equal(a.status,0,a.stdout+a.stderr)}const original=sha(input),target=path.join(report.work,kind+'.lzo');let error='';try{await bridge('convertArchive',input,null,target,{format:'lzo',level:1})}catch(e){error=String(e)}assert.match(error,/一个顶层文件|一个普通文件/);assert.ok(!existsSync(target));assert.equal(sha(input),original);failure.push({kind,error,sourceUnchanged:true,noPublishedOutput:true})}
  if(!report.corrected){report.corrected=await bridge('convertArchive',path.join(report.work,'multiple.zip'),null,path.join(report.work,'corrected.7z'),{format:'7z',level:1,password:'target1662',encrypt_filenames:true});save()}
  const dir=path.join(report.work,'corrected-restored'),a=spawnSync(seven,['x','-y','-ptarget1662','-o'+dir,report.corrected.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(a.status,0,a.stdout+a.stderr);for(const n of ['notes.txt','cover.png'])assert.equal(sha(path.join(dir,n)),sha(path.join(generated,n)));assert.equal(sha(zip),sourceSha);return{conversion:proof,sourcePasswordOnly:true,sourceBasenameRetained:true,failure,correctedTargetKeepsAllFiles:true}
 })
 await step('real active LZOP cancellation does not publish and subsequent creation completes verified output',async()=>{
  const source=path.join(report.work,'cancel.bin'),out=path.join(report.work,'cancel.lzo')
  if(!existsSync(source)){const seed=readFileSync(path.join(generated,'random.bin')),fd=openSync(source,'w');try{for(let i=0;i<288;i++)writeSync(fd,seed)}finally{closeSync(fd)}}
  const digest=async p=>{const h=createHash('sha256');for await(const chunk of createReadStream(p))h.update(chunk);return h.digest('hex')},before=await digest(source)
  if(!report.cancel){const id=await bridge('startSevenZipCompression',source,out,'lzo');await driver.wait(async()=>{const progress=await bridge('taskProgress',id);assert.equal(await bridge('taskStatus',id),'compressing');return progress>0},20000);await bridge('cancelTask',id);await driver.wait(async()=>await bridge('taskStatus',id)==='cancelled',20000);await driver.wait(()=>!existsSync(out),5000);report.cancel={taskId:id,activeProgress:true,published:false};save()}
  assert.equal(await digest(source),before)
  if(!report.retry){const result=await bridge('runArchiveRoundTrip',path.join(generated,'random.bin'),path.join(report.work,'retry.lzo'),path.join(report.work,'retry-restored'),'lzo');report.retry=result;save()}
  assert.equal(sha(path.join(report.work,'retry-restored/random.bin')),sha(path.join(generated,'random.bin')));return{...report.cancel,sourceUnchanged:true,retry:checkOfficial(path.join(report.work,'retry.lzo'),path.join(generated,'random.bin'))}
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

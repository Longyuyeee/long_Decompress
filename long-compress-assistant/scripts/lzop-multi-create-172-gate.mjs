import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, openSync, writeSync, closeSync, createReadStream } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runLzopMembers172Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/lzop-multi-create-172'),file=path.join(base,'desktop.json'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.7.2',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[lzop172]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){assert.notEqual(await el.getAttribute('aria-disabled'),'true');await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value,enter=false)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value);if(enter)await el.sendKeys(Key.ENTER)}
 const shot=async(name,width,height,focus)=>{resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();if(focus){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),await find(focus));await settle()}const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);if(layout.panes.length){assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1))}for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}
 const generated=path.join(base,'generated/source'),seven=path.join(root,'src-tauri/resources/archive-engine/7z.exe')
 const selected=['one/notes.txt','two/notes.txt','资料.txt','cover.png','empty.txt'].map(n=>path.join(generated,n))
 const expected=selected.map((source,i)=>({path:i===1?'notes (1).txt':path.basename(source),bytes:readFileSync(source).length,sha256:sha(source)}))
 const check=(dir,entries)=>{assert.deepEqual(readdirSync(dir).sort(),entries.map(e=>e.path).sort());for(const e of entries)assert.equal(sha(path.join(dir,e.path)),e.sha256)}
 const outputs=existsSync(path.join(base,'desktop-outputs.json'))?JSON.parse(readFileSync(path.join(base,'desktop-outputs.json'),'utf8')):[]
 const record=(archive,entries)=>{if(!outputs.some(o=>o.archive===archive)){outputs.push({archive,archiveSha256:sha(archive),entries});writeFileSync(path.join(base,'desktop-outputs.json'),JSON.stringify(outputs,null,2)+'\n')}}
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio))
 for(const prompt of await driver.findElements(By.css('[aria-label="关闭默认打开提醒"]')))if(await prompt.isDisplayed())await prompt.click()
 await step('real UI groups ordinary files and creates all members using inline and batch modal without changing global ZIP',async()=>{
  await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]')
  await bridge('queueDesktopDialogSelections',[selected]);await click('[data-testid="dropzone-file"]')
  await click('[data-testid="open-global-compression-settings"]');await choose('[role="dialog"] [data-testid="compression-format-select"]','ZIP');await fill('[role="dialog"] [data-testid="compression-output-path"]',path.join(report.work,'created'));await click('[data-testid="save-global-compression-settings"]')
  for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),box);await box.click()}
  await click('[data-testid="compression-grouping-actions"] button');await find('[data-testid="compression-group-row"]')
  if(!(await driver.findElements(By.css('[data-testid="compression-draft-config"]'))).length)await click('[data-testid="compression-group-row"] .compression-job-row')
  await choose('[data-testid="compression-draft-config"] [data-testid="compression-format-select"]','LZOP')
  assert.equal((await driver.findElements(By.css('[data-testid="compression-level-control"],[data-testid="compression-password-input"],[role="dialog"]'))).length,0)
  await click('[data-testid="compression-analysis"] .analysis-button');await find('.analysis-metrics');await shot('inline-wide',1100,650,'[data-testid="compression-draft-config"]');await shot('inline-narrow',760,520,'[data-testid="compression-draft-config"]')
  const extra=path.join(report.work,'separate.txt');writeFileSync(extra,'separate task keeps its own configuration')
  await bridge('queueDesktopDialogSelections',[[extra]]);await click('[data-testid="dropzone-file"]');await click('[data-testid="compression-draft-group-checkbox"]');await click('[data-testid="compression-group-checkbox"]')
  await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-format"]','LZOP（多文件，无加密）');assert.equal((await driver.findElements(By.css('[data-testid="batch-level"]'))).length,0);await shot('batch-narrow',760,520);await click('[data-testid="batch-config-apply"]')
  await click('[data-testid="open-global-compression-settings"]');assert.match(await(await find('[role="dialog"] [data-testid="compression-format-select"]')).getText(),/ZIP/);await click('[data-testid="save-global-compression-settings"]')
  await click('[data-testid="start-compression"]')
  const created=await driver.wait(async()=>{const history=await bridge('taskHistory');const jobs=[selected[0],extra].map(source=>history.find(r=>r.taskType==='compression'&&r.sourcePaths?.includes(source)));for(const row of jobs.filter(Boolean))assert.notEqual(row.status,'failed',row.errorMessage);return jobs.every(r=>r?.status==='completed')?jobs:false},90000)
  const task=created.find(t=>t.sourcePaths.length===selected.length);assert.ok(task);assert.equal(task.executionConfig.settings.format,'lzo');report.groupArchive=task.archiveOutput?.primaryPath||task.outputPath;record(report.groupArchive,expected)
  const out=path.join(report.work,'ui-restored');await bridge('extractArchive',report.groupArchive,out,null);check(out,expected)
  return{allMemberHashesMatch:true,inlinePreserved:true,batchModalPreserved:true,globalZipPreserved:true,historySourceCount:task.sourcePaths.length,archive:report.groupArchive}
 })
 await step('new members remain special folders in equal dual panes with text image preview and selective extraction',async()=>{
  await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',path.dirname(report.groupArchive),true);await fill('[data-testid="file-manager-path-input-right"]',report.work,true)
  await click(`[data-file-pane="left"] .file-row[data-path$=${JSON.stringify(path.basename(report.groupArchive))}]`,true)
  for(const entry of expected)await find(`[data-file-pane="left"] [data-entry-path=${JSON.stringify(entry.path)}]`)
  await shot('browser-wide',1100,650);await shot('browser-narrow',760,520)
  await click('[data-file-pane="left"] [data-entry-path="资料.txt"]',true);await find('[data-testid="archive-text-preview"]');await shot('text-narrow',760,520);await click('.preview-close')
  await click('[data-file-pane="left"] [data-entry-path="cover.png"]',true);await find('[data-testid="archive-image-preview"]');await click('.preview-close')
  const out=path.join(report.work,'selected');await bridge('extractArchive',report.groupArchive,out,null,{selected_entries:['notes (1).txt','empty.txt']});check(out,expected.filter(e=>['notes (1).txt','empty.txt'].includes(e.path)))
  return{equalPanes:true,selectiveEmptyFileRetained:true,textPreview:true,imageMemberReadable:true}
 })
 await step('password ZIP converts all top-level files to LZOP and back while folder input offers TAR.LZO without publishing loss',async()=>{
  const zip=path.join(report.work,'encrypted.zip'),names=['资料.txt','cover.png','empty.txt'],entries=expected.filter(e=>names.includes(e.path))
  if(!existsSync(zip)){const r=spawnSync(seven,['a','-tzip','-psource172','-mem=AES256',zip,...names],{cwd:generated,windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr)}
  const original=sha(zip);report.encryptedZip=zip
  if(!report.converted){report.converted=await bridge('convertArchive',zip,'source172',path.join(report.work,'converted.lzo'),{format:'lzo',level:1});save()}
  record(report.converted.primaryPath,entries);const out=path.join(report.work,'converted-restored');await bridge('extractArchive',report.converted.primaryPath,out,null);check(out,entries)
  if(!report.back){report.back=await bridge('convertArchive',report.converted.primaryPath,null,path.join(report.work,'back.7z'),{format:'7z',level:1,password:'target172',encrypt_filenames:true});save()}
  const back=path.join(report.work,'back-restored'),r=spawnSync(seven,['x','-y','-ptarget172','-o'+back,report.back.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr);check(back,entries);assert.equal(sha(zip),original)
  const folder=path.join(report.work,'keep-directory'),folderZip=path.join(report.work,'folder.zip');mkdirSync(path.join(folder,'empty'),{recursive:true});writeFileSync(path.join(folder,'a.txt'),'keep empty folders too')
  if(!existsSync(folderZip)){const r=spawnSync(seven,['a','-tzip',folderZip,folder],{windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr)}
  const before=sha(folderZip),target=path.join(report.work,'folder.lzo');let error='';try{await bridge('convertArchive',folderZip,null,target,{format:'lzo',level:1})}catch(e){error=String(e)}assert.match(error,/TAR.LZO/);assert.ok(!existsSync(target));assert.equal(sha(folderZip),before)
  if(!report.folderRetry){report.folderRetry=await bridge('convertArchive',folderZip,null,path.join(report.work,'folder.tar.lzo'),{format:'tar.lzo',level:1});save()}
  const restored=path.join(report.work,'folder-restored');await bridge('extractArchive',report.folderRetry.primaryPath,restored,null);assert.ok(existsSync(path.join(restored,'keep-directory/empty')))
  return{sourcePasswordOnly:true,allFilesConverted:true,sourcePreserved:true,folderFailure:error,folderRetryKeptEmptyDirectory:true}
 })
 await step('cancellation during a later member publishes no earlier member and the next creation succeeds',async()=>{
  const first=path.join(report.work,'first.txt'),later=path.join(report.work,'later.bin'),target=path.join(report.work,'cancel.lzo');writeFileSync(first,'first member is complete before cancelling')
  if(!existsSync(later)){const chunk=readFileSync(path.join(generated,'random.bin')),fd=openSync(later,'w');try{for(let i=0;i<192;i++)writeSync(fd,chunk)}finally{closeSync(fd)}}
  const digest=async p=>{const h=createHash('sha256');for await(const chunk of createReadStream(p))h.update(chunk);return h.digest('hex')},before=await digest(later)
  if(!report.cancel){const id=await bridge('startSevenZipCompression',[first,later],target,'lzo');await driver.wait(async()=>{const progress=await bridge('taskProgress',id);assert.equal(await bridge('taskStatus',id),'compressing');return progress>0},20000);await bridge('cancelTask',id);await driver.wait(async()=>await bridge('taskStatus',id)==='cancelled',20000);await driver.wait(()=>!existsSync(target),5000);report.cancel={id,activeLaterMember:true,published:false};save()}
  assert.equal(await digest(later),before)
  if(!report.retry){report.retry=await bridge('convertArchive',report.encryptedZip,'source172',path.join(report.work,'retry.lzo'),{format:'lzo',level:1});save()}
  record(report.retry.primaryPath,expected.filter(e=>['资料.txt','cover.png','empty.txt'].includes(e.path)))
  return{...report.cancel,sourceUnchanged:true,retry:report.retry.primaryPath}
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


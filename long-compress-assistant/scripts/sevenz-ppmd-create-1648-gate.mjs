import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runSevenzPpmd1648Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/sevenz-ppmd-create-1648'),file=path.join(base,'desktop.json'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.48',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true});const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[sevenz-ppmd-1648]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value)}
 const ipc=async(cmd,args)=>{const r=await driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args);assert.ok(r.ok,r.error);return r.value}
 const shot=async(name,width,height)=>{resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('[data-testid="compression-draft-config"],.compression-execution-panel')].map(e=>e.getBoundingClientRect().toJSON()),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts=report.layouts.filter(item=>item.name!==name);report.layouts.push({name,...layout});save()}
 await step('original inline edit automatic individual and batch modal in wide and small windows',async()=>{
  await bridge('clearCompressionWorkspace');await bridge('clearTasks');await click('[data-testid="nav-Compress"]');await bridge('queueDesktopDialogSelections',[[path.join(base,'source')]]);await click('[data-testid="dropzone-folder"]');await find('[data-testid="compression-draft-row"]')
  await bridge('queueDesktopDialogSelections',[[path.join(base,'source/empty.txt')]]);await click('[data-testid="dropzone-file"]');await click('[data-testid="open-global-compression-settings"]');assert.equal((await driver.findElements(By.css('[data-testid="compression-sevenz-method"]'))).length,0);await choose('[role="dialog"] [data-testid="compression-format-select"]','7Z');assert.match(await(await find('[role="dialog"] [data-testid="compression-sevenz-method"]')).getText(),/LZMA2/);await fill('[role="dialog"] [data-testid="compression-output-path"]',path.join(report.work,'created'));await click('[data-testid="save-global-compression-settings"]');await settle()
  await click('[data-testid="compression-draft-row"]');await choose('[data-testid="compression-draft-config"] [data-testid="compression-sevenz-method"]','PPMd');await shot('inline-wide',1100,650);await shot('inline-narrow',760,520)
  assert.equal((await driver.findElements(By.css('[role="dialog"]'))).length,0);await click('[data-testid="compression-analysis"] .analysis-button');await find('.analysis-metrics');await choose('[data-testid="compression-draft-config"] [data-testid="compression-sevenz-method"]','LZMA2 · 默认');await find('.analysis-stale');for(const box of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]')))await box.click();await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]');await choose('[data-testid="batch-sevenz-method"]','PPMd');await shot('batch-narrow',760,520);await click('[data-testid="batch-config-apply"]');await settle();await find('[data-testid="compression-draft-config"] [data-testid="compression-sevenz-method"]');await click('[data-testid="open-global-compression-settings"]');assert.match(await(await find('[role="dialog"] [data-testid="compression-sevenz-method"]')).getText(),/LZMA2/);await click('[data-testid="save-global-compression-settings"]');return{inlinePreserved:true,batchModalPreserved:true,globalLzma2Preserved:true,methodVisibleOnlyForSevenZip:true}
 })
 await step('UI Ppmd creation history and native preview extraction conversion keep selected target method',async()=>{
  await click('[data-testid="nav-Compress"]');const recover=await driver.findElements(By.css('[data-testid="pending-draft-recovery"] button'));for(const button of recover)if((await button.getText()).includes('恢复')){await button.click();await find('[data-testid="compression-draft-row"]');break}
  if(!report.created){await click('[data-testid="start-compression"]');const task=await driver.wait(async()=>{const rows=await bridge('taskHistory');return rows.find(r=>r.taskType==='compression'&&r.sourcePaths.includes(path.join(base,'source')))},60000);assert.equal(task.status,'completed',task.errorMessage);assert.equal(task.executionConfig.settings.sevenZipMethod,'ppmd');report.created=task.archiveOutput?.primaryPath||task.outputPath;save()}
  const p=report.created,entry='source/notes/说明 [1].txt',request={filePath:p,entryPath:entry,password:null,previewId:randomUUID()};const text=await ipc('preview_archive_text',request);assert.equal(createHash('sha256').update(text.content).digest('hex'),sha(path.join(base,'source/notes/说明 [1].txt')))
  const image=await ipc('preview_archive_image',{...request,entryPath:'source/gallery/01-cover.png',previewId:randomUUID()});assert.equal(createHash('sha256').update(Buffer.from(image.dataUrl.split(',')[1],'base64')).digest('hex'),sha(path.join(base,'source/gallery/01-cover.png')))
  const output=path.join(report.work,'ui-restored');await bridge('extractArchive',p,output,null);assert.equal(sha(path.join(output,entry)),sha(path.join(base,'source/notes/说明 [1].txt')))
  if(!report.zip){const zip=await bridge('convertArchive',p,null,path.join(report.work,'intermediate.zip'),{format:'zip',zip_method:'ppmd',level:6});report.zip=zip.primaryPath;save()}
  const intermediate=spawnSync(path.join(root,'src-tauri/resources/archive-engine/7z.exe'),['l','-slt',report.zip],{windowsHide:true,encoding:'utf8'});assert.equal(intermediate.status,0,intermediate.stdout+intermediate.stderr);assert.match(intermediate.stdout,/Method = PPMd\b/i)
  for(const method of ['ppmd']){const out=path.join(report.work,'converted-'+method+'.7z'),r=await bridge('convertArchive',report.zip,null,out,{format:'7z',level:1,seven_zip_method:method,password:'target1648',encrypt_filenames:true,create_solid_archive:true,split_size:8192});const check=spawnSync(path.join(root,'src-tauri/resources/archive-engine/7z.exe'),['l','-slt','-ptarget1648',r.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(check.status,0,check.stdout+check.stderr);assert.ok(check.stdout.includes('PPMD'));assert.ok(existsSync(out+'.002'));const preview=await ipc('preview_archive_text',{...request,filePath:out+'.002',password:'target1648',previewId:randomUUID()});assert.equal(preview.content,text.content)}
  return{creationMethod:'ppmd',historyPreserved:true,textPreviewHash:sha(path.join(base,'source/notes/说明 [1].txt')),extraction:true,zipPpmd8ToPasswordSolidNumericPpmdH:true}
 })
 report.passed=true;save()
}
function resizeHost(application,width,height,scale) {
  const escaped=application.replaceAll("'","''")
  const result=spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-Command',`Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class SevenzImageWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hwnd,int command);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][SevenzImageWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
$taskHwnd=(Get-Process -Id $taskApp.ProcessId).MainWindowHandle
if ([SevenzImageWindow]::IsIconic($taskHwnd)) { [void][SevenzImageWindow]::ShowWindow($taskHwnd,9) }
if (-not [SevenzImageWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

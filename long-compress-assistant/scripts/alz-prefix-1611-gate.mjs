import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

function resizeHost(application,width,height,scale) {
  const escaped=application.replaceAll("'","''")
  const result=spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-Command',`Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class AlzTextWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][AlzTextWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
if (-not [AlzTextWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

export async function runAlzPrefix1611Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/alz-prefix-1611'),file=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives/alz-prefix-1611')
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),helper=path.join(path.dirname(application),'resources/alz-engine/long-alz.exe')
 assert.equal(sha(helper),sha(path.join(root,'src-tauri/resources/alz-engine/long-alz.exe')))
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.11',applicationSha256:sha(application),helperSha256:sha(helper),controlled:true,checks:[],layouts:[],work:path.join(base,randomUUID())}
 if(report.applicationSha256!==sha(application)&&process.argv.includes('--alz-prefix-1611-resume-copy-fix')){
  assert.equal(report.passed,true);assert.equal(report.checks.length,5)
  const previous=report.applicationSha256
  report.previousCandidates=[...(report.previousCandidates||[]),{applicationSha256:previous,scope:'Five passed groups retained; only the scan-limit UI error copy changed.'}]
  for(const item of [...report.checks,...report.layouts])item.applicationSha256??=previous
  report.applicationSha256=sha(application);report.passed=false
 }
 assert.equal(report.applicationSha256,sha(application));assert.equal(report.helperSha256,sha(helper));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,applicationSha256:sha(application),...await fn()});save();console.log('[alz-prefix-1611]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
 const ipc=async(cmd,args)=>{const result=await invoke(cmd,args);assert.equal(result.ok,true,result.error);return result.value}
 const request=(source,extra={})=>({filePath:source,entryPath:'logs/session.txt',password:null,previewId:randomUUID(),...extra})
 const close=async()=>{for(const el of await driver.findElements(By.css('.preview-close')))if(await el.isDisplayed())await el.click()}
 const open=async source=>{await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="file-manager-open-archive"]');await find('[data-entry-path="logs/"]');await driver.actions().doubleClick(await find('[data-entry-path="logs/"]')).perform();await driver.actions().doubleClick(await find('[data-entry-path="logs/session.txt"]')).perform();await find('[data-testid="archive-entry-preview"]')}
 const scale=await driver.executeScript(()=>devicePixelRatio)
 const shot=async(name,width,height)=>{
  await settle();resizeHost(application,width,height,scale);await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
  const layout=await driver.executeScript(()=>{const r=document.querySelector('.preview-dialog').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:r.x,y:r.y,width:r.width,height:r.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width),closeVisible:!!document.querySelector('.preview-close')}})
  assert.ok(layout.overflow<=1&&layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2);assert.ok(layout.closeVisible)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,applicationSha256:sha(application),...layout});save()
 }
 await step('large text uses the original preview and explicitly stays partial in wide and narrow windows',async()=>{
  await open(path.join(fixtures,'deflate.alz'));await find('[data-testid="archive-text-preview"]')
  const meta=await(await find('.preview-meta')).getText();assert.match(meta,/部分内容.*未校验整文件/);assert.doesNotMatch(meta,/完整显示/)
  const text=await driver.executeScript(()=>{const e=document.querySelector('[data-testid="archive-text-preview"]');e.scrollTop=e.scrollHeight;const scroll=e.scrollTop>0;e.scrollTop=0;return{start:e.textContent.slice(0,90),length:e.textContent.length,scroll}})
  assert.match(text.start,/服务已就绪/);assert.ok(text.scroll&&text.length>100000)
  await shot('prefix-wide',1100,650);await shot('prefix-narrow',760,520);await close();return{text,meta}
 })
 await step('vault candidates and middle native volume read only a prefix',async()=>{
  await bridge('seedVaultPassword','a wrong prefix key','wrong');await bridge('seedVaultPassword','b public prefix key','日志密码1611!')
  const source=path.join(fixtures,'password-volumes.a00'),listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID(),zipPasswordEncoding:'utf8'})
  const value=await ipc('preview_archive_text',request(source,{zipPasswordEncoding:'utf8',sourceRevision:listing.revision}));assert.ok(value.truncated&&value.byteSize<=1048576&&value.totalSize>value.byteSize);assert.match(value.content,/服务已就绪/)
  const wrong=await invoke('preview_archive_text',request(source,{zipPasswordEncoding:'utf8',password:'wrong supplied key'}));assert.equal(wrong.ok,false);assert.match(wrong.error,/密码/)
  return{controlled:true,prefixBytes:value.byteSize,totalBytes:value.totalSize,wrongKey:wrong.error}
 })
 await step('content preceding a bad whole-file CRC stays explicitly unverified',async()=>{
  await open(path.join(fixtures,'bad-crc-later.alz'));await find('[data-testid="archive-text-preview"]');const meta=await(await find('.preview-meta')).getText();assert.match(meta,/部分内容.*未校验整文件/);await close();return{partial:true,meta,fullFailureCoveredByTargetedRust:true}
 })
 await step('actual compressed inputs above 64 MiB either yield an early prefix or a clear scan-limit result',async()=>{
  const value=await ipc('preview_archive_text',request(path.join(base,'large-store.alz')));assert.ok(value.truncated&&value.byteSize==1048576);assert.match(value.content,/large stored text/)
  await open(path.join(base,'scan-budget.alz'));const stage=await find('.preview-stage');await driver.wait(async()=>/64 MiB/.test(await stage.getText()),30000);assert.match(await stage.getText(),/提取/);assert.equal((await driver.findElements(By.css('[data-testid="archive-text-preview"]'))).length,0)
  await shot('scan-limit-narrow',760,520);await close();return{actualLargeSource:true,largePrefixBytes:value.byteSize,scanLimit:true}
 })
 await step('early cancel and replaced source never publish a prefix; a new request still works',async()=>{
  const source=path.join(report.work,'changed.alz');copyFileSync(path.join(fixtures,'deflate.alz'),source);const listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID()});appendFileSync(source,Buffer.from([0]))
  const changed=await invoke('preview_archive_text',request(source,{sourceRevision:listing.revision}));assert.equal(changed.ok,false);assert.match(changed.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stopped=await invoke('preview_archive_text',request('not-opened.alz',{previewId:id}));assert.equal(stopped.ok,false);assert.match(stopped.error,/取消/)
  const next=await ipc('preview_archive_text',request(path.join(fixtures,'utf8-boundary.alz')));assert.equal(next.encoding,'UTF-8');assert.equal(next.byteSize,1048575);assert.ok(next.truncated&&!next.content.includes('�'))
  return{changed:changed.error,cancelled:stopped.error,newPrefixBytes:next.byteSize}
 })
 await step('scan limit gives an extraction hint without internal I/O markers',async()=>{
  await open(path.join(base,'scan-budget.alz'));const stage=await find('.preview-stage');await driver.wait(async()=>/64 MiB/.test(await stage.getText()),30000)
  const message=await stage.getText();assert.match(message,/已停止预览.*请先提取后打开/);assert.doesNotMatch(message,/archive-inspection|alz-io|文件读写失败/)
  await shot('scan-limit-final-narrow',760,520);await close();return{message,displayOnlyContinuation:true}
 })
 report.passed=true;save()
}

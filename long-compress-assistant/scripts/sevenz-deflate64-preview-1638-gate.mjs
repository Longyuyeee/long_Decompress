import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By } from 'selenium-webdriver'

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

export async function runSevenzDeflate641638Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/sevenz-deflate64-preview-1638'),receipt=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives/sevenz-deflate64-preview-1638'),m=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),source=name=>{const c=m.cases.find(c=>c.name===name);return path.join(fixtures,name,c.members[0].name)}
 const report=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.6.38',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(receipt,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[sevenz-deflate64-1638]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
 const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
 const scale=await driver.executeScript(()=>devicePixelRatio)
 const shot=async(name,width,height,reader=false)=>{
  await settle();resizeHost(application,width,height,scale)
  await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
  const layout=await driver.executeScript(reader=>{const dialog=document.querySelector(reader?'.image-reader':'.preview-dialog').getBoundingClientRect(),image=document.querySelector(reader?'[data-testid="reader-image"]':'.preview-image');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:dialog.x,y:dialog.y,width:dialog.width,height:dialog.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width),image:image?{width:image.naturalWidth,height:image.naturalHeight}:null,closeVisible:!!document.querySelector(reader?'[data-testid="reader-close"]':'.preview-close')}},reader)
  assert.ok(layout.overflow<=1);assert.ok(layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1)
  assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2);assert.ok(layout.closeVisible);if(name!=='text-wide')assert.ok(layout.image?.width>0)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts=report.layouts.filter(x=>x.name!==name);report.layouts.push({name,...layout});save()
 }
 const readyImage=async(reader=false)=>{await find(reader?'[data-testid="reader-image"]':'.preview-image');await driver.wait(()=>driver.executeScript(reader=>{const e=document.querySelector(reader?'[data-testid="reader-image"]':'.preview-image');return e?.complete&&e.naturalWidth>0},reader),15000)}
 const close=async()=>{for(const selector of ['[data-testid="reader-close"]','.preview-close'])for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())await el.click()}
 const open=async(p=source('deflate64'))=>{resizeHost(application,1100,650,scale);await driver.wait(()=>driver.executeScript(()=>innerWidth===1100&&innerHeight===650),10000);await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[p]]);await click('[data-testid="file-manager-open-archive"]');await find('[data-entry-path="gallery/"]');await driver.actions().doubleClick(await find('[data-entry-path="gallery/"]')).perform();await find('[data-entry-path="gallery/01-cover.png"]')}
 const read=async()=>{await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage();await click('[data-testid="archive-preview-read-images"]');await readyImage(true)}


 const textRequest=(p,entry=m.selected,extra={})=>({filePath:p,entryPath:entry,password:null,previewId:randomUUID(),...extra})
 const openText=async(p=source('deflate64'))=>{await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[p]]);await click('[data-testid="file-manager-open-archive"]');await driver.actions().doubleClick(await find('[data-entry-path="notes/"]')).perform();await driver.actions().doubleClick(await find('[data-entry-path="'+m.selected+'"]')).perform();await find('[data-testid="archive-text-preview"]')}
 await step('complete selected Deflate64 text is exact scrollable and stays in equal wide panes',async()=>{
  resizeHost(application,1100,650,scale);await openText();const meta=await(await find('.preview-meta')).getText();assert.match(meta,/完整显示/);assert.doesNotMatch(meta,/部分内容/)
  const text=await driver.executeScript(()=>document.querySelector('[data-testid="archive-text-preview"]').textContent);assert.equal(createHash('sha256').update(text).digest('hex'),m.textSha256)
  const scroll=await driver.executeScript(()=>{const e=document.querySelector('[data-testid="archive-text-preview"]');e.scrollTop=e.scrollHeight;const v=e.scrollTop>0;e.scrollTop=0;return v});assert.ok(scroll);await shot('text-wide',1100,650);await close();return{meta,sha256:m.textSha256,scroll}
 })
 await step('Deflate64 image pixels render in the original narrow modal',async()=>{
  await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage()
  const pixel=await driver.executeScript(()=>{const i=document.querySelector('.preview-image'),c=document.createElement('canvas');c.width=i.naturalWidth;c.height=i.naturalHeight;const ctx=c.getContext('2d');ctx.drawImage(i,0,0);return [...ctx.getImageData(27,7,1,1).data]});assert.deepEqual(pixel,[140,20,60,255]);await shot('image-narrow',760,520);await close();return{pixel,png:[320,240]}
 })
 await step('original reader turns zooms skips broken raster locates and extracts only current page',async()=>{
  await read();assert.equal(await(await find('[data-testid="reader-total"]')).getText(),'/ 6');await click('[data-testid="reader-next"]');await readyImage(true);await click('[data-testid="reader-original"]');await click('[data-testid="reader-zoom-in"]');assert.equal(await(await find('[data-testid="reader-zoom"]')).getText(),'125%');await click('[data-testid="reader-fit"]');await shot('reader-narrow',760,520,true)
  await click('[data-testid="reader-next"]');await find('[data-testid="reader-error"]');await click('[data-testid="reader-skip"]');await readyImage(true);await click('[data-testid="reader-locate"]');assert.match(await(await find('[data-entry-path="gallery/04-cover.jpg"]')).getAttribute('class'),/focused/);await close()
  // Extract deliberately returns to the task/list context; reopen for this action.
  await read();await click('[data-testid="reader-next"]');await readyImage(true)
  const out=path.join(report.work,'current-page-'+randomUUID());mkdirSync(out,{recursive:true});await bridge('queueDesktopDialogSelections',[out]);await click('[data-testid="reader-extract"]');const file=path.join(out,'gallery/02-grid.bmp');await driver.wait(()=>existsSync(file),20000);assert.equal(sha(file),m.images[1].sha256);assert.deepEqual(readdirSync(path.dirname(file)),['02-grid.bmp']);await close();return{zoom:true,skip:true,locate:true,currentPageSha256:sha(file)}
 })
 await step('original maximum and non-first volume feed complete text and both image commands while invalid endings have next steps',async()=>{
  const c=m.cases.find(c=>c.name==='volumes'),member=path.join(fixtures,'volumes',c.members[Math.floor(c.members.length/2)].name)
  for(const p of [source('maximum'),member]){const listing=await ipc('browse_archive',{filePath:p,browseId:randomUUID()});const v=await ipc('preview_archive_text',textRequest(p,m.selected,{sourceRevision:listing.revision}));assert.equal(v.truncated,false);assert.equal(createHash('sha256').update(v.content).digest('hex'),m.textSha256)
   for(const cmd of ['preview_archive_image','read_archive_image']){const im=await ipc(cmd,{...textRequest(p,'gallery/01-cover.png'),sourceRevision:listing.revision,requestId:randomUUID(),revision:listing.revision});assert.equal(createHash('sha256').update(Buffer.from(im.dataUrl.split(',')[1],'base64')).digest('hex'),m.images[0].sha256)}
  }
  const errors={};for(const[name,entry,key,reason]of[['bad-data',m.selected,null,/Deflate64/],['solid',m.selected,null,/Deflate64/]]){const r=await invoke('preview_archive_text',textRequest(path.join(fixtures,name+'.7z'),entry,{password:key}));assert.equal(r.ok,false);assert.match(r.error,reason);errors[name]=r.error}


  return{maximum:true,nonFirstMember:true,readerCommand:true,errors}
 })
 await step('cancelled changed and missing-volume requests cannot replace fresh Deflate64 content and unrelated large data is skipped',async()=>{
  const p=path.join(report.work,'changed.7z');copyFileSync(source('deflate64'),p);const listing=await ipc('browse_archive',{filePath:p,browseId:randomUUID()});appendFileSync(p,Buffer.from([0]));const stale=await invoke('preview_archive_text',textRequest(p,m.selected,{sourceRevision:listing.revision}));assert.equal(stale.ok,false);assert.match(stale.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stopped=await invoke('preview_archive_text',textRequest(source('deflate64'),m.selected,{previewId:id}));assert.equal(stopped.ok,false);assert.match(stopped.error,/取消/)
  const c=m.cases.find(c=>c.name==='volumes'),members=c.members.map(f=>{const p=path.join(report.work,f.name);copyFileSync(path.join(fixtures,'volumes',f.name),p);return p}),snapshot=await ipc('browse_archive',{filePath:members[1],browseId:randomUUID()});rmSync(members.at(-1));const missing=await invoke('preview_archive_text',textRequest(members[1],m.selected,{sourceRevision:snapshot.revision}));assert.equal(missing.ok,false);assert.match(missing.error,/IMAGE_READER_CHANGED/)
  const fresh=await ipc('preview_archive_text',textRequest(source('deflate64')));assert.equal(fresh.truncated,false);const big=await ipc('preview_archive_text',textRequest(path.join(base,'large-unrelated.7z')));assert.equal(createHash('sha256').update(big.content).digest('hex'),m.textSha256);return{stale:stale.error,cancelled:stopped.error,missingTail:missing.error,unrelatedLargeSkipped:true,fresh:true}
 })
 report.passed=true;save()
}

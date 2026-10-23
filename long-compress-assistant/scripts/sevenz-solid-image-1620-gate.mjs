import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

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

export async function runSevenzSolidImage1620Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/sevenz-solid-image-1620'),receipt=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives/sevenz-solid-image-1620'),m=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),source=name=>{const c=m.cases.find(c=>c.name===name);return path.join(fixtures,name,c.members[0].name)}
 const report=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.6.20',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(receipt,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[sevenz-solid-image-1620]',name)}
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
  assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2);assert.ok(layout.closeVisible);assert.ok(layout.image?.width>0)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()
 }
 const readyImage=async(reader=false)=>{await find(reader?'[data-testid="reader-image"]':'.preview-image');await driver.wait(()=>driver.executeScript(reader=>{const e=document.querySelector(reader?'[data-testid="reader-image"]':'.preview-image');return e?.complete&&e.naturalWidth>0},reader),15000)}
 const close=async()=>{for(const selector of ['[data-testid="reader-close"]','.preview-close'])for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())await el.click()}
 const open=async(p=source('lzma2'))=>{resizeHost(application,1100,650,scale);await driver.wait(()=>driver.executeScript(()=>innerWidth===1100&&innerHeight===650),10000);await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[p]]);await click('[data-testid="file-manager-open-archive"]');await find('[data-entry-path="gallery/"]');await driver.actions().doubleClick(await find('[data-entry-path="gallery/"]')).perform();await find('[data-entry-path="gallery/01-cover.png"]')}
 const read=async()=>{await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage();await click('[data-testid="archive-preview-read-images"]');await readyImage(true)}
 await step('selected original PNG renders exact pixels in the original wide and narrow panes',async()=>{
  await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage()
  const pixel=await driver.executeScript(()=>{const img=document.querySelector('.preview-image'),c=document.createElement('canvas');c.width=img.naturalWidth;c.height=img.naturalHeight;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);return [...ctx.getImageData(27,7,1,1).data]});assert.deepEqual(pixel,[140,20,60,255])
  await shot('image-wide',1100,650);await shot('image-narrow',760,520);await close();return{pixel,png:[320,240],equalPanes:true}
 })
 await step('original reader turns pages zooms skips bad content and locates JPEG without preloading',async()=>{
  await read();assert.match(await(await find('[data-testid="reader-total"]')).getText(),/6/)
  await click('[data-testid="reader-next"]');await readyImage(true);assert.match(await(await find('[data-testid="reader-current-path"]')).getText(),/02-grid.bmp/)
  await click('[data-testid="reader-original"]');await click('[data-testid="reader-zoom-in"]');assert.match(await(await find('[data-testid="reader-zoom"]')).getText(),/125%/);await click('[data-testid="reader-fit"]');await shot('reader-narrow',760,520,true)
  await click('[data-testid="reader-next"]');await find('[data-testid="reader-error"]');await click('[data-testid="reader-skip"]');await readyImage(true);assert.match(await(await find('[data-testid="reader-current-path"]')).getText(),/04-cover.jpg/)
  await click('[data-testid="reader-locate"]');const focused=await find('[data-entry-path="gallery/04-cover.jpg"]');assert.match(await focused.getAttribute('class'),/focused/);await close();return{bmp:[96,64],jpeg:[320,240],zoom:true,skip:true,locate:true}
 })
 await step('current page extraction keeps the exact selected 7Z path and content',async()=>{
  await read();await click('[data-testid="reader-next"]');await readyImage(true)
  const out=path.join(report.work,'current-page');mkdirSync(out,{recursive:true});await bridge('queueDesktopDialogSelections',[out]);await click('[data-testid="reader-extract"]')
  const file=path.join(out,'gallery/02-grid.bmp');await driver.wait(()=>existsSync(file),20000);assert.equal(sha(file),m.images[1].sha256);assert.deepEqual(readdirSync(path.dirname(file)),['02-grid.bmp']);await close();return{currentPageOnly:true,sha256:sha(file)}
 })
 await step('solid LZMA and non-first numeric volume feed preview and reader with exact selected bytes',async()=>{
  const c=m.cases.find(c=>c.name==='volumes'),member=path.join(fixtures,'volumes',c.members[Math.floor(c.members.length/2)].name)
  for(const p of [source('lzma'),member]){
   const listing=await ipc('browse_archive',{filePath:p,browseId:randomUUID()})
   for(const cmd of ['preview_archive_image','read_archive_image']){const value=await ipc(cmd,{filePath:p,entryPath:'gallery/01-cover.png',password:null,previewId:randomUUID(),sourceRevision:listing.revision,requestId:randomUUID(),revision:listing.revision});assert.equal(createHash('sha256').update(Buffer.from(value.dataUrl.split(',')[1],'base64')).digest('hex'),m.images[0].sha256)}
  }
  return{lzma:true,nonFirstNumericMember:true,previewAndReader:true}
 })
 await step('shared limits damaged ending encryption cancellation and changed sources keep clear next steps',async()=>{
  const errors={}
  for(const[name,entry,password,reason]of[['oversize','oversize.bmp',null,/8 MiB/],['oversize-shared','preview.png',null,/64 MiB/],['dimensions','dimensions.png',null,/dimensions exceed/],['encrypted','gallery/01-cover.png','7Z固实图片1620!',/加密固实/]]){const p=name==='encrypted'?source(name):path.join(fixtures,name+'.7z');const r=await invoke('preview_archive_image',{filePath:p,entryPath:entry,password});assert.equal(r.ok,false);assert.match(r.error,reason);errors[name]=r.error}
  await open(path.join(fixtures,'bad-end.7z'));await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await find('[data-testid="archive-entry-preview"]');const stage=await find('.preview-stage');await driver.wait(async()=>/CRC|校验/.test(await stage.getText()),15000);const damaged=await stage.getText();assert.doesNotMatch(damaged,/sevenz-preview|archive-inspection/);assert.equal((await driver.findElements(By.css('.preview-image'))).length,0);await close()
  const p=path.join(report.work,'changed.7z');copyFileSync(source('lzma2'),p);const listing=await ipc('browse_archive',{filePath:p,browseId:randomUUID()});appendFileSync(p,Buffer.from([0]));const changed=await invoke('preview_archive_image',{filePath:p,entryPath:'gallery/01-cover.png',sourceRevision:listing.revision});assert.equal(changed.ok,false);assert.match(changed.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stopped=await invoke('read_archive_image',{filePath:source('lzma2'),entryPath:'gallery/01-cover.png',requestId:id,revision:listing.revision});assert.equal(stopped.ok,false);assert.match(stopped.error,/CANCELLED/)
  const large=await ipc('preview_archive_image',{filePath:path.join(base,'large-unrelated.7z'),entryPath:'gallery/01-cover.png'});assert.equal(large.width,320);return{errors,damaged,changed:changed.error,cancelled:stopped.error,unrelatedLargeSkipped:true}
 })
 report.passed=true;save()
}

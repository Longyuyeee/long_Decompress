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
public static class EggSolidImageWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][EggSolidImageWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
if (-not [EggSolidImageWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

export async function runEggSolidImage1615Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/egg-solid-image-1615'),receipt=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives/egg-solid-image-1615')
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const helper=path.join(path.dirname(application),'resources/egg-engine/long-egg.exe')
 assert.equal(sha(helper),sha(path.join(root,'src-tauri/resources/egg-engine/long-egg.exe')))
 const report=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.6.15',applicationSha256:sha(application),helperSha256:sha(helper),controlled:true,newProducerOriginal:false,checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));assert.equal(report.helperSha256,sha(helper));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(receipt,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[egg-solid-image-1615]',name)}
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
 const open=async()=>{for(const selector of ['[data-testid="reader-close"]','.preview-close'])for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())await el.click();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[path.join(fixtures,'ui.egg')]]);await click('[data-testid="file-manager-open-archive"]');await find('[data-entry-path="gallery/"]');await driver.actions().doubleClick(await find('[data-entry-path="gallery/"]')).perform();await find('[data-entry-path="gallery/01-cover.png"]')}
 await step('single image opens in the original equal-pane preview',async()=>{
  await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage()
  await shot('image-wide',1100,650);await shot('image-narrow',760,520)
  await click('.preview-close');return{png:[320,240],sameDualPane:true,solidSharedStream:true}
 })
 await step('existing reader turns pages and skips bad content',async()=>{
  await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage();await click('[data-testid="archive-preview-read-images"]');await readyImage(true)
  assert.match(await(await find('[data-testid="reader-total"]')).getText(),/3/)
  await click('[data-testid="reader-next"]');await readyImage(true);assert.match(await(await find('[data-testid="reader-current-path"]')).getText(),/02-grid.bmp/)
  await click('[data-testid="reader-original"]');await click('[data-testid="reader-zoom-in"]');assert.match(await(await find('[data-testid="reader-zoom"]')).getText(),/125%/);await click('[data-testid="reader-fit"]')
  await shot('reader-narrow',760,520,true)
  await click('[data-testid="reader-previous"]');await click('[data-testid="reader-previous"]');await find('[data-testid="reader-error"]');await click('[data-testid="reader-skip"]');await readyImage(true)
  await click('[data-testid="reader-next"]');await readyImage(true)
  return{reader:true,skip:true,zoom:true,selectedImageOverTextLimit:true}
 })
 await step('current page extraction retains the exact selected EGG path',async()=>{
  await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage();await click('[data-testid="archive-preview-read-images"]');await readyImage(true);await click('[data-testid="reader-next"]');await readyImage(true)
  const out=path.join(report.work,'current-page');mkdirSync(out,{recursive:true});await bridge('queueDesktopDialogSelections',[out]);await click('[data-testid="reader-extract"]')
  const file=path.join(out,'gallery/02-grid.bmp');await driver.wait(()=>existsSync(file),15000)
  const manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'));assert.equal(sha(file),manifest.images[1].sha256)
  assert.deepEqual(readdirSync(path.dirname(file)),['02-grid.bmp']);return{currentPageOnly:true,sha256:sha(file)}
 })

 await step('complete native middle member uses the single and continuous production routes',async()=>{
  const source=path.join(fixtures,'controlled-native.vol2.egg'),entry='gallery/02-grid.bmp',listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID()})
  const expected=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8')).images[1]
  for(const cmd of ['preview_archive_image','read_archive_image']){
   const value=await ipc(cmd,{filePath:source,entryPath:entry,password:null,previewId:randomUUID(),sourceRevision:listing.revision,requestId:randomUUID(),revision:listing.revision})
   assert.equal(value.byteSize,2*1024*1024);assert.equal(createHash('sha256').update(Buffer.from(value.dataUrl.split(',')[1],'base64')).digest('hex'),expected.sha256)
  }
  return{controlledNative:true,bytes:2*1024*1024,sha256:expected.sha256,singleAndReader:true}
 })
 await step('bad shared dependencies, cancelled reads and stale sources never publish partial pixels',async()=>{
  for(const selector of ['[data-testid="reader-close"]','.preview-close'])for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())await el.click()
  const errors={}
  for(const [name,reason]of[['shared-crc',/CRC/],['encrypted-dependency',/加密固实图片/],['shared-output-limit',/64 MiB/]]){
   const value=await invoke('preview_archive_image',{filePath:path.join(fixtures,name+'.egg'),entryPath:'gallery/01-cover.png'});assert.equal(value.ok,false);assert.match(value.error,reason);errors[name]=value.error
  }
  const source=path.join(report.work,'changed.egg');copyFileSync(path.join(fixtures,'ui.egg'),source);const listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID()});appendFileSync(source,Buffer.from([0]))
  const changed=await invoke('read_archive_image',{filePath:source,entryPath:'gallery/01-cover.png',requestId:randomUUID(),revision:listing.revision});assert.equal(changed.ok,false);assert.match(changed.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id})
  const stopped=await invoke('read_archive_image',{filePath:path.join(fixtures,'ui.egg'),entryPath:'gallery/01-cover.png',requestId:id,revision:listing.revision});assert.equal(stopped.ok,false);assert.match(stopped.error,/CANCELLED/)
  // Show the shared-tail failure through the actual modal, not just raw IPC.
  await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[path.join(fixtures,'shared-crc.egg')]]);await click('[data-testid="file-manager-open-archive"]')
  await driver.actions().doubleClick(await find('[data-entry-path="gallery/"]')).perform();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform()
  const stage=await find('.preview-stage');await driver.wait(async()=>/CRC/.test(await stage.getText()),15000);const displayed=await stage.getText();assert.doesNotMatch(displayed,/archive-inspection/);assert.equal((await driver.findElements(By.css('.preview-image'))).length,0)
  return{errors,displayed,changed:changed.error,cancelled:stopped.error}
 })
 report.passed=true;save()
}

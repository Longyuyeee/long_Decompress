import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
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

export async function runAlzImage1610Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/alz-image-1610'),receipt=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives/alz-image-1610')
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const helper=path.join(path.dirname(application),'resources/alz-engine/long-alz.exe')
 assert.equal(sha(helper),sha(path.join(root,'src-tauri/resources/alz-engine/long-alz.exe')))
 const report=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.6.10',applicationSha256:sha(application),helperSha256:sha(helper),controlled:true,newProducerOriginal:false,checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));assert.equal(report.helperSha256,sha(helper));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(receipt,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[alz-image-1610]',name)}
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
 const open=async()=>{await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[path.join(fixtures,'ui.alz')]]);await click('[data-testid="file-manager-open-archive"]');await find('[data-entry-path="gallery/"]');await driver.actions().doubleClick(await find('[data-entry-path="gallery/"]')).perform();await find('[data-entry-path="gallery/01-cover.png"]')}
 await step('single image opens in the original equal-pane preview',async()=>{
  await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage()
  await shot('image-wide',1100,650);await shot('image-narrow',760,520)
  await click('.preview-close');return{png:[320,240],sameDualPane:true}
 })
 await step('existing reader turns pages and skips bad content',async()=>{
  await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage();await click('[data-testid="archive-preview-read-images"]');await readyImage(true)
  assert.match(await(await find('[data-testid="reader-total"]')).getText(),/3/)
  await click('[data-testid="reader-next"]');await readyImage(true);assert.match(await(await find('[data-testid="reader-current-path"]')).getText(),/02-grid.bmp/)
  await click('[data-testid="reader-original"]');await click('[data-testid="reader-zoom-in"]');assert.match(await(await find('[data-testid="reader-zoom"]')).getText(),/125%/);await click('[data-testid="reader-fit"]')
  await shot('reader-narrow',760,520,true)
  await click('[data-testid="reader-previous"]');await click('[data-testid="reader-previous"]');await find('[data-testid="reader-error"]');await click('[data-testid="reader-skip"]');await readyImage(true)
  await click('[data-testid="reader-next"]');await readyImage(true)
  return{reader:true,skip:true,zoom:true}
 })
 await step('current page extraction retains the exact selected ALZ path',async()=>{
  await open();await driver.actions().doubleClick(await find('[data-entry-path="gallery/01-cover.png"]')).perform();await readyImage();await click('[data-testid="archive-preview-read-images"]');await readyImage(true);await click('[data-testid="reader-next"]');await readyImage(true)
  const out=path.join(report.work,'current-page');mkdirSync(out,{recursive:true});await bridge('queueDesktopDialogSelections',[out]);await click('[data-testid="reader-extract"]')
  const file=path.join(out,'gallery/02-grid.bmp');await driver.wait(()=>existsSync(file),15000)
  const manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'));assert.equal(sha(file),manifest.images[1].sha256)
  assert.deepEqual(readdirSync(path.dirname(file)),['02-grid.bmp']);return{currentPageOnly:true,sha256:sha(file)}
 })
 await step('password vault and middle native member preview the exact complete image',async()=>{
  const source=path.join(fixtures,'password-volumes.a00');await bridge('seedVaultPassword','ALZ image controlled public','图片密码1610!')
  const listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID(),zipPasswordEncoding:'utf8'})
  const result=await ipc('preview_archive_image',{filePath:source,entryPath:'相册/01-封面.png',password:null,zipPasswordEncoding:'utf8',previewId:randomUUID(),sourceRevision:listing.revision})
  const bytes=Buffer.from(result.dataUrl.split(',')[1],'base64'),manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
  assert.equal(createHash('sha256').update(bytes).digest('hex'),manifest.images[0].sha256)
  const wrong=await invoke('preview_archive_image',{filePath:source,entryPath:'相册/01-封面.png',password:'wrong',zipPasswordEncoding:'utf8'});assert.equal(wrong.ok,false);assert.match(wrong.error,/密码/)
  return{controlled:true,original:false,middleMember:true,vault:true,bytes:bytes.length,wrongKey:wrong.error}
 })
 await step('limits, stale source and early cancellation stay distinct; text budget is unchanged',async()=>{
  const errors={}
  for(const[name,reason]of[['oversize',/8 MiB/],['dimensions',/dimensions exceed/],['crc',/CRC/]]){const result=await invoke('preview_archive_image',{filePath:path.join(fixtures,name+'.alz'),entryPath:'preview.png',password:null});assert.equal(result.ok,false);assert.match(result.error,reason);errors[name]=result.error}
  const source=path.join(report.work,'changed.alz');copyFileSync(path.join(fixtures,'ui.alz'),source);const listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID()});appendFileSync(source,Buffer.from([0]))
  const changed=await invoke('preview_archive_image',{filePath:source,entryPath:'gallery/01-cover.png',sourceRevision:listing.revision});assert.equal(changed.ok,false);assert.match(changed.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stopped=await invoke('preview_archive_image',{filePath:'not-opened.alz',entryPath:'not-opened.png',previewId:id});assert.equal(stopped.ok,false);assert.match(stopped.error,/CANCELLED/)
  const text=await ipc('preview_archive_text',{filePath:path.join(root,'tests/fixtures/archives/alz-text-169/store.alz'),entryPath:'docs/readme.txt'});assert.match(text.content,/包内说明/)
  const tooBig=await invoke('preview_archive_text',{filePath:path.join(fixtures,'at-limit.alz'),entryPath:'preview.bmp'});assert.equal(tooBig.ok,false);assert.match(tooBig.error,/1 MiB/)
  return{errors,changed:changed.error,cancelled:stopped.error,textStillBounded:true}
 })
 report.passed=true;save()
}

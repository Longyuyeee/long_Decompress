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

export async function runSevenzEncryptedSolidText1621Gate({driver,bridge,root,application}){
 const base=path.join(root,'test-results/sevenz-encrypted-solid-text-1621'),file=path.join(base,'desktop.json'),sample=path.join(root,'tests/fixtures/archives/sevenz-encrypted-solid-text-1621'),m=JSON.parse(readFileSync(path.join(sample,'manifest.json'),'utf8'))
 const selected='docs/02-说明 [1].txt',key='7Z加密固实1621!',source=name=>{const c=m.cases.find(c=>c.name===name);return path.join(sample,name,c.members[0].name)}
 const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.21',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[sevenz-encrypted-solid-text-1621]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
 const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
 const request=(p,entry=selected,extra={})=>({filePath:p,entryPath:entry,password:null,previewId:randomUUID(),...extra})
 const close=async()=>{for(const el of await driver.findElements(By.css('.preview-close')))if(await el.isDisplayed())await el.click()}
 const open=async(p,entry=selected)=>{resizeHost(application,1100,650,scale);await driver.wait(()=>driver.executeScript(()=>innerWidth===1100&&innerHeight===650),10000);await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[p]]);await click('[data-testid="file-manager-open-archive"]');const parts=entry.split('/');for(let i=1;i<parts.length;i++)await driver.actions().doubleClick(await find('[data-entry-path="'+parts.slice(0,i).join('/')+'/"]')).perform();await driver.actions().doubleClick(await find('[data-entry-path="'+entry+'"]')).perform();await find('[data-testid="archive-entry-preview"]')}
 const scale=await driver.executeScript(()=>devicePixelRatio)
 const shot=async(name,width,height)=>{
  await settle();resizeHost(application,width,height,scale);await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
  const layout=await driver.executeScript(()=>{const r=document.querySelector('.preview-dialog').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:r.x,y:r.y,width:r.width,height:r.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width)}})
  assert.ok(layout.overflow<=1&&layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()
 }
 await step('header password opens via the original vault and complete selected text fits equal wide and narrow panes',async()=>{
  const missing=await invoke('browse_archive',{filePath:source('headers'),browseId:randomUUID()});assert.equal(missing.ok,false);assert.match(missing.error,/ARCHIVE_BROWSE_PASSWORD/)
  await bridge('seedVaultPassword','a wrong encrypted solid','wrong');await bridge('seedVaultPassword','b public encrypted solid',key)
  await open(source('headers'));await find('[data-testid="archive-text-preview"]');const meta=await(await find('.preview-meta')).getText();assert.match(meta,/完整显示/)
  const text=await(await find('[data-testid="archive-text-preview"]')).getAttribute('textContent');assert.equal(createHash('sha256').update(text).digest('hex'),m.textSha256)
  const scroll=await driver.executeScript(()=>{const e=document.querySelector('[data-testid="archive-text-preview"]');e.scrollTop=e.scrollHeight;const ok=e.scrollTop>0;e.scrollTop=0;return ok});assert.ok(scroll)
  await shot('encrypted-text-wide',1100,650);await shot('encrypted-text-narrow',760,520);await close();return{missing:missing.error,meta,scroll,sha256:m.textSha256}
 })
 await step('both methods and non-first filename-encrypted member use metadata-only then selected vault reads',async()=>{
  const c=m.cases.find(c=>c.name==='headers-volumes'),member=path.join(sample,'headers-volumes',c.members[Math.floor(c.members.length/2)].name)
  for(const p of [source('content-lzma'),source('content-lzma2'),member,path.join(sample,'controlled-bzip2-header.7z')]){const listing=await ipc('browse_archive',{filePath:p,browseId:randomUUID()});assert.equal(listing.entries.find(e=>e.path===selected).compressedSize,null);const value=await ipc('preview_archive_text',request(p,selected,{sourceRevision:listing.revision}));assert.equal(value.truncated,false);assert.equal(createHash('sha256').update(value.content).digest('hex'),m.textSha256)}
  return{lzma:true,lzma2:true,nonFirstHeaderMember:true,sharedSizeUnattributed:true}
 })
 await step('wrong manual key damage and oversized dependency never publish partial text',async()=>{
  const errors={};for(const[name,p,password,entry,reason]of[['manual',source('headers'),'wrong manual',selected,/密码/],['damaged',path.join(sample,'damaged-data.7z'),key,selected,/密码|校验/],['oversize',path.join(sample,'oversize-shared.7z'),key,'readme.txt',/64 MiB/]]){const r=await invoke('preview_archive_text',request(p,entry,{password}));assert.equal(r.ok,false);assert.match(r.error,reason);errors[name]=r.error}
  return errors
 })
 await step('unrelated corrupt 65 MiB data is not consulted when browsing and reading selected encrypted text',async()=>{
  const p=path.join(base,'unrelated.7z'),listing=await ipc('browse_archive',{filePath:p,browseId:randomUUID()});const value=await ipc('preview_archive_text',request(p,selected,{sourceRevision:listing.revision}));assert.equal(value.truncated,false);assert.equal(createHash('sha256').update(value.content).digest('hex'),m.textSha256);return{metadataOnly:true,selectedOnly:true}
 })
 await step('changed encrypted source and cancelled request are discarded before fresh full text',async()=>{
  const changed=path.join(report.work,'changed.7z');copyFileSync(source('headers'),changed);const listing=await ipc('browse_archive',{filePath:changed,browseId:randomUUID()});appendFileSync(changed,Buffer.from([0]))
  const stale=await invoke('preview_archive_text',request(changed,selected,{sourceRevision:listing.revision}));assert.equal(stale.ok,false);assert.match(stale.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stopped=await invoke('preview_archive_text',request(source('headers'),selected,{previewId:id}));assert.equal(stopped.ok,false);assert.match(stopped.error,/取消/)
  const fresh=await ipc('preview_archive_text',request(source('headers')));assert.equal(fresh.truncated,false);return{stale:stale.error,cancelled:stopped.error,freshBytes:fresh.byteSize}
 })
 report.passed=true;save()
}

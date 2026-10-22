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
public static class EggSolidTextWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][EggSolidTextWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
if (-not [EggSolidTextWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

export async function runEggSolidText1614Gate({driver,bridge,root,application}){
 const base=path.join(root,'test-results/egg-solid-text-1614'),file=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives'),sample=path.join(fixtures,'egg-solid-text-1614')
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),helper=path.join(path.dirname(application),'resources/egg-engine/long-egg.exe')
 assert.equal(sha(helper),sha(path.join(root,'src-tauri/resources/egg-engine/long-egg.exe')))
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.14',applicationSha256:sha(application),helperSha256:sha(helper),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));assert.equal(report.helperSha256,sha(helper));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[egg-solid-text-1614]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
 const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
 const request=(source,entry='readme.txt',extra={})=>({filePath:source,entryPath:entry,password:null,previewId:randomUUID(),...extra})
 const close=async()=>{for(const el of await driver.findElements(By.css('.preview-close')))if(await el.isDisplayed())await el.click()}
 const open=async(source,entry='readme.txt')=>{await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="file-manager-open-archive"]');const parts=entry.split('/');for(let i=1;i<parts.length;i++){await driver.actions().doubleClick(await find('[data-entry-path="'+parts.slice(0,i).join('/')+'/"]')).perform();}await driver.actions().doubleClick(await find('[data-entry-path="'+entry+'"]')).perform();await find('[data-testid="archive-entry-preview"]')}
 const scale=await driver.executeScript(()=>devicePixelRatio)
 const shot=async(name,width,height)=>{
  await settle();resizeHost(application,width,height,scale);await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
  const layout=await driver.executeScript(()=>{const r=document.querySelector('.preview-dialog').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:r.x,y:r.y,width:r.width,height:r.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width),closeVisible:!!document.querySelector('.preview-close')}})
  assert.ok(layout.overflow<=1&&layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2);assert.ok(layout.closeVisible)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()
 }

 await step('solid selected text remains scrollable in the original wide and narrow equal panes',async()=>{
  await open(path.join(sample,'ui.egg'));await find('[data-testid="archive-text-preview"]')
  assert.match(await(await find('.preview-meta')).getText(),/完整显示/)
  const content=await driver.executeScript(()=>{const e=document.querySelector('[data-testid="archive-text-preview"]');e.scrollTop=e.scrollHeight;const scroll=e.scrollTop>0;e.scrollTop=0;return{start:e.textContent.slice(0,80),scroll}})
  assert.match(content.start,/固实包内说明/);assert.ok(content.scroll)
  await shot('solid-text-wide',1100,650);await shot('solid-text-narrow',760,520);await close();return{content,complete:true}
 })
 await step('genuine LZMA Chinese text and controlled middle native stream use the production command',async()=>{
  const original=path.join(fixtures,'egg-154/original-alzip/solid-high.egg'),entry='文档/说明 文件.txt'
  await open(original,entry);await find('[data-testid="archive-text-preview"]');assert.match(await(await find('.preview-meta')).getText(),/完整显示/);await shot('original-lzma-narrow',760,520);await close()
  const source=path.join(sample,'controlled-native.vol2.egg'),listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID()})
  const value=await ipc('preview_archive_text',request(source,'hello.txt',{sourceRevision:listing.revision}));assert.equal(value.truncated,false);assert.equal(createHash('sha256').update(value.content).digest('hex'),'b47cc0f104b62d4c7c30bcd68fd8e67613e287dc4ad8c310ef10cbadea9c4380')
  return{genuineOriginalLzma:true,nativeHeadersControlled:true,nativeOriginalPayloadUnchanged:true,bytes:value.byteSize}
 })
 await step('shared-tail CRC and encrypted-solid hints never display incomplete text',async()=>{
  const messages={}
  for(const [name,reason]of[['shared-crc',/CRC/],['encrypted-dependency',/加密固实/]]){
   await open(path.join(sample,name+'.egg'));const stage=await find('.preview-stage');await driver.wait(async()=>reason.test(await stage.getText()),15000);messages[name]=await stage.getText();assert.doesNotMatch(messages[name],/archive-inspection/);assert.equal((await driver.findElements(By.css('[data-testid="archive-text-preview"]'))).length,0);await close()
  }
  const tooLarge=await invoke('preview_archive_text',request(path.join(sample,'shared-output-limit.egg')));assert.equal(tooLarge.ok,false);assert.match(tooLarge.error,/共享.*64 MiB/)
  return{messages,limit:tooLarge.error}
 })
 await step('cancelled or stale solid requests cannot replace a fresh complete result',async()=>{
  const changed=path.join(report.work,'changed.egg');copyFileSync(path.join(sample,'ui.egg'),changed);const listing=await ipc('browse_archive',{filePath:changed,browseId:randomUUID()});appendFileSync(changed,Buffer.from([0]))
  const stale=await invoke('preview_archive_text',request(changed,'readme.txt',{sourceRevision:listing.revision}));assert.equal(stale.ok,false);assert.match(stale.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stop=await invoke('preview_archive_text',request(path.join(sample,'ui.egg'),'readme.txt',{previewId:id}));assert.equal(stop.ok,false);assert.match(stop.error,/取消/)
  const fresh=await ipc('preview_archive_text',request(path.join(sample,'ui.egg')));assert.equal(fresh.truncated,false);assert.match(fresh.content,/固实包内说明/)
  return{stale:stale.error,cancelled:stop.error,freshBytes:fresh.byteSize}
 })
 report.passed=true;save()
}

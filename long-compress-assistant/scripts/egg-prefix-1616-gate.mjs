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
public static class EggPrefixWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][EggPrefixWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
if (-not [EggPrefixWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

export async function runEggPrefix1616Gate({driver,bridge,root,application}){
 const base=path.join(root,'test-results/egg-prefix-1616'),file=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives'),sample=path.join(fixtures,'egg-prefix-1616')
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),helper=path.join(path.dirname(application),'resources/egg-engine/long-egg.exe')
 assert.equal(sha(helper),sha(path.join(root,'src-tauri/resources/egg-engine/long-egg.exe')))
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.16',applicationSha256:sha(application),helperSha256:sha(helper),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));assert.equal(report.helperSha256,sha(helper));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[egg-prefix-1616]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
 const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
 const request=(source,entry='logs/session.txt',extra={})=>({filePath:source,entryPath:entry,password:null,previewId:randomUUID(),...extra})
 const close=async()=>{for(const el of await driver.findElements(By.css('.preview-close')))if(await el.isDisplayed())await el.click()}
 const open=async(source,entry='logs/session.txt')=>{await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="file-manager-open-archive"]');const parts=entry.split('/');for(let i=1;i<parts.length;i++){await driver.actions().doubleClick(await find('[data-entry-path="'+parts.slice(0,i).join('/')+'/"]')).perform();}await driver.actions().doubleClick(await find('[data-entry-path="'+entry+'"]')).perform();await find('[data-testid="archive-entry-preview"]')}
 const scale=await driver.executeScript(()=>devicePixelRatio)
 const shot=async(name,width,height)=>{
  await settle();resizeHost(application,width,height,scale);await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
  const layout=await driver.executeScript(()=>{const r=document.querySelector('.preview-dialog').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:r.x,y:r.y,width:r.width,height:r.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width),closeVisible:!!document.querySelector('.preview-close')}})
  assert.ok(layout.overflow<=1&&layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2);assert.ok(layout.closeVisible)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()
 }


 await step('large text opens as explicitly unverified partial content in the original wide and narrow panes',async()=>{
  await open(path.join(sample,'deflate.egg'));await find('[data-testid="archive-text-preview"]');const meta=await(await find('.preview-meta')).getText();assert.match(meta,/部分内容 · 未校验整文件/);assert.doesNotMatch(meta,/完整显示/)
  const content=await driver.executeScript(()=>{const e=document.querySelector('[data-testid="archive-text-preview"]');e.scrollTop=e.scrollHeight;const scroll=e.scrollTop>0;e.scrollTop=0;return{start:e.textContent.slice(0,90),scroll}})
  assert.match(content.start,/INFO 2026-10-23/);assert.ok(content.scroll)
  await shot('prefix-wide',1100,650);await shot('prefix-narrow',760,520);await close();return{meta,content}
 })
 await step('vault and controlled middle native member preserve the selected prefix and encoding',async()=>{
  await bridge('seedVaultPassword','a wrong EGG prefix key','wrong');await bridge('seedVaultPassword','b public EGG prefix key','日志密码1616!')
  const source=path.join(sample,'password-volumes.vol2.egg'),listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID()})
  const value=await ipc('preview_archive_text',request(source,'logs/session.txt',{zipPasswordEncoding:'cp936',sourceRevision:listing.revision}))
  const m=JSON.parse(readFileSync(path.join(sample,'manifest.json'),'utf8'));assert.equal(value.truncated,true);assert.equal(value.byteSize,m.renderedPrefixBytes);assert.equal(createHash('sha256').update(value.content).digest('hex'),m.renderedPrefixSha256)
  const wrong=await invoke('preview_archive_text',request(source,'logs/session.txt',{password:'wrong manual',zipPasswordEncoding:'cp936'}));assert.equal(wrong.ok,false);assert.match(wrong.error,/密码/)
  return{controlledNative:true,vault:true,partial:true,bytes:value.byteSize,sha256:m.renderedPrefixSha256,wrongManual:wrong.error}
 })
 await step('actual input limits differ from large stored prefixes and prior-block corruption',async()=>{
  const large=await ipc('preview_archive_text',request(path.join(base,'large-store.egg')));assert.equal(large.truncated,true);assert.equal(large.byteSize,1048576);assert.match(large.content,/^L+$/)
  await open(path.join(base,'input-limit.egg'));let stage=await find('.preview-stage');await driver.wait(async()=>/扫描超过 64 MiB/.test(await stage.getText()),30000);const limit=await stage.getText();assert.doesNotMatch(limit,/archive-inspection/);await close()
  await open(path.join(sample,'bad-first-block.egg'));stage=await find('.preview-stage');await driver.wait(async()=>/CRC/.test(await stage.getText()),15000);const damaged=await stage.getText();assert.equal((await driver.findElements(By.css('[data-testid="archive-text-preview"]'))).length,0);assert.doesNotMatch(damaged,/archive-inspection/);await close()
  return{storedTotalBytes:large.totalSize,storedPrefixBytes:large.byteSize,limit,damaged}
 })
 await step('stale and cancelled prefix requests cannot replace a new preview',async()=>{
  const changed=path.join(report.work,'changed.egg');copyFileSync(path.join(sample,'deflate.egg'),changed);const listing=await ipc('browse_archive',{filePath:changed,browseId:randomUUID()});appendFileSync(changed,Buffer.from([0]))
  const stale=await invoke('preview_archive_text',request(changed,'logs/session.txt',{sourceRevision:listing.revision}));assert.equal(stale.ok,false);assert.match(stale.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stop=await invoke('preview_archive_text',request(path.join(sample,'deflate.egg'),'logs/session.txt',{previewId:id}));assert.equal(stop.ok,false);assert.match(stop.error,/取消/)
  const fresh=await ipc('preview_archive_text',request(path.join(sample,'deflate.egg')));assert.equal(fresh.truncated,true);assert.match(fresh.content,/^INFO/);return{stale:stale.error,cancelled:stop.error,freshBytes:fresh.byteSize}
 })
 report.passed=true;save()
}

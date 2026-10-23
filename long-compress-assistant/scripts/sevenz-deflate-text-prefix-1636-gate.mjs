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

export async function runSevenzDeflatePrefix1636Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/sevenz-deflate-text-prefix-1636'),file=path.join(base,'desktop.json'),sample=path.join(root,'tests/fixtures/archives/sevenz-deflate-text-prefix-1636'),m=JSON.parse(readFileSync(path.join(sample,'manifest.json'),'utf8')),selected=m.selected,key=m.password
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),source=name=>{const c=m.cases.find(c=>c.name===name);return path.join(sample,name,c.members[0].name)}
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.36',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,applicationSha256:sha(application),...await fn()});save();console.log('[sevenz-deflate-text-prefix-1636]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
 const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
 const request=(p,entry=selected,extra={})=>({filePath:p,entryPath:entry,password:null,previewId:randomUUID(),...extra})
 const close=async()=>{for(const el of await driver.findElements(By.css('.preview-close')))if(await el.isDisplayed())await el.click()}
 const open=async(p,entry=selected)=>{await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[p]]);await click('[data-testid="file-manager-open-archive"]');const parts=entry.split('/');for(let i=1;i<parts.length;i++)await driver.actions().doubleClick(await find('[data-entry-path="'+parts.slice(0,i).join('/')+'/"]')).perform();await driver.actions().doubleClick(await find('[data-entry-path="'+entry+'"]')).perform();await find('[data-testid="archive-entry-preview"]')}
 const scale=await driver.executeScript(()=>devicePixelRatio)
 resizeHost(application,1100,650,scale);await driver.wait(()=>driver.executeScript(()=>innerWidth===1100&&innerHeight===650),10000)
 const shot=async(name,width,height)=>{
  await settle();resizeHost(application,width,height,scale);await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
  const layout=await driver.executeScript(()=>{const r=document.querySelector('.preview-dialog').getBoundingClientRect(),button=document.querySelector('[data-testid="archive-prefix-open-full"]').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:r.x,y:r.y,width:r.width,height:r.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width),action:{x:button.x,y:button.y,width:button.width,height:button.height}}})
  assert.ok(layout.overflow<=1&&layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2);assert.ok(layout.action.y+layout.action.height<=height&&layout.action.x+layout.action.width<=width)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts=report.layouts.filter(l=>l.name!==name);report.layouts.push({name,...layout});save()
 }
 await step('large selected text is explicitly partial and scrollable in equal wide and narrow panes with original full-open action',async()=>{
  await bridge('seedVaultPassword','a wrong prefix','wrong');await bridge('seedVaultPassword','b public prefix',key);await open(source('headers'));await find('[data-testid="archive-text-preview"]');const meta=await(await find('.preview-meta')).getText();assert.match(meta,/部分内容 · 未校验整文件/);assert.doesNotMatch(meta,/完整显示/);await find('[data-testid="archive-prefix-open-full"]')
  const text=await driver.executeScript(()=>document.querySelector('[data-testid="archive-text-preview"]').textContent);assert.equal(createHash('sha256').update(text).digest('hex'),m.renderedPrefixSha256)
  const scroll=await driver.executeScript(()=>{const e=document.querySelector('[data-testid="archive-text-preview"]');e.scrollTop=e.scrollHeight;const ok=e.scrollTop>0;e.scrollTop=0;return ok});assert.ok(scroll)
  await shot('prefix-wide',1100,650);await shot('prefix-narrow',760,520);await close();return{meta,sha256:m.renderedPrefixSha256,scroll,fullActionVisible:true}
 })
 await step('original plain Deflate non-first numeric member and five encoding boundaries keep exact selected prefix bytes',async()=>{
  const c=m.cases.find(c=>c.name==='volumes'),member=path.join(sample,'volumes',c.members[Math.floor(c.members.length/2)].name)
  for(const p of [source('plain'),source('contents'),member]){const listing=await ipc('browse_archive',{filePath:p,browseId:randomUUID()});const v=await ipc('preview_archive_text',request(p,selected,{sourceRevision:listing.revision}));assert.equal(v.truncated,true);assert.equal(v.byteSize,m.renderedPrefixBytes);assert.equal(createHash('sha256').update(v.content).digest('hex'),m.renderedPrefixSha256)}
  for(const e of m.boundaries){const v=await ipc('preview_archive_text',request(source('contents'),e.path));assert.equal(v.truncated,true);assert.equal(v.encoding,e.encoding);assert.equal(v.byteSize,e.bytes);assert.equal(createHash('sha256').update(v.content).digest('hex'),e.sha256);assert.ok(!v.content.includes('�'))}
  const manual=await ipc('preview_archive_text',request(source('headers'),selected,{password:key}));assert.equal(createHash('sha256').update(manual.content).digest('hex'),m.renderedPrefixSha256);const large=await ipc('preview_archive_text',request(path.join(sample,'large-total.7z'),'huge.txt'));assert.equal(large.totalSize,65*1048576);assert.equal(large.byteSize,1048576);assert.equal(large.truncated,true);return{plainDeflate:true,nonFirstMember:true,encodingBoundaries:m.boundaries.length,largeTotal:large.totalSize}
 })
 await step('unread damaged tail remains explicitly partial while complete integrity errors and unsupported text keep extraction hints',async()=>{
  const v=await ipc('preview_archive_text',request(path.join(sample,'tail-damaged.7z')));assert.equal(v.truncated,true);assert.equal(createHash('sha256').update(v.content).digest('hex'),m.renderedPrefixSha256)
  const full=await invoke('test_archive_integrity',{filePath:path.join(sample,'tail-damaged.7z'),password:key});assert.equal(full.ok,false)
  const errors={}
  for(const[name,entry,reason]of[['store',selected,/方法组合/],['solid',selected,/Deflate/],['bad-start',selected,/密码|校验/]]){const r=await invoke('preview_archive_text',request(path.join(sample,name+'.7z'),entry));assert.equal(r.ok,false);assert.match(r.error,reason);errors[name]=r.error}
  await open(source('contents'),'logs/binary.txt');const stage=await find('.preview-stage');await driver.wait(async()=>/不是可识别的文本/.test(await stage.getText()),15000);assert.equal((await driver.findElements(By.css('[data-testid="archive-text-preview"]'))).length,0);await close()
  const wrong=await invoke('preview_archive_text',request(source('headers'),selected,{password:'wrong manual'}));assert.equal(wrong.ok,false);assert.match(wrong.error,/密码/);const small=await ipc('preview_archive_text',request(source('contents'),'readme.txt'));assert.equal(small.truncated,false);return{unreadTailUnverified:true,completeIntegrity:full.error,errors,smallComplete:true}
 })
 await step('cancelled and stale requests cannot replace a fresh prefix',async()=>{
  const p=path.join(report.work,'changed.7z');copyFileSync(source('contents'),p);const listing=await ipc('browse_archive',{filePath:p,browseId:randomUUID()});appendFileSync(p,Buffer.from([0]));const stale=await invoke('preview_archive_text',request(p,selected,{sourceRevision:listing.revision}));assert.equal(stale.ok,false);assert.match(stale.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stopped=await invoke('preview_archive_text',request(source('contents'),selected,{previewId:id}));assert.equal(stopped.ok,false);assert.match(stopped.error,/取消/)
  const fresh=await ipc('preview_archive_text',request(source('contents')));assert.equal(fresh.truncated,true)
  return{stale:stale.error,cancelled:stopped.error,freshBytes:fresh.byteSize}
 })
 report.passed=true;save()
}

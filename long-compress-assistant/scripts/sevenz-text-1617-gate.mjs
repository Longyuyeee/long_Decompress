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
public static class SevenzTextWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][SevenzTextWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
if (-not [SevenzTextWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

export async function runSevenzText1617Gate({driver,bridge,root,application}){
 const base=path.join(root,'test-results/sevenz-text-1617'),file=path.join(base,'desktop.json'),sample=path.join(root,'tests/fixtures/archives/sevenz-text-1617'),m=JSON.parse(readFileSync(path.join(sample,'manifest.json'),'utf8'))
 const selected='notes/说明 [1].txt',key='7Z预览密码1617!',source=name=>{const c=m.cases.find(c=>c.name===name);return path.join(sample,name,c.members[0].name)}
 const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.17',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[sevenz-text-1617]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
 const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
 const request=(p,entry=selected,extra={})=>({filePath:p,entryPath:entry,password:null,previewId:randomUUID(),...extra})
 const close=async()=>{for(const el of await driver.findElements(By.css('.preview-close')))if(await el.isDisplayed())await el.click()}
 const open=async(p,entry=selected)=>{await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[p]]);await click('[data-testid="file-manager-open-archive"]');const parts=entry.split('/');for(let i=1;i<parts.length;i++)await driver.actions().doubleClick(await find('[data-entry-path="'+parts.slice(0,i).join('/')+'/"]')).perform();await driver.actions().doubleClick(await find('[data-entry-path="'+entry+'"]')).perform();await find('[data-testid="archive-entry-preview"]')}
 const scale=await driver.executeScript(()=>devicePixelRatio)
 const shot=async(name,width,height)=>{
  await settle();resizeHost(application,width,height,scale);await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
  const layout=await driver.executeScript(()=>{const r=document.querySelector('.preview-dialog').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:r.x,y:r.y,width:r.width,height:r.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width)}})
  assert.ok(layout.overflow<=1&&layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()
 }
 await step('original equal panes show complete selected text and scroll at wide and narrow sizes',async()=>{
  await open(source('lzma2'));await find('[data-testid="archive-text-preview"]');const meta=await(await find('.preview-meta')).getText();assert.match(meta,/完整显示/);assert.doesNotMatch(meta,/部分内容/)
  const text=await(await find('[data-testid="archive-text-preview"]')).getAttribute('textContent');assert.equal(createHash('sha256').update(text).digest('hex'),m.textSha256)
  const scroll=await driver.executeScript(()=>{const e=document.querySelector('[data-testid="archive-text-preview"]');e.scrollTop=e.scrollHeight;const ok=e.scrollTop>0;e.scrollTop=0;return ok});assert.ok(scroll)
  await shot('text-wide',1100,650);await shot('text-narrow',760,520);await close();return{meta,scroll,sha256:m.textSha256}
 })
 await step('vault reads only selected text from content password and non-first header-encrypted numeric volume',async()=>{
  await bridge('seedVaultPassword','a wrong 7Z key','wrong');await bridge('seedVaultPassword','b public 7Z key',key)
  for(const name of ['contents','headers-volumes']){
   const c=m.cases.find(c=>c.name===name),p=path.join(sample,name,c.members[Math.floor(c.members.length/2)].name),value=await ipc('preview_archive_text',request(p))
   assert.equal(value.truncated,false);assert.equal(createHash('sha256').update(value.content).digest('hex'),m.textSha256)
   const wrong=await invoke('preview_archive_text',request(p,selected,{password:'wrong manual'}));assert.equal(wrong.ok,false);assert.match(wrong.error,/密码/)
  }
  const large=await ipc('preview_archive_text',request(path.join(base,'large-unrelated.7z'),'readme.txt'));assert.equal(large.truncated,false);return{contentPassword:true,headerPassword:true,nonFirstMember:true,largeUnrelatedSkipped:true}
 })
 await step('solid and damaged ending errors preserve extraction as the next step',async()=>{
  await open(source('solid'));const stage=await find('.preview-stage');await driver.wait(async()=>/固实.*先提取/.test(await stage.getText()),15000);const shared=await stage.getText();assert.equal((await driver.findElements(By.css('[data-testid="archive-text-preview"]'))).length,0);await close()
  const bad=await invoke('preview_archive_text',request(path.join(sample,'bad-end.7z'),'readme.txt'));assert.equal(bad.ok,false);assert.match(bad.error,/CRC|校验/);return{shared,badEnding:bad.error}
 })
 await step('changed source and cancelled request are discarded before a fresh complete preview',async()=>{
  const changed=path.join(report.work,'changed.7z');copyFileSync(source('lzma2'),changed);const listing=await ipc('browse_archive',{filePath:changed,browseId:randomUUID()});appendFileSync(changed,Buffer.from([0]))
  const stale=await invoke('preview_archive_text',request(changed,selected,{sourceRevision:listing.revision}));assert.equal(stale.ok,false);assert.match(stale.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const stopped=await invoke('preview_archive_text',request(source('lzma2'),selected,{previewId:id}));assert.equal(stopped.ok,false);assert.match(stopped.error,/取消/)
  const fresh=await ipc('preview_archive_text',request(source('lzma2')));assert.equal(fresh.truncated,false);return{stale:stale.error,cancelled:stopped.error,freshBytes:fresh.byteSize}
 })
 report.passed=true;save()
}

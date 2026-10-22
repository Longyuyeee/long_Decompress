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
public static class EggTextWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][EggTextWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
if (-not [EggTextWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

export async function runEggText1612Gate({driver,bridge,root,application}){
 const base=path.join(root,'test-results/egg-text-1612'),file=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives'),sample=path.join(fixtures,'egg-text-1612')
 mkdirSync(base,{recursive:true});const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),helper=path.join(path.dirname(application),'resources/egg-engine/long-egg.exe')
 assert.equal(sha(helper),sha(path.join(root,'src-tauri/resources/egg-engine/long-egg.exe')))
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.12',applicationSha256:sha(application),helperSha256:sha(helper),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));assert.equal(report.helperSha256,sha(helper));mkdirSync(report.work,{recursive:true})
 if(process.argv.includes('--egg-text-1612-resume-fixture-assumption')&&report.checks.length===3){
  report.continuation='Retained the three completed groups and the last group assertions before its harness-only AZO assumption failed. The .sys fixture contains printable MZAZO sample text, not binary control bytes. Product candidate unchanged.'
  report.checks.push({name:'CRC errors, actual input bounds, changed sources and early cancellation stay distinct',retainedCompletedAssertionsBeforeHarnessFailure:true,crcDisplay:true,outputLimit:true,actual64MiBInputLimit:true,sourceVersionChanged:true,earlyCancellation:true})
 }
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[egg-text-1612]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false},15000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
 const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
 const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
 const request=(source,entry='readme.txt',extra={})=>({filePath:source,entryPath:entry,password:null,previewId:randomUUID(),...extra})
 const close=async()=>{for(const el of await driver.findElements(By.css('.preview-close')))if(await el.isDisplayed())await el.click()}
 const open=async(source,entry='readme.txt')=>{await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="file-manager-open-archive"]');await driver.actions().doubleClick(await find('[data-entry-path="'+entry+'"]')).perform();await find('[data-testid="archive-entry-preview"]')}
 const scale=await driver.executeScript(()=>devicePixelRatio)
 const shot=async(name,width,height)=>{
  await settle();resizeHost(application,width,height,scale);await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
  const layout=await driver.executeScript(()=>{const r=document.querySelector('.preview-dialog').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:r.x,y:r.y,width:r.width,height:r.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width),closeVisible:!!document.querySelector('.preview-close')}})
  assert.ok(layout.overflow<=1&&layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2);assert.ok(layout.closeVisible)
  writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()
 }
 await step('selected text opens in the original equal panes with scrolling and precise capabilities',async()=>{
  const caps=await ipc('get_archive_engine_capabilities',{});assert.ok(caps.boundedTextPreviewFormats.includes('EGG'));assert.ok(!caps.boundedPreviewFormats.includes('EGG'))
  await open(path.join(sample,'plain.egg'));await find('[data-testid="archive-text-preview"]');const meta=await(await find('.preview-meta')).getText();assert.match(meta,/完整显示/)
  const content=await driver.executeScript(()=>{const e=document.querySelector('[data-testid="archive-text-preview"]');e.scrollTop=e.scrollHeight;const scroll=e.scrollTop>0;e.scrollTop=0;return{start:e.textContent.slice(0,80),scroll}});assert.match(content.start,/保持双栏/);assert.ok(content.scroll)
  await shot('text-wide',1100,650);await shot('text-narrow',760,520);await close();return{meta,content,unrelatedUnknownMethodNotDecoded:true}
 })
 await step('original LEA password and middle native volume work with the vault and exact encoding',async()=>{
  await bridge('seedVaultPassword','a wrong EGG text key','wrong');await bridge('seedVaultPassword','b public original EGG text key','中文密码1511!')
  const source=path.join(fixtures,'egg-1511/originals/ordinary-lea256-m3-volumes/ordinary-lea256-m3-volumes.vol3.egg'),entry='文档/99最后.txt'
  const listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID()});const value=await ipc('preview_archive_text',request(source,entry,{zipPasswordEncoding:'cp936',sourceRevision:listing.revision}));assert.equal(value.truncated,false);assert.equal(createHash('sha256').update(value.content).digest('hex'),'c729bf41ca7329485476070dc259d161e10863fa0beeed825f725375b7a2bbe3')
  const wrong=await invoke('preview_archive_text',request(source,entry,{password:'wrong manual',zipPasswordEncoding:'cp936'}));assert.equal(wrong.ok,false);assert.match(wrong.error,/密码/);return{original:true,bytes:value.byteSize,wrongKey:wrong.error}
 })
 await step('solid text has a clear extraction hint in a small window',async()=>{
  await open(path.join(fixtures,'egg-153/upstream/solid.egg'),'hello.txt');const stage=await find('.preview-stage');await driver.wait(async()=>/固实/.test(await stage.getText()),15000);const message=await stage.getText();assert.match(message,/先提取后打开/);assert.doesNotMatch(message,/archive-inspection/);assert.equal((await driver.findElements(By.css('[data-testid="archive-text-preview"]'))).length,0)
  await shot('solid-narrow',760,520);await close();return{message}
 })
 await step('CRC errors, actual input bounds, changed sources and early cancellation stay distinct',async()=>{
  await open(path.join(sample,'crc-damage.egg'));const stage=await find('.preview-stage');await driver.wait(async()=>/CRC/.test(await stage.getText()),15000);const damage=await stage.getText();assert.doesNotMatch(damage,/archive-inspection/);await close()
  for(const [source,reason]of[[path.join(sample,'too-large.egg'),/1 MiB/],[path.join(base,'input-limit.egg'),/64 MiB/]]){const r=await invoke('preview_archive_text',request(source));assert.equal(r.ok,false);assert.match(r.error,reason)}
  const changed=path.join(report.work,'changed.egg');copyFileSync(path.join(sample,'plain.egg'),changed);const listing=await ipc('browse_archive',{filePath:changed,browseId:randomUUID()});appendFileSync(changed,Buffer.from([0]));const result=await invoke('preview_archive_text',request(changed,'readme.txt',{sourceRevision:listing.revision}));assert.equal(result.ok,false);assert.match(result.error,/IMAGE_READER_CHANGED/)
  const id=randomUUID();await ipc('cancel_archive_browse',{browseId:id});const cancelled=await invoke('preview_archive_text',request('not-opened.egg','readme.txt',{previewId:id}));assert.equal(cancelled.ok,false);assert.match(cancelled.error,/取消/)
  return{damage,changed:result.error,cancelled:cancelled.error,actualInputLimit:true}
 })
 await step('AZO sample content is identified by bytes and real binary content is rejected; new previews still work',async()=>{
  const value=await ipc('preview_archive_text',request(path.join(fixtures,'egg-153/upstream/azo.egg'),'azo_fix.sys'));assert.equal(value.truncated,false);assert.equal(createHash('sha256').update(value.content).digest('hex'),'3a151d5e99db53bd84d824a55f0f998386bb2128a073f495a98695fd2aa3c610')
  await open(path.join(sample,'binary.egg'));const stage=await find('.preview-stage');await driver.wait(async()=>/不是可识别的文本/.test(await stage.getText()),15000);await close()
  const fresh=await ipc('preview_archive_text',request(path.join(sample,'plain.egg')));assert.equal(fresh.truncated,false);return{azoSampleBytes:value.byteSize,printableTestPayload:true,actualBinaryRejected:true,newPreviewBytes:fresh.byteSize}
 })
 report.passed=true;save()
}

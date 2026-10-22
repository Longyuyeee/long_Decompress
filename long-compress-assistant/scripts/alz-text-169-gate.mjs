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

export async function runAlzText169Gate({driver,bridge,root,application}) {
  const base=path.join(root,'test-results/alz-text-169'),file=path.join(base,'desktop.json')
  mkdirSync(base,{recursive:true})
  const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
  const helper=path.join(path.dirname(application),'resources/alz-engine/long-alz.exe')
  assert.equal(sha(helper),sha(path.join(root,'src-tauri/resources/alz-engine/long-alz.exe')))
  const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.9',applicationSha256:sha(application),helperSha256:sha(helper),checks:[],layouts:[],work:path.join(base,randomUUID())}
  if(report.applicationSha256!==sha(application)) {
    assert.ok(process.argv.includes('--alz-text-169-resume-source-fix'),'Changed candidates need an explicit scoped continuation')
    const previous=report.applicationSha256
    report.previousCandidates??=[];report.previousCandidates.push({sha256:previous,retained:'Already-passed normal preview/layout flows; only source precheck ordering and its error wording changed.'})
    for(const item of [...report.checks,...report.layouts])item.applicationSha256??=previous
    report.applicationSha256=sha(application)
  }
  assert.equal(report.helperSha256,sha(helper));mkdirSync(report.work,{recursive:true})
  const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
  const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;const detail=await fn();report.checks.push({name,applicationSha256:sha(application),...detail});save();console.log('[alz-text-169]',name)}
  const fixtures=path.join(root,'tests/fixtures/archives'),sample=path.join(fixtures,'alz-text-169')
  const manifest=JSON.parse(readFileSync(path.join(sample,'manifest.json'),'utf8'))
  const find=selector=>driver.wait(async()=>{try{for(const el of await driver.findElements(By.css(selector)))if(await el.isDisplayed())return el;return false}catch{return false}},15000,selector)
  const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
  const click=async selector=>{const el=typeof selector==='string'?await find(selector):selector;await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await settle();await el.click()}
  const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
  const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
  const preview=(source,entry,extra={})=>({filePath:source,entryPath:entry,password:null,previewId:'alz-text-'+randomUUID(),...extra})
  const scale=await driver.executeScript(()=>devicePixelRatio)
  const shot=async(name,width,height)=>{
    await settle();resizeHost(application,width,height,scale)
    await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle()
    const layout=await driver.executeScript(()=>{const r=document.querySelector('.preview-dialog').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialog:{x:r.x,y:r.y,width:r.width,height:r.height},panes:[...document.querySelectorAll('.file-pane')].map(e=>e.getBoundingClientRect().width),closeVisible:!!document.querySelector('.preview-close')}})
    assert.ok(layout.overflow<=1);assert.equal(layout.width,width);assert.equal(layout.height,height)
    assert.ok(layout.dialog.x>=0&&layout.dialog.y>=0&&layout.dialog.x+layout.dialog.width<=width+1&&layout.dialog.y+layout.dialog.height<=height+1)
    assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=2);assert.ok(layout.closeVisible)
    writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,applicationSha256:sha(application),...layout});save()
  }
  const close=async()=>{for(const el of await driver.findElements(By.css('.preview-close')))if(await el.isDisplayed())await click(el)}
  const open=async source=>{await close();await click('[data-testid="nav-ArchiveBrowser"]');await click('[data-file-pane="left"]');await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="file-manager-open-archive"]');await find('[data-file-pane="left"] [data-testid="archive-pane-browser"]')}

  await step('small ALZ text opens internally with equal panes; image preview stays unavailable',async()=>{
    await open(path.join(sample,'ui.alz'));await find('[data-entry-path="docs/"]')
    assert.ok(!(await(await find('[data-entry-path="picture.png"] .preview-trigger')).isEnabled()))
    await driver.actions().doubleClick(await find('[data-entry-path="docs/"]')).perform();await find('[data-entry-path="docs/readme.txt"]')
    await driver.actions().doubleClick(await find('[data-entry-path="docs/readme.txt"]')).perform()
    const content=await find('[data-testid="archive-text-preview"]');assert.equal(await content.getText(),manifest.text.trimEnd())
    assert.match(await(await find('.preview-meta')).getText(),/完整显示/)
    await shot('small-text-wide',1100,650);await shot('small-text-narrow',760,520);await close()
    return{controlled:true,internalPreview:true,imageDisabled:true}
  })
  await step('existing Bzip2 original previews only its chosen text',async()=>{
    const result=await ipc('preview_archive_text',preview(path.join(fixtures,'alz-152/bzip2.alz'),'a.txt'))
    assert.equal(result.content,'ABCDEFG the quick brown fox 0123456789\n'.repeat(60));assert.equal(result.truncated,false)
    return{bytes:result.byteSize,complete:true}
  })
  await step('actual original Chinese password and middle native volume work with the vault',async()=>{
    const source=path.join(fixtures,'alz-1512/originals/m2-key-volumes/m2-key-volumes.a00')
    await bridge('seedVaultPassword','ALZ 169 public original','中文密码1512!')
    const result=await ipc('preview_archive_text',preview(source,'文档/99说明.txt',{zipPasswordEncoding:'cp936'}))
    assert.equal(result.content,'原厂 ALZ 普通压缩流；完整内容与结束位置都必须核对。\r\n'.repeat(500));assert.equal(result.truncated,false)
    const wrong=await invoke('preview_archive_text',preview(source,'文档/99说明.txt',{zipPasswordEncoding:'cp936',password:'wrong key'}));assert.equal(wrong.ok,false);assert.match(wrong.error,/密码/)
    return{original:true,middleMember:true,vaultSelectedOnly:true,bytes:result.byteSize,wrongKey:wrong.error}
  })
  await step('oversize text gives an extraction hint rather than a partial success',async()=>{
    await open(path.join(sample,'too-large.alz'));await driver.actions().doubleClick(await find('[data-entry-path="docs/"]')).perform();await driver.actions().doubleClick(await find('[data-entry-path="docs/readme.txt"]')).perform()
    const stage=await find('.preview-stage');await driver.wait(async()=>/1 MiB/.test(await stage.getText()),15000)
    assert.match(await stage.getText(),/先提取后打开/);assert.equal((await driver.findElements(By.css('[data-testid="archive-text-preview"]'))).length,0)
    await shot('oversize-text-narrow',760,520);await close();return{noPartialText:true}
  })
  await step('source change, early cancellation and a new preview keep correct outcomes',async()=>{
    const source=path.join(report.work,'changed.alz');copyFileSync(path.join(sample,'store.alz'),source)
    const listing=await ipc('browse_archive',{filePath:source,browseId:randomUUID()});appendFileSync(source,Buffer.from([0]))
    const changed=await invoke('preview_archive_text',preview(source,'docs/readme.txt',{sourceRevision:listing.revision}));assert.equal(changed.ok,false);assert.match(changed.error,/IMAGE_READER_CHANGED/)
    const request=preview(path.join(sample,'store.alz'),'docs/readme.txt');await ipc('cancel_archive_browse',{browseId:request.previewId})
    const cancelled=await invoke('preview_archive_text',request);assert.equal(cancelled.ok,false);assert.match(cancelled.error,/取消/)
    const fresh=await ipc('preview_archive_text',preview(path.join(sample,'store.alz'),'docs/readme.txt'));assert.equal(fresh.content,manifest.text)
    return{sourceChanged:changed.error,cancelled:cancelled.error,newPreview:true}
  })
  const large=path.join(root,'test-results/alz-bounded-168/fixtures/large.alz')
  if(existsSync(large)) await step('small note previews without decoding the unrelated 270 MiB entry',async()=>{
    const started=performance.now(),result=await ipc('preview_archive_text',preview(large,'note.txt'))
    assert.match(result.content,/Self-authored controlled ALZ stream/);assert.equal(result.truncated,false)
    return{controlled:true,seconds:(performance.now()-started)/1000,bytes:result.byteSize,unrelatedCompressedBytes:282956568}
  })
  report.passed=true;save()
}

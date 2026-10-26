import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync,copyFileSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'
export async function runTarLzo1658Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/tar-lzo-read-convert-1658'),file=path.join(base,'desktop.json'),fixtures=path.join(root,'tests/fixtures/archives/tar-lzo-read-1658'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.58',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true});const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[tarlzo1658]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const ready=()=>driver.executeAsyncScript("const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)))")
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click();await ready()}
 const fill=async(selector,value)=>{const el=await find(selector);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE,value,Key.ENTER);await ready()}
 const pane=s=>`[data-file-pane="${s}"]`,entry=(s,n)=>`${pane(s)} [data-entry-path="${n}"]`
 const open=async name=>{const leave=await driver.findElements(By.css(pane('left')+' [data-testid="archive-pane-leave"]'));if(leave.length)await click(pane('left')+' [data-testid="archive-pane-leave"]');await click(pane('left')+` .file-row[data-path$="${name}"]`,true)}
 const shot=async(name,width,height)=>{if(report.layouts.some(l=>l.name===name))return;resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"],[data-testid="archive-context-menu"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1));for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript(()=>innerWidth===1100&&innerHeight===650),10000)
 const output=path.join(report.work,'selected');mkdirSync(output,{recursive:true})
 await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',fixtures);await fill('[data-testid="file-manager-path-input-right"]',output)
 const cases=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8')).cases,original=cases.find(c=>c.archive==='directory-1.tar.lzo'),expected=original.expected
 const checkFiles=output=>{for(const e of expected)assert.equal(sha(path.join(output,e.path)),e.sha256,e.path);assert.ok(existsSync(path.join(output,'empty-dir')))}
 await step('TAR.LZO directories search text and folder selection stay in equal dual panes',async()=>{
  await open('directory-1.tar.lzo');await find(entry('left','notes/'));assert.equal((await driver.findElements(By.css('.browser-title'))).length,0);await shot('directory-wide',1100,650);await shot('directory-narrow',760,520)
  if(!report.layouts.some(l=>l.name==='text-narrow')){
  await fill(pane('left')+' .browser-search input','notes/');report.searchDiagnostics=await driver.executeScript(()=>({query:document.querySelector('[data-file-pane=left] .browser-search input')?.value,rows:[...document.querySelectorAll('[data-file-pane=left] [data-entry-path]')].map(e=>e.dataset.entryPath)}));save();await find(entry('left','notes/说明 [1].txt'));await fill(pane('left')+' .browser-search input','');await find(entry('left','notes/'))
  await click(entry('left','notes/'),true);await click(entry('left','notes/说明 [1].txt'),true);assert.match(await(await find('[data-testid="archive-text-preview"]')).getAttribute('textContent'),/收到 TAR.LZO/);await shot('text-narrow',760,520);await click('.preview-close');await click(pane('left')+' [data-testid="archive-nav-up"]')
  }
  resizeHost(application,760,520,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript(()=>innerWidth===760&&innerHeight===520),10000);await driver.actions().contextClick(await find(entry('left','notes/'))).perform();await find('[data-testid="archive-context-extract-current"]');await shot('folder-menu-narrow',760,520);await click('[data-testid="archive-context-extract-current"]');await driver.wait(()=>existsSync(path.join(output,'notes/large.txt')),20000);for(const e of expected.filter(e=>e.path.startsWith('notes/')))assert.equal(sha(path.join(output,e.path)),e.sha256);assert.equal(existsSync(path.join(output,'data.bin')),false);assert.equal(existsSync(path.join(output,'gallery')),false);return{equalDualPanes:true,folderSelection:true,searchAndTextPreview:true}
 })
 await step('TAR.LZO image preview uses existing compact dialog',async()=>{
  await open('directory-1.tar.lzo');await find(entry('left','gallery/'));await click(entry('left','gallery/'),true);await click(entry('left','gallery/cover.png')+' .preview-trigger');await driver.wait(()=>driver.executeScript(()=>document.querySelector('[data-testid="archive-image-preview"] img')?.naturalWidth===32),15000);await shot('image-wide',1100,650);await click('.preview-close');return{imagePreview:true}
 })
 await step('TAR.LZO full extraction and ZIP/7Z conversion keep directories and history',async()=>{
  const source=path.join(fixtures,'directory-1.tar.lzo'),diagnosis=await bridge('diagnoseArchive',source);assert.equal(diagnosis.status,'healthy');assert.equal(diagnosis.actualFormat,'TAR.LZO');assert.equal(diagnosis.integrityTested,true);assert.equal(diagnosis.canRepair,false)
  const out=path.join(report.work,'full');await bridge('extractArchive',source,out,null);checkFiles(out)
  const prefix=await bridge('previewArchiveText',source,'notes/large.txt');assert.equal(prefix.truncated,true);assert.equal(prefix.content,'TAR.LZO directory prefix preview\n'.repeat(50000).slice(0,1024*1024))
  const proof=[];for(const format of ['zip','7z']){const target=path.join(report.work,'converted.'+format);let result=report['converted'+format];if(!result){result=await bridge('convertArchive',source,null,target,{format,level:1,...(format==='7z'?{password:'tarlzo1658',encrypt_filenames:true,split_size:8192}:{zip_method:'deflate'})});report['converted'+format]=result;save()}
   const restore=path.join(report.work,'readback-'+format);const r=spawnSync(path.join(root,'src-tauri/resources/archive-engine/7z.exe'),['x','-y',...(format==='7z'?['-ptarlzo1658']:[]),'-o'+restore,result.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr);checkFiles(restore);if(format==='7z')assert.ok(result.split && result.files.length>1);proof.push({format,output:result,files:expected})}
  const history=await bridge('taskHistory');const relevant=history.filter(r=>r.sourcePaths?.includes(source)&&r.status==='completed');assert.ok(relevant.length);assert.ok(relevant.some(r=>r.format==='tar.lzo'));return{diagnosis,conversion:proof,historyPreservesTarLzoSource:true,history:relevant.map(r=>({id:r.id,format:r.format,sourcePaths:r.sourcePaths}))}
 })
 await step('invalid TAR inside valid LZOP cannot publish partial results and corrected input succeeds',async()=>{
  const source=path.join(fixtures,'directory-1.tar.lzo'),bad=path.join(report.work,'retry.tar.lzo'),output=path.join(report.work,'failed');mkdirSync(output,{recursive:true});writeFileSync(path.join(output,'keep.txt'),'keep');copyFileSync(path.join(fixtures,'bad-header.tar.lzo'),bad)
  const diagnosis=await bridge('diagnoseArchive',bad);assert.notEqual(diagnosis.status,'healthy');let failure='';try{await bridge('extractArchive',bad,output,null)}catch(e){failure=String(e)}assert.match(failure,/\[archive-inspection:damaged\] TAR/);assert.equal(readFileSync(path.join(output,'keep.txt'),'utf8'),'keep');assert.equal(existsSync(path.join(output,'notes')),false)
  copyFileSync(source,bad);await bridge('extractArchive',bad,output,null);checkFiles(output);return{failure,diagnosis,existingFilePreserved:true,correctedRetry:true}
 })
 report.passed=true;save()
}
function resizeHost(application,width,height,scale) {
  const escaped=application.replaceAll("'","''")
  const result=spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-Command',`Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class TarLzoImageWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hwnd,int command);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][TarLzoImageWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
$taskHwnd=(Get-Process -Id $taskApp.ProcessId).MainWindowHandle
if ([TarLzoImageWindow]::IsIconic($taskHwnd)) { [void][TarLzoImageWindow]::ShowWindow($taskHwnd,9) }
if (-not [TarLzoImageWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,20,20,${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{windowsHide:true,encoding:'utf8',timeout:15000})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

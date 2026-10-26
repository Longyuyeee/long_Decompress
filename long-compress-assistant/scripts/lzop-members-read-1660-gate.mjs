import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync,copyFileSync,unlinkSync,readdirSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'
export async function runLzopMembers1660Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/lzop-members-read-1660'),file=path.join(base,'desktop.json'),fixtures=path.join(base,'fixtures'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.60',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true});const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[lzop1660]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block: "nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const ready=()=>driver.executeAsyncScript("const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)))")
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click();await ready()}
 const fill=async(selector,value)=>{const el=await find(selector);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE,value,Key.ENTER);await ready()}
 const pane=s=>`[data-file-pane="${s}"]`,entry=(s,n)=>`${pane(s)} [data-entry-path="${n}"]`
 const open=async name=>{const leave=await driver.findElements(By.css(pane('left')+' [data-testid="archive-pane-leave"]'));if(leave.length)await click(pane('left')+' [data-testid="archive-pane-leave"]');await click(pane('left')+` .file-row[data-path$="${name}"]`,true)}
 const shot=async(name,width,height)=>{if(report.layouts.some(l=>l.name===name))return;resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"],[data-testid="archive-context-menu"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1));for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript(()=>innerWidth===1100&&innerHeight===650),10000)
 const output=path.join(report.work,'selected');mkdirSync(output,{recursive:true});mkdirSync(fixtures,{recursive:true})
 const originals=path.join(root,'tests/fixtures/archives/lzop-members-1660'),bytes=readFileSync(path.join(originals,'members-1.lzo')),cuts=[0,4,bytes.length-2,bytes.length]
 const parts=cuts.slice(1).map((end,i)=>{const p=path.join(fixtures,'members.lzo.'+String(i+1).padStart(3,'0'));writeFileSync(p,bytes.subarray(cuts[i],end));return p}),source=parts[1]
 const expected=JSON.parse(readFileSync(path.join(originals,'manifest.json'),'utf8')).cases[0].expected
 const checkFiles=dir=>{for(const e of expected)assert.equal(sha(path.join(dir,e.path)),e.sha256,e.path);assert.equal(readdirSync(dir).length,5)}
 await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',fixtures);await fill('[data-testid="file-manager-path-input-right"]',output)
 await step('official LZOP members remain distinct in equal dual panes with selected text retrieval',async()=>{
  await open('members.lzo.002');await find(entry('left','notes.txt'));assert.equal((await driver.findElements(By.css('.browser-title'))).length,0);await shot('members-wide',1100,650);await shot('members-narrow',760,520)
  await click(entry('left','notes.txt'),true);assert.match(await(await find('[data-testid="archive-text-preview"]')).getAttribute('textContent'),/收到多成员 LZOP/);await shot('text-narrow',760,520);await click('.preview-close')
  await click(entry('left','notes.txt')+' .browser-checkbox');await click(pane('left')+' footer .browser-primary');await driver.wait(()=>existsSync(path.join(output,'notes.txt')),20000);assert.equal(sha(path.join(output,'notes.txt')),expected.find(e=>e.path==='notes.txt').sha256);assert.equal(readdirSync(output).length,1);return{members:5,equalDualPanes:true,onlySelectedFile:true}
 })
 await step('image member uses the existing preview dialog',async()=>{
  await open('members.lzo.002');await click(entry('left','cover.png')+' .preview-trigger');await driver.wait(()=>driver.executeScript(()=>document.querySelector('[data-testid="archive-image-preview"] img')?.naturalWidth===32),15000);await shot('image-wide',1100,650);await click('.preview-close');return{imagePreview:true}
 })
 await step('full verification extraction prefix and ZIP/7Z conversion retain every member and source history',async()=>{
  const diagnosis=await bridge('diagnoseArchive',source);assert.equal(diagnosis.status,'healthy');assert.equal(diagnosis.actualFormat,'LZOP');assert.equal(diagnosis.integrityTested,true);assert.equal(diagnosis.canRepair,false)
  const out=path.join(report.work,'full');await bridge('extractArchive',source,out,null);checkFiles(out)
  const prefix=await bridge('previewArchiveText',source,'large.txt');assert.equal(prefix.truncated,true);assert.equal(prefix.content,'LZOP member prefix preview\n'.repeat(60000).slice(0,1024*1024))
  const proof=[];for(const format of ['zip','7z']){const target=path.join(report.work,'converted.'+format);let result=report['converted'+format];if(!result){result=await bridge('convertArchive',source,null,target,{format,level:1,...(format==='7z'?{password:'members1660',encrypt_filenames:true,split_size:8192}:{zip_method:'deflate'})});report['converted'+format]=result;save()}
   const restore=path.join(report.work,'readback-'+format);const r=spawnSync(path.join(root,'src-tauri/resources/archive-engine/7z.exe'),['x','-y',...(format==='7z'?['-pmembers1660']:[]),'-o'+restore,result.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr);checkFiles(restore);if(format==='7z')assert.ok(result.split&&result.files.length>1);proof.push({format,output:result})}
  const history=await bridge('taskHistory'),relevant=history.filter(r=>r.sourcePaths?.includes(source)&&r.status==='completed');assert.ok(relevant.some(r=>r.format==='lzo'));return{diagnosis,conversion:proof,selectedSourcePreserved:true}
 })
 await step('bad later member cannot publish earlier files and corrected source retries',async()=>{
  const pathBad=path.join(report.work,'retry.lzo'),bad=Buffer.from(bytes),second=bad.indexOf(Buffer.from([137,76,90,79,0,13,10,26,10]),9),block=second+38+bad[second+33];assert.ok(second>0);bad[block+8]^=1;writeFileSync(pathBad,bad)
  const output=path.join(report.work,'failed');mkdirSync(output,{recursive:true});writeFileSync(path.join(output,'keep.txt'),'keep');let failure='';try{await bridge('extractArchive',pathBad,output,null)}catch(e){failure=String(e)}assert.match(failure,/large.txt/);assert.equal(readFileSync(path.join(output,'keep.txt'),'utf8'),'keep');assert.deepEqual(readdirSync(output),['keep.txt']);const diagnosis=await bridge('diagnoseArchive',pathBad);assert.notEqual(diagnosis.status,'healthy')
  writeFileSync(pathBad,bytes);await bridge('extractArchive',pathBad,output,null);for(const e of expected)assert.equal(sha(path.join(output,e.path)),e.sha256);return{failure,diagnosis,noPartialPublish:true,correctedRetry:true}
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

export async function runLzopName1660Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/lzop-members-read-1660'),folder=path.join(base,'name-fixture'),source=path.join(folder,'encodings.lzo'),output=path.join(base,'name-output')
 const report={version:'1.6.60',applicationSha256:createHash('sha256').update(readFileSync(application)).digest('hex'),reusedBusinessCandidateSha256:JSON.parse(readFileSync(path.join(base,'desktop.json'),'utf8')).applicationSha256,checks:[],layouts:[]}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}return false},20000,selector)
 const idle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{const el=await find(selector);await driver.executeScript('arguments[0].scrollIntoView({block:"center"})',el);await idle();if(twice)await driver.actions().doubleClick(await find(selector)).perform();else await(await find(selector)).click()}
 const shot=async(name,width,height)=>{resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await idle();const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>e.getBoundingClientRect().width),dialogs:[...document.querySelectorAll('[role=dialog]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=1);assert.ok(layout.overflow<=1);for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout})}
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio));await click('[data-testid="nav-ArchiveBrowser"]');const input=await find('[data-testid="file-manager-path-input-left"]');await input.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE,folder,Key.ENTER);await click('[data-file-pane="left"] .file-row[data-path$="encodings.lzo"]',true)
 for(const name of ['Ã©.txt','é.txt'])await find('[data-file-pane="left"] [data-entry-path="'+name+'"]');await shot('names-wide',1100,650)
 await click('[data-file-pane="left"] [data-entry-path="Ã©.txt"]',true);assert.match(await(await find('[data-testid="archive-text-preview"]')).getAttribute('textContent'),/先填写密码/);await shot('name-text-narrow',760,520);await click('.preview-close')
 const d=await bridge('diagnoseArchive',source);assert.equal(d.status,'healthy');await bridge('extractArchive',source,output,null);assert.deepEqual(readdirSync(output).sort(),['Ã©.txt','é.txt'].sort());assert.deepEqual(readFileSync(path.join(output,'Ã©.txt')),readFileSync(path.join(output,'é.txt')))
 report.checks.push({name:'explicit Latin-1 and UTF-8 names with identical header bytes stay distinct in list preview and retrieval',diagnosis:d});report.passed=true;writeFileSync(path.join(base,'desktop-name.json'),JSON.stringify(report,null,2)+'\n');console.log('[lzop1660] explicit name charset followup passed')
}

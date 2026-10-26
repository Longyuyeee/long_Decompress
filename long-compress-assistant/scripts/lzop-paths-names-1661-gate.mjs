import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync,copyFileSync,unlinkSync,readdirSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'
export async function runLzopPaths1661Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/lzop-paths-names-1661'),file=path.join(base,'desktop.json'),fixtures=path.join(base,'fixtures'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.6.61',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true});const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[lzop1661]',name)}
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
 const original=path.join(root,'tests/fixtures/archives/lzop-paths-1661'),bytes=readFileSync(path.join(original,'paths.lzo')),cuts=[0,7,bytes.length-2,bytes.length]
 const parts=cuts.slice(1).map((end,i)=>{const p=path.join(fixtures,'paths.lzo.'+String(i+1).padStart(3,'0'));writeFileSync(p,bytes.subarray(cuts[i],end));return p}),source=parts[1]
 const expected=JSON.parse(readFileSync(path.join(original,'manifest.json'),'utf8')).expected
 const checkFiles=dir=>{for(const e of expected)assert.equal(sha(path.join(dir,e.path)),e.sha256,e.path)}
 await click('[data-testid="nav-ArchiveBrowser"]');await fill('[data-testid="file-manager-path-input-left"]',fixtures);await fill('[data-testid="file-manager-path-input-right"]',output)
 await open('paths.lzo.002')
 await step('official path members remain folders in equal dual panes with folder retrieval and search',async()=>{
  await find(entry('left','docs/'));assert.equal((await driver.findElements(By.css('.browser-title'))).length,0);await shot('paths-wide',1100,650);await shot('paths-narrow',760,520)
  await driver.actions().contextClick(await find(entry('left','docs/'))).perform();await click('[data-testid="archive-context-extract-current"]');await driver.wait(()=>existsSync(path.join(output,'docs/more/notes.txt')),20000)
  for(const e of expected.filter(e=>e.path.startsWith('docs/')))assert.equal(sha(path.join(output,e.path)),e.sha256);assert.deepEqual(readdirSync(output),['docs'])
  await click(entry('left','docs/'),true);await find(entry('left','docs/notes.txt'));await click(entry('left','docs/notes.txt'),true);assert.match(await(await find('[data-testid="archive-text-preview"]')).getAttribute('textContent'),/按目录浏览/);await shot('path-text-narrow',760,520);await click('.preview-close')
  await fill(pane('left')+' .browser-search input','more/notes');await find(entry('left','docs/more/notes.txt'));await fill(pane('left')+' .browser-search input','');await click(pane('left')+' [data-testid="archive-nav-up"]');return{folders:['docs/','docs/more/','pictures/'],folderOnlyRetrieval:true,recursiveSearch:true}
 })
 await step('path image uses existing preview and original never changes',async()=>{
  await find(entry('left','pictures/'));await click(entry('left','pictures/'),true);await click(entry('left','pictures/cover.png')+' .preview-trigger');await driver.wait(()=>driver.executeScript(()=>document.querySelector('[data-testid="archive-image-preview"] img')?.naturalWidth===32),15000);await shot('path-image-wide',1100,650);await click('.preview-close');assert.equal(sha(path.join(original,'paths.lzo')),JSON.parse(readFileSync(path.join(original,'manifest.json'),'utf8')).archiveSha256);return{image:true,originalUnchanged:true}
 })
 await step('preserve paths flat collisions and ZIP encrypted split 7Z conversions retain full directory contents',async()=>{
  const d=await bridge('diagnoseArchive',source);assert.equal(d.status,'healthy');assert.equal(d.integrityTested,true)
  const full=path.join(report.work,'full');await bridge('extractArchive',source,full,null,{preserve_paths:true});checkFiles(full)
  const flat=path.join(report.work,'flat');await bridge('extractArchive',source,flat,null,{preserve_paths:false});assert.equal(sha(path.join(flat,'notes.txt')),expected[0].sha256);assert.equal(sha(path.join(flat,'notes (member 2).txt')),expected[1].sha256);assert.equal(readdirSync(flat).length,4)
  const proof=[];for(const format of ['zip','7z']){const target=path.join(report.work,'paths.'+format);const result=await bridge('convertArchive',source,null,target,{format,level:1,...(format==='7z'?{password:'paths1661',encrypt_filenames:true,split_size:1024}:{zip_method:'deflate'})});const out=path.join(report.work,'readback-'+format);const r=spawnSync(path.join(root,'src-tauri/resources/archive-engine/7z.exe'),['x','-y',...(format==='7z'?['-ppaths1661']:[]),'-o'+out,result.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr);checkFiles(out);if(format==='7z')assert.ok(result.split&&result.files.length>1);proof.push({format,output:result})}
  const history=await bridge('taskHistory');assert.ok(history.some(h=>h.sourcePaths?.includes(source)&&h.status==='completed'));return{diagnosis:d,conversions:proof,flatCollisionStable:true,sourceHistory:true}
 })
 await step('legacy name choice stays in secondary options and reaches preview retrieval conversion and history',async()=>{
  // Genuine official CP936 single member has the path flag without multipart.
  const legacy=path.join(fixtures,'legacy.lzo');copyFileSync(path.join(original,'official-cp936.lzo'),legacy)
  await click(pane('left')+' [data-testid="archive-pane-leave"]');await fill('[data-testid="file-manager-path-input-left"]',fixtures);await open('legacy.lzo');await find(entry('left','member-001.bin'));await click(pane('left')+' [data-testid="archive-pane-tools"]');assert.match(await(await find('[data-testid="zip-password-encoding-field"]')).getText(),/LZOP 成员名称编码/)
  await click('[data-testid="browser-zip-password-encoding"]');const options=await driver.findElements(By.css('[role="option"]'));const cp=await Promise.all(options.map(async el=>({el,text:await el.getText()})));await cp.find(o=>o.text.includes('CP936')).el.click();await click(pane('left')+' button[title="重新读取"]');await find(entry('left','docs/'));await click(pane('left')+' [data-testid="archive-pane-tools"]');await click(entry('left','docs/'),true);await find(entry('left','docs/资料.txt'));await click(entry('left','docs/资料.txt'),true);assert.ok((await(await find('[data-testid="archive-text-preview"]')).getText()).length>0);await click('.preview-close');await click(pane('left')+' [data-testid="archive-pane-tools"]');await shot('name-options-narrow',760,520);await click(pane('left')+' [data-testid="archive-pane-tools"]')
  const chosen=path.join(report.work,'legacy-selected');mkdirSync(chosen,{recursive:true});await fill('[data-testid="file-manager-path-input-right"]',chosen);await click(entry('left','docs/资料.txt')+' .browser-checkbox');await click(pane('left')+' footer .browser-primary');await driver.wait(()=>existsSync(path.join(chosen,'docs/资料.txt')),20000)
  const target=path.join(report.work,'legacy.zip');const result=await bridge('convertArchive',legacy,null,target,{format:'zip',level:1,zip_method:'deflate'},'cp936');const restore=path.join(report.work,'legacy-readback');const r=spawnSync(path.join(root,'src-tauri/resources/archive-engine/7z.exe'),['x','-y','-o'+restore,result.primaryPath],{windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr);assert.equal(sha(path.join(restore,'docs/资料.txt')),sha(path.join(chosen,'docs/资料.txt')))
  const history=await bridge('taskHistory');assert.ok(history.some(h=>h.sourcePaths?.includes(legacy)&&h.executionConfig?.zipPasswordEncoding==='cp936'));return{fixture:'official Windows lzop CP936 single path member',codepage:'cp936',secondaryOptions:true,previewRetrievalConversionSameName:true,historyEncoding:true}
 })
 await step('damaged later path member publishes no earlier files and corrected source retries',async()=>{
  const bad=Buffer.from(bytes),second=bad.indexOf(Buffer.from([137,76,90,79,0,13,10,26,10]),9),block=second+38+bad[second+33];assert.ok(second>0);bad[block+8]^=1;const source=path.join(report.work,'retry.lzo');writeFileSync(source,bad);const output=path.join(report.work,'failed');mkdirSync(output,{recursive:true});writeFileSync(path.join(output,'keep.txt'),'keep');let failure='';try{await bridge('extractArchive',source,output,null,{preserve_paths:true})}catch(e){failure=String(e)}assert.match(failure,/docs\/more\/notes.txt/);assert.deepEqual(readdirSync(output),['keep.txt']);assert.equal(readFileSync(path.join(output,'keep.txt'),'utf8'),'keep');writeFileSync(source,bytes);await bridge('extractArchive',source,output,null,{preserve_paths:true});checkFiles(output);return{failure,noPartialPublish:true,correctedRetry:true}
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


// The final candidate changes only the new refresh button's cramped text.
// Reuse business receipts and inspect the affected secondary options, not the old matrix.
export async function runLzopFinal1661Gate({driver,root,application}) {
 const base=path.join(root,'test-results/lzop-paths-names-1661'),business=JSON.parse(readFileSync(path.join(base,'desktop-business.json'),'utf8'))
 assert.equal(business.passed,true)
 const report={version:'1.6.61',applicationSha256:createHash('sha256').update(readFileSync(application)).digest('hex'),reusedBusinessCandidateSha256:business.applicationSha256,checks:[],layouts:[]}
 const find=s=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(s))){if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}return false},20000,s)
 const idle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(s,twice=false)=>{const el=await find(s);await driver.executeScript('arguments[0].scrollIntoView({block:"center"})',el);await idle();if(twice)await driver.actions().doubleClick(await find(s)).perform();else await(await find(s)).click()}
 const pane='[data-file-pane="left"]'
 await click('[data-testid="nav-ArchiveBrowser"]');if((await driver.findElements(By.css(pane+' [data-testid="archive-pane-leave"]'))).length)await click(pane+' [data-testid="archive-pane-leave"]')
 const input=await find('[data-testid="file-manager-path-input-left"]');await input.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE,path.join(base,'fixtures'),Key.ENTER);await click(pane+' .file-row[data-path$="legacy.lzo"]',true);await find(pane+' [data-entry-path="member-001.bin"]');await click(pane+' [data-testid="archive-pane-tools"]');await click('[data-testid="browser-zip-password-encoding"]')
 for(const option of await driver.findElements(By.css('[role="option"]'))){if((await option.getText()).includes('CP936')){await option.click();break}}
 const refresh=await find(pane+' button[title="重新读取"]');assert.equal((await refresh.getText()).trim(),'');await refresh.click();await find(pane+' [data-entry-path="docs/"]')
 for(const [name,width,height] of [['final-options-wide',1100,650],['final-options-narrow',760,520]]){
  resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await idle()
  const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>e.getBoundingClientRect().width),options:document.querySelector('.archive-options').getBoundingClientRect().toJSON()}));assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0]-layout.panes[1])<=1);assert.ok(layout.overflow<=1);assert.ok(layout.options.x>=0&&layout.options.y>=0&&layout.options.right<=width+1&&layout.options.bottom<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout})
 }
 await click(pane+' [data-testid="archive-pane-tools"]');await click(pane+' [data-entry-path="docs/"]',true);await find(pane+' [data-entry-path="docs/资料.txt"]')
 report.checks.push({name:'final compact refresh control and source name codepage fit the original secondary options in equal wide/narrow panes',officialCp936Names:true});report.passed=true;writeFileSync(path.join(base,'desktop-final-ui.json'),JSON.stringify(report,null,2)+'\n');console.log('[lzop1661] final affected UI passed; earlier business receipts reused')
}

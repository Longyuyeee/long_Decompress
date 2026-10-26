import assert from 'node:assert/strict'
import { createHash, randomUUID, randomBytes } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, openSync, writeSync, closeSync, createReadStream, unlinkSync, appendFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runLzopPrefix174Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/lzop-prefix-rescue-174'),file=path.join(base,'desktop.json'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.7.4',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[lzop174]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){assert.notEqual(await el.getAttribute('aria-disabled'),'true');await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value,enter=false)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value);if(enter)await el.sendKeys(Key.ENTER)}
 const shot=async(name,width,height,focus)=>{resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();if(focus){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),await find(focus));await settle()}const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);if(layout.panes.length){assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1))}for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}
 const native=JSON.parse(readFileSync(path.join(base,'native-fixture.json'),'utf8')),oracle=JSON.parse(readFileSync(path.join(root,'tests/fixtures/archives/lzop-prefix-174/manifest.json'),'utf8')),full=readFileSync(path.join(root,'tests/fixtures/archives/lzop-prefix-174/complete.lzo'))
 const manifestFile=path.join(base,'desktop-outputs.json'),outputs=existsSync(manifestFile)?JSON.parse(readFileSync(manifestFile,'utf8')):[]
 const walk=dir=>readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(dir,e.name)).map(p=>e.name+'/'+p):[e.name])
 const check=(dir,names)=>{assert.deepEqual(walk(dir).sort(),[...names].sort());for(const n of names)assert.equal(sha(path.join(dir,n)),oracle.entries.find(e=>e.name===n).sha256)}
 const record=(dir,names)=>{if(!outputs.some(o=>o.outputDirectory===dir)){outputs.push({outputDirectory:dir,entries:names});writeFileSync(manifestFile,JSON.stringify(outputs,null,2)+'\n')}}
 const closeRecovery=async()=>{for(const el of await driver.findElements(By.css('[role="dialog"] .recovery-button')))if(await el.isDisplayed()&&(await el.getText())==='关闭'){await el.click();return}throw Error('Missing close')}
 const openRecovery=async source=>{await bridge('openRecoveryDiagnosis',source);await find('[data-testid="open-lzop-recovery"]');await click('[data-testid="open-lzop-recovery"]');await find('[data-testid="lzop-recovery-dialog"]')}
 const waitResult=()=>driver.wait(async()=>{const errors=await driver.findElements(By.css('[data-testid="lzop-recovery-error"]'));for(const e of errors)assert.equal(await e.getText(),'');const rows=await bridge('taskHistory');return rows.find(r=>r.executionConfig?.kind==='archive-recovery'&&r.status==='completed'&&!outputs.some(o=>o.outputDirectory===r.outputPath))||false},90000)
 resizeHost(application,1100,650,await driver.executeScript(()=>devicePixelRatio))
 for(const prompt of await driver.findElements(By.css('[aria-label="关闭默认打开提醒"]')))if(await prompt.isDisplayed())await prompt.click()
 await step('failed extraction opens compact rescue modal and saves selected checked members with distinct history',async()=>{
  await bridge('clearTasks');await bridge('clearTaskHistory');const before=sha(native.source);let failure='';try{await bridge('extractArchive',native.source,path.join(report.work,'normal-failed'),null)}catch(e){failure=String(e)}assert.ok(failure)
  const failed=(await bridge('extractionRecoveryTasks')).find(t=>t.status==='failed');assert.ok(failed);report.failedId=failed.id
  await click('[data-testid="nav-Decompress"]');await click(`[data-task-id="${failed.id}"]`);await find('[data-testid="inline-extraction-recovery"]');await click('[data-testid="retry-rescue"]');await find('[data-testid="open-lzop-recovery"]')
  assert.match(await(await find('[data-testid="diagnostic-report"]')).getText(),/可部分取回完整(?:成员|条目)/);await click('[data-testid="open-lzop-recovery"]');await find('[data-recovery-member="3"]');await click('[data-recovery-member="3"] input');await fill('[data-testid="lzop-recovery-output"]',report.work)
  assert.equal(await(await find('.stop-detail')).getAttribute('open'),null);await shot('selection-wide',1100,650,'[data-testid="lzop-recovery-members"]');await shot('selection-narrow',760,520,'[data-testid="lzop-recovery-members"]');await click('[data-testid="lzop-recovery-confirm"]');await find('[data-testid="lzop-recovery-result"]')
  const row=await waitResult(),names=oracle.entries.slice(0,3).map(e=>e.name);check(row.outputPath,names);record(row.outputPath,names);assert.equal(row.executionConfig.recoveredFiles,3);assert.equal(row.retryOfTaskId,failed.id);assert.equal(sha(native.source),before);assert.equal((await bridge('taskHistory')).find(r=>r.id===failed.id).status,'failed');await shot('result-narrow',760,520,'[data-testid="lzop-recovery-result"]');await closeRecovery()
  await click('[data-testid="nav-History"]');await fill('[data-testid="history-search"]','LZOP 部分取回');await find('[data-testid="history-record-row"]');assert.match(await(await find('[data-testid="history-status-badge"]')).getText(),/已部分取回/);assert.equal((await driver.findElements(By.css('[data-testid="history-retry"]'))).length,0);await shot('history-wide',1100,650);await click('[data-testid="history-record-row"] h3');await find('[data-testid="history-recovery-details"]');assert.equal((await driver.findElements(By.css('[data-testid="history-extraction-draft"],[data-testid="completed-extraction-output"]'))).length,0);await shot('history-detail-narrow',760,520,'[data-testid="history-recovery-details"]');await click('[data-testid="history-detail"] header button')
  return{originalFailureRetained:true,selectedFiles:3,emptyFileRetained:true,originalUnchanged:true,historyLabel:'已部分取回',output:row.outputPath}
 })
 await step('nonfirst numeric volume offers only continuous-prefix files and complete group restores normally after replacement',async()=>{
  const dir=path.join(report.work,'volumes');mkdirSync(dir,{recursive:true});const size=262144,parts=[];for(let at=0,n=1;at<full.length;at+=size,n++){const p=path.join(dir,`资料.lzo.${String(n).padStart(3,'0')}`);writeFileSync(p,full.subarray(at,at+size));parts.push(p)}
  const missing=parts[2],withheld=readFileSync(missing);unlinkSync(missing);await openRecovery(parts.at(-1));assert.match(await(await find('[data-testid="lzop-recovery-count"]')).getText(),/^2 /);await click('.stop-detail summary');await shot('volume-gap-narrow',760,520,'.stop-detail');await fill('[data-testid="lzop-recovery-output"]',report.work);await click('[data-testid="lzop-recovery-confirm"]');await find('[data-testid="lzop-recovery-result"]');const row=await waitResult(),names=oracle.entries.slice(0,2).map(e=>e.name);check(row.outputPath,names);record(row.outputPath,names);await closeRecovery()
  writeFileSync(missing,withheld);const out=path.join(report.work,'after-replacement');await bridge('extractArchive',parts.at(-1),out,null);check(out,oracle.entries.map(e=>e.name));assert.equal(createHash('sha256').update(Buffer.concat(parts.map(p=>readFileSync(p)))).digest('hex'),oracle.sha256)
  return{nonfirstEntry:true,missingMiddleNotBridged:true,verifiedPrefixFiles:2,replacedGroupExtractedFiles:5}
 })
 await step('no complete member disables rescue and source replacement requires a fresh check',async()=>{
  const empty=path.join(report.work,'unreadable.lzo');writeFileSync(empty,Buffer.from('broken header'));await openRecovery(empty);assert.equal(await(await find('[data-testid="lzop-recovery-confirm"]')).isEnabled(),false);assert.match(await(await find('.empty-message')).getText(),/没有可确认/);await closeRecovery()
  const changed=path.join(report.work,'changed.lzo');writeFileSync(changed,readFileSync(native.source));await openRecovery(changed);await fill('[data-testid="lzop-recovery-output"]',report.work);writeFileSync(changed,full);const before=new Set(readdirSync(report.work));await click('[data-testid="lzop-recovery-confirm"]');await driver.wait(async()=>{const e=await driver.findElements(By.css('[data-testid="lzop-recovery-error"]'));return e.length&&(await e[0].getText()).includes('重新检查')},30000);assert.deepEqual(new Set(readdirSync(report.work)),before);await click('[data-testid="lzop-recovery-recheck"]');await driver.wait(async()=>!(await(await find('[data-testid="lzop-recovery-confirm"]')).isEnabled()),30000);assert.match(await(await find('.empty-message')).getText(),/现有内容完整/);await closeRecovery()
  return{noFabricatedFiles:true,changedSourcePublishedNothing:true,recheckReturnsNormalFlow:true}
 })
 await step('real copy-stage cancellation removes staged output and a subsequent small rescue succeeds',async()=>{
  const data=path.join(report.work,'large.bin'),archive=path.join(report.work,'cancel.lzo'),dir=path.join(report.work,'cancel-output');mkdirSync(dir,{recursive:true});if(!existsSync(data)){const chunk=randomBytes(2097152),fd=openSync(data,'w');try{for(let i=0;i<48;i++)writeSync(fd,chunk)}finally{closeSync(fd)}}
  if(!existsSync(archive)){const id=await bridge('startSevenZipCompression',data,archive,'lzo');await driver.wait(async()=>{const status=await bridge('taskStatus',id);assert.notEqual(status,'failed');return status==='completed'},120000);appendFileSync(archive,Buffer.from('broken-tail'))}
  const before=sha(archive);await openRecovery(archive);await fill('[data-testid="lzop-recovery-output"]',dir);await bridge('armLzopRecoveryCancellation');await click('[data-testid="lzop-recovery-confirm"]');await driver.wait(async()=>{const rows=await bridge('taskHistory');return rows.find(r=>r.executionConfig?.kind==='archive-recovery'&&r.sourcePaths.includes(archive)&&r.status==='cancelled')||false},90000);const cut=await bridge('lzopRecoveryCancellation');assert.ok(cut?.bytes>0&&cut.bytes<cut.total,JSON.stringify(cut));assert.deepEqual(readdirSync(dir),[]);assert.equal(sha(archive),before);await closeRecovery()
  await openRecovery(native.source);await fill('[data-testid="lzop-recovery-output"]',dir);await click('[data-testid="lzop-recovery-confirm"]');await find('[data-testid="lzop-recovery-result"]');const row=await waitResult(),names=oracle.entries.slice(0,4).map(e=>e.name);check(row.outputPath,names);record(row.outputPath,names);await closeRecovery()
  return{copyStage:cut,partialFilesPublished:0,nextRescueSucceeded:true,sourceUnchanged:true}
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

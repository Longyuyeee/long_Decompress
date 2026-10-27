import assert from 'node:assert/strict'
import { createHash, randomUUID, randomBytes } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, openSync, writeSync, closeSync, createReadStream, unlinkSync, appendFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

import {createServer} from 'node:http'
const fixture=process.argv.includes('--updater-download-176-only')?await startFixture():null
async function startFixture(){
 const f={paused:false,mode:'legacy',requests:[],checks:0,size:4194304}
 const server=createServer((req,res)=>{
  if(req.url==='/latest.json'){f.checks++;res.writeHead(200,{'Content-Type':'application/json'});res.end(JSON.stringify({version:'99.0.0',notes:'Updater transfer regression fixture',pub_date:'2026-10-27T00:00:00Z',platforms:{'windows-x86_64':{url:`http://127.0.0.1:${server.address().port}/update.bin`,signature:'*fixture-invalid-signature*'}}}));return}
  if(req.url!=='/update.bin'){res.writeHead(404);res.end();return}
  const record={mode:f.mode,bytes:0,complete:false};f.requests.push(record);res.writeHead(200,{'Content-Type':'application/octet-stream','Content-Length':f.size})
  const chunk=Buffer.alloc(65536,117);let timer=setInterval(()=>{if(f.paused)return;res.write(chunk);record.bytes+=chunk.length;if(record.bytes===f.size){clearInterval(timer);record.complete=true;res.end()}},80)
  res.on('close',()=>clearInterval(timer))
 });await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve)});server.unref();f.server=server;process.env.LONG_E2E_UPDATER_ENDPOINT=`http://127.0.0.1:${server.address().port}/latest.json`;return f
}

export async function runUpdaterDownload176Gate({driver,bridge,root,application}) {
 const base=path.join(root,'test-results/updater-download-176'),file=path.join(base,'desktop.json'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const report=existsSync(file)?JSON.parse(readFileSync(file,'utf8')):{version:'1.7.6',applicationSha256:sha(application),checks:[],layouts:[],work:path.join(base,randomUUID())}
 assert.equal(report.applicationSha256,sha(application));mkdirSync(report.work,{recursive:true})
 const save=()=>writeFileSync(file,JSON.stringify(report,null,2)+'\n')
 const step=async(name,fn)=>{if(report.checks.some(c=>c.name===name))return;report.checks.push({name,...await fn()});save();console.log('[updater176]',name)}
 const find=selector=>driver.wait(async()=>{for(const el of await driver.findElements(By.css(selector))){try{if(!await el.isDisplayed())await driver.executeScript('arguments[0].scrollIntoView({block:"nearest"})',el);if(await el.isDisplayed())return el}catch(e){if(e.name!=='StaleElementReferenceError')throw e}}return false},20000,selector)
 const settle=()=>driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
 const click=async(selector,twice=false)=>{for(const toast of await driver.findElements(By.css('.toast-item button')))if(await toast.isDisplayed())await toast.click();await find(selector);await driver.executeScript(s=>document.querySelector(s)?.scrollIntoView({block:'center'}),selector);await settle();const el=await find(selector);if(twice)await driver.actions().doubleClick(el).perform();else await el.click()}
 const choose=async(selector,label)=>{await click(selector);for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.findElement(By.css('strong')).getAttribute('textContent'))===label){assert.notEqual(await el.getAttribute('aria-disabled'),'true');await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click();return}throw Error('Missing option '+label)}
 const fill=async(selector,value,enter=false)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value);if(enter)await el.sendKeys(Key.ENTER)}
 const shot=async(name,width,height,focus)=>{resizeHost(application,width,height,await driver.executeScript(()=>devicePixelRatio));await driver.wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,width,height),10000);await settle();if(focus){await driver.executeScript(e=>e.scrollIntoView({block:'center'}),await find(focus));await settle()}const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('[data-file-pane]')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})),dialogs:[...document.querySelectorAll('[role="dialog"]')].map(e=>e.getBoundingClientRect().toJSON())}));assert.ok(layout.overflow<=1);if(layout.panes.length){assert.equal(layout.panes.length,2);assert.ok(Math.abs(layout.panes[0].width-layout.panes[1].width)<=1);assert.ok(layout.panes.every(p=>p.overflow<=1))}for(const d of layout.dialogs)assert.ok(d.x>=0&&d.y>=0&&d.x+d.width<=width+1&&d.y+d.height<=height+1);writeFileSync(path.join(base,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});save()}


 const state=()=>bridge('exerciseUpdater','state')
 const waitError=()=>driver.wait(async()=>{const s=await state();return s.status==='error'?s:false},30000)
 try{
 await bridge('exerciseUpdater','gold-theme')
 for(const prompt of await driver.findElements(By.css('[aria-label="关闭默认打开提醒"]')))if(await prompt.isDisplayed())await prompt.click()
 await step('the original framework API retains two installers after two checks and combines both transfers',async()=>{
  await bridge('exerciseUpdater','legacy-check');await bridge('exerciseUpdater','legacy-check');assert.equal(fixture.requests.length,0)
  await bridge('exerciseUpdater','legacy-start');const over=await driver.wait(async()=>{const s=await state();if(s.bytes>fixture.size&&s.phase==='downloading'){fixture.paused=true;return s}return false},15000)
  assert.equal(fixture.requests.length,2);assert.equal(over.total,null);assert.equal(over.percent,null);assert.ok(over.bytes>fixture.size);await shot('legacy-overlap-indeterminate',760,520,'[data-testid="update-progress"]')
  fixture.paused=false;await waitError();await driver.wait(()=>fixture.requests.every(r=>r.complete),15000)
  return{nativeChecks:2,actualDownloadRequests:2,combinedBytesObserved:over.bytes,singlePayloadBytes:fixture.size,impossibleFractionHidden:true,noInstallerStarted:true}
 })
 fixture.mode='fixed';
 await step('three new checks retain one response and one real download with native reentry blocked',async()=>{
  await bridge('exerciseUpdater','check');await bridge('exerciseUpdater','check');await bridge('exerciseUpdater','check');assert.equal((await state()).status,'available');assert.equal(fixture.requests.filter(r=>r.mode==='fixed').length,0)
  await bridge('exerciseUpdater','start');const started=await driver.wait(async()=>{const s=await state();if(s.bytes>0&&s.bytes<fixture.size&&s.phase==='downloading'){fixture.paused=true;return s}return false},15000)
  const duplicate=await bridge('exerciseUpdater','duplicate-install'),check=await bridge('exerciseUpdater','native-check');assert.match(duplicate.error,/正在进行/);assert.match(check.error,/正在进行/);assert.equal(fixture.requests.filter(r=>r.mode==='fixed').length,1)
  assert.equal(started.total,fixture.size);await shot('single-transfer-wide',1100,650,'[data-testid="update-progress"]');await shot('single-transfer-narrow',760,520,'[data-testid="update-progress"]')
  fixture.paused=false;const samples=[];const final=await driver.wait(async()=>{const s=await state();samples.push(s);assert.ok(s.bytes<=fixture.size);assert.equal(s.total,fixture.size);return s.status==='error'?s:false},30000)
  assert.equal(final.bytes,fixture.size);assert.match(final.error,/signature|base64|decode|encoding|Invalid/i);assert.ok(samples.every((s,i)=>i===0||s.bytes>=samples[i-1].bytes));assert.equal(fixture.requests.filter(r=>r.mode==='fixed').length,1)
  return{nativeChecks:3,actualDownloadRequests:1,bytes:final.bytes,total:final.total,monotonic:true,nativeDuplicateInstallRejected:duplicate.error,nativeCheckDuringInstallRejected:check.error,invalidSignatureStopsBeforeInstallation:true}
 })
 fixture.mode='retry'
 await step('retry clears the old transfer and consumes only the newly checked response',async()=>{
  await bridge('exerciseUpdater','check');const clean=await state();assert.equal(clean.bytes,0);assert.equal(clean.total,null)
  await bridge('exerciseUpdater','start');const final=await waitError();assert.equal(final.bytes,fixture.size);assert.equal(final.total,fixture.size);assert.equal(fixture.requests.filter(r=>r.mode==='retry').length,1)
  const consumed=await bridge('exerciseUpdater','duplicate-install');assert.match(consumed.error,/重新检查|先检查/);assert.equal(fixture.requests.filter(r=>r.mode==='retry').length,1)
  return{counterReset:true,actualDownloadRequests:1,responseConsumedOnce:true,noInstallerStarted:true}
 })
 report.network={manifestRequests:fixture.checks,downloads:fixture.requests};report.passed=true;save()
 }finally{report.network={manifestRequests:fixture.checks,downloads:fixture.requests};save();fixture.paused=false;fixture.server.closeAllConnections();fixture.server.close()}
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

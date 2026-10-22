import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { By } from 'selenium-webdriver'

function nativeWindow(application,bounds) {
  const escaped=application.replaceAll("'","''")
  const script=`Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class WindowMemoryResize {
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int width,int height,uint flags);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd,out RECT rect);
}
'@
[void][WindowMemoryResize]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
$taskHandle=(Get-Process -Id $taskApp.ProcessId).MainWindowHandle
${bounds ? `if (-not [WindowMemoryResize]::SetWindowPos($taskHandle,[IntPtr]::Zero,${bounds.x},${bounds.y},${bounds.width},${bounds.height},0x14)) { exit 3 }` : ''}
$taskRect=New-Object WindowMemoryResize+RECT
if (-not [WindowMemoryResize]::GetWindowRect($taskHandle,[ref]$taskRect)) { exit 4 }
[pscustomobject]@{left=$taskRect.Left;top=$taskRect.Top;width=$taskRect.Right-$taskRect.Left;height=$taskRect.Bottom-$taskRect.Top} | ConvertTo-Json -Compress`
  const result=spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-Command',script],{encoding:'utf8',timeout:15000,windowsHide:true})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
  return JSON.parse(result.stdout.trim())
}

export async function runWindowMemory166Gate({getDriver,restart,bridge,root,application,dataDirectory,forwardContextAction}) {
  const out = path.join(root,'test-results','window-memory-166')
  mkdirSync(out,{recursive:true})
  const stateFile = path.join(dataDirectory,'window-state.json')
  const resizeOnly=process.argv.includes('--window-memory-166-resize-only')
  const report = resizeOnly ? JSON.parse(readFileSync(path.join(out,'desktop.json'),'utf8')) : {version:'1.6.6',issue:248,applicationSha256:createHash('sha256').update(readFileSync(application)).digest('hex'),checks:[],layouts:[],limitations:['Physical desktop has its current monitor configuration; 5K, negative monitor coordinates and DPI/work-area changes additionally covered by pure bounds tests.']}
  assert.equal(report.applicationSha256,createHash('sha256').update(readFileSync(application)).digest('hex'))
  report.passed=false
  const saveReport = () => writeFileSync(path.join(out,'desktop.json'),JSON.stringify(report,null,2)+'\n')
  const state = () => JSON.parse(readFileSync(stateFile,'utf8'))
  const readNativeRect=()=>nativeWindow(application)
  const scale=await getDriver().executeScript(()=>devicePixelRatio)
  const resizeNative = ({x,y,width,height})=>nativeWindow(application,{x:Math.round(x*scale),y:Math.round(y*scale),width:Math.round(width*scale),height:Math.round(height*scale)})
  const wait = (fn,label) => getDriver().wait(async()=>{try{return await fn()}catch{return false}},15000,label)
  const click = async label => (await getDriver().findElement(By.css(`.window-titlebar button[aria-label="${label}"]`))).click()
  const equalBounds = (actual,expected,label) => {
    for(const key of ['left','top','width','height']) assert.ok(Math.abs(actual[key]-expected[key])<=2,`${label}: ${key} ${actual[key]} vs ${expected[key]}`)
  }
  const shot = async name => {
    const layout = await getDriver().executeScript(()=>({width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth,bodyWidth:document.body.scrollWidth,titlebar:!!document.querySelector('.window-titlebar'),nav:[...document.querySelectorAll('aside nav>button')].map(e=>({text:e.textContent.trim(),width:e.clientWidth,scroll:e.scrollWidth}))}))
    assert.ok(layout.scrollWidth<=layout.width+1 && layout.bodyWidth<=layout.width+1,`${name}: no horizontal page overflow`)
    assert.ok(layout.titlebar && layout.nav.length===11 && layout.nav.every(n=>n.text && n.scroll<=n.width+1))
    writeFileSync(path.join(out,`${name}.png`),Buffer.from(await getDriver().takeScreenshot(),'base64'))
    report.layouts=report.layouts.filter(item=>item.name!==name);report.layouts.push({name,...layout});saveReport()
  }
  const record = name => {report.checks=[...new Set([...report.checks.filter(item=>item!=='normal size and position survive real process restart'),name])];saveReport();console.log(`[window-memory-166] ${name}`)}
  // WebView2's browser window command can acknowledge without resizing its
  // native host. Mutate the actual HWND and require measured physical bounds.
  resizeNative({x:60,y:40,width:1100,height:600})
  await wait(()=>{const r=readNativeRect();return r.left===60*scale&&r.top===40*scale&&r.width===1100*scale&&r.height===600*scale},'native resize reached requested bounds')
  const normal = readNativeRect()
  await wait(()=>existsSync(stateFile)&&!state().maximized&&Math.abs(state().width*scale-normal.width)<=2&&Math.abs(state().height*scale-normal.height)<=2,'resized normal window saved')
  await restart({preserveProfile:true})
  equalBounds(readNativeRect(),normal,'real process restart normal bounds')
  report.normalBounds=normal
  await shot('restored-normal');record('measured native resize and move survive real process restart')

  const beforeMax = state()
  await click('最大化或还原')
  await wait(()=>state().maximized,'maximum persisted')
  assert.equal(state().width,beforeMax.width);assert.equal(state().height,beforeMax.height)
  const maximized = readNativeRect()
  await restart({preserveProfile:true})
  await wait(()=>state().maximized,'maximum restored')
  equalBounds(readNativeRect(),maximized,'maximized restart')
  await click('最大化或还原')
  await wait(()=>!state().maximized,'normal restored from maximum')
  equalBounds(readNativeRect(),normal,'restore button retains previous normal bounds')
  record('maximum survives restart and restore button retains original normal bounds')

  if(resizeOnly) {
    resizeNative({x:80,y:60,width:760,height:520})
    await wait(()=>{const r=readNativeRect();return r.width===760*scale&&r.height===520*scale},'native small size')
    await shot('small-window');report.passed=true;saveReport();return
  }

  await click('最大化或还原');await wait(()=>state().maximized,'maximum before minimize')
  await click('最小化')
  await restart({preserveProfile:true})
  equalBounds(readNativeRect(),maximized,'minimized maximum reopens visibly maximized')
  await click('最大化或还原');await wait(()=>!state().maximized,'return normal')
  equalBounds(readNativeRect(),normal,'minimize did not overwrite normal bounds')
  record('minimizing before exit preserves maximum and normal bounds, next ordinary launch visible')

  await bridge('setCloseToTray',true)
  await click('关闭');await wait(async()=>!(await bridge('isWindowVisible')),'X hides actual window')
  const marker = path.join(out,'tray-restored.marker')
  forwardContextAction('--desktop-e2e-restore',[marker])
  await wait(()=>existsSync(marker)&&readFileSync(marker,'utf8')==='visible','second instance restores tray')
  equalBounds(readNativeRect(),normal,'tray wake retains bounds')
  record('actual X still hides to tray and second instance wakes same window at same bounds')

  await click('最大化或还原');await wait(()=>state().maximized,'maximum before hidden startup')
  await restart({preserveProfile:true,applicationArgs:['--autostart']})
  assert.equal(await bridge('isWindowVisible'),false,'logon activation must stay hidden')
  const autoMarker=path.join(out,'autostart-restored.marker')
  forwardContextAction('--desktop-e2e-restore',[autoMarker])
  await wait(()=>existsSync(autoMarker)&&readFileSync(autoMarker,'utf8')==='visible','autostart tray wake')
  equalBounds(readNativeRect(),maximized,'hidden startup retained maximum')
  await click('最大化或还原');await wait(()=>!state().maximized,'normal after hidden maximum')
  equalBounds(readNativeRect(),normal,'hidden startup retained normal geometry')
  record('maximized logon activation remains hidden; wake and restore retain both bounds')

  await restart({preserveProfile:true,afterExit:()=>{
    const saved=state();saved.monitor='disconnected-monitor';saved.x=50000;saved.y=-50000;saved.width=10000;saved.height=8000
    writeFileSync(stateFile,JSON.stringify(saved))
  }})
  const fallback=readNativeRect()
  const screen=await getDriver().executeScript(()=>({width:screen.width*devicePixelRatio,height:screen.height*devicePixelRatio}))
  assert.ok(fallback.left>=-2 && fallback.top>=-2 && fallback.width<=screen.width+2 && fallback.height<=screen.height+2,'disconnected/oversized window fits available primary screen')
  await shot('disconnected-screen-fallback');record('disconnected monitor, offscreen position and oversized bounds fall back visibly')

  await restart({preserveProfile:true,afterExit:()=>writeFileSync(stateFile,'{interrupted')})
  assert.equal(await bridge('isWindowVisible'),true)
  const defaults=readNativeRect();assert.ok(defaults.width>=760&&defaults.height>=520)
  record('corrupted state falls back to usable default window')
  resizeNative({x:80,y:60,width:760,height:520})
  await wait(()=>{const r=readNativeRect();return r.width===760*scale&&r.height===520*scale},'native small size')
  await shot('small-window')
  report.passed=true;saveReport()
}

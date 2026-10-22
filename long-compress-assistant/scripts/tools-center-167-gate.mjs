import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

function resizeHost(application,width,height,scale) {
  const escaped=application.replaceAll("'","''")
  const result=spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-Command',`Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ToolCenterWindow {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
}
'@
[void][ToolCenterWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$taskApp=Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq '${escaped}' }
if (@($taskApp).Count -ne 1) { exit 2 }
if (-not [ToolCenterWindow]::SetWindowPos((Get-Process -Id $taskApp.ProcessId).MainWindowHandle,[IntPtr]::Zero,${Math.round(10*scale)},${Math.round(10*scale)},${Math.round(width*scale)},${Math.round(height*scale)},0x14)) { exit 3 }`],{encoding:'utf8',timeout:15000,windowsHide:true})
  assert.ifError(result.error);assert.equal(result.status,0,result.stderr)
}

export async function runToolsCenter167Gate({driver,bridge,root,application,bundledSevenZip,forwardContextAction}) {
  const out=path.join(root,'test-results','tools-center-167');mkdirSync(out,{recursive:true})
  const reportPath=path.join(out,'desktop.json'), sha=createHash('sha256').update(readFileSync(application)).digest('hex')
  const report=existsSync(reportPath)?JSON.parse(readFileSync(reportPath,'utf8')):{version:'1.6.7',candidateSha256:sha,checks:[],layouts:[]}
  const layoutOnly=process.argv.includes('--tools-center-167-layout-only')
  if(!layoutOnly)assert.equal(report.candidateSha256,sha,'checkpoints belong to this exact candidate')
  report.layoutCandidateSha256=sha;report.passed=false
  const save=()=>writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
  const wait=(fn,label)=>driver.wait(async()=>{try{return await fn()}catch{return false}},20000,label)
  const find=async selector=>{await wait(async()=>{const list=await driver.findElements(By.css(selector));return list.length&&await list[0].isDisplayed()},selector);return driver.findElement(By.css(selector))}
  const settle=()=>wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),'finite UI motion settled')
  const click=async selector=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await settle();await el.click()}
  const fill=async(selector,value)=>{const el=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'nearest'}),el);await el.clear();await el.sendKeys(value)}
  const tool=async name=>{if(!(await driver.findElements(By.css('[data-testid="tools-center"]'))).length)await click('[data-testid="nav-ToolsCenter"]');await click(`[data-testid="tool-${name}"]`);await wait(async()=>await(await find(`[data-testid="tool-${name}"]`)).getAttribute('aria-current')==='page',name)}
  const hotkey=async(key,shift=true)=>{await driver.executeScript(()=>document.activeElement?.blur());const actions=driver.actions().keyDown(Key.CONTROL);if(shift)actions.keyDown(Key.SHIFT);actions.sendKeys(key);if(shift)actions.keyUp(Key.SHIFT);await actions.keyUp(Key.CONTROL).perform()}
  const step=async(name,fn)=>{if(report.checks.includes(name))return;await fn();report.checks.push(name);save();console.log(`[tools-center-167] ${name}`)}
  const scale=await driver.executeScript(()=>devicePixelRatio)
  const resize=async(w,h)=>{resizeHost(application,w,h,scale);await wait(()=>driver.executeScript((w,h)=>innerWidth===w&&innerHeight===h,w,h),'native host resized');await settle()}
  const shot=async(name,width,height)=>{
    await driver.executeScript(()=>document.activeElement?.blur());await settle()
    const layout=await driver.executeScript(()=>{
      const nav=document.querySelector('.app-sidebar>nav'),nr=nav.getBoundingClientRect(),stage=document.querySelector('.tools-stage'),s=stage.getBoundingClientRect()
      const overflow=[document.documentElement,document.querySelector('.tools-center'),stage,stage.firstElementChild].filter(e=>e&&e.scrollWidth>e.clientWidth+1).map(e=>({class:e.className,client:e.clientWidth,scroll:e.scrollWidth}))
      return {width:innerWidth,height:innerHeight,color:document.documentElement.dataset.colorScheme,overflow,stageHeight:s.height,navScrollbar:getComputedStyle(nav).scrollbarWidth,
        invisibleNav:[...nav.querySelectorAll('.nav-entry')].filter(e=>{const r=e.getBoundingClientRect();return r.top<nr.top-1||r.bottom>nr.bottom+1}).map(e=>e.textContent.trim()),
        cutLabels:[...document.querySelectorAll('.sidebar-nav-label,.tools-tabs button span')].filter(e=>e.scrollWidth>e.clientWidth+1).map(e=>e.textContent.trim()),headings:[...document.querySelectorAll('.app-main h1')].map(e=>e.textContent.trim()),
        tabs:[...document.querySelectorAll('.tools-tabs button')].map(e=>({text:e.textContent.trim(),active:e.getAttribute('aria-current')}))}
    })
    assert.equal(layout.width,width,'capture must use the named actual viewport');assert.equal(layout.height,height)
    assert.deepEqual(layout.overflow,[],JSON.stringify(layout));assert.deepEqual(layout.invisibleNav,[],JSON.stringify(layout));assert.deepEqual(layout.cutLabels,[]);assert.equal(layout.navScrollbar,'none');assert.deepEqual(layout.headings,['工具中心']);assert.equal(layout.tabs.length,4);assert.equal(layout.tabs.filter(t=>t.active==='page').length,1);assert.ok(layout.stageHeight>=layout.height*.68,JSON.stringify(layout))
    writeFileSync(path.join(out,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'))
    report.layouts=report.layouts.filter(item=>item.name!==name);report.layouts.push({name,...layout});save()
  }
  const fixture=path.join(out,'fixtures');mkdirSync(fixture,{recursive:true})
  const left=path.join(fixture,'left'),right=path.join(fixture,'right');mkdirSync(left,{recursive:true});mkdirSync(right,{recursive:true})
  const payload='Tool center retained work\n',file=path.join(left,'readme.txt');writeFileSync(file,payload);writeFileSync(path.join(right,'readme.txt'),payload)
  const archive=path.join(fixture,'工具中心资料.zip')
  if(!existsSync(archive)){const result=spawnSync(bundledSevenZip,['a','-tzip','-y',archive,file],{encoding:'utf8',windowsHide:true});assert.equal(result.status,0,result.stderr)}

  await resize(1100,650)
  if(!layoutOnly) {
    await step('eight sidebar entries and tool-center shortcut, with all four original shortcuts retained',async()=>{
      const nav=await driver.findElements(By.css('aside nav>button'));assert.equal(nav.length,8)
      assert.equal(await(await find('[data-testid="nav-ToolsCenter"] .sidebar-shortcut')).getText(),'Ctrl+Shift+T')
      for(const old of ['SourceComparison','Workflows','ArchiveLibrary','FileIntegrity'])assert.equal((await driver.findElements(By.css(`[data-testid="nav-${old}"]`))).length,0)
      await hotkey('t');await find('[data-testid="tools-center"]')
      for(const [name,key,shift]of [['Workflows','w',true],['ArchiveLibrary','l',true],['FileIntegrity','i',false],['SourceComparison','d',true]]){await hotkey(key,shift);await wait(async()=>await(await find(`[data-testid="tool-${name}"]`)).getAttribute('aria-current')==='page','original tool shortcut')}
    })
    await step('comparison native reads, selection and inline expansion survive category switching',async()=>{
      await tool('SourceComparison')
      for(const [side,folder] of [['left',left],['right',right]]){await fill(`[data-comparison-path="${side}"]`,folder);await click(`[data-comparison-read="${side}"]`)}
      await find('[data-comparison-row="readme.txt"]');await click('[data-comparison-select="left:readme.txt"]');await click('[data-comparison-expand="readme.txt"]')
      await tool('Workflows');await tool('SourceComparison')
      assert.equal(await(await find('[data-comparison-select="left:readme.txt"]')).isSelected(),true)
      assert.equal(await(await find('[data-comparison-expand="readme.txt"]')).getAttribute('aria-expanded'),'true')
    })
    await step('workflow draft and expanded step retained; modal review blocks navigation shortcuts',async()=>{
      await tool('Workflows');await click('[data-testid="workflow-new-open"]');await find('[data-testid="workflow-templates"]')
      await hotkey('l');assert.ok((await driver.getCurrentUrl()).includes('workflows'))
      await click('[data-testid="workflow-template-repack"]');await fill('[data-testid="workflow-plan-name"]','工具中心 · 未保存方案')
      await click('[data-workflow-step="0"]');await tool('ArchiveLibrary');await tool('Workflows')
      assert.equal(await(await find('[data-testid="workflow-plan-name"]')).getAttribute('value'),'工具中心 · 未保存方案')
      assert.equal(await(await find('[data-workflow-step="0"]')).getAttribute('aria-expanded'),'true')
    })
    await step('library adds and indexes a real archive, with search retained across tool categories',async()=>{
      await tool('ArchiveLibrary');await bridge('queueDesktopDialogSelections',[[archive]]);await click('[data-testid="library-add-files"]');await find('[data-testid="library-index-pending"]');await click('[data-testid="library-index-pending"]')
      await wait(async()=>(await(await find('[data-testid="archive-library"]')).getText()).includes('本次处理'),'library index complete')
      await click('[data-testid="library-search-tab"]');await fill('[data-testid="library-query"]','readme');await find('[data-entry-path="readme.txt"]');await tool('SourceComparison');await tool('ArchiveLibrary')
      assert.equal(await(await find('[data-testid="library-query"]')).getAttribute('value'),'readme');await find('[data-entry-path="readme.txt"]')
    })
    await step('checksum native result and expanded details retained while changing tool category',async()=>{
      await tool('FileIntegrity');await bridge('queueDesktopDialogSelections',[[file]]);await click('[data-testid="checksum-calculate-choose"]');await click('[data-testid="checksum-calculate-start"]');await find('[data-testid="checksum-calculation-entry"][data-status="success"]');await click('[data-testid="checksum-calculation-toggle"]')
      const digest=createHash('sha256').update(payload).digest('hex');assert.ok((await(await find('[data-testid="checksum-calculation-entry"]')).getText()).toLowerCase().includes(digest))
      await tool('Workflows');await tool('FileIntegrity');await find('[data-testid="checksum-calculation-details"]');assert.ok((await(await find('[data-testid="checksum-calculation-entry"]')).getText()).toLowerCase().includes(digest))
    })
    await step('Explorer checksum handoff opens the correct tool inside the center',async()=>{
      await click('[data-testid="nav-Decompress"]');forwardContextAction('--checksum-create',[file]);await find('[data-testid="tools-center"]');await wait(async()=>await(await find('[data-testid="tool-FileIntegrity"]')).getAttribute('aria-current')==='page','Explorer tool');await find('[data-testid="checksum-source-summary"]')
    })
  }
  for(const [theme,width,height]of [['glacier',1100,650],['obsidian',760,520]]) {
    if(process.argv.includes('--narrow-only') && width!==760)continue
    await click('[data-testid="nav-Settings"]');await click(`[data-theme="${theme}"]`);await resize(width,height)
    // WebView2 element operations can restore the driver's old host bounds.
    // Set and verify the actual native viewport after those operations too.
    for(const name of ['SourceComparison','Workflows','ArchiveLibrary','FileIntegrity']){await tool(name);await resize(width,height);await shot(`${name}-${theme}-${width}x${height}`,width,height)}
  }
  report.passed=true;report.completedAt=new Date().toISOString();save();console.log(JSON.stringify({checks:report.checks.length,layouts:report.layouts.length,candidate:sha}))
}

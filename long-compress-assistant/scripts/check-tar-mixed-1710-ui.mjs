import { chromium } from '@playwright/test'
import assert from 'node:assert/strict'
import { readFileSync,writeFileSync } from 'node:fs'
const base='test-results/tar-mixed-1710'
const native=JSON.parse(readFileSync(`${base}/native.json`,'utf8'))
// Explicit native-value IPC replay with real Vue/CSS; no desktop input.
const browser=await chromium.launch({headless:true}),errors=[],layouts=[]
try{
 const page=await browser.newPage({viewport:{width:1100,height:700}});page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(native=>{
  window.__mixedNative=native;window.__mixedHistory=[]
  window.__TAURI_IPC__=m=>{
   let value
   if(m.cmd==='load_app_settings')value='{}'
   if(m.cmd==='take_archive_opening_reminder')value=false
   if(m.cmd==='take_pending_context_actions')value=[]
   if(m.cmd==='check_app_update')value={shouldUpdate:false}
   if(m.cmd==='list_task_history_page')value={records:window.__mixedHistory,nextCursor:null}
   if(m.cmd==='diagnose_archive')value=native.diagnosis
   if(m.cmd==='scan_lzop_recovery')value=native.report
   if(m.cmd==='recover_lzop_members'){
    if(m.revision!==native.report.revision||m.selected.length!==native.report.entries.length)throw Error('different source snapshot or selection')
    value=native.result
   }
   if(m.cmd==='save_task_history')window.__mixedHistory.push(m.record)
   window[`_${m.callback}`]?.(value)
  }
 },native)
 await page.goto('http://127.0.0.1:1440/#/integrity',{waitUntil:'domcontentloaded',timeout:60000})
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');const app=useAppStore();app.theme='obsidian';app.accentColor='#d9b76f';app.openArchiveForDiagnosis({path:window.__mixedNative.source,taskId:'mixed-original-failure'})})
 await page.getByTestId('open-lzop-recovery').click();const dialog=page.getByRole('dialog');await page.getByTestId('lzop-recovery-members').waitFor()
 assert.match(await page.getByTestId('lzop-recovery-count').innerText(),/6 个完整文件.*2 个目录/)
 assert.equal(await dialog.locator('[data-recovery-member]').count(),8)
 assert.equal(await dialog.locator('.stop-detail').getAttribute('open'),null)
 const child=native.report.entries.find(e=>e.name.endsWith('/readme.txt')&&!e.name.startsWith('other/'))
 const row=dialog.locator(`[data-recovery-member="${child.id}"]`)
 assert.equal(await row.locator('.member-copy strong').innerText(),'readme.txt');assert.equal(await row.locator('.member-copy').getAttribute('title'),child.name)
 const shot=async name=>{await page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))));await page.screenshot({path:`${base}/${name}.png`})}
 for(const[width,height]of[[1100,700],[760,520]]){
  await page.setViewportSize({width,height});await page.getByTestId('lzop-recovery-confirm').scrollIntoViewIfNeeded()
  const layout=await dialog.evaluate(el=>{const b=el.querySelector('[data-testid="lzop-recovery-confirm"]').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:el.scrollWidth-el.clientWidth,buttonTop:b.top,buttonBottom:b.bottom}})
  assert.ok(layout.overflow<=1&&layout.buttonTop>=0&&layout.buttonBottom<=height,JSON.stringify(layout));layouts.push(layout);await shot(`mixed-selection-${width}x${height}`)
 }
 const filter=dialog.getByRole('textbox',{name:'筛选可取回文件'});await filter.fill('-notes.txt');assert.equal(await dialog.locator('[data-recovery-member]').count(),1);await filter.fill('')
 const directory=native.report.entries.find(e=>e.isDirectory&&!e.name.endsWith('/empty'));const input=dialog.locator(`[data-recovery-member="${directory.id}"] input`)
 await input.uncheck();assert.equal(await row.locator('input').isChecked(),false);await input.check();assert.equal(await row.locator('input').isChecked(),true)
 await page.getByTestId('lzop-recovery-confirm').click();await page.getByTestId('lzop-recovery-result').waitFor();assert.match(await page.getByTestId('lzop-recovery-result').innerText(),/已部分取回 6 个文件、2 个目录/)
 await page.waitForFunction(()=>window.__mixedHistory.length===1);const history=await page.evaluate(()=>window.__mixedHistory[0]);assert.equal(history.executionConfig.kind,'archive-recovery');assert.equal(history.retryOfTaskId,'mixed-original-failure')
 await shot('mixed-result-760x520');assert.deepEqual(errors,[])
 writeFileSync(`${base}/ui.json`,JSON.stringify({passed:true,fixtureNativeIPC:true,nativeReportsReplayed:true,checks:['mixed names, filtering, directory selection, original result and partial history','wide/narrow overflow and footer visibility'],layouts,errors},null,2))
 console.log('2 background UI groups passed; no desktop input.')
}finally{await browser.close()}

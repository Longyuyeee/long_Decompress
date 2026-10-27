import {chromium} from '@playwright/test'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
const base='test-results/tar-time-1711',native=JSON.parse(readFileSync(`${base}/native.json`,'utf8')).find(r=>r.case==='pax')
// Headless real Vue/CSS with replayed native values; no desktop or real IPC.
const browser=await chromium.launch({headless:true}),errors=[],layouts=[]
try{
 const page=await browser.newPage({viewport:{width:1100,height:700}});page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(native=>{window.__timeNative=native;window.__timeHistory=[];window.__TAURI_IPC__=m=>{
  let value
  if(m.cmd==='load_app_settings')value='{}'
  if(m.cmd==='take_archive_opening_reminder')value=false
  if(m.cmd==='take_pending_context_actions')value=[]
  if(m.cmd==='check_app_update')value={shouldUpdate:false}
  if(m.cmd==='list_task_history_page')value={records:window.__timeHistory,nextCursor:null}
  if(m.cmd==='recover_lzop_members'){if(m.revision!==native.report.revision)throw Error('wrong revision');value=native.result}
  if(m.cmd==='save_task_history')window.__timeHistory.push(m.record)
  window[`_${m.callback}`]?.(value)
 }},native)
 await page.goto('http://127.0.0.1:1441/#/integrity',{waitUntil:'domcontentloaded',timeout:60000})
 await page.evaluate(async()=>{
  const{useAppStore}=await import('/src/stores/app.ts');const store=useAppStore();store.theme='obsidian';store.accentColor='#d9b76f'
  const url=performance.getEntriesByType('resource').find(r=>/\/vue\.js\?/.test(r.name)).name
  const{createApp}=await import(url);const{default:Dialog}=await import('/src/components/archive/LzopRecoveryDialog.vue')
  const host=document.createElement('div');document.body.append(host);const parent=document.querySelector('#app').__vue_app__
  const child=createApp(Dialog,{path:window.__timeNative.source,initial:window.__timeNative.report,originTaskId:'time-original-failure',onClose:()=>child.unmount()});child.use(parent.config.globalProperties.$pinia);child.mount(host)
 })
 await page.getByTestId('lzop-recovery-confirm').click();await page.getByTestId('lzop-recovery-result').waitFor()
 const details=page.getByTestId('recovery-time-details');assert.equal(await details.getAttribute('open'),null)
 await details.locator('summary').click();assert.match(await page.getByTestId('recovery-time-summary').innerText(),/5 个文件已保留，2 个未完整保留/)
 assert.match(await details.innerText(),/1969-12-31 23:59:58\.8765433 UTC/);assert.match(await details.innerText(),/早于 Windows 支持范围/)
 const dialog=page.getByRole('dialog'),shot=async name=>{await page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))));await page.screenshot({path:`${base}/${name}.png`})}
 for(const[width,height]of[[1100,700],[760,520]]){
  await page.setViewportSize({width,height});await dialog.getByRole('button',{name:'关闭',exact:true}).scrollIntoViewIfNeeded()
  const layout=await dialog.evaluate(el=>{const box=el.querySelector('.app-dialog'),b=box.querySelector(':scope >div:last-child button').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:box.scrollWidth-box.clientWidth,buttonBottom:b.bottom}})
  assert.ok(layout.overflow<=1&&layout.buttonBottom<=height);layouts.push(layout);await shot(`time-result-${width}x${height}`)
 }
 const warning=details.getByText(/早于 Windows 支持范围/);await warning.scrollIntoViewIfNeeded();const warningBox=await warning.boundingBox();assert.ok(warningBox.y>=0&&warningBox.y+warningBox.height<=520);await shot('time-warning-760x520')
 await dialog.getByRole('button',{name:'关闭',exact:true}).click();await page.waitForFunction(()=>window.__timeHistory.length===1)
 const record=await page.evaluate(()=>window.__timeHistory[0]);assert.deepEqual(record.executionConfig.timePreservation,native.result.timePreservation);assert.equal(record.status,'completed')
 await page.evaluate(()=>{location.hash='#/history'});await page.getByTestId('history-record-row').click();await page.getByTestId('history-recovery-time').waitFor();assert.match(await page.getByTestId('history-recovery-time').innerText(),/5 个文件已保留，2 个未完整保留/);await page.getByTestId('history-recovery-time').scrollIntoViewIfNeeded();await shot('time-history-760x520')
 assert.deepEqual(errors,[]);writeFileSync(`${base}/ui.json`,JSON.stringify({passed:true,headless:true,nativeValueReplay:true,realDesktopIPC:false,checks:['folded time details, exact negative fraction, range/precision warnings separate from content success','wide/narrow result layout and visible footer','persisted time counts in original history details'],layouts,errors},null,2));console.log('3 background UI groups passed; no desktop input.')
}finally{await browser.close()}

import {chromium} from '@playwright/test'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
const base='test-results/tar-directory-time-1712',native=JSON.parse(readFileSync(`${base}/native.json`,'utf8')).filter(r=>r.case==='pax')
// Native-value replay in real Vue/CSS. No desktop input or real Tauri IPC.
const browser=await chromium.launch({headless:true}),errors=[],layouts=[]
try{
 const page=await browser.newPage({viewport:{width:1100,height:700}});page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(native=>{window.__directoryNative=native;window.__directoryHistory=[];window.__TAURI_IPC__=m=>{
  let value
  if(m.cmd==='load_app_settings')value='{}'
  if(m.cmd==='take_archive_opening_reminder')value=false
  if(m.cmd==='take_pending_context_actions')value=[]
  if(m.cmd==='check_app_update')value={shouldUpdate:false}
  if(m.cmd==='list_task_history_page')value={records:window.__directoryHistory,nextCursor:null}
  if(m.cmd==='recover_lzop_members'){const record=native.find(r=>r.mode===window.__directoryMode);if(m.revision!==record.report.revision||JSON.stringify([...m.selected].sort())!==JSON.stringify(record.result.entries.map(e=>e.id).sort()))throw Error('different source or selection');value=record.result}
  if(m.cmd==='save_task_history')window.__directoryHistory.push(m.record)
  window[`_${m.callback}`]?.(value)
 }},native)
 await page.goto('http://127.0.0.1:1442/#/integrity',{waitUntil:'domcontentloaded',timeout:60000})
 const shot=async name=>{await page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))));await page.screenshot({path:`${base}/${name}.png`})}
 for(const mode of ['full','directory-only']){
  await page.evaluate(async mode=>{
   window.__directoryMode=mode;const native=window.__directoryNative.find(r=>r.mode===mode)
   const{useAppStore}=await import('/src/stores/app.ts');const store=useAppStore();store.theme='obsidian';store.accentColor='#d9b76f'
   const url=performance.getEntriesByType('resource').find(r=>/\/vue\.js\?/.test(r.name)).name
   const{createApp}=await import(url);const{default:Dialog}=await import('/src/components/archive/LzopRecoveryDialog.vue')
   const host=document.createElement('div');document.body.append(host);const parent=document.querySelector('#app').__vue_app__
   const child=createApp(Dialog,{path:native.source,initial:native.report,originTaskId:'directory-original-failure',onClose:()=>{child.unmount();host.remove()}});child.use(parent.config.globalProperties.$pinia);child.use(parent.config.globalProperties.$router);child.mount(host)
  },mode)
  const dialog=page.getByRole('dialog')
  if(mode==='directory-only'){
   await dialog.getByRole('button',{name:'取消所列',exact:true}).click();await dialog.locator('[data-recovery-member="1"] input').check()
   for(const id of[2,4,7])await dialog.locator(`[data-recovery-member="${id}"] input`).uncheck()
  }
  await page.getByTestId('lzop-recovery-confirm').click();await page.getByTestId('lzop-recovery-result').waitFor()
  const details=page.getByTestId('recovery-time-details');assert.equal(await details.getAttribute('open'),null);await details.locator('summary').click()
  const summary=await page.getByTestId('recovery-time-summary').innerText();assert.match(summary,mode==='full'?/3 个文件已保留；4 个目录已保留，2 个未完整保留/:/原修改时间：1 个目录已保留。/)
  for(const[width,height]of(mode==='full'?[[1100,700],[760,520]]:[[760,520]])){
   await page.setViewportSize({width,height});await dialog.getByRole('button',{name:'关闭',exact:true}).scrollIntoViewIfNeeded();await page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))));const layout=await dialog.evaluate(el=>{const box=el.querySelector('.app-dialog'),b=box.querySelector(':scope >div:last-child button').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:box.scrollWidth-box.clientWidth,footerBottom:b.bottom}});assert.ok(layout.overflow<=1&&layout.footerBottom<=height,JSON.stringify({mode,...layout}));layouts.push({mode,...layout});await shot(`directory-${mode}-${width}x${height}`)
  }
  if(mode==='full'){const warning=details.getByText(/目录原修改时间早于 Windows 支持范围/);await warning.scrollIntoViewIfNeeded();const b=await warning.boundingBox();assert.ok(b.y>=0&&b.y+b.height<=520);await shot('directory-warning-760x520')}
  await dialog.getByRole('button',{name:'关闭',exact:true}).click()
 }
 await page.waitForFunction(()=>window.__directoryHistory.length===2)
 const histories=await page.evaluate(()=>window.__directoryHistory);assert.equal(histories[1].status,'completed');assert.equal(histories[1].executionConfig.timePreservation.preservedDirectories,1)
 await page.evaluate(()=>{location.hash='#/history'});await page.getByTestId('history-record-row').first().click();const time=page.getByTestId('history-recovery-time');await time.waitFor();assert.match(await time.innerText(),/原修改时间：1 个目录已保留。/);await time.scrollIntoViewIfNeeded();await shot('directory-history-760x520')
 assert.deepEqual(errors,[]);writeFileSync(`${base}/ui.json`,JSON.stringify({passed:true,nativeValueReplay:true,realDesktopIPC:false,checks:['full and directory-only results keep details folded, show separate file/directory counts and save history','wide/narrow layout, scrollable warnings and visible footer','original history details include directory-only times'],layouts,errors},null,2));console.log('3 headless UI groups passed; no desktop input.')
}finally{await browser.close()}

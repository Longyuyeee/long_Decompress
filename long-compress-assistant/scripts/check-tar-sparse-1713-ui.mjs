import {chromium} from '@playwright/test'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
const base='test-results/tar-sparse-1713',native=JSON.parse(readFileSync(`${base}/native.json`,'utf8')).filter(r=>['tail','selected'].includes(r.mode))
const producer=JSON.parse(readFileSync('tests/fixtures/archives/tar-sparse-1713/manifest.json','utf8'))
// Headless real Vue/CSS with native report/result replay, not real desktop IPC.
const browser=await chromium.launch({headless:true}),errors=[],layouts=[]
try{
 const page=await browser.newPage({viewport:{width:1100,height:700}});page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(native=>{window.__sparseNative=native;window.__sparseHistory=[];window.__TAURI_IPC__=m=>{
  let value
  if(m.cmd==='load_app_settings')value='{}'
  if(m.cmd==='take_archive_opening_reminder')value=false
  if(m.cmd==='take_pending_context_actions')value=[]
  if(m.cmd==='check_app_update')value={shouldUpdate:false}
  if(m.cmd==='list_task_history_page')value={records:window.__sparseHistory,nextCursor:null}
  if(m.cmd==='recover_lzop_members'){const r=native.find(r=>r.mode===window.__sparseMode);if(m.revision!==r.report.revision||JSON.stringify([...m.selected].sort())!==JSON.stringify(r.result.entries.map(e=>e.id).sort()))throw Error('different source snapshot or selection');value=r.result}
  if(m.cmd==='save_task_history')window.__sparseHistory.push(m.record)
  window[`_${m.callback}`]?.(value)
 }},native)
 await page.goto('http://127.0.0.1:1443/#/integrity',{waitUntil:'domcontentloaded',timeout:60000})
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))))
 const shot=async name=>{await settle();await page.screenshot({path:`${base}/${name}.png`})}
 for(const mode of ['tail','selected']){
  await page.evaluate(async mode=>{
   window.__sparseMode=mode;const r=window.__sparseNative.find(r=>r.mode===mode);const{useAppStore}=await import('/src/stores/app.ts');const store=useAppStore();store.theme='obsidian';store.accentColor='#d9b76f'
   const url=performance.getEntriesByType('resource').find(r=>/\/vue\.js\?/.test(r.name)).name;const{createApp}=await import(url);const{default:Dialog}=await import('/src/components/archive/LzopRecoveryDialog.vue');const host=document.createElement('div');document.body.append(host);const parent=document.querySelector('#app').__vue_app__
   const child=createApp(Dialog,{path:r.source,initial:r.report,originTaskId:'sparse-original-failure',onClose:()=>{child.unmount();host.remove()}});child.use(parent.config.globalProperties.$pinia);child.use(parent.config.globalProperties.$router);child.mount(host)
  },mode)
  const dialog=page.getByRole('dialog');await page.getByTestId('lzop-recovery-members').waitFor()
  if(mode==='tail'){
   const filter=dialog.getByRole('textbox',{name:'筛选可取回文件'});await filter.fill('全零');assert.match(await dialog.locator('[data-recovery-member="5"]').innerText(),/包内数据 0 B/);await page.setViewportSize({width:760,height:520});await dialog.getByRole('button',{name:/取回所选/}).scrollIntoViewIfNeeded();await shot('sparse-selection-760x520');await filter.fill('')
  }else{await dialog.getByRole('button',{name:'取消所列',exact:true}).click();for(const id of[2,4,6])await dialog.locator(`[data-recovery-member="${id}"] input`).check()}
  await page.getByTestId('lzop-recovery-confirm').click();await page.getByTestId('lzop-recovery-result').waitFor();const details=page.getByTestId('recovery-time-details');assert.equal(await details.getAttribute('open'),null);await details.locator('summary').click();assert.match(await details.innerText(),/稀疏文件 · 完整大小/)
  for(const[width,height]of(mode==='tail'?[[1100,700],[760,520]]:[[760,520]])){
   await page.setViewportSize({width,height});await dialog.getByRole('button',{name:'关闭',exact:true}).scrollIntoViewIfNeeded();await settle();const layout=await dialog.evaluate(el=>{const box=el.querySelector('.app-dialog'),b=box.querySelector(':scope >div:last-child button').getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:box.scrollWidth-box.clientWidth,footerBottom:b.bottom}});assert.ok(layout.overflow<=1&&layout.footerBottom<=height,JSON.stringify(layout));layouts.push({mode,...layout});await shot(`sparse-${mode}-${width}x${height}`)
  }
  if(mode==='tail'){const large=details.getByText(/稀疏文件 · 完整大小 67,108,864 B/);await large.scrollIntoViewIfNeeded();const stored=producer.records.find(r=>r.name==='大空洞.bin').storedBytes;assert.ok((await large.innerText()).includes(`包内数据 ${stored.toLocaleString('en-US')} B`));await shot('sparse-size-details-760x520')}
  await dialog.getByRole('button',{name:'关闭',exact:true}).click()
 }
 await page.waitForFunction(()=>window.__sparseHistory.length===2);const histories=await page.evaluate(()=>window.__sparseHistory);assert.equal(histories[1].processedBytes,75497472);assert.equal(histories[1].executionConfig.recoveredFiles,2);assert.equal(histories[1].executionConfig.recoveredDirectories,1)
 await page.evaluate(()=>{location.hash='#/history'});await page.getByTestId('history-record-row').first().click();await page.getByTestId('history-recovery-details').scrollIntoViewIfNeeded();assert.match(await page.getByTestId('history-recovery-details').innerText(),/2 个完整文件、1 个目录/);await shot('sparse-history-760x520')
 assert.deepEqual(errors,[]);writeFileSync(`${base}/ui.json`,JSON.stringify({passed:true,nativeValueReplay:true,realDesktopIPC:false,checks:['zero stored bytes, filtering and sparse metadata in original list','folded result, logical/stored sizes and wide/narrow layout','selected sparse files plus folder, logical byte history and original partial tag'],layouts,errors},null,2));console.log('3 background UI groups passed; no desktop input.')
}finally{await browser.close()}

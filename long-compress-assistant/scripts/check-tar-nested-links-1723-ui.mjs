import {chromium} from '@playwright/test'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
const base='test-results/tar-nested-links-1723',native=JSON.parse(readFileSync(`${base}/native.json`,'utf8')),cases=native.filter(c=>c.outer),first=cases[0],locked=cases.find(c=>c.encrypted)
const browser=await chromium.launch({headless:true}),errors=[],checks=[]
try{
 const page=await browser.newPage({viewport:{width:1200,height:760}});page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(cases=>{window.__nestedCases=cases;window.__nestedCalls=[];window.__holdNested=false;window.__failedOnce=false;window.__TAURI_IPC__=m=>{let value
  if(m.cmd==='load_app_settings')value='{}'
  if(m.cmd==='take_archive_opening_reminder')value=false
  if(m.cmd==='take_pending_context_actions')value=[]
  if(m.cmd==='check_app_update')value={shouldUpdate:false}
  if(m.cmd==='list_task_history_page')value={records:[],nextCursor:null}
  if(m.cmd==='file_manager_locations')value=[{name:'资料',path:'C:/Samples',kind:'home'},{name:'结果',path:'D:/Output',kind:'drive'}]
  if(m.cmd==='list_files')value=[{path:m.path+'/结果文件夹',name:'结果文件夹',is_dir:true,size:0}]
  if(m.cmd==='get_archive_engine_capabilities')value={available:true,fullEngine:true,formats:[],browseExtensions:['lzo','tar','zip','7z'],nestedExtensions:['zip','7z'],boundedPreviewFormats:['ZIP','7Z','TAR'],boundedTextPreviewFormats:['ZIP','7Z','TAR'],imagePreviewExtensions:['png'],textPreviewExtensions:['txt'],message:'ready'}
  if(m.cmd==='browse_archive'){
   const child=cases.find(c=>c.receipt.cachePath===m.filePath)
   if(child){if(child.encrypted&&m.password!=='Nested1723!'){window[`_${m.error}`]?.('ARCHIVE_BROWSE_PASSWORD|这一层需要单独的密码');return}value={...child.child,revision:'child-revision',sourcePath:m.filePath}}
   else value={...cases[0].outer,revision:cases[0].revision,sourcePath:cases[0].source}
  }
  if(m.cmd==='materialize_nested_archive'){
   const c=cases.find(c=>c.alias===m.entryPath);if(!c||m.sourceRevision!==cases[0].revision||!m.requestId||m.targetDepth!==2)throw Error('Unbound nested request')
   window.__nestedCalls.push(m);if(window.__holdNested){window.__held={m,c};return}value=c.receipt
  }
  if(m.cmd==='preview_archive_text'){const c=cases.find(c=>c.receipt.cachePath===m.filePath);value=c?.text}
  window[`_${m.callback}`]?.(value)
 }},cases)
 await page.goto('http://127.0.0.1:1453/#/browser',{waitUntil:'domcontentloaded',timeout:60000})
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');const store=useAppStore();store.theme='obsidian';store.accentColor='#d9b76f';store.openArchiveInBrowser(window.__nestedCases[0].source)})
 const manager=page.getByTestId('dual-pane-file-manager'),left=manager.locator('[data-file-pane="left"]'),right=manager.locator('[data-file-pane="right"]');await left.locator('[data-entry-path="links/"]').waitFor()
 const peerState=()=>right.evaluate(el=>({path:el.querySelector('input')?.value,entries:[...el.querySelectorAll('.file-row')].map(row=>({path:row.dataset.path,selected:row.classList.contains('selected')}))}));const peer=await peerState()
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))))
 const layout=async name=>{await settle();const value=await manager.evaluate(el=>{const panes=[...el.querySelectorAll('.pane-grid > .file-pane')].map(e=>{const r=e.getBoundingClientRect();return{width:r.width,overflow:e.scrollWidth-e.clientWidth}});return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes}});assert.equal(value.panes.length,2);assert.ok(value.overflow<=1&&value.panes.every(p=>p.overflow<=1)&&Math.abs(value.panes[0].width-value.panes[1].width)<=1,JSON.stringify(value));assert.deepEqual(await peerState(),peer);assert.equal(await left.getByTestId('file-pane-navigation-left').count(),1);const crumb=await left.locator('.path-breadcrumbs').evaluate(el=>{const r=el.getBoundingClientRect(),c=el.querySelector('.current').getBoundingClientRect();return{left:c.left-r.left,right:c.right-r.right}});assert.ok(crumb.left>=-1&&crumb.right<=1,JSON.stringify(crumb));checks.push({name,...value});await page.screenshot({path:`${base}/${name}.png`,animations:'disabled'})}
 await left.locator('[data-entry-path="links/"]').dblclick();const alias=left.locator('[data-entry-path="links/copy.zip"]');await alias.click();await layout('outer-alias-1200x760')
 await alias.dblclick();await left.locator('[data-entry-path="readme.txt"]').waitFor();await layout('nested-1200x760');await page.setViewportSize({width:900,height:600});await layout('nested-900x600')
 await left.locator('[data-entry-path="readme.txt"]').dblclick();await page.getByTestId('archive-text-preview').waitFor();assert.match(await page.getByTestId('archive-text-preview').innerText(),/保持原双栏浏览/);await page.getByRole('button',{name:'关闭预览',exact:true}).click()
 await left.getByRole('button',{name:'上一级',exact:true}).click();await alias.waitFor();assert.ok((await alias.getAttribute('class')).includes('selected'));await layout('return-900x600')
 await page.evaluate(()=>window.__holdNested=true);await alias.dblclick();await left.getByTestId('archive-nested-cancel').waitFor();await layout('opening-900x600');await left.getByTestId('archive-nested-cancel').click();await page.evaluate(()=>{const{m,c}=window.__held;window[`_${m.callback}`]?.(c.receipt);window.__holdNested=false});await alias.waitFor();assert.equal(await left.locator('[data-entry-path="readme.txt"]').count(),0)
 await left.locator('[data-entry-path="links/加密副本.7z"]').dblclick();await page.getByTestId('archive-nested-error').waitFor();assert.equal(await left.locator('input[type="password"]').inputValue(),'');await left.getByTestId('archive-pane-tools').click();await left.locator('input[type="password"]').fill('Nested1723!');await left.getByTestId('archive-nested-retry').click();await left.locator('[data-entry-path="secret.txt"]').waitFor();if(await left.getByTestId('archive-pane-tools').getAttribute('aria-expanded')==='true')await left.getByTestId('archive-pane-tools').click();await layout('encrypted-inner-900x600')
 assert.deepEqual(errors,[]);writeFileSync(`${base}/ui.json`,JSON.stringify({passed:true,nativeValueReplay:true,realDesktopIPC:false,checks,requests:await page.evaluate(()=>window.__nestedCalls),errors},null,2));console.log('4 background UI groups passed: equal shared panes, nested/return and text preview, cancellation, separate inner password.')
}finally{await browser.close()}

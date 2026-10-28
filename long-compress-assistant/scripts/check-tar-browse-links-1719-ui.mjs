import {chromium} from '@playwright/test'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
const base='test-results/tar-browse-links-1719',native=JSON.parse(readFileSync(`${base}/native.json`,'utf8'))[0]
const browser=await chromium.launch({headless:true}),errors=[],checks=[]
try{
 const page=await browser.newPage({viewport:{width:1200,height:760}});page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(native=>{window.__linkNative=native;window.__previewCalls=[];window.__TAURI_IPC__=m=>{
  let value
  if(m.cmd==='load_app_settings')value='{}'
  if(m.cmd==='take_archive_opening_reminder')value=false
  if(m.cmd==='take_pending_context_actions')value=[]
  if(m.cmd==='check_app_update')value={shouldUpdate:false}
  if(m.cmd==='list_task_history_page')value={records:[],nextCursor:null}
  if(m.cmd==='file_manager_locations')value=[{name:'资料',path:'C:/Samples',kind:'home'},{name:'目标目录',path:'D:/Output',kind:'drive'}]
  if(m.cmd==='list_files')value=[{path:m.path+'/结果文件夹',name:'结果文件夹',is_dir:true,size:0}]
  if(m.cmd==='get_archive_engine_capabilities')value={available:true,fullEngine:true,formats:[],browseExtensions:['tar','lzo','zip'],nestedExtensions:['zip'],boundedPreviewFormats:['TAR'],boundedTextPreviewFormats:['TAR'],imagePreviewExtensions:['png'],textPreviewExtensions:['txt'],message:'ready'}
  if(m.cmd==='browse_archive')value={...native.report,revision:native.revision,sourcePath:native.source}
  if(m.cmd==='preview_archive_text'){if(m.sourceRevision!==native.revision)throw Error('different source revision');value=native.texts.find(e=>e.entryPath===m.entryPath);window.__previewCalls.push({kind:'text',path:m.entryPath})}
  if(m.cmd==='preview_archive_image'){if(m.sourceRevision!==native.revision)throw Error('different source revision');if(m.entryPath!==native.image.entryPath)throw Error('different alias');value=native.image;window.__previewCalls.push({kind:'image',path:m.entryPath})}
  window[`_${m.callback}`]?.(value)
 }},native)
 await page.goto('http://127.0.0.1:1449/#/browser',{waitUntil:'domcontentloaded',timeout:60000})
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');const store=useAppStore();store.theme='obsidian';store.accentColor='#d9b76f';store.openArchiveInBrowser(window.__linkNative.source)})
 const manager=page.getByTestId('dual-pane-file-manager');await manager.waitFor();const row=manager.locator('[data-entry-path="阅读副本.txt"]');await row.waitFor()
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))))
 const shot=async name=>{await settle();await page.screenshot({path:`${base}/${name}.png`})}
 const layout=async name=>{await settle();const value=await manager.evaluate(el=>{const panes=[...el.querySelectorAll('.pane-grid > .file-pane')].map(e=>{const r=e.getBoundingClientRect();return{width:r.width,overflow:e.scrollWidth-e.clientWidth}});return{width:innerWidth,height:innerHeight,overflow:el.scrollWidth-el.clientWidth,panes}});assert.ok(value.overflow<=1,JSON.stringify(value));assert.equal(value.panes.length,2);assert.ok(Math.abs(value.panes[0].width-value.panes[1].width)<=1,JSON.stringify(value));assert.ok(value.panes.every(p=>p.overflow<=1),JSON.stringify(value));checks.push({name,...value});await shot(name)}
 if(process.argv.includes('--details-only')){
  await page.setViewportSize({width:900,height:600});await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');useAppStore().openArchiveInBrowser(window.__linkNative.source,{focusEntryPath:window.__linkNative.texts[2].entryPath})});const row=manager.locator(`[data-entry-path="${native.texts[2].entryPath}"]`);await row.waitFor();await row.click({button:'right'});await page.getByTestId('archive-context-details').click();const dd=page.locator('.archive-link-source dd');await dd.waitFor();const value=await dd.evaluate(el=>({overflow:el.scrollWidth-el.clientWidth,whiteSpace:getComputedStyle(el).whiteSpace,lines:el.getBoundingClientRect().height/parseFloat(getComputedStyle(el).lineHeight)}));assert.ok(value.overflow<=1&&value.whiteSpace==='normal'&&value.lines>1,JSON.stringify(value));await shot('source-details-900x600');writeFileSync(`${base}/ui-details.json`,JSON.stringify({passed:true,nativeValueReplay:true,realDesktopIPC:false,...value},null,2));console.log('Focused long-source details layout passed.')
 }else{
 assert.match(await row.innerText(),/TXT · 链接内容/);assert.match(await row.locator('.file-label').getAttribute('title'),/内容来源：docs\/readme.txt/)
 await layout('links-1200x760');await page.setViewportSize({width:900,height:600});await layout('links-900x600')
 await page.getByTestId('archive-pane-tools').click();const search=page.getByRole('textbox',{name:'搜索包内文件'});await search.fill('阅读副本');assert.equal(await manager.locator('[data-file-pane="left"] .file-row').count(),1);await search.fill('');await page.getByTestId('archive-pane-tools').click()
 await row.dblclick();await page.getByTestId('archive-text-preview').waitFor();assert.match(await page.locator('.preview-meta').innerText(),/仅显示前 1 MiB/);await shot('text-prefix-900x600');await page.getByRole('button',{name:'关闭预览',exact:true}).click()
 await row.click({button:'right'});const menu=page.getByTestId('archive-context-menu');await menu.waitFor();assert.equal(await page.getByTestId('archive-context-default-open').isDisabled(),true);assert.equal(await page.getByTestId('archive-context-extract-choose').isDisabled(),true);await shot('readonly-menu-900x600');await page.keyboard.press('Escape')
 await manager.locator('[data-entry-path="gallery/"]').dblclick();await manager.locator('[data-entry-path="gallery/封面副本.png"]').dblclick();await page.getByTestId('archive-image-preview').waitFor();assert.match(await page.locator('.preview-meta').innerText(),/32 × 24/);await shot('image-900x600');await page.getByRole('button',{name:'关闭预览',exact:true}).click()
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');useAppStore().openArchiveInBrowser(window.__linkNative.source,{focusEntryPath:window.__linkNative.texts[2].entryPath})});const long=manager.locator(`[data-entry-path="${native.texts[2].entryPath}"]`);await long.waitFor();await long.click({button:'right'});await page.getByTestId('archive-context-details').click();assert.match(await page.getByTestId('archive-entry-details').innerText(),/内容来源/);await shot('source-details-900x600');await page.getByRole('button',{name:'关闭条目详情',exact:true}).click()
 assert.equal(await manager.locator('.pane-grid > .file-pane').count(),2);assert.deepEqual(errors,[]);writeFileSync(`${base}/ui.json`,JSON.stringify({passed:true,nativeValueReplay:true,realDesktopIPC:false,checks,previews:await page.evaluate(()=>window.__previewCalls),errors},null,2));console.log('3 background UI groups passed: shared panes, bound previews, compact source and read-only menu.')
 }
}finally{await browser.close()}

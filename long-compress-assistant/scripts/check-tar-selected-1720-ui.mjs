import {chromium} from '@playwright/test'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
const base='test-results/tar-selected-1720',native=JSON.parse(readFileSync('test-results/tar-browse-links-1719/native.json','utf8'))[0],receipt=JSON.parse(readFileSync(`${base}/single.json`,'utf8')),verified=JSON.parse(readFileSync(`${base}/verified.json`,'utf8')).checks.at(-1)
assert.equal(native.source.replaceAll('\\','/').replace('/src-tauri/..',''),receipt.source.replaceAll('\\','/').replace('/src-tauri/..',''))
native.source=receipt.source
const browser=await chromium.launch({headless:true}),errors=[],checks=[]
try{
 const page=await browser.newPage({viewport:{width:1200,height:760}});page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(({native,receipt,verified})=>{window.__linkNative=native;window.__receipt=receipt;window.__previewCalls=[];window.__extractCalls=[];window.__extracted=false;window.__TAURI_IPC__=m=>{
  let value
  if(m.cmd==='load_app_settings')value='{}'
  if(m.cmd==='take_archive_opening_reminder')value=false
  if(m.cmd==='take_pending_context_actions')value=[]
  if(m.cmd==='check_app_update')value={shouldUpdate:false}
  if(m.cmd==='list_task_history_page')value={records:[],nextCursor:null}
  if(m.cmd==='file_manager_locations')value=[{name:'资料',path:'C:/Samples',kind:'home'},{name:'目标目录',path:receipt.outputDirectory,kind:'drive'}]
  if(m.cmd==='list_files')value=window.__extracted&&m.path===receipt.outputDirectory?verified.actualFiles.map(f=>({...f,modified:new Date(f.modified_time*1000).toISOString()})):[]
  if(m.cmd==='extract_file'){if(m.filePath!==receipt.source||m.outputPath!==receipt.outputDirectory||JSON.stringify(m.options.selected_entries)!==JSON.stringify(['阅读副本.txt'])||!m.options.preserve_paths)throw Error('Selection/output differs from real native receipt');window.__extractCalls.push(m);window.__extracted=true;value=receipt.outputDirectory}
  if(m.cmd==='get_archive_engine_capabilities')value={available:true,fullEngine:true,formats:[],browseExtensions:['tar','lzo','zip'],nestedExtensions:['zip'],boundedPreviewFormats:['TAR'],boundedTextPreviewFormats:['TAR'],imagePreviewExtensions:['png'],textPreviewExtensions:['txt'],message:'ready'}
  if(m.cmd==='browse_archive')value={...native.report,revision:native.revision,sourcePath:native.source}
  if(m.cmd==='preview_archive_text'){if(m.sourceRevision!==native.revision)throw Error('different source revision');value=native.texts.find(e=>e.entryPath===m.entryPath);window.__previewCalls.push({kind:'text',path:m.entryPath})}
  if(m.cmd==='preview_archive_image'){if(m.sourceRevision!==native.revision)throw Error('different source revision');if(m.entryPath!==native.image.entryPath)throw Error('different alias');value=native.image;window.__previewCalls.push({kind:'image',path:m.entryPath})}
  window[`_${m.callback}`]?.(value)
 }},{native,receipt,verified})
 await page.goto('http://127.0.0.1:1450/#/browser',{waitUntil:'domcontentloaded',timeout:60000})
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');const store=useAppStore();store.theme='obsidian';store.accentColor='#d9b76f';store.openArchiveInBrowser(window.__linkNative.source)})
 const manager=page.getByTestId('dual-pane-file-manager');await manager.waitFor();const row=manager.locator('[data-entry-path="阅读副本.txt"]');await row.waitFor()
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))))
 const shot=async name=>{await settle();await page.screenshot({path:`${base}/${name}.png`})}
 const layout=async name=>{await settle();const value=await manager.evaluate(el=>{const panes=[...el.querySelectorAll('.pane-grid > .file-pane')].map(e=>{const r=e.getBoundingClientRect();return{width:r.width,overflow:e.scrollWidth-e.clientWidth}});return{width:innerWidth,height:innerHeight,overflow:el.scrollWidth-el.clientWidth,panes}});assert.ok(value.overflow<=1,JSON.stringify(value));assert.equal(value.panes.length,2);assert.ok(Math.abs(value.panes[0].width-value.panes[1].width)<=1,JSON.stringify(value));assert.ok(value.panes.every(p=>p.overflow<=1),JSON.stringify(value));checks.push({name,...value});await shot(name)}

 assert.match(await row.innerText(),/TXT · 链接内容/)
 await layout('selected-1200x760');await page.setViewportSize({width:900,height:600});await layout('selected-900x600')
 await row.click({button:'right'});const menu=page.getByTestId('archive-context-menu');await menu.waitFor();assert.equal(await page.getByTestId('archive-context-default-open').isDisabled(),true);assert.equal(await page.getByTestId('archive-context-extract-choose').isEnabled(),true);assert.equal(await page.getByTestId('archive-context-extract-current').isEnabled(),true);await shot('extract-menu-900x600')
 await page.getByTestId('archive-context-extract-current').click();const published=manager.locator('[data-file-pane="right"] .file-row').filter({hasText:'阅读副本.txt'});await published.waitFor();assert.equal(await manager.locator('[data-file-pane="right"] .file-row').count(),1);await layout('published-other-pane-900x600')
 const history=await page.evaluate(async()=>{const{useTaskStore}=await import('/src/stores/task.ts');const{createTaskHistoryRecord}=await import('/src/types/taskHistory.ts');const task=useTaskStore().tasks.find(t=>t.status==='completed');return createTaskHistoryRecord(task)});assert.deepEqual(history.executionConfig.tarSelection,{paths:['阅读副本.txt'],keepStructure:true,conflictPolicy:'rename'});assert.equal(history.outputPath,receipt.outputDirectory)
 await row.dblclick();await page.getByTestId('archive-text-preview').waitFor();assert.match(await page.locator('.preview-meta').innerText(),/仅显示前 1 MiB/);await shot('retained-preview-900x600');await page.getByRole('button',{name:'关闭预览',exact:true}).click()
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');useAppStore().openArchiveInBrowser(window.__linkNative.source,{focusEntryPath:window.__linkNative.texts[2].entryPath})});const long=manager.locator(`[data-entry-path="${native.texts[2].entryPath}"]`);await long.waitFor();await long.click({button:'right'});await page.getByTestId('archive-context-details').click();const dd=page.locator('.archive-link-source dd');await dd.waitFor();assert.ok(await dd.evaluate(el=>el.scrollWidth-el.clientWidth<=1&&getComputedStyle(el).whiteSpace==='normal'));await shot('source-details-900x600')
 assert.deepEqual(errors,[]);writeFileSync(`${base}/ui.json`,JSON.stringify({passed:true,nativeValueReplay:true,realDesktopIPC:false,checks,extractions:await page.evaluate(()=>window.__extractCalls),history,previews:await page.evaluate(()=>window.__previewCalls),errors},null,2));console.log('3 background UI groups passed: equal shared panes, exact single-alias publication and history scope, retained preview/compact details.')
}finally{await browser.close()}

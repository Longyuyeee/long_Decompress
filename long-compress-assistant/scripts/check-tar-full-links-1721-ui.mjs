import {chromium} from '@playwright/test'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,statSync,statfsSync} from 'node:fs'
const base='test-results/tar-full-links-1721',receipt=JSON.parse(readFileSync(`${base}/native.json`,'utf8'))[2],native=JSON.parse(readFileSync(`${base}/integrity.json`,'utf8'))[2],verified=JSON.parse(readFileSync(`${base}/verified.json`,'utf8')).checks[2]
assert.equal(native.source,receipt.source)
const disk=statfsSync(receipt.outputDirectory),sourceBytes=statSync(receipt.source).size
const browser=await chromium.launch({headless:true}),errors=[],checks=[]
try{
 const page=await browser.newPage({viewport:{width:1200,height:760}});page.setDefaultTimeout(30000);page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(({receipt,native,verified,sourceBytes,diskBytes})=>{window.__fullNative={receipt,native};window.__fullCalls=[];window.__TAURI_IPC__=m=>{
  let value
  if(m.cmd==='load_app_settings')value='{}'
  if(m.cmd==='take_archive_opening_reminder')value=false
  if(m.cmd==='take_pending_context_actions')value=[]
  if(m.cmd==='check_app_update')value={shouldUpdate:false}
  if(m.cmd==='list_task_history_page')value={records:[],nextCursor:null}
  if(m.cmd==='preview_extraction_output')value=receipt.outputDirectory
  if(m.cmd==='get_file_info')value={name:m.path===receipt.source?'gnu.tar.lzo':'结果',path:m.path,is_dir:m.path!==receipt.source,size:m.path===receipt.source?sourceBytes:0}
  if(m.cmd==='preflight_operation_resources')value={operation:'decompression',outputPath:receipt.outputDirectory,probePath:receipt.outputDirectory,mountPoint:null,fileSystem:null,location:'local',medium:'unknown',totalBytes:diskBytes.total,availableBytes:diskBytes.available,estimatedOutputBytes:receipt.publishedBytes,requiredBytes:receipt.publishedBytes,reserveBytes:67108864,estimateSource:'provided_estimate',estimateReliable:true,status:'ready',canStart:true,summary:'可开始',warnings:[]}
  if(m.cmd==='extract_file'){if(m.filePath!==receipt.source||m.outputPath!==receipt.outputDirectory||m.options.selected_entries.length!==0||!m.options.preserve_paths)throw Error('Whole-archive command differs from actual native receipt');window.__fullCalls.push(m);value=receipt.outputDirectory}
  if(m.cmd==='diagnose_archive'){if(m.filePath!==receipt.source)throw Error('different diagnostic source');value=native.diagnosis}
  if(m.cmd==='file_manager_locations')value=[{name:'资料',path:'C:/Samples',kind:'home'},{name:'完整结果',path:receipt.outputDirectory,kind:'drive'}]
  if(m.cmd==='list_files')value=m.path===receipt.outputDirectory?verified.rootEntries.map(e=>({...e,modified:new Date(e.modified*1000).toISOString()})):[]
  if(m.cmd==='get_archive_engine_capabilities')value={available:true,fullEngine:true,formats:[],browseExtensions:['tar','lzo','zip'],nestedExtensions:['zip'],boundedPreviewFormats:['TAR'],boundedTextPreviewFormats:['TAR'],imagePreviewExtensions:['png'],textPreviewExtensions:['txt'],message:'ready'}
  if(m.cmd==='browse_archive')value={...native.browse,sourcePath:receipt.source}
  window[`_${m.callback}`]?.(value)
 }},{receipt,native,verified,sourceBytes,diskBytes:{total:disk.blocks*disk.bsize,available:disk.bavail*disk.bsize}})
 await page.goto('http://127.0.0.1:1451/#/decompress',{waitUntil:'domcontentloaded',timeout:60000})
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');const app=useAppStore();app.theme='obsidian';app.accentColor='#d9b76f';const{useTaskStore}=await import('/src/stores/task.ts');const{receipt}=window.__fullNative;useTaskStore().addTask({id:'full1721-ui',name:'gnu.tar.lzo',type:'decompression',sourceFiles:[receipt.source],outputPath:receipt.outputDirectory,format:'TAR.LZO',configurationMode:'individual',extractToSubfolder:false,recycleSourceAfterExtract:false})})
 const shot=async name=>{await page.screenshot({path:`${base}/${name}.png`,animations:'disabled'})}
 const rootLayout=async(name,selector)=>{const value=await page.locator(selector).evaluate(el=>({width:innerWidth,height:innerHeight,overflow:el.scrollWidth-el.clientWidth,bodyOverflow:document.documentElement.scrollWidth-innerWidth}));assert.ok(value.overflow<=1&&value.bodyOverflow<=1,JSON.stringify(value));checks.push({name,...value});await shot(name)}
 await page.getByTestId('task-row').click();await page.getByTestId('decompression-task-details').waitFor();await rootLayout('whole-task-1200x760','[data-testid="decompression-workspace-shell"]')
 await page.getByTestId('start-decompression-queue').click();await page.getByTestId('completed-extraction-output').waitFor();assert.match(await page.getByTestId('completed-extraction-output').innerText(),/解压结果/)
 const history=await page.evaluate(async()=>{const{useTaskStore}=await import('/src/stores/task.ts');const{createTaskHistoryRecord}=await import('/src/types/taskHistory.ts');return createTaskHistoryRecord(useTaskStore().tasks.find(t=>t.id==='full1721-ui'))});assert.equal(history.status,'completed');assert.equal(history.outputPath,receipt.outputDirectory);assert.equal(history.executionConfig.tarSelection,undefined)
 await page.setViewportSize({width:900,height:600});await rootLayout('whole-complete-900x600','[data-testid="decompression-workspace-shell"]');const taskPanes=await page.getByTestId('decompression-task-details').evaluate(el=>[...el.querySelectorAll('.task-detail-layout > div')].map(e=>({width:e.getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth})));assert.ok(taskPanes.length===2&&taskPanes.every(p=>p.overflow<=1)&&Math.abs(taskPanes[0].width-taskPanes[1].width)<=1,JSON.stringify(taskPanes))
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');const app=useAppStore();app.setSuccess(null);app.openArchiveForDiagnosis({path:window.__fullNative.receipt.source});const{default:router}=await import('/src/router/index.ts');await router.push('/tools/integrity')})
 const report=page.getByTestId('diagnostic-report');await report.waitFor();assert.match(await report.innerText(),/完整性正常/);assert.match(await report.innerText(),/9 \/ 2/);assert.equal(await page.getByTestId('open-lzop-recovery').count(),0);await rootLayout('whole-diagnosis-900x600','[data-testid="tools-center"]');await page.setViewportSize({width:1200,height:760});await rootLayout('whole-diagnosis-1200x760','[data-testid="tools-center"]')
 await page.evaluate(async()=>{const{useAppStore}=await import('/src/stores/app.ts');useAppStore().openArchiveInBrowser(window.__fullNative.receipt.source)})
 const manager=page.getByTestId('dual-pane-file-manager');await manager.waitFor();await manager.locator('[data-entry-path="阅读副本.txt"]').waitFor();const right=manager.locator('[data-file-pane="right"]');assert.equal(await right.locator('.file-row').count(),verified.rootEntries.length)
 for(const size of [{width:1200,height:760},{width:900,height:600}]){await page.setViewportSize(size);const panes=await manager.evaluate(el=>[...el.querySelectorAll('.pane-grid > .file-pane')].map(p=>({width:p.getBoundingClientRect().width,overflow:p.scrollWidth-p.clientWidth})));assert.ok(panes.length===2&&panes.every(p=>p.overflow<=1)&&Math.abs(panes[0].width-panes[1].width)<=1,JSON.stringify(panes));await rootLayout(`whole-browser-${size.width}x${size.height}`,'[data-testid="dual-pane-file-manager"]')}
 assert.deepEqual(errors,[]);writeFileSync(`${base}/ui.json`,JSON.stringify({passed:true,nativeValueReplay:true,realDesktopIPC:false,resourceProbeReplay:'live filesystem values, IPC stub',checks,taskPanes,commands:await page.evaluate(()=>window.__fullCalls),history,errors},null,2));console.log('3 background UI groups passed: whole-archive task/result and history, complete diagnosis, equal browser panes with actual result tree.')
}finally{await browser.close()}

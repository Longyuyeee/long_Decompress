import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, renameSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runLibrary1547Gate({driver:initialDriver,getDriver,restart,bridge,artifactDirectory,application,root}) {
 let driver=initialDriver
 const base=path.join(root,'test-results/library-1547');mkdirSync(base,{recursive:true})
 const receipt=path.join(base,'desktop-progress.json'),hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex'),binarySha256=hash(application)
 const state=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.5.47',binarySha256,work:path.join(base,randomUUID()),checks:[],layouts:[],screenshots:[]}
 if(state.binarySha256!==binarySha256){assert.equal(process.env.LIBRARY_1547_RESUME_FROM,state.binarySha256,'Acknowledge retained evidence from the earlier program');state.previousIdentities=[...(state.previousIdentities||[]),state.binarySha256];state.binarySha256=binarySha256}
 mkdirSync(state.work,{recursive:true});const save=()=>writeFileSync(receipt,JSON.stringify(state,null,2)),passed=kind=>state.checks.some(row=>row.kind===kind),done=(kind,evidence={})=>{state.checks.push({kind,binarySha256,...evidence});save();console.log('library1547: '+kind)}
 const find=selector=>driver.wait(async()=>{try{const rows=await driver.findElements(By.css(selector));for(const row of rows)if(await row.isDisplayed())return row;return false}catch(error){if(error.name==='StaleElementReferenceError')return false;throw error}},20000,'Missing visible: '+selector)
 const click=async selector=>{const element=typeof selector==='string'?await find(selector):selector;await driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});e.click()},element)}
 const fill=async(selector,value)=>{const e=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
 const text=async selector=>(await find(selector)).getAttribute('textContent')
 const library=async()=>{await click('[data-testid="nav-ArchiveLibrary"]');await find('[data-testid="archive-library"]')}
 const row=name=>`[data-source-name="${name}"]`
 async function query(value,count){await fill('[data-testid="library-query"]',value);await driver.sleep(300);await driver.wait(async()=>(await driver.findElements(By.css('.library-hit'))).length===count&&!((await text('.results-caption')).includes('正在查找')),15000)}
 async function screenshot(name,anchor){
  if(state.layouts.filter(row=>row.name===name).length===2)return
  for(const[width,height]of[[1100,700],[760,520]]){
   await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),await find(anchor))
   await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
   const layout=await driver.executeScript(selector=>{const anchor=[...document.querySelectorAll(selector)].find(e=>e.offsetHeight);const page=document.querySelector('.library-page');return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,anchorOverflow:anchor?anchor.scrollWidth-anchor.clientWidth:0,pageOverflow:page?page.scrollWidth-page.clientWidth:0,columns:page?getComputedStyle(document.querySelector('.library-workspace')).gridTemplateColumns:''}},anchor)
   assert.ok(layout.overflow<=1&&layout.anchorOverflow<=1&&layout.pageOverflow<=1,JSON.stringify(layout))
   const file=`library1547-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,binarySha256,...layout});state.screenshots.push(file);save()
  }await driver.manage().window().setRect({width:1100,height:700})
 }
 const complete=source=>driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.sourcePaths?.includes(source)&&['completed','failed','cancelled'].includes(row.status))||false,90000)
 if(!state.fixture){
  state.fixture=path.join(state.work,'资料目录');mkdirSync(state.fixture,{recursive:true});state.zip=path.join(state.fixture,'项目资料.zip');state.tar=path.join(state.fixture,'旧资料.tar');state.big=path.join(state.work,'可停止的大目录.zip')
  const script=`import zipfile,tarfile,io,sys,pathlib
p=pathlib.Path(sys.argv[1]);b=pathlib.Path(sys.argv[2])
with zipfile.ZipFile(p/'项目资料.zip','w',zipfile.ZIP_DEFLATED) as z:
 z.writestr('docs/合同说明.txt','ZIP contract contents')
 for i in range(63):z.writestr('pages/file%03d.txt'%i,'page')
with tarfile.open(p/'旧资料.tar','w') as t:
 data=b'TAR contract contents';i=tarfile.TarInfo('资料/合同旧稿.txt');i.size=len(data);i.mtime=1700000000;t.addfile(i,io.BytesIO(data))
with zipfile.ZipFile(p/'空资料.zip','w'):pass
(p/'损坏资料.zip').write_bytes(b'not a complete ZIP')
with zipfile.ZipFile(b,'w',zipfile.ZIP_STORED) as z:
 for i in range(30000):z.writestr('pages/file%05d.txt'%i,'x')
`
  execFileSync('python',['-X','utf8','-c',script,state.fixture,state.big],{windowsHide:true,maxBuffer:1024*1024})
  // One unsupported-format candidate keeps errors concrete; no password enters the index.
  const protectedFile=path.join(state.work,'secret.txt');writeFileSync(protectedFile,'protected fixture')
  execFileSync(path.join(root,'src-tauri/resources/archive-engine/7z.exe'),['a','-tzip','-pfixture-1547','-mem=AES256',path.join(state.fixture,'密码资料.zip'),protectedFile],{windowsHide:true,maxBuffer:1024*1024})
  state.originalZipHash=hash(state.zip);state.originalTarHash=hash(state.tar);save()
 }
 await library()
 if(!passed('scan-confirm-index-search')){
  await bridge('clearTasks');await bridge('clearTaskHistory');await bridge('queueDesktopDialogSelections',[state.fixture]);await click('[data-testid="library-scan-folder"]');await find('[data-testid="library-candidates"]');assert.match(await text('[data-testid="library-candidates"]'),/5 份/);await screenshot('candidates','[data-testid="library-candidates"]');await click('[data-testid="library-confirm-candidates"]')
  await driver.wait(async()=>(await driver.findElements(By.css('.library-source'))).length===5,15000);assert.equal((await driver.findElements(By.css('[data-source-status="ready"]'))).length,0)
  await click('[data-testid="library-index-pending"]');await driver.wait(async()=>(await text('[data-testid="library-notice"]')).includes('需要处理'),60000)
  assert.equal((await driver.findElements(By.css('[data-source-status="ready"]'))).length,3);assert.equal((await driver.findElements(By.css('[data-source-status="failed"]'))).length,2)
  await click(row('密码资料.zip')+' .source-summary');assert.match(await text(row('密码资料.zip')+' .source-details'),/密码/);await screenshot('source-failure',row('密码资料.zip')+' .source-details')
  await query('合同',2);await screenshot('cross-search','[data-testid="library-results"]')
  done('scan-confirm-index-search',{manualIndex:true,readyAndFailuresSeparated:true,unicodeCrossSearch:true})
 }
 if(!passed('original-preview-and-return')){
  await query('合同',2)
  await click('[data-hit-source="项目资料.zip"] [data-testid="library-preview-hit"]');await driver.wait(async()=>(await text('[data-testid="archive-text-preview"]')).includes('ZIP contract contents'),20000);assert.match(await text('.browser-page footer'),/已选择 1 \/ 64/);await screenshot('hit-preview','[data-testid="archive-text-preview"]');await click('[aria-label="关闭预览"]');await click('[data-testid="archive-library-origin"] a');await find('[data-testid="archive-library"]');assert.equal(await(await find('[data-testid="library-query"]')).getAttribute('value'),'合同')
  done('original-preview-and-return',{manualIndex:true,readyAndFailuresSeparated:true,unicodeCrossSearch:true,originalPreview:true,exactSingleSelection:true,returnPreservesSearch:true})
 }
 if(!passed('single-hit-extraction-source-guard-and-refresh')){
  await query('合同',2);state.output=path.join(state.work,'取回资料');mkdirSync(state.output,{recursive:true});await bridge('queueDesktopDialogSelections',[state.output]);await click('[data-hit-source="旧资料.tar"] [data-testid="library-extract-hit"]');const task=await complete(state.tar);assert.equal(task.status,'completed',JSON.stringify(task));assert.equal(readFileSync(path.join(state.output,'资料','合同旧稿.txt'),'utf8'),'TAR contract contents');assert.equal(readdirSync(path.join(state.output,'资料')).length,1);assert.equal(hash(state.zip),state.originalZipHash);assert.equal(hash(state.tar),state.originalTarHash)
  await click('[data-testid="archive-library-origin"] a');await library();const moving=state.zip+'.moved';renameSync(state.zip,moving)
  await click('[data-hit-source="项目资料.zip"] [data-testid="library-preview-hit"]');await find('[data-testid="library-error"]');assert.ok((await driver.getCurrentUrl()).endsWith('/library'));await screenshot('missing-source','[data-testid="library-error"]');renameSync(moving,state.zip)
  execFileSync('python',['-X','utf8','-c',`import zipfile,sys
with zipfile.ZipFile(sys.argv[1],'a') as z:z.writestr('docs/新增合同.txt','new indexed content')`,state.zip],{windowsHide:true,maxBuffer:1024*1024})
  await click('[data-hit-source="项目资料.zip"] [data-testid="library-preview-hit"]');await driver.wait(async()=>(await text('[data-testid="library-error"]')).includes('已变化'),15000)
  await click(row('项目资料.zip')+' [data-testid="library-index-source"]');await driver.wait(async()=>(await text(row('项目资料.zip'))).includes('可搜索'),60000);await query('新增合同',1);await screenshot('refreshed-hit','[data-testid="library-results"]');state.changedZipHash=hash(state.zip);save()
  done('single-hit-extraction-source-guard-and-refresh',{taskId:task.id,singleFileContent:true,originalTarPreserved:true,missingAndChangedBlocked:true,manualRefresh:true})
 }
 if(!passed('stop-index-and-manual-continue')){
  await bridge('queueDesktopDialogSelections',[[state.big]]);await click('[data-testid="library-add-files"]');await click(row('可停止的大目录.zip')+' .source-summary');await click(row('可停止的大目录.zip')+' [data-testid="library-index-source"]');await find('[data-testid="library-progress"]');await click('[data-testid="library-stop"]');await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="library-progress"]'))).length,30000)
  assert.equal(await(await find(row('可停止的大目录.zip'))).getAttribute('data-source-status'),'interrupted');await screenshot('stopped-source',row('可停止的大目录.zip')); await click(row('可停止的大目录.zip')+' [data-testid="library-index-source"]');await driver.wait(async()=>await(await find(row('可停止的大目录.zip'))).getAttribute('data-source-status')==='ready',90000)
  done('stop-index-and-manual-continue',{stopCurrentSource:true,manualContinue:true,entryCount:30000})
 }
 if(!passed('persisted-restart')){
  await driver.executeScript(e=>{e.value='';e.dispatchEvent(new Event('change',{bubbles:true}))},await find('[data-testid="library-source-filter"]'))
  await query('合同',3);await bridge('setCloseToTray',false);await restart({preserveProfile:true,requestExit:()=>click('.control-btn[aria-label="关闭"]')});driver=getDriver();await library();await query('合同',3);assert.equal((await driver.findElements(By.css('[data-source-status="ready"]'))).length,4);await screenshot('persisted-search','[data-testid="library-results"]')
  done('persisted-restart',{stopCurrentSource:true,manualContinue:true,normalCloseSameProfile:true,persistedWithoutRebuild:true})
 }
 if(!passed('filters-pagination-and-remove-original-preserved')){
  await query('file',50);await find('[data-testid="library-next-page"]');await click('[data-testid="library-next-page"]');await driver.wait(async()=>(await text('.library-results .lib-pagination')).includes('2 /'),15000);await query('合同',3)
  const zipId=await(await find(row('项目资料.zip'))).getAttribute('data-source-id');const filter=await find('[data-testid="library-source-filter"]');await driver.executeScript((e,v)=>{e.value=v;e.dispatchEvent(new Event('change',{bubbles:true}))},filter,zipId);await driver.wait(async()=>(await driver.findElements(By.css('.library-hit'))).length===2,15000)
  const category=await find('[data-testid="library-category"]');await driver.executeScript(e=>{e.value='image';e.dispatchEvent(new Event('change',{bubbles:true}))},category);await driver.wait(async()=>(await driver.findElements(By.css('.library-hit'))).length===0,15000);await driver.executeScript(e=>{e.value='all';e.dispatchEvent(new Event('change',{bubbles:true}))},category)
  await click(row('空资料.zip')+' .source-summary');await click(row('空资料.zip')+' [data-testid="library-remove-source"]');await find('[data-testid="library-confirm-remove"]');assert.ok(existsSync(path.join(state.fixture,'空资料.zip')));await click('[data-testid="library-confirm-remove"]');await driver.wait(async()=>(await driver.findElements(By.css(row('空资料.zip')))).length===0,15000);assert.ok(existsSync(path.join(state.fixture,'空资料.zip')));assert.equal(hash(state.zip),state.changedZipHash);assert.equal(hash(state.tar),state.originalTarHash)
  done('filters-pagination-and-remove-original-preserved',{sourceAndTypeFilters:true,pagination:true,removeIndexOnly:true,originalHashes:true})
 }
 if(!passed('source-list-scroll-with-expanded-row')){
  await click(row('可停止的大目录.zip')+' .source-summary')
  for(const[width,height]of[[1100,700],[760,520]]){
   await driver.manage().window().setRect({width,height});await find('.source-details')
   const scrolling=await driver.executeScript(()=>{const list=document.querySelector('.source-list');const rows=[...list.querySelectorAll('.library-source')];list.scrollTop=list.scrollHeight;return{height:list.clientHeight,content:list.scrollHeight,scrollTop:list.scrollTop,clipped:rows.filter(e=>e.scrollHeight>e.clientHeight+1).length}})
   assert.equal(scrolling.clipped,0,JSON.stringify(scrolling));if(scrolling.content>scrolling.height)assert.ok(scrolling.scrollTop>0,JSON.stringify(scrolling));if(width===760)assert.ok(scrolling.content>scrolling.height,JSON.stringify(scrolling))
  }
  await screenshot('source-scroll','.source-list')
  await query('合同',3);await screenshot('final-cross-search','[data-testid="library-results"]')
  done('source-list-scroll-with-expanded-row',{noShrunkOrClippedRows:true,wideAndNarrowScroll:true})
 }
 console.log(JSON.stringify({version:state.version,checks:state.checks.length,layouts:state.layouts.length,binarySha256},null,2))
}

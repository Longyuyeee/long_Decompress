import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, renameSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runLibrary1548Gate({driver:initialDriver,getDriver,restart,bridge,artifactDirectory,application,root,dataDirectory}) {
 let driver=initialDriver
 const base=path.join(root,'test-results/library-1548');mkdirSync(base,{recursive:true})
 const receipt=path.join(base,'desktop-progress.json'),hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex'),binarySha256=hash(application)
 const state=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.5.48',binarySha256,work:path.join(base,randomUUID()),checks:[],layouts:[],screenshots:[]}
 if(state.binarySha256!==binarySha256){assert.equal(process.env.LIBRARY_1548_RESUME_FROM,state.binarySha256,'Acknowledge retained evidence from the earlier program');state.previousIdentities=[...(state.previousIdentities||[]),state.binarySha256];state.binarySha256=binarySha256}
 mkdirSync(state.work,{recursive:true});const save=()=>writeFileSync(receipt,JSON.stringify(state,null,2)),passed=kind=>state.checks.some(row=>row.kind===kind),done=(kind,evidence={})=>{state.checks.push({kind,binarySha256,...evidence});save();console.log('library1548: '+kind)}
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
   const file=`library1548-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,binarySha256,...layout});state.screenshots.push(file);save()
  }await driver.manage().window().setRect({width:1100,height:700})
 }
 const complete=source=>driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.sourcePaths?.includes(source)&&['completed','failed','cancelled'].includes(row.status))||false,90000)
 const database=()=>JSON.parse(execFileSync('python',['-X','utf8','-c',`import sqlite3,json,sys
c=sqlite3.connect(sys.argv[1]);c.row_factory=sqlite3.Row
print(json.dumps([dict(row) for row in c.execute('SELECT * FROM archive_library_sources')],ensure_ascii=False))`,path.join(dataDirectory,'data.db')],{encoding:'utf8',windowsHide:true}))
 const source=name=>database().find(row=>row.name===name)
 const waitRow=async(name,status)=>driver.wait(async()=>source(name)?.status===status,90000)
 async function expand(name){const e=await find(row(name)+' .source-summary');if(await e.getAttribute('aria-expanded')!=='true')await click(e)}
 async function refresh(name){await expand(name);await click(row(name)+' [data-testid="library-index-source"]');await waitRow(name,'ready');await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="library-progress"]'))).length,90000)}
 async function vault(){await click('[data-testid="library-vault"]');await click('[data-vault-entry="资源库密码1548"]');await driver.wait(async()=>!(await driver.findElements(By.css('[data-vault-entry="资源库密码1548"]'))).length,10000)}
 async function unlock(name){await expand(name);await click(row(name)+' [data-testid="library-source-credentials"]');await vault();await click('[data-testid="library-credential-apply"]');await waitRow(name,'ready');await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="library-credentials"]'))).length,90000)}
 async function allSources(){await driver.executeScript(e=>{e.value='';e.dispatchEvent(new Event('change',{bubbles:true}))},await find('[data-testid="library-source-filter"]'))}
 if(!state.fixture){
  state.fixture=path.join(state.work,'维护资料');mkdirSync(state.fixture,{recursive:true});state.zip=path.join(state.fixture,'密码内容.zip');state.seven=path.join(state.fixture,'加密目录.7z');state.tar=path.join(state.fixture,'日常资料.tar.gz');state.big=path.join(state.fixture,'000-暂停资料.zip');state.next=path.join(state.fixture,'001-后续资料.zip')
  const script=`import zipfile,tarfile,io,sys,pathlib
p=pathlib.Path(sys.argv[1])
with zipfile.ZipFile(p/'split-base.zip','w',zipfile.ZIP_STORED) as z:z.writestr('资料/分卷说明.txt','split library contents');z.writestr('padding.bin',b'x'*180000)
b=(p/'split-base.zip').read_bytes()
for i,start in enumerate(range(0,len(b),30000)): (p/('资料.zip.%03d'%(i+1))).write_bytes(b[start:start+30000])
with tarfile.open(p/'日常资料.tar.gz','w:gz') as t:
 data=b'maintenance original';info=tarfile.TarInfo('资料/维护说明.txt');info.size=len(data);t.addfile(info,io.BytesIO(data))
with zipfile.ZipFile(p/'000-暂停资料.zip','w',zipfile.ZIP_STORED) as z:
 for i in range(100000):z.writestr('items/file%06d.txt'%i,'x')
with zipfile.ZipFile(p/'001-后续资料.zip','w') as z:z.writestr('after.txt','next source')
(p/'密码说明.txt').write_text('protected library contents',encoding='utf-8')
`
  execFileSync('python',['-X','utf8','-c',script,state.fixture],{windowsHide:true,maxBuffer:1024*1024})
  const seven=path.join(root,'src-tauri/resources/archive-engine/7z.exe'),input=path.join(state.fixture,'密码说明.txt');execFileSync(seven,['a','-tzip','-pfixture-1548','-mem=AES256',state.zip,input],{windowsHide:true,maxBuffer:1024*1024});execFileSync(seven,['a','-t7z','-pfixture-1548','-mhe=on',state.seven,input],{windowsHide:true,maxBuffer:1024*1024})
  state.members=readdirSync(state.fixture).filter(name=>/资料.zip.\d+$/.test(name)).map(name=>path.join(state.fixture,name));assert.ok(state.members.length>2)
  state.zipHash=hash(state.zip);state.sevenHash=hash(state.seven);state.tarHash=hash(state.tar);save()
 }
 await library()
 if(!passed('upgrade-existing-index-and-extended-group-import')){
  assert.ok(database().some(row=>row.name==='项目资料.zip'&&row.entry_count===65),'The copied v1.5.47 cache must survive migration')
  await bridge('seedVaultPassword','资源库密码1548','fixture-1548')
  await bridge('queueDesktopDialogSelections',[[state.zip,state.seven,state.tar,state.members[1],state.members[0]]]);await click('[data-testid="library-add-files"]');await driver.wait(async()=>['密码内容.zip','加密目录.7z','日常资料.tar.gz','资料.zip.001'].every(name=>source(name)),15000)
  assert.equal(database().filter(item=>item.path.includes('资料.zip.00')).length,1)
  await refresh('日常资料.tar.gz');await refresh('资料.zip.001');assert.equal(source('资料.zip.001').member_count,state.members.length)
  await expand('密码内容.zip');await click(row('密码内容.zip')+' [data-testid="library-index-source"]');await waitRow('密码内容.zip','locked');await click(row('密码内容.zip')+' [data-testid="library-source-credentials"]');await fill('[data-testid="library-password"]','wrong');await click('[data-testid="library-credential-apply"]');await waitRow('密码内容.zip','locked');await find('[data-testid="library-credential-error"]');await screenshot('wrong-password','[data-testid="library-credentials"]')
  await vault();await click('[data-testid="library-password-encoding"]');const option=await driver.wait(async()=>{for(const e of await driver.findElements(By.css('[role="option"]')))if(await e.isDisplayed()&&(await e.getAttribute('textContent')).includes('UTF-8'))return e;return false},10000);await click(option);await click('[data-testid="library-credential-apply"]');await waitRow('密码内容.zip','ready');await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="library-credentials"]'))).length,90000)
  await unlock('加密目录.7z');await allSources();await query('密码说明',2);await screenshot('password-search','[data-testid="library-results"]');assert.equal(source('密码内容.zip').source_encoding,'utf8');assert.equal(hash(state.zip),state.zipHash);assert.equal(hash(state.seven),state.sevenHash)
  done('upgrade-existing-index-and-extended-group-import',{oldCachePreserved:true,groupDeduplication:true,compressedTar:true,aesZipAndHeaderEncryptedSevenZip:true,wrongPasswordThenVault:true,encodingSavedWithoutPassword:true})
 }
 if(!passed('password-hit-original-extraction-and-result')){
  await allSources();await query('密码说明',2);state.output=path.join(state.work,'密码资料取回');mkdirSync(state.output,{recursive:true});await bridge('queueDesktopDialogSelections',[state.output]);await click('[data-hit-source="加密目录.7z"] [data-testid="library-extract-hit"]');const task=await complete(state.seven);assert.equal(task.status,'completed',JSON.stringify(task));assert.equal(readFileSync(path.join(state.output,'密码说明.txt'),'utf8'),'protected library contents');assert.equal(readdirSync(state.output).length,1);await click('[data-testid="archive-library-origin"] a')
  done('password-hit-original-extraction-and-result',{taskId:task.id,sessionCredentialsPassed:true,selectedContentOnly:true,originalPreserved:hash(state.seven)===state.sevenHash})
 }
 if(!passed('offline-cache-relocate-and-incremental-refresh')){
  await allSources();await query('维护说明',1);state.moved=path.join(state.work,'重新定位的资料.tar.gz');renameSync(state.tar,state.moved);await expand('日常资料.tar.gz');await click(row('日常资料.tar.gz')+' [data-testid="library-audit-source"]');await driver.wait(async()=>source('日常资料.tar.gz').availability==='missing',15000);await find('[data-testid="library-hit-availability"]');assert.match(await text('[data-testid="library-hit-availability"]'),/离线/);await screenshot('offline-cache','[data-testid="library-results"]')
  await click('[data-testid="library-preview-hit"]');await find('[data-testid="library-error"]');assert.ok((await driver.getCurrentUrl()).endsWith('/library'));await bridge('queueDesktopDialogSelections',[state.moved]);await click(row('日常资料.tar.gz')+' [data-testid="library-relocate-source"]');await driver.wait(async()=>source('重新定位的资料.tar.gz')?.availability==='changed',15000);await click('[data-testid="library-preview-hit"]');assert.ok((await driver.getCurrentUrl()).endsWith('/library'));await refresh('重新定位的资料.tar.gz');assert.equal(hash(state.moved),state.tarHash)
  const unchanged=source('资料.zip.001').indexed_at;execFileSync('python',['-X','utf8','-c',`import tarfile,io,sys
with tarfile.open(sys.argv[1],'w:gz') as t:
 for name,data in [('资料/维护说明.txt',b'maintenance updated'),('资料/新增维护说明.txt',b'new maintained file')]:
  info=tarfile.TarInfo(name);info.size=len(data);t.addfile(info,io.BytesIO(data))`,state.moved],{windowsHide:true,maxBuffer:1024*1024})
  await click(row('重新定位的资料.tar.gz')+' [data-testid="library-audit-source"]');await driver.wait(async()=>source('重新定位的资料.tar.gz').availability==='changed',15000);await allSources();await query('维护说明',1);await refresh('重新定位的资料.tar.gz');await query('维护说明',2);assert.equal(source('资料.zip.001').indexed_at,unchanged);await screenshot('updated-cache','[data-testid="library-results"]');state.changedTarHash=hash(state.moved);save()
  done('offline-cache-relocate-and-incremental-refresh',{cachedOfflineSearch:true,staleActionsBlocked:true,sameSourceIdentity:true,explicitRefresh:true,unrelatedIndexNotRebuilt:true})
 }
 if(!passed('missing-volume-cache-and-restored-source-skip')){
  await allSources();await query('分卷说明',1);const missing=state.members[1],held=missing+'.held';renameSync(missing,held);await expand('资料.zip.001');await click(row('资料.zip.001')+' [data-testid="library-audit-source"]');await driver.wait(async()=>source('资料.zip.001').availability==='incomplete',15000);assert.match(await text('[data-testid="library-hit-availability"]'),/缺少分卷/);await screenshot('missing-volume',row('资料.zip.001')+' .source-details');await click('[data-testid="library-extract-hit"]');assert.ok((await driver.getCurrentUrl()).endsWith('/library'));renameSync(held,missing)
  const before=source('资料.zip.001').indexed_at;await refresh('资料.zip.001');assert.equal(source('资料.zip.001').indexed_at,before);assert.equal(source('资料.zip.001').availability,'available')
  done('missing-volume-cache-and-restored-source-skip',{missingMemberNamed:true,cachedHitsRetained:true,extractBlocked:true,restoredUnchangedIndexSkipped:true})
 }
 if(!passed('pause-continue-and-normal-restart-password-reentry')){
  // These two obsolete v1.5.47 failure fixtures are no longer part of the current batch.
  for(const name of ['密码资料.zip','损坏资料.zip'])if(source(name)){await expand(name);await click(row(name)+' [data-testid="library-remove-source"]');await click('[data-testid="library-confirm-remove"]');await driver.wait(async()=>!source(name),10000)}
  await bridge('queueDesktopDialogSelections',[[state.big,state.next]]);await click('[data-testid="library-add-files"]');await driver.wait(async()=>source('000-暂停资料.zip')&&source('001-后续资料.zip')&&await(await find('[data-testid="library-index-pending"]')).isEnabled(),20000);await click('[data-testid="library-index-pending"]');await find('[data-testid="library-progress"]');await click('[data-testid="library-pause"]');await waitRow('000-暂停资料.zip','ready');await driver.wait(async()=>(await text('[data-testid="library-progress"]')).includes('已暂停'),90000);assert.equal(source('001-后续资料.zip').status,'pending');await screenshot('paused-batch','[data-testid="library-progress"]');const first=source('000-暂停资料.zip').indexed_at;await click('[data-testid="library-pause"]');await waitRow('001-后续资料.zip','ready');assert.equal(source('000-暂停资料.zip').indexed_at,first)
  await bridge('setCloseToTray',false);await restart({preserveProfile:true,requestExit:()=>click('.control-btn[aria-label="关闭"]')});driver=getDriver();await library();await allSources();await query('密码说明',2);await click('[data-hit-source="密码内容.zip"] [data-testid="library-preview-hit"]');assert.equal(await(await find('[data-testid="library-password"]')).getAttribute('value'),'');await screenshot('restart-password','[data-testid="library-credentials"]');await vault();await click('[data-testid="library-credential-apply"]');await driver.wait(async()=>(await text('[data-testid="archive-text-preview"]')).includes('protected library contents'),90000);await screenshot('recovered-preview','[data-testid="archive-text-preview"]');assert.equal(hash(state.zip),state.zipHash);await click('[aria-label="关闭预览"]');await click('[data-testid="archive-library-origin"] a')
  done('pause-continue-and-normal-restart-password-reentry',{pauseStopsNextDispatch:true,continueDoesNotRepeatFirst:true,normalCloseSameProfile:true,noPersistedPassword:true,vaultAndPreviewAfterRestart:true})
 }
 console.log(JSON.stringify({version:state.version,checks:state.checks.length,layouts:state.layouts.length,binarySha256},null,2))
}

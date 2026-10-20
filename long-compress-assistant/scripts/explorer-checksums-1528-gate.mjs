import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import path from 'node:path'
import {By} from 'selenium-webdriver'
const sha=file=>createHash('sha256').update(readFileSync(file)).digest('hex')
export function prepareExplorerChecksum1528Startup(root){
  const base=path.join(root,'test-results/explorer-checksums-1528');mkdirSync(base,{recursive:true})
  const stateFile=path.join(base,'fixtures.json')
  if(!existsSync(stateFile)){
    const work=path.join(base,'实际 文件-'+randomUUID());mkdirSync(work)
    const files=['甲/同名 文件.txt','乙/同名 文件.txt','后续 文件.txt'].map((name,index)=>{const file=path.join(work,name);mkdirSync(path.dirname(file),{recursive:true});writeFileSync(file,index===0?'hello':'独立内容 '+index);return file})
    const manifests={};for(const algorithm of ['sha256','md5']){const file=path.join(work,'收到的清单.'+algorithm);writeFileSync(file,files.slice(0,2).map(f=>createHash(algorithm).update(readFileSync(f)).digest('hex')+' *'+path.relative(work,f).replaceAll('\\','/')).join('\n')+'\n');manifests[algorithm]=file}
    manifests.sfv=path.join(work,'收到的清单.sfv');writeFileSync(manifests.sfv,'甲/同名 文件.txt 3610A686\n')
    writeFileSync(stateFile,JSON.stringify({work,files,manifests},null,2))
  }
  const fixture=JSON.parse(readFileSync(stateFile,'utf8'));return ['--checksum-create',...fixture.files.slice(0,2)]
}
export async function runExplorerChecksum1528Gate({driver,bridge,waitForElement,artifactDirectory,application,root,forwardContextAction}){
 const base=path.join(root,'test-results/explorer-checksums-1528'),fixture=JSON.parse(readFileSync(path.join(base,'fixtures.json'),'utf8')),progress=path.join(base,'desktop-progress.json'),binarySha256=sha(application)
 const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.28',binarySha256,sourceCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim(),ui:[],screenshots:[],layouts:[]}
 assert.equal(state.binarySha256,binarySha256);const save=()=>writeFileSync(progress,JSON.stringify(state,null,2)),done=(kind,data={})=>{state.ui.push({kind,...data});save();console.log('explorer-1528: '+kind)},passed=kind=>state.ui.some(row=>row.kind===kind)
 const click=async target=>{for(const t of await driver.findElements(By.css('.toast-item button'))){try{await driver.executeScript(e=>e.click(),t)}catch{}}const e=typeof target==='string'?await waitForElement(target):target;await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect();return !e.disabled&&r.width>0&&r.height>0},e),10000);await driver.executeScript(e=>e.click(),e)}
 const sizes=async(name,selector)=>{for(const[width,height]of[[1100,700],[760,520]]){await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement(selector));await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000);const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('.checksum-handoff,.checksum-panel')].filter(e=>e.offsetParent).map(e=>e.scrollWidth-e.clientWidth)}));assert.ok(layout.overflow<=1&&layout.panels.every(v=>v<=1),JSON.stringify(layout));const file='explorer1528-'+name+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots=[...new Set([...state.screenshots,file])];state.layouts.push({name,...layout});save()}await driver.manage().window().setRect({width:1100,height:700})}
 const calcRows=()=>driver.findElements(By.css('[data-testid="checksum-calculation-entry"]'))
 const calculate=async(count)=>{await click('[data-testid="checksum-calculate-start"]');await driver.wait(async()=>await driver.executeScript(()=>document.querySelectorAll('[data-testid="checksum-calculation-entry"][data-status="success"]').length)===count&&await(await waitForElement('[data-testid="checksum-calculate-start"]')).isEnabled(),30000)}
 const verify=async(file,count)=>{await(await waitForElement('[data-testid="checksum-staged-path"]')).sendKeys(path.basename(file));await driver.executeScript((e,value)=>{e.value=value;e.dispatchEvent(new Event('change',{bubbles:true}))},await waitForElement('[data-testid="checksum-staged-path"]'),file);await click('[data-testid="checksum-staged-start"]');await driver.wait(async()=>(await(await waitForElement('[data-testid="checksum-heading"]')).getText()).includes('均匹配')&&await driver.executeScript(()=>document.querySelectorAll('[data-testid="checksum-entry"][data-status="matched"]').length)===count&&await(await waitForElement('[data-testid="checksum-verification"]')).getAttribute('aria-busy')==='false',30000)}
 await driver.manage().window().setRect({width:1100,height:700})
 if(!passed('cold-start-selection-export')){
  await driver.wait(async()=>(await(await waitForElement('[data-testid="checksum-source-summary"]')).getText()).includes('2 个文件'),15000);assert.equal((await calcRows()).length,0)
  await click('[data-testid="checksum-source-toggle"]');assert.deepEqual(await driver.executeScript(()=>[...document.querySelectorAll('.checksum-source-paths li')].map(e=>e.textContent)),fixture.files.slice(0,2));await sizes('ready-files','[data-testid="checksum-source-summary"]')
  await calculate(2);assert.deepEqual(await driver.executeScript(()=>[...document.querySelectorAll('.checksum-main>code')].map(e=>e.textContent)),fixture.files.slice(0,2).map(sha))
  const manifest=path.join(fixture.work,'导出的清单-'+randomUUID()+'.sha256');await bridge('queueDesktopDialogSelections',[manifest]);await click('[data-testid="checksum-export"]');await driver.wait(()=>existsSync(manifest),15000)
  const actual=readFileSync(manifest,'utf8').trim().split('\n').map(row=>row.trim());assert.deepEqual(actual,fixture.files.slice(0,2).map(file=>sha(file)+' *'+path.relative(fixture.work,file).replaceAll('\\','/')));done('cold-start-selection-export',{chineseSpaces:true,sameNames:true,explicitStart:true,independentHashes:true,manifest})
 }
 if(!passed('single-instance-bursts-and-merge')){
  const old=await driver.executeScript(()=>[...document.querySelectorAll('.checksum-main>code')].map(e=>e.textContent))
  forwardContextAction('--checksum-create',[fixture.files[1],fixture.files[2]]);forwardContextAction('--checksum-create',[fixture.files[0]])
  await driver.wait(async()=>(await(await waitForElement('[data-testid="checksum-explorer-pending"]')).getText()).includes('3 个文件'),15000)
  assert.deepEqual(await driver.executeScript(()=>[...document.querySelectorAll('.checksum-main>code')].map(e=>e.textContent)),old);await sizes('pending-files','[data-testid="checksum-explorer-pending"]')
  await click('[data-testid="checksum-explorer-append"]');assert.equal((await calcRows()).length,0);await calculate(3)
  const actual=await driver.executeScript(()=>[...document.querySelectorAll('.checksum-main>code')].map(e=>e.textContent));assert.deepEqual(actual,fixture.files.map(sha));done('single-instance-bursts-and-merge',{priorResultsPreserved:true,noAutomaticCalculation:true,uniqueFiles:3})
 }
 if(!passed('multiple-manifests-explicit-start')){
  forwardContextAction('--checksum-verify',[fixture.manifests.sha256,fixture.manifests.md5]);await click('[data-testid="checksum-explorer-use"]');await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="checksum-staged-path"] option'))).length===2,15000)
  assert.equal((await driver.findElements(By.css('[data-testid="checksum-report"]'))).length,0);await sizes('ready-manifests','[data-testid="checksum-explorer-manifests"]');await verify(fixture.manifests.md5,2)
  forwardContextAction('--checksum-verify',[fixture.manifests.sfv,fixture.manifests.sha256]);await click('[data-testid="checksum-explorer-use"]');await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="checksum-staged-path"] option'))).length===3,15000)
  assert.match(await(await waitForElement('[data-testid="checksum-report"]')).getText(),/收到的清单.md5/);await verify(fixture.manifests.sha256,2);await click('[data-testid="checksum-entry"][data-line="2"] [data-testid="checksum-row"]');await sizes('verified-details','[data-testid="checksum-details"]');await verify(fixture.manifests.sfv,1)
  done('multiple-manifests-explicit-start',{manifests:3,noImplicitReading:true,allIndependentManifests:true})
 }
 if(!passed('busy-request-kept-and-causes')){
  // A missing source produces an actionable real result, not a fabricated success.
  const missing=path.join(fixture.work,'已经移动.bin');forwardContextAction('--checksum-create',[missing]);await click('[data-testid="checksum-explorer-use"]');assert.equal((await calcRows()).length,0);await click('[data-testid="checksum-calculate-start"]');await driver.wait(async()=>await driver.executeScript(()=>document.querySelectorAll('[data-testid="checksum-calculation-entry"][data-status="failed"]').length)===1,15000)
  await sizes('missing-file','[data-testid="checksum-calculation-entry"]');done('busy-request-kept-and-causes',{missingSourceRealFailure:true,noExtractionTask:true})
 }
 state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'explorer-checksums-1528-results.json'),JSON.stringify(state,null,2))
}

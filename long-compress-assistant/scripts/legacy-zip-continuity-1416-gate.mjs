import assert from 'node:assert/strict'
import {mkdirSync,readFileSync,writeFileSync,existsSync,readdirSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

// All work is performed through genuine controls. The bridge supplies only
// file-dialog responses, native exit, and read-only task/history observations.
export async function runLegacyZipContinuity1416Gate({getDriver,restart,bridge,waitForElement,fixtureDirectory,artifactDirectory,root,application}) {
  let driver=getDriver()
  const work=path.join(fixtureDirectory,'legacy-zip-continuity-1416'),evidence=path.join(artifactDirectory,'legacy-zip-continuity-1416')
  mkdirSync(work);mkdirSync(evidence,{recursive:true})
  const source=path.join(root,'tests/fixtures/archives/legacy-zip-password-1416'),payload=readFileSync(path.join(source,'payload.txt'))
  const hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex')
  const cases=[{code:'cp932',password:'日本語パスワード+!1416'},{code:'cp936',password:'简体密码测试+!1416'},{code:'cp950',password:'繁體密碼測試+!1416'}]
  const originals=cases.map(item=>({...item,originalHash:hash(path.join(source,item.code+'.zip'))}))
  const damaged=originals.map(item=>{const bytes=readFileSync(path.join(source,item.code+'.zip')),file=path.join(work,item.code+'-directory-lost.zip');writeFileSync(file,bytes.subarray(0,bytes.readUInt32LE(bytes.length-22+16)));return {...item,file,damagedHash:hash(file)}})
  const screenshots=[],layouts=[],results=[]
  const click=async selector=>{
    const el=typeof selector==='string'?await waitForElement(selector):selector
    let previous='',stable=0
    await driver.wait(async()=>{
      const s=await driver.executeScript(el=>{el.scrollIntoView({block:'center',behavior:'instant'});const r=el.getBoundingClientRect();return {box:JSON.stringify(r.toJSON()),hit:!el.disabled&&el.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))}},el)
      if(!s.hit||previous!==s.box){previous=s.box;stable=Date.now();return false}return Date.now()-stable>=300
    },10000,`reachable control: ${typeof selector==='string'?selector:'element'}`,100)
    await el.click()
  }
  const fill=async(selector,value)=>{const el=typeof selector==='string'?await waitForElement(selector):selector;await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value)}
  const select=async(selector,code)=>{
    await click(selector)
    const option=await driver.wait(async()=>{for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.getAttribute('textContent')).includes(code.toUpperCase()))return el;return false},10000)
    await click(option)
  }
  const text=async(selector,expected)=>{
    const el=typeof selector==='string'?await waitForElement(selector):selector
    await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el)
    await driver.wait(async()=>expected.test(await el.getText()),10000,String(expected))
  }
  const shot=async(name,selector)=>{
    const el=typeof selector==='string'?await waitForElement(selector):selector;await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el)
    await driver.wait(()=>driver.executeScript(()=>[...document.getAnimations()].filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),5000)
    const bounds=await driver.executeScript(el=>({width:innerWidth,height:innerHeight,scroll:document.documentElement.scrollWidth,panelWidth:el.clientWidth,panelScroll:el.scrollWidth}),el)
    assert.ok(bounds.scroll<=bounds.width+1&&bounds.panelScroll<=bounds.panelWidth+1,JSON.stringify(bounds));layouts.push({name,...bounds})
    const file=name+'.png';writeFileSync(path.join(evidence,file),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push(file)
  }
  const independent=(item,archive,name)=>{
    const tool=path.join(root,'test-results/zip-encoding-1416/independent-7zip',item.code,'7z.exe'),out=path.join(work,name)
    const r=spawnSync(tool,['x',archive,`-p${item.password}`,`-o${out}`,'-y'],{encoding:'utf8',windowsHide:true,timeout:60000})
    assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr);assert.deepEqual(readFileSync(path.join(out,'payload.txt')),payload)
  }
  const tasks=()=>bridge('extractionRecoveryTasks')
  const run=async ids=>{
    await click('[data-testid="start-decompression-queue"]')
    return driver.wait(async()=>{const all=await tasks(),chosen=ids.map(id=>all.find(t=>t.id===id));return chosen.every(t=>t&&['completed','failed','cancelled'].includes(t.status))?chosen:false},60000)
  }
  const expand=async id=>{
    const el=await waitForElement(`[data-testid="toggle-archive-task-${id}"]`);if(await el.getAttribute('aria-expanded')!=='true')await click(el)
    await driver.wait(()=>driver.executeScript(()=>[...document.querySelectorAll('.details-drawer')].every(el=>el.style.overflow!=='hidden'&&!el.getAnimations().some(a=>a.playState==='running'))),10000)
  }
  const preview=async code=>{
    await waitForElement('.browser-row[data-entry-path="payload.txt"]');await text('[data-testid="browser-zip-password-encoding"]',new RegExp(code.toUpperCase()))
    await click('.browser-row[data-entry-path="payload.txt"] .preview-trigger')
    assert.equal(await (await waitForElement('[data-testid="archive-text-preview"]')).getAttribute('textContent'),payload.toString('utf8'))
    await click('.preview-close')
  }
  const snapshot=async()=>{
    const raw=await driver.executeScript(()=>localStorage.getItem('long-archive-recovery-workspace-v1'))
    assert.ok(raw);for(const item of cases)assert.ok(!raw.includes(item.password));assert.ok(!raw.includes('changed source'))
    return JSON.parse(raw)
  }
  console.log('[legacy-zip-continuity-1416] real failure -> rescue -> single result restart')
  const single=damaged[0],out=path.join(work,'用户修改的输出');mkdirSync(out)
  await bridge('queueDesktopDialogSelections',[[single.file]]);await click('[data-testid="dropzone-file"]')
  const initial=await driver.wait(async()=>(await tasks()).find(t=>t.source===single.file),15000)
  await expand(initial.id);await select('[data-testid="task-zip-password-encoding"]','cp932')
  assert.equal((await run([initial.id]))[0].status,'failed')
  await expand(initial.id);await fill('[data-testid="retry-output"]',out);await fill('[data-testid="retry-password"]',single.password)
  const subfolder=await waitForElement('[data-testid="retry-subfolder"]');if(await subfolder.isSelected())await click(subfolder)
  await click('[data-testid="retry-rescue"]');await waitForElement('[data-testid="recovery-task-context"]')
  await text('[data-testid="diagnostic-report"]',/目录信息丢失/);await text('[data-testid="diagnosis-zip-password-encoding"]',/CP932/)
  const repaired=path.join(work,'单条救回.zip');await bridge('queueDesktopDialogSelections',[repaired]);await click('[data-testid="start-archive-repair"]')
  await waitForElement('[data-testid="repair-result"]');independent(single,repaired,'single-independent')
  await select('[data-testid="diagnosis-zip-password-encoding"]','cp950');await fill('.archive-password','changed source')
  await text('[data-testid="repair-output-encryption"]',/CP932/)
  const savedSingle=await snapshot();assert.equal(savedSingle.rows.length,1);assert.equal(savedSingle.rows[0].zipPasswordEncoding,'cp950');assert.equal(savedSingle.rows[0].resultZipPasswordEncoding,'cp932')
  await restart();driver=getDriver();await click('[data-testid="nav-FileIntegrity"]')
  await waitForElement('[data-testid="archive-recovery-restore"]');assert.equal((await driver.findElements(By.css('[data-testid="repair-result"]'))).length,0)
  await click('[data-testid="archive-recovery-restore"]');await waitForElement('[data-testid="single-repair-result-password"]')
  assert.equal(await (await waitForElement('[data-testid="single-repair-result-password"]')).getAttribute('value'),'')
  assert.equal(await (await waitForElement('.archive-password')).getAttribute('value'),'')
  await text('[data-testid="diagnosis-zip-password-encoding"]',/CP950/);await text('[data-testid="repair-output-encryption"]',/CP932/)
  await click('[data-testid="queue-recovered-archive"]');assert.equal((await tasks()).filter(t=>t.source===repaired).length,0)
  for(const [width,height] of [[1100,700],[760,520]]){await driver.manage().window().setRect({width,height});await shot('single-result-restored-'+width,'[data-testid="single-repair-result-password"]')}
  await fill('[data-testid="single-repair-result-password"]',single.password)
  await click('[data-testid="browse-repaired-archive"]');await preview('cp932')
  await click('[data-testid="nav-FileIntegrity"]');await click('[data-testid="queue-recovered-archive"]')
  const queued=await driver.wait(async()=>(await tasks()).find(t=>t.source===repaired),10000)
  assert.equal(queued.output,out);assert.equal(queued.extractToSubfolder,false);assert.equal(queued.retryOfTaskId,initial.id)
  await click('[data-testid="nav-Decompress"]');assert.equal((await run([queued.id]))[0].status,'completed')
  assert.deepEqual(readFileSync(path.join(out,'payload.txt')),payload)
  const history=await bridge('taskHistory');assert.equal(history.find(t=>t.id===queued.id)?.executionConfig.zipPasswordEncoding,'cp932');assert.equal(history.find(t=>t.id===initial.id)?.status,'failed')
  results.push({scenario:'failed-task-single-rescue-native-restart-browse-extract',failedTask:initial.id,completedTask:queued.id,sourceEncoding:'cp950',resultEncoding:'cp932',outputHash:hash(path.join(out,'payload.txt')),repairedHash:hash(repaired)})
  console.log('[legacy-zip-continuity-1416] three-codepage batch repair -> native restart -> refill -> browse and extract')
  await click('[data-testid="nav-FileIntegrity"]');await bridge('queueDesktopDialogSelections',[damaged.map(item=>item.file)]);await click('.archive-select')
  const rowFor=async item=>driver.wait(async()=>{for(const row of await driver.findElements(By.css('[data-testid="archive-batch-row"]')))if(await row.getAttribute('data-source')===item.file)return row;return false},15000)
  for(const item of damaged){const row=await rowFor(item);await click(await row.findElement(By.css('[data-testid="archive-batch-expand"]')));await fill(await row.findElement(By.css('[data-testid="archive-batch-password"]')),item.password);await select(await row.findElement(By.css('[data-testid="recovery-row-zip-password-encoding"]')),item.code)}
  await click('[data-testid="batch-diagnose"]');await driver.wait(async()=>{const s=await driver.findElements(By.css('[data-testid="archive-batch-status"]'));return s.length===3&&(await Promise.all(s.map(el=>el.getText()))).every(t=>t.includes('目录丢失'))},30000)
  const batchOut=path.join(work,'批量恢复');mkdirSync(batchOut);await bridge('queueDesktopDialogSelections',[batchOut]);await click('[data-testid="batch-repair"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-batch-result"]'))).length===3,60000)
  for(const item of damaged){const folder=path.join(batchOut,readdirSync(batchOut).find(name=>name.includes(item.code)));item.repaired=path.join(folder,readdirSync(folder).find(name=>name.endsWith('.zip')));independent(item,item.repaired,item.code+'-independent')}
  const first=await rowFor(single);await select(await first.findElement(By.css('[data-testid="recovery-row-zip-password-encoding"]')),'cp950');await fill(await first.findElement(By.css('[data-testid="archive-batch-password"]')),'changed source')
  await snapshot();await restart();driver=getDriver();await click('[data-testid="nav-FileIntegrity"]');await click('[data-testid="archive-recovery-restore"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-batch-row"]'))).length===3,15000)
  assert.equal(await (await waitForElement('[data-testid="batch-repair"]')).getAttribute('disabled'),'true')
  await click('[data-testid="batch-queue-recovered"]');assert.equal((await tasks()).filter(t=>t.source.startsWith(batchOut)).length,0)
  for(const item of damaged){
    const row=await rowFor(item),input=await row.findElement(By.css('[data-testid="archive-batch-result-password"]'))
    assert.equal(await input.getAttribute('value'),'');assert.equal(await row.findElement(By.css('[data-testid="archive-batch-password"]')).getAttribute('value'),'')
    await text(await row.findElement(By.css('[data-testid="recovery-row-zip-password-encoding"]')),new RegExp(item===single?'CP950':item.code.toUpperCase()))
    await fill(input,item.password)
    await driver.manage().window().setRect({width:760,height:520});await shot('batch-result-restored-'+item.code,input)
    await click(await row.findElement(By.css('[data-testid="browse-batch-repaired-archive"]')));await preview(item.code);await click('[data-testid="nav-FileIntegrity"]')
  }
  await click('[data-testid="batch-queue-recovered"]')
  const batchTasks=(await tasks()).filter(t=>t.source.startsWith(batchOut));assert.equal(batchTasks.length,3)
  await click('[data-testid="nav-Decompress"]');assert.ok((await run(batchTasks.map(t=>t.id))).every(t=>t.status==='completed'))
  const finalHistory=await bridge('taskHistory')
  for(const item of damaged){const task=batchTasks.find(t=>t.source===item.repaired);assert.ok(task);assert.equal(finalHistory.find(t=>t.id===task.id)?.executionConfig.zipPasswordEncoding,item.code);const file=path.join(task.output,path.basename(task.source,'.zip'),'payload.txt');assert.deepEqual(readFileSync(file),payload);results.push({scenario:'batch-repair-native-restart-browse-extract',code:item.code,taskId:task.id,repairedHash:hash(item.repaired),payloadHash:hash(file)})}
  for(const item of damaged){assert.equal(hash(item.file),item.damagedHash);assert.equal(hash(path.join(source,item.code+'.zip')),item.originalHash)}
  await driver.manage().window().setRect({width:1100,height:700});await shot('completed-real-extractions',`[data-testid="toggle-archive-task-${batchTasks[0].id}"]`)
  writeFileSync(path.join(evidence,'results.json'),JSON.stringify({verifiedAt:new Date().toISOString(),applicationSha256:hash(application),source:'pinned Info-ZIP originals; private damaged copies remove only central directories',sameProfileNativeRestarts:2,results,screenshots,layouts},null,2))
  console.log(`[legacy-zip-continuity-1416] final real continuity workflows passed: ${evidence}`)
}

import assert from 'node:assert/strict'
import { mkdirSync, copyFileSync, readFileSync, writeFileSync, existsSync, readdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

// Real desktop controls and production commands; the bridge substitutes only
// OS dialog selections and exposes read-only task observations.
export async function runLegacyZipWorkspace1416Gate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, root, application }) {
  const source = path.join(root, 'tests/fixtures/archives/legacy-zip-password-1416')
  const work = path.join(fixtureDirectory, 'legacy-zip-workspace-1416')
  const evidence = path.join(artifactDirectory, 'legacy-zip-workspace-1416')
  mkdirSync(work); mkdirSync(evidence, {recursive:true})
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const payload = readFileSync(path.join(source,'payload.txt'))
  const cases = [
    {code:'cp932',password:'日本語パスワード+!1416',label:'日文（CP932）'},
    {code:'cp936',password:'简体密码测试+!1416',label:'简体中文（CP936）'},
    {code:'cp950',password:'繁體密碼測試+!1416',label:'繁体中文（CP950）'},
  ]
  const screenshots=[], results=[]
  const click = async selector => {
    const el=typeof selector==='string' ? await waitForElement(selector) : selector
    let previous='', stable=0
    await driver.wait(async()=>{
      const state=await driver.executeScript(el=>{
        el.scrollIntoView({block:'center',behavior:'instant'})
        const r=el.getBoundingClientRect()
        return {box:JSON.stringify(r.toJSON()),hit:!el.disabled && el.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))}
      },el)
      if(!state.hit || previous!==state.box){previous=state.box;stable=Date.now();return false}
      return Date.now()-stable>=300
    },10000,`reachable control: ${typeof selector==='string'?selector:'element'}`,100)
    await el.click()
  }
  const fill = async (selector,value) => {
    const el=typeof selector==='string'?await waitForElement(selector):selector
    await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE)
    if(value)await el.sendKeys(value)
  }
  const expectVisibleText = async (selector,expected) => {
    const el=await waitForElement(selector)
    await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el)
    await driver.wait(async()=>expected.test(await el.getText()),10000,`visible text ${expected} in ${selector}`)
  }
  const select = async (selector,label) => {
    await click(selector)
    const option=await driver.wait(async()=>{
      for(const el of await driver.findElements(By.css('[role="option"]')))if((await el.getText()).includes(label))return el
      return false
    },10000,`option: ${label}`)
    await click(option)
  }
  const shot = async name => {
    await driver.wait(()=>driver.executeScript(()=>[...document.getAnimations()].filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),5000)
    const file=`${name}.png`;writeFileSync(path.join(evidence,file),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push(file)
  }
  const viewMenu = async (selector,name,width,height) => {
    await driver.manage().window().setRect({width,height});await click(selector)
    const bounds=await driver.executeScript(()=>{
      const m=document.querySelector('[role="listbox"]');return {menu:m.getBoundingClientRect().toJSON(),width:innerWidth,height:innerHeight,scroll:document.documentElement.scrollWidth,menuScroll:m.scrollWidth,menuWidth:m.clientWidth}
    })
    assert.ok(bounds.scroll<=bounds.width+1,JSON.stringify(bounds))
    assert.ok(bounds.menuScroll<=bounds.menuWidth+1,JSON.stringify(bounds))
    assert.ok(bounds.menu.left>=0 && bounds.menu.right<=bounds.width+1 && bounds.menu.top>=0 && bounds.menu.bottom<=bounds.height+1,JSON.stringify(bounds))
    await shot(`${name}-${width}x${height}`);await driver.actions().sendKeys(Key.ESCAPE).perform()
  }
  const openBrowser = async file => {
    await bridge('queueDesktopDialogSelections',[file])
    const first=await driver.findElements(By.css('[data-testid="file-manager-open-archive"]'))
    await click(first.length?first[0]:'.browser-page > header .browser-primary')
    await waitForElement('.browser-row:not(.directory)')
  }
  const preview = async () => {
    await click('.browser-row:not(.directory) .preview-trigger')
    const text=await waitForElement('[data-testid="archive-text-preview"]')
    assert.equal(await text.getAttribute('textContent'),payload.toString('utf8'))
    await click('.preview-close')
  }
  const extract = async name => {
    const out=path.join(work,name);mkdirSync(out)
    await bridge('queueDesktopDialogSelections',[out]);await click('.output-target')
    const checkbox=await waitForElement('.browser-row:not(.directory) .browser-checkbox')
    if(!(await checkbox.getAttribute('class')).split(' ').includes('checked'))await click(checkbox)
    await click('.browser-page footer .browser-primary')
    await driver.wait(()=>existsSync(path.join(out,'payload.txt')),30000,'selected entry must be published')
    assert.deepEqual(readFileSync(path.join(out,'payload.txt')),payload)
    return hash(path.join(out,'payload.txt'))
  }
  const independent = (item,archive,name) => {
    const tool=path.join(root,'test-results/zip-encoding-1416/independent-7zip',item.code,'7z.exe')
    assert.ok(existsSync(tool),'independent codepage-specific 7-Zip tool must exist')
    const out=path.join(work,name)
    const run=spawnSync(tool,['x',archive,`-p${item.password}`,`-o${out}`,'-y'],{encoding:'utf8',windowsHide:true})
    assert.equal(run.status,0,run.stdout+run.stderr)
    assert.deepEqual(readFileSync(path.join(out,'payload.txt')),payload)
    return hash(path.join(out,'payload.txt'))
  }
  await click('[data-testid="nav-ArchiveBrowser"]')
  for(const item of cases){
    console.log(`[legacy-zip-workspace-1416] browse ${item.code}`)
    const file=path.join(work,`${item.code}.zip`);copyFileSync(path.join(source,`${item.code}.zip`),file);item.file=file;item.hash=hash(file)
    await openBrowser(file)
    await select('[data-testid="browser-zip-password-encoding"]',item.label)
    await fill('.browser-toolbar input[type="password"]',item.password)
    if(item.code==='cp932'){
      for(const [w,h] of [[1100,700],[920,620],[760,520]])await viewMenu('[data-testid="browser-zip-password-encoding"]','browser-menu',w,h)
      await driver.manage().window().setRect({width:1100,height:700})
    }
    await preview()
    const outputHash=await extract(`${item.code}-selected`)
    results.push({scenario:'original-preview-selected-extraction',code:item.code,sourceHash:item.hash,outputHash})
  }
  // A parent and child deliberately use different encodings and passwords.
  const outer=path.join(work,'parent-cp932.zip'),inner=path.join(work,'inner.zip');copyFileSync(cases[2].file,inner)
  const built=spawnSync(path.join(root,'test-results/zip-encoding-1416/tools/zip-ja-JP.exe'),['-X','-j','-P',cases[0].password,outer,'inner.zip'],{cwd:work,encoding:'utf8',windowsHide:true})
  assert.equal(built.status,0,built.stdout+built.stderr)
  console.log('[legacy-zip-workspace-1416] nested archive with independent credentials')
  await openBrowser(outer);await select('[data-testid="browser-zip-password-encoding"]',cases[0].label);await fill('.browser-toolbar input[type="password"]',cases[0].password)
  await driver.actions().doubleClick(await waitForElement('.browser-row[data-entry-path="inner.zip"]')).perform()
  await waitForElement('.browser-row[data-entry-path="payload.txt"]')
  assert.equal(await (await waitForElement('.browser-toolbar input[type="password"]')).getAttribute('value'),'')
  await expectVisibleText('[data-testid="browser-zip-password-encoding"]',/自动/)
  await select('[data-testid="browser-zip-password-encoding"]',cases[2].label);await fill('.browser-toolbar input[type="password"]',cases[2].password);await preview()
  await click('[data-testid="archive-chain"] button:first-of-type')
  assert.equal(await (await waitForElement('.browser-toolbar input[type="password"]')).getAttribute('value'),cases[0].password)
  await expectVisibleText('[data-testid="browser-zip-password-encoding"]',/CP932/)
  results.push({scenario:'nested-parent-cp932-child-cp950',restoredParent:true})
  const damaged=cases.map(item=>{
    const bytes=readFileSync(item.file),file=path.join(work,`${item.code}-directory-lost.zip`)
    writeFileSync(file,bytes.subarray(0,bytes.readUInt32LE(bytes.length-22+16)))
    return {...item,file,damagedHash:hash(file)}
  })
  console.log('[legacy-zip-workspace-1416] single repair and recovered browse/queue')
  await click('[data-testid="nav-FileIntegrity"]');await click('[data-testid="archive-diagnostic-mode"]')
  await bridge('queueDesktopDialogSelections',[[damaged[0].file]]);await click('.archive-select')
  await select('[data-testid="diagnosis-zip-password-encoding"]',cases[0].label);await fill('.archive-password',cases[0].password)
  for(const [w,h] of [[1100,700],[760,520]])await viewMenu('[data-testid="diagnosis-zip-password-encoding"]','diagnosis-menu',w,h)
  await driver.manage().window().setRect({width:1100,height:700})
  await click('[data-testid="start-archive-diagnosis"]')
  const report=await waitForElement('[data-testid="diagnostic-report"]');assert.match(await report.getText(),/目录信息丢失/)
  const repaired=path.join(work,'single-repaired.zip')
  await bridge('queueDesktopDialogSelections',[repaired]);await click('[data-testid="start-archive-repair"]');await waitForElement('[data-testid="repair-result"]')
  assert.match(await (await waitForElement('[data-testid="repair-output-encryption"]')).getText(),/CP932/)
  independent(cases[0],repaired,'single-independent')
  await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement('[data-testid="repair-result"]'))
  await shot('single-repair-result')
  await select('[data-testid="diagnosis-zip-password-encoding"]',cases[2].label);await fill('.archive-password','changed-after-repair')
  await click('[data-testid="queue-recovered-archive"]')
  const queued=await driver.wait(async()=>(await bridge('extractionRecoveryTasks')).find(t=>t.source===repaired),10000)
  await click('[data-testid="browse-repaired-archive"]');await waitForElement('.browser-row[data-entry-path="payload.txt"]')
  await expectVisibleText('[data-testid="browser-zip-password-encoding"]',/CP932/);await preview()
  await click('[data-testid="nav-Decompress"]');await click('[data-testid="start-decompression-queue"]')
  const completed=await driver.wait(async()=>{const t=(await bridge('extractionRecoveryTasks')).find(t=>t.id===queued.id);return t?.status==='completed'?t:false},30000)
  const saved=await driver.wait(async()=>(await bridge('taskHistory')).find(t=>t.id===completed.id),10000)
  assert.equal(saved.executionConfig.zipPasswordEncoding,'cp932')
  const files=directory=>readdirSync(directory,{withFileTypes:true}).flatMap(entry=>entry.isDirectory()?files(path.join(directory,entry.name)):[path.join(directory,entry.name)])
  const extracted=files(work).filter(file=>file.includes('single-repaired') && file.endsWith('payload.txt'))
  assert.ok(extracted.length);for(const file of extracted)assert.deepEqual(readFileSync(file),payload)
  results.push({scenario:'single-repair-independent-read-browse-queue',code:'cp932',taskId:queued.id,repairedHash:hash(repaired)})
  console.log('[legacy-zip-workspace-1416] batch three legacy encodings')
  await click('[data-testid="nav-FileIntegrity"]');await click('[data-testid="archive-diagnostic-mode"]')
  await bridge('queueDesktopDialogSelections',[damaged.map(i=>i.file)]);await click('.archive-select')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-batch-row"]'))).length===3,15000)
  for(const item of damaged){
    const row=await driver.wait(async()=>{for(const e of await driver.findElements(By.css('[data-testid="archive-batch-row"]')))if(await e.getAttribute('data-source')===item.file)return e;return false},10000)
    await click(await row.findElement(By.css('[data-testid="archive-batch-expand"]')))
    await fill(await row.findElement(By.css('[data-testid="archive-batch-password"]')),item.password)
    await select(await row.findElement(By.css('[data-testid="recovery-row-zip-password-encoding"]')),item.label)
  }
  const firstEncoding=await waitForElement('[data-testid="recovery-row-zip-password-encoding"]')
  for(const [w,h] of [[1100,700],[760,520]])await viewMenu(firstEncoding,'recovery-row-menu',w,h)
  await click('[data-testid="batch-recovery-config"]')
  for(const [w,h] of [[1100,700],[760,520]])await viewMenu('[data-testid="recovery-batch-zip-password-encoding"]','recovery-batch-menu',w,h)
  await click('[data-testid="archive-batch-config-apply"]')
  await driver.manage().window().setRect({width:1100,height:700});await click('[data-testid="batch-diagnose"]')
  await driver.wait(async()=>{
    const s=await driver.findElements(By.css('[data-testid="archive-batch-status"]'));return s.length===3 && (await Promise.all(s.map(e=>e.getText()))).every(t=>t.includes('目录丢失'))
  },30000,'all three originals must be diagnosed as repairable directory loss')
  const batchOut=path.join(work,'batch-recovered');mkdirSync(batchOut)
  await bridge('queueDesktopDialogSelections',[batchOut]);await click('[data-testid="batch-repair"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-batch-result"]'))).length===3,60000)
  for(const item of cases){
    const directory=readdirSync(batchOut).find(name=>name.includes(item.code));assert.ok(directory)
    const full=path.join(batchOut,directory),output=readdirSync(full).find(name=>name.endsWith('.zip'));assert.ok(output)
    const archive=path.join(full,output);independent(item,archive,`${item.code}-batch-independent`)
    results.push({scenario:'batch-repair-independent-full-read',code:item.code,repairedHash:hash(archive)})
  }
  await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement('[data-testid="archive-batch-result"]'))
  await shot('batch-repair-results')
  await click('[data-testid="batch-queue-recovered"]')
  const tasks=(await bridge('extractionRecoveryTasks')).filter(t=>t.source.startsWith(batchOut))
  assert.equal(tasks.length,3)
  await click('[data-testid="browse-batch-repaired-archive"]');await waitForElement('.browser-row[data-entry-path="payload.txt"]');await preview()
  await click('[data-testid="nav-Decompress"]');await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async()=>{const all=await bridge('extractionRecoveryTasks');return tasks.every(t=>all.find(item=>item.id===t.id)?.status==='completed')},60000,'repaired batch tasks must actually complete')
  const history=await bridge('taskHistory')
  for(const item of cases){const t=tasks.find(t=>t.source.includes(item.code));assert.equal(history.find(h=>h.id===t.id)?.executionConfig.zipPasswordEncoding,item.code)}
  // Queued results intentionally preserve the original source's extraction
  // destination, not the separate directory chosen for saving repaired ZIPs.
  const batchPayloads=tasks.map(t=>path.join(t.output,path.basename(t.source,'.zip'),'payload.txt'))
  assert.equal(batchPayloads.length,3)
  for(const file of batchPayloads)assert.deepEqual(readFileSync(file),payload)
  results.push({scenario:'repaired-batch-real-queue',taskIds:tasks.map(t=>t.id),outputHashes:batchPayloads.map(hash)})
  for(const item of cases)assert.equal(hash(path.join(source,`${item.code}.zip`)),item.hash)
  for(const item of damaged)assert.equal(hash(item.file),item.damagedHash)
  writeFileSync(path.join(evidence,'results.json'),JSON.stringify({verifiedAt:new Date().toISOString(),applicationSha256:hash(application),source:'pinned, unmodified Info-ZIP originals; damaged copies remove central directory only',results,screenshots},null,2))
  console.log(`[legacy-zip-workspace-1416] real workspace workflows passed: ${evidence}`)
}

import assert from 'node:assert/strict'
import {mkdirSync,copyFileSync,readFileSync,writeFileSync,existsSync} from 'node:fs'
import {createHash,randomBytes} from 'node:crypto'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

// File-dialog responses and read-only observations only: all task creation,
// choices, execution, draft restoration and password re-entry use real controls.
export async function runLegacyZipOutput1416Gate({getDriver,restart,bridge,waitForElement,fixtureDirectory,artifactDirectory,root,application,bundledSevenZip}) {
  let driver=getDriver()
  const work=path.join(fixtureDirectory,'legacy-zip-output-1416'),evidence=path.join(artifactDirectory,'legacy-zip-output-1416')
  mkdirSync(work);mkdirSync(evidence,{recursive:true})
  const hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex')
  const source=path.join(root,'tests/fixtures/archives/legacy-zip-password-1416'),payload=readFileSync(path.join(source,'payload.txt'))
  const cases=[{code:'cp932',password:'日本語パスワード+!1416'},{code:'cp936',password:'简体密码测试+!1416'},{code:'cp950',password:'繁體密碼測試+!1416'}]
  const screenshots=[],results=[],layouts=[]
  const click=async selector=>{
    const el=typeof selector==='string'?await waitForElement(selector):selector
    let previous='',stable=0
    await driver.wait(async()=>{
      const s=await driver.executeScript(el=>{el.scrollIntoView({block:'center',behavior:'instant'});const r=el.getBoundingClientRect();return {box:JSON.stringify(r.toJSON()),hit:!el.disabled&&el.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))}},el)
      if(!s.hit||previous!==s.box){previous=s.box;stable=Date.now();return false}return Date.now()-stable>=300
    },10000,`reachable control: ${typeof selector==='string'?selector:'element'}`,100)
    await el.click()
  }
  const fill=async(selector,value)=>{const el=typeof selector==='string'?await waitForElement(selector):selector;await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(String(value))await el.sendKeys(String(value))}
  const select=async(selector,label)=>{
    await click(selector)
    const option=await driver.wait(async()=>{for(const e of await driver.findElements(By.css('[role="option"]'))){const text=await e.findElement(By.css('strong')).getAttribute('textContent');if(label==='ZIP'?text.trim()==='ZIP':text.includes(label))return e}return false},10000,`option ${label}`)
    await click(option)
  }
  const shot=async name=>{
    await driver.wait(()=>driver.executeScript(()=>[...document.getAnimations()].filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),5000)
    const geometry=await driver.executeScript(()=>({width:innerWidth,scroll:document.documentElement.scrollWidth,menus:[...document.querySelectorAll('[role="listbox"]')].map(m=>({r:m.getBoundingClientRect().toJSON(),width:m.clientWidth,scroll:m.scrollWidth})),height:innerHeight,
      splitTitles:[...document.querySelectorAll('.split-settings-copy strong')].map(el=>({height:el.getBoundingClientRect().height,fontSize:parseFloat(getComputedStyle(el).fontSize)})),
      passwordPair:['conversion-source-password','conversion-password'].map(id=>document.querySelector(`[data-testid="${id}"]`)?.getBoundingClientRect().top),
      encodingPair:['conversion-source-zip-password-encoding','conversion-target-zip-password-encoding'].map(id=>document.querySelector(`[data-testid="${id}"]`)?.getBoundingClientRect().top)}))
    assert.ok(geometry.scroll<=geometry.width+1,JSON.stringify(geometry))
    for(const m of geometry.menus)assert.ok(m.scroll<=m.width+1&&m.r.left>=0&&m.r.right<=geometry.width+1&&m.r.top>=0&&m.r.bottom<=geometry.height+1,JSON.stringify(geometry))
    for(const title of geometry.splitTitles)assert.ok(title.height<=title.fontSize*2.5,'split title must not stack vertically: '+JSON.stringify(title))
    if(geometry.width>=640)for(const pair of [geometry.passwordPair,geometry.encodingPair])if(pair.every(v=>typeof v==='number'))assert.ok(Math.abs(pair[0]-pair[1])<=2,'source and target controls must align: '+JSON.stringify(pair))
    layouts.push({name,...geometry})
    const file=name+'.png';writeFileSync(path.join(evidence,file),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push(file)
  }
  const menuSizes=async(selector,name)=>{for(const [width,height] of [[1100,700],[760,520]]){await driver.manage().window().setRect({width,height});await click(selector);await shot(`${name}-${width}x${height}`);await driver.actions().sendKeys(Key.ESCAPE).perform()}await driver.manage().window().setRect({width:1100,height:700})}
  const command=(tool,args)=>{const r=spawnSync(tool,args,{encoding:'utf8',windowsHide:true,timeout:60000});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr);return r.stdout}
  const independent=(item,archive,out,filename,expected)=>{
    const tool=path.join(root,'test-results/zip-encoding-1416/independent-7zip',item.code,'7z.exe')
    command(tool,['x',archive,`-p${item.password}`,`-o${out}`,'-y'])
    assert.deepEqual(readFileSync(path.join(out,filename)),expected)
  }
  const historyFor=output=>driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.outputPath.replaceAll('\\','/').toLowerCase()===output.replaceAll('\\','/').toLowerCase()&&['completed','failed'].includes(r.status)),60000,`terminal history: ${output}`)
  await click('[data-testid="nav-Compress"]')
  const creationPayload=randomBytes(2600000),creates=[]
  for(const [index,format] of ['zip','zipx'].entries()){
    const item=cases[index],file=path.join(work,`create-${item.code}.bin`);writeFileSync(file,creationPayload)
    creates.push({item,file,format,output:path.join(work,`created-${item.code}.${format}`)})
  }
  await bridge('queueDesktopDialogSelections',[creates.map(c=>c.file)]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="compression-draft-row"]'))).length===2,15000)
  for(const [index,c] of creates.entries()){
    console.log(`[legacy-zip-output-1416] configure ${c.format} ${c.item.code}`)
    const row=(await driver.findElements(By.css('[data-testid="compression-draft-row"]')))[index]
    if(!(await row.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
    await select('[data-testid="compression-format-select"]',c.format==='zip'?'ZIP':'ZIPX')
    await fill('[data-testid="compression-password-input"]',c.item.password)
    await select('[data-testid="zip-password-encoding"]',c.item.code.toUpperCase())
    await fill('[data-testid="compression-filename-input"]',`created-${c.item.code}`)
    await fill('[data-testid="compression-output-path"]',work)
    await click('[data-testid="compression-split-toggle"]');await fill('[data-testid="compression-split-size"]','1')
    if(c.format==='zip')await select('[data-testid="compression-zip-split-mode"]','标准 ZIP')
    if(index===0)await menuSizes('[data-testid="zip-password-encoding"]','compression-output-menu')
    await click(await row.findElement(By.css('[data-testid="compression-archive-name"]')))
  }
  await click('button[aria-label="选择全部待压缩任务"]');await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]')
  await menuSizes('[data-testid="batch-zip-password-encoding"]','compression-batch-menu')
  await click('[data-testid="batch-config-cancel"]')
  await click('[data-testid="start-compression"]')
  for(const c of creates){
    const output=c.format==='zip'?c.output:c.output+'.001',record=await historyFor(output)
    assert.equal(record.status,'completed',JSON.stringify(record));assert.equal(record.executionConfig.settings.zipPasswordEncoding,c.item.code)
    assert.ok(record.archiveOutput.files.length>1,'real source must span multiple volumes')
    independent(c.item,output,path.join(work,`created-${c.item.code}-independent`),path.basename(c.file),creationPayload)
    assert.deepEqual(readFileSync(c.file),creationPayload)
    results.push({scenario:'real-compression-volumes',format:c.format,encoding:c.item.code,taskId:record.id,volumes:record.archiveOutput.files.length,inputHash:hash(c.file)})
  }
  await click('[data-testid="open-archive-conversion"]')
  const files=cases.map(item=>{const file=path.join(work,item.code+'.zip');copyFileSync(path.join(source,item.code+'.zip'),file);return file})
  const plain=path.join(work,'plain.7z');command(bundledSevenZip,['a','-t7z',plain,creates[0].file])
  await bridge('queueDesktopDialogSelections',[[...files,plain]]);await click('[data-testid="conversion-choose-source"]')
  const rows=()=>driver.findElements(By.css('[data-testid="conversion-item"]'))
  await driver.wait(async()=>(await rows()).length===4,15000)
  await click(await (await rows())[0].findElement(By.css('[data-testid="conversion-row"]')))
  await select('[data-testid="conversion-format"]','ZIP')
  await click(await (await rows())[0].findElement(By.css('[data-testid="conversion-row"]')))
  await click('[data-testid="conversion-batch-config"]')
  await select('[data-testid="conversion-batch-source-encoding"]','CP932');await select('[data-testid="conversion-batch-target-encoding"]','CP950')
  await menuSizes('[data-testid="conversion-batch-source-encoding"]','conversion-batch-source')
  await menuSizes('[data-testid="conversion-batch-target-encoding"]','conversion-batch-target')
  await click('[data-testid="conversion-batch-apply"]')
  await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="conversion-batch-fields"]'))).length,15000)
  const snapshot=()=>driver.executeScript(()=>JSON.parse(localStorage.getItem('long-pending-archive-drafts-v1')||'null'))
  await driver.wait(async()=>{const s=await snapshot();return s?.conversion.items.length===4&&s.conversion.items[0].settings?.zipPasswordEncoding==='cp950'},10000)
  const mixed=await snapshot();assert.equal(mixed.conversion.items[3].settings,undefined);assert.equal(mixed.conversion.items[1].settings.zipPasswordEncoding,'utf8')
  await click(await (await rows())[3].findElement(By.css('[data-testid="conversion-checkbox"]')))
  const outputs=[]
  for(const [index,item] of cases.entries()){
    const target=cases[(index+2)%3],output=path.join(work,`converted-${item.code}-to-${target.code}.zip`);outputs.push({item,target,output})
    await click(await (await rows())[index].findElement(By.css('[data-testid="conversion-row"]')))
    await select('[data-testid="conversion-format"]','ZIP')
    await fill('[data-testid="conversion-source-password"]',item.password);await fill('[data-testid="conversion-password"]',target.password)
    await select('[data-testid="conversion-source-zip-password-encoding"]',item.code.toUpperCase());await select('[data-testid="conversion-target-zip-password-encoding"]',target.code.toUpperCase())
    await fill('[data-testid="conversion-output"]',output)
    if(index===0){await menuSizes('[data-testid="conversion-source-zip-password-encoding"]','conversion-inline-source');await menuSizes('[data-testid="conversion-target-zip-password-encoding"]','conversion-inline-target')}
    await click(await (await rows())[index].findElement(By.css('[data-testid="conversion-row"]')))
  }
  await driver.wait(async()=>{const s=await snapshot();return s?.conversion.items[2].settings?.zipPasswordEncoding==='cp936'&&!JSON.stringify(s).includes(cases[0].password)},10000)
  const saved=await snapshot();assert.ok(cases.every(item=>!JSON.stringify(saved).includes(item.password)))
  console.log('[legacy-zip-output-1416] restart same isolated profile with real pending drafts')
  await restart();driver=getDriver()
  assert.equal((await bridge('taskHistory')).filter(r=>r.executionConfig?.kind==='archive-conversion').length,0,'restart must not run drafts')
  await click('[data-testid="pending-draft-recovery"] button')
  await waitForElement('[data-testid="archive-conversion"]');await driver.wait(async()=>(await rows()).length===4,15000)
  for(const [index,c] of outputs.entries()){
    await click(await (await rows())[index].findElement(By.css('[data-testid="conversion-row"]')))
    for(const id of ['conversion-source-password','conversion-password'])assert.equal(await (await waitForElement(`[data-testid="${id}"]`)).getAttribute('value'),'')
    // Target ZIP encryption controls appear after the user refills its password.
    await fill('[data-testid="conversion-source-password"]',c.item.password);await fill('[data-testid="conversion-password"]',c.target.password)
    for(const [id,encoding] of [['conversion-source-zip-password-encoding',c.item.code],['conversion-target-zip-password-encoding',c.target.code]]){
      const el=await waitForElement(`[data-testid="${id}"]`);await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el)
      await driver.wait(async()=>(await el.getText()).includes(encoding.toUpperCase()),10000)
    }
    if(index===0){await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement('[data-testid="conversion-source-password"]'));await shot('conversion-restored-passwords')}
    await click(await (await rows())[index].findElement(By.css('[data-testid="conversion-row"]')))
  }
  await click('[data-testid="conversion-start"]')
  for(const c of outputs){
    const record=await historyFor(c.output);assert.equal(record.status,'completed',JSON.stringify(record))
    assert.equal(record.executionConfig.sourceZipPasswordEncoding,c.item.code);assert.equal(record.executionConfig.zipPasswordEncoding,c.target.code)
    independent(c.target,c.output,path.join(work,`${c.item.code}-converted-independent`),'payload.txt',payload)
    assert.equal(hash(files[cases.indexOf(c.item)]),hash(path.join(source,c.item.code+'.zip')))
    results.push({scenario:'restarted-cross-encoding-conversion',source:c.item.code,target:c.target.code,taskId:record.id,archiveHash:hash(c.output),payloadHash:hash(path.join(work,`${c.item.code}-converted-independent/payload.txt`))})
  }
  await shot('conversion-completed')
  writeFileSync(path.join(evidence,'results.json'),JSON.stringify({verifiedAt:new Date().toISOString(),applicationSha256:hash(application),source:'unmodified pinned Info-ZIP archives; real UI-created multi-volume ZIP/ZIPX; same-profile native restart',results,screenshots,layouts},null,2))
  console.log(`[legacy-zip-output-1416] real creation, conversion and restart passed: ${evidence}`)
}

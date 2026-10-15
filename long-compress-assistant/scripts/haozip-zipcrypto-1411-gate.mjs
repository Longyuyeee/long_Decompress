import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync, existsSync } from 'node:fs'
import { createHash, randomBytes } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

const sha = file => createHash('sha256').update(readFileSync(file)).digest('hex')
const tree = directory => readdirSync(directory, {withFileTypes:true}).flatMap(entry => {
  const file = path.join(directory,entry.name)
  return entry.isDirectory() ? tree(file).map(item=>({...item,path:entry.name+'/'+item.path}))
    : [{path:entry.name,sha256:sha(file)}]
}).sort((a,b)=>a.path.localeCompare(b.path))
const run = (tool,args,cwd) => {
  const result=spawnSync(tool,args,{cwd,windowsHide:true,timeout:120000})
  assert.ifError(result.error)
  assert.equal(result.status,0,`${tool} must finish successfully: ${result.stdout}\n${result.stderr}`)
}

// Genuine HaoZip CLI standard disks. Never rename them to .haozipNN.zip or
// count this control as evidence of the GUI's encrypted forum output.
export async function runHaozipZipcrypto1411Gate({driver,bridge,waitForElement,fixtureDirectory,artifactDirectory,bundledSevenZip,application,root:projectRoot}) {
  const root=path.join(fixtureDirectory,'好压原生密码分卷'),source=path.join(root,'原文件')
  mkdirSync(path.join(source,'中文目录','空目录'),{recursive:true})
  writeFileSync(path.join(source,'说明.txt'),'我从第二卷打开，先输错密码，再改好并解压全部资料。\n')
  writeFileSync(path.join(source,'中文目录','原文件.bin'),randomBytes(163871))
  const expected=tree(source),password='HaoZip sample + !',main=path.join(root,'收到的密码资料.zip')
  const encoder=process.env.LONG_HAOZIP_C || path.join(projectRoot,'test-results/haozip-tools/portable/HaoZipC.exe')
  const validator=process.env.LONG_BANDIZIP_C || 'E:/Bandzip/Bandizip/bz.exe'
  assert.ok(existsSync(encoder),'requires the real HaoZip CLI')
  assert.ok(existsSync(validator),'requires independent Bandizip validation')
  run(encoder,['a','-tzip','-mx=0','-v64k','-mem=ZipCrypto',`-p${password}`,'-y',main,'*'],source)
  const members=[main.replace(/\.zip$/,'.z01'),main.replace(/\.zip$/,'.z02'),main]
  members.forEach(member=>assert.ok(existsSync(member)))
  const before=members.map(member=>({name:path.basename(member),sha256:sha(member)}))
  run(validator,['t',`-p:${password}`,main])
  const independent=path.join(root,'独立Bandizip回解')
  run(validator,['x',`-p:${password}`,`-o:${independent}`,'-y',main])
  assert.deepEqual(tree(independent),expected)
  const click=async selector=>{
    const element=await waitForElement(selector)
    await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),element)
    await driver.wait(()=>driver.executeScript(el=>{const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !el.disabled&&(hit===el||el.contains(hit))},element),10000)
    await element.click()
  }
  const fill=async(selector,value)=>{const element=await waitForElement(selector);await element.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await element.sendKeys(value)}
  const screenshots=[]
  const shot=async name=>{writeFileSync(path.join(artifactDirectory,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push(name+'.png')}
  const records=[]
  for(const [index,member] of members.entries()) {
    const output=path.join(root,`第${index+1}卷实际回解`);mkdirSync(output)
    await assert.rejects(()=>bridge('extractArchive',member,output,'wrong-haozip-password'),/password|密码|口令|加密/i)
    assert.deepEqual(tree(output),[])
    await bridge('extractArchive',member,output,password);assert.deepEqual(tree(output),expected)
    const diagnosis=await bridge('diagnoseArchive',member,password)
    assert.equal(diagnosis.status,'healthy',JSON.stringify(diagnosis));assert.equal(diagnosis.integrityTested,true)
    const converted=path.join(root,`第${index+1}卷转换.zip`)
    await bridge('convertArchive',member,password,converted,{format:'zip',password:null})
    const convertedOutput=converted+'-独立回解'
    run(bundledSevenZip,['x','-y',`-o${convertedOutput}`,converted])
    assert.deepEqual(tree(convertedOutput),expected)
    records.push({member:path.basename(member),wrongPasswordRejected:true,correctPasswordExtracted:true,integrityTested:true,convertedAndIndependentlyRead:true})
  }
  await bridge('clearTasks')
  await driver.actions().keyDown(Key.CONTROL).sendKeys('b').keyUp(Key.CONTROL).perform()
  await bridge('queueDesktopDialogSelections',[members[1]])
  await click('[data-testid="file-manager-open-archive"]');await waitForElement('.browser-summary')
  assert.match(await (await waitForElement('.browser-summary')).getText(),/已加密/)
  await fill('.browser-toolbar input[type="password"]','wrong-haozip-password')
  await fill('.browser-search input','说明.txt')
  await click('.browser-row:not(.directory) .preview-trigger')
  await driver.wait(async()=>/无法预览/.test(await (await waitForElement('.preview-stage')).getText()),10000)
  assert.match(await (await waitForElement('.preview-stage')).getText(),/密码不正确，请修改密码后重试/)
  assert.equal((await driver.findElements(By.css('.preview-text'))).length,0)
  await shot('haozip-zipcrypto-1411-wrong-password')
  await click('.preview-close')
  await fill('.browser-toolbar input[type="password"]',password)
  await click('.browser-row:not(.directory) .preview-trigger');await waitForElement('.preview-text')
  assert.match(await (await waitForElement('.preview-text')).getText(),/从第二卷打开/)
  await shot('haozip-zipcrypto-1411-corrected-preview');await click('.preview-close')
  await fill('.browser-search input','')
  const uiOutput=path.join(root,'我改好了密码并选择输出');mkdirSync(uiOutput)
  await bridge('queueDesktopDialogSelections',[uiOutput]);await click('.browser-page > footer .output-target')
  const count=(await bridge('archiveBrowserTaskState')).length
  await click('.browser-page > footer .browser-primary')
  const task=await driver.wait(async()=>{const tasks=await bridge('archiveBrowserTaskState');return tasks.length>count&&['completed','failed'].includes(tasks.at(-1).status)?tasks.at(-1):false},60000)
  assert.equal(task.status,'completed',JSON.stringify(task));assert.deepEqual(tree(uiOutput),expected)
  for(const close of await driver.findElements(By.css('button[aria-label="关闭成功提示"]'))) await close.click()
  await driver.wait(async()=>(await driver.findElements(By.css('button[aria-label="关闭成功提示"]'))).length===0,10000)
  for(const [width,height] of [[1100,700],[760,520]]) {
    await driver.manage().window().setRect({width,height})
    const action=await waitForElement('.browser-page > footer .browser-primary')
    // A resize can settle after setRect resolves. Require the reachable action
    // to stay at the same geometry across successive polls before capturing.
    let previous='',stable=0
    await driver.wait(async()=>{
      const state=await driver.executeScript(el=>{el.scrollIntoView({block:'center',behavior:'instant'});const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return {reachable:r.top>=0&&r.bottom<=innerHeight+1&&(hit===el||el.contains(hit)),geometry:[r.x,r.y,r.width,r.height,innerWidth,innerHeight]}},action)
      const geometry=JSON.stringify(state.geometry)
      stable=state.reachable&&geometry===previous?stable+1:0;previous=geometry
      return stable>=2
    },10000)
    await shot(`haozip-zipcrypto-1411-completed-${width}x${height}`)
  }
  // "已经加到队列里了，第二卷后来被移走。告诉我缺哪个文件，
  // 补回原处后让我填密码继续；不要丢掉我选好的独立输出目录。"
  await driver.manage().window().setRect({width:1100,height:700})
  const retryOutput=path.join(root,'补回分卷后仍用这个目录');mkdirSync(retryOutput)
  await click('[data-testid="return-to-file-manager"]')
  await fill('[data-testid="file-manager-path-input-left"]',retryOutput)
  await (await waitForElement('[data-testid="file-manager-path-input-left"]')).sendKeys(Key.ENTER)
  await driver.wait(async()=>await (await waitForElement('[data-testid="bookmark-current-left"]')).isEnabled(),10000)
  await click('[data-testid="bookmark-current-left"]')
  await driver.wait(async()=>await (await waitForElement('[data-testid="bookmark-current-left"]')).getAttribute('aria-pressed')==='true',10000)
  await bridge('clearTasks');await click('[data-testid="nav-Decompress"]')
  await bridge('queueDesktopDialogSelections',[[members[1],members[0],members[2]]])
  await click('.decompress-view [data-testid="dropzone-file"]')
  await driver.wait(async()=>(await bridge('archiveBrowserTaskState')).length===1,10000)
  await click('[data-testid^="toggle-archive-task-"]')
  await click('[data-testid="decompression-config-panel"] [data-testid="saved-output-select"]')
  let savedOutput
  for(const item of await driver.findElements(By.css('.location-menu [role="menuitem"]'))) {
    if(await item.getAttribute('title')===retryOutput) { savedOutput=item;break }
  }
  assert.ok(savedOutput,'the independently chosen output must be available')
  await savedOutput.click()
  if(await (await waitForElement('.task-subfolder-option')).getAttribute('aria-checked')==='true') await click('.task-subfolder-option')
  assert.equal((await bridge('archiveBrowserTaskState'))[0].outputPath,retryOutput)
  renameSync(members[1],members[1]+'.暂缺')
  let missingTask
  try {
    await click('[data-testid="start-decompression-queue"]')
    missingTask=await driver.wait(async()=>{
      const task=(await bridge('archiveBrowserTaskState'))[0]
      return task?.status==='failed'?task:false
    },15000,'a volume moved after import must end with a missing-volume reason')
    assert.equal(missingTask.error,`[archive-inspection:incomplete] 缺少分卷：${path.basename(members[1])}`)
    assert.doesNotMatch(missingTask.error,/密码不正确|需要密码|wrong password|password required/i)
    assert.deepEqual(tree(retryOutput),[],'missing disks must not publish partial results')
    await waitForElement('[data-testid="failed-extraction-retry"]')
    assert.match(await (await waitForElement('[data-testid="retry-missing-volumes"]')).getText(),/\.z02/)
    const alerts=await driver.findElements(By.css('.toast-stack [role="alert"]'))
    assert.ok(alerts.length,'the execution failure must also be visible in a toast')
    for(const alert of alerts) assert.doesNotMatch(await alert.getText(),/\[archive-inspection:/)
    assert.doesNotMatch(await (await waitForElement('[data-testid="decompression-log-viewport"]')).getText(),/\[archive-inspection:/)
    for(const close of await driver.findElements(By.css('.toast-stack button[aria-label$="提示"]'))) await close.click()
    await driver.wait(async()=>(await driver.findElements(By.css('.toast-stack [role="alert"]'))).length===0,10000)
    for(const [width,height] of [[1100,700],[760,520]]) {
      await driver.manage().window().setRect({width,height})
      await click('[data-testid="retry-password"]')
      await driver.wait(()=>driver.executeScript(()=>{
        const fields=document.querySelector('[data-testid="failed-extraction-retry"]')
        const missing=document.querySelector('[data-testid="retry-missing-volumes"]')
        return fields&&missing&&fields.scrollWidth<=fields.clientWidth+1&&missing.scrollWidth<=missing.clientWidth+1
      }),10000,'the missing-volume correction controls must fit the actual pane')
      const finalReason=await waitForElement('[data-testid="decompression-log-viewport"] .task-log-entry:last-child')
      let previous='',stable=0
      await driver.wait(async()=>{
        const state=await driver.executeScript(el=>{
          el.scrollIntoView({block:'nearest',behavior:'instant'})
          const r=el.getBoundingClientRect(),p=el.closest('.log-viewport').getBoundingClientRect()
          return {visible:r.top>=Math.max(0,p.top)&&r.bottom<=Math.min(innerHeight,p.bottom)+1,geometry:[r.x,r.y,r.width,r.height,innerWidth,innerHeight]}
        },finalReason)
        const geometry=JSON.stringify(state.geometry)
        stable=state.visible&&geometry===previous?stable+1:0;previous=geometry
        return stable>=2
      },10000,'the final reason must be fully readable after ordinary scrolling and resize settling')
      await shot(`haozip-zipcrypto-1411-moved-volume-failure-${width}x${height}`)
    }
  } finally {renameSync(members[1]+'.暂缺',members[1])}
  await driver.manage().window().setRect({width:1100,height:700})
  assert.equal(await (await waitForElement('[data-testid="retry-output"]')).getAttribute('value'),retryOutput)
  await fill('[data-testid="retry-password"]',password)
  await click('[data-testid="create-retry-drafts"]')
  await driver.wait(async()=>{
    const tasks=await bridge('archiveBrowserTaskState')
    return tasks.length===2&&tasks[1].status==='pending'
  },15000,'supplying the missing volume must permit a new user-confirmed draft')
  assert.equal((await bridge('archiveBrowserTaskState'))[1].outputPath,retryOutput)
  await click('[data-testid="start-decompression-queue"]')
  const retried=await driver.wait(async()=>{
    const task=(await bridge('archiveBrowserTaskState'))[1]
    return ['completed','failed'].includes(task?.status)?task:false
  },60000)
  assert.equal(retried.status,'completed',JSON.stringify(retried));assert.deepEqual(tree(retryOutput),expected)
  assert.equal((await bridge('archiveBrowserTaskState'))[0].error,missingTask.error,'the original failure reason must remain recorded')
  for(const close of await driver.findElements(By.css('.toast-stack button[aria-label$="提示"]'))) await close.click()
  await driver.wait(async()=>(await driver.findElements(By.css('.toast-stack [role="alert"]'))).length===0,10000)
  const rows=await driver.findElements(By.css('[data-testid="task-row"]'))
  await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),rows.at(-1))
  await shot('haozip-zipcrypto-1411-supplied-volume-retried')
  members.forEach((member,index)=>assert.equal(sha(member),before[index].sha256))
  writeFileSync(path.join(artifactDirectory,'haozip-zipcrypto-1411-results.json'),JSON.stringify({application,binarySha256:sha(application),encoderSha256:sha(encoder),encoderArguments:['a','-tzip','-mx=0','-v64k','-mem=ZipCrypto','-p<test-password>','-y','<archive>','*'],independentBandizipSha256:sha(validator),sourcePayloads:expected,originalVolumes:before,records,ui:{middleMemberOpened:true,wrongPasswordPreviewRejected:true,correctedPreview:true,completeSelectedExtraction:true,minimumWindowActionReachable:true,movedVolumeFinalReason:missingTask.error,suppliedVolumeRetried:true,individualRetryOutputPreserved:true,originalFailureRetained:true,screenshots},genuineFixtureScope:'HaoZip CLI ZipCrypto native standard ZIP disks; NOT GUI-generated encrypted forum volumes'},null,2))
}

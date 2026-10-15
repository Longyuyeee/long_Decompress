import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs'
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
  members.forEach((member,index)=>assert.equal(sha(member),before[index].sha256))
  writeFileSync(path.join(artifactDirectory,'haozip-zipcrypto-1411-results.json'),JSON.stringify({application,binarySha256:sha(application),encoderSha256:sha(encoder),encoderArguments:['a','-tzip','-mx=0','-v64k','-mem=ZipCrypto','-p<test-password>','-y','<archive>','*'],independentBandizipSha256:sha(validator),sourcePayloads:expected,originalVolumes:before,records,ui:{middleMemberOpened:true,wrongPasswordPreviewRejected:true,correctedPreview:true,completeSelectedExtraction:true,minimumWindowActionReachable:true,screenshots},genuineFixtureScope:'HaoZip CLI ZipCrypto native standard ZIP disks; NOT GUI-generated encrypted forum volumes'},null,2))
}

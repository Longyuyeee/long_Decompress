import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync, copyFileSync } from 'node:fs'
import { createHash, randomBytes } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

const sha = bytes => createHash('sha256').update(bytes).digest('hex')
const tree = root => readdirSync(root, {withFileTypes:true}).flatMap(entry => {
  const file = path.join(root, entry.name)
  return entry.isDirectory() ? tree(file).map(item => ({...item, path:entry.name+'/'+item.path}))
    : [{path:entry.name, sha256:sha(readFileSync(file))}]
}).sort((a,b)=>a.path.localeCompare(b.path))
const run = (tool,args,cwd) => {
  const result = spawnSync(tool,args,{cwd,windowsHide:true,encoding:'utf8',timeout:120000})
  assert.ifError(result.error); assert.equal(result.status,0,`${tool}: ${result.stdout}\n${result.stderr}`)
  return result.stdout
}

export function makeSfxFixtures(root, projectRoot, sevenZip) {
  mkdirSync(root,{recursive:true})
  const source=path.join(root,'原文件');mkdirSync(source)
  mkdirSync(path.join(source,'中文目录'))
  writeFileSync(path.join(source,'说明.txt'),'收到自解压包，不运行它，先看看文件再解压。\n')
  writeFileSync(path.join(source,'中文目录','数据.bin'),randomBytes(160*1024+27))
  const winrar=process.env.LONG_SFX_WINRAR || path.join(projectRoot,'test-results/archive-upgrade-tools/winrar/WinRAR.exe')
  const rar=process.env.LONG_SFX_RAR || 'E:/Winrar/Rar.exe'
  const module=process.env.LONG_SFX_7Z_MODULE || 'E:/7z/7-Zip/7z.sfx'
  for(const tool of [winrar,rar,module]) assert.ok(existsSync(tool),`Missing independent SFX fixture tool: ${tool}`)
  const cases=[]
  for(const mode of ['plain','zipcrypto','aes']) {
    const file=path.join(root,`ZIP-${mode}.exe`),password=mode==='plain'?null:'SFX ZIP + !'
    run(winrar,['a','-afzip','-sfx','-m0','-r','-ibck','-inul','-cfg-',...(password?[`-p${password}`]:[]),...(mode==='zipcrypto'?['-mezl']:[]),file,'.\\*'],source)
    cases.push({file,password,format:'ZIP',mode})
  }
  for(const mode of ['plain','contents','headers']) {
    const file=path.join(root,`7Z-${mode}.exe`),password=mode==='plain'?null:'SFX 7Z + !'
    run(sevenZip,['a',`-sfx${module}`,'-mx=1',...(password?[`-p${password}`,`-mhe=${mode==='headers'?'on':'off'}`]:[]),file,'.\\*'],source)
    cases.push({file,password,format:'7Z',mode})
  }
  for(const encryption of ['plain','contents','headers']) {
    const file=path.join(root,`RAR-${encryption}.exe`),password=encryption==='plain'?null:'SFX RAR + !'
    run(rar,['a','-sfx','-m0','-r','-idq','-cfg-',...(password?[`${encryption==='headers'?'-hp':'-p'}${password}`]:[]),file,'.\\*'],source)
    cases.push({file,password,format:'RAR',mode:encryption})
  }
  for(const item of cases) {
    assert.equal(readFileSync(item.file).subarray(0,2).toString(),'MZ')
    const independent=path.join(root,`${path.basename(item.file)}-外部回解`)
    run(sevenZip,['x','-y',...(item.password?[`-p${item.password}`]:[]),`-o${independent}`,item.file])
    assert.deepEqual(tree(independent),tree(source),'independent SFX read must match original payload')
  }
  return {source,cases}
}

export async function runSfx1411Gate({driver,bridge,waitForElement,fixtureDirectory,artifactDirectory,bundledSevenZip,application,root:projectRoot}) {
  const root=path.join(fixtureDirectory,'sfx-1411')
  const {source,cases}=makeSfxFixtures(root,projectRoot,bundledSevenZip)
  const results=[],screenshots=[]
  const browseResult=async(file,password)=>driver.executeAsyncScript((file,password,done)=>{
    const ids=crypto.getRandomValues(new Uint32Array(2))
    const cleanup=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id))
    window['_'+ids[0]]=value=>{cleanup();done({ok:true,value})}
    window['_'+ids[1]]=error=>{cleanup();done({ok:false,error:String(error)})}
    window.__TAURI_IPC__({cmd:'browse_archive',filePath:file,password,browseId:crypto.randomUUID(),callback:ids[0],error:ids[1]})
  },file,password)
  const click=async selector=>{
    const element=await waitForElement(selector)
    await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),element)
    await driver.wait(()=>driver.executeScript(el=>{const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !el.disabled&&(hit===el||el.contains(hit))},element),10000)
    await element.click()
  }
  const fill=async(selector,value)=>{const el=await waitForElement(selector);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value)}
  const shot=async name=>{writeFileSync(path.join(artifactDirectory,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push(name+'.png')}
  await bridge('clearTasks')
  for(const item of cases) {
    const before=sha(readFileSync(item.file)),output=path.join(root,path.basename(item.file)+'-应用回解');mkdirSync(output)
    if(item.password) {
      await assert.rejects(()=>bridge('extractArchive',item.file,output,'wrong-sfx-password'),/password|密码|口令|加密/i)
      assert.deepEqual(tree(output),[])
    }
    if(item.mode==='headers') {
      const diagnostic=await browseResult(item.file,'wrong-sfx-password')
      console.log(`SFX ${item.format} wrong header password: ${JSON.stringify(diagnostic)}`)
      assert.equal(diagnostic.ok,false)
      assert.match(diagnostic.error,/^ARCHIVE_BROWSE_PASSWORD\|/,'a known wrong header password must not be reported as archive corruption')
    }
    const extracted=await bridge('extractArchive',item.file,output,item.password)
    assert.ok(extracted);assert.deepEqual(tree(output),tree(source))
    const diagnosis=await bridge('diagnoseArchive',item.file,item.password)
    assert.equal(diagnosis.actualFormat,item.format,JSON.stringify(diagnosis))
    assert.equal(diagnosis.status,'healthy',JSON.stringify(diagnosis))
    assert.equal(diagnosis.integrityTested,true)
    const converted=path.join(root,path.basename(item.file)+'.zip')
    await bridge('convertArchive',item.file,item.password,converted,{format:'zip',password:null})
    const convertedOutput=converted+'-外部回解'
    run(bundledSevenZip,['x','-y',`-o${convertedOutput}`,converted]);assert.deepEqual(tree(convertedOutput),tree(source))
    assert.equal(sha(readFileSync(item.file)),before)
    results.push({format:item.format,mode:item.mode,file:path.basename(item.file),sourceSha256:before,wrongPasswordRejected:!!item.password,extracted:true,diagnosedHealthy:true,converted:true,originalUnchanged:true})
  }
  writeFileSync(path.join(artifactDirectory,'sfx-1411-operations.json'),JSON.stringify({application,sha256:sha(readFileSync(application)),cases:results},null,2))
  // Real UI: open the received ZIP SFX, preview text, select all and extract.
  await driver.actions().keyDown(Key.CONTROL).sendKeys('b').keyUp(Key.CONTROL).perform()
  await waitForElement('[data-testid="file-manager-open-archive"]')
  const file=cases[0].file,output=path.join(root,'我选择的目录');mkdirSync(output)
  await bridge('queueDesktopDialogSelections',[file,output])
  await click('[data-testid="file-manager-open-archive"]')
  await waitForElement('.browser-summary')
  assert.match(await (await waitForElement('.browser-summary')).getText(),/ZIP/)
  await fill('.browser-search input','说明.txt')
  await click('.browser-row:not(.directory) .preview-trigger')
  assert.match(await (await waitForElement('.preview-text')).getText(),/不运行它/)
  await shot('sfx-1411-text-preview');await click('.preview-close');await fill('.browser-search input','')
  await click('.browser-page > footer .output-target')
  const count=(await bridge('archiveBrowserTaskState')).length
  await click('.browser-page > footer .browser-primary')
  const task=await driver.wait(async()=>{const tasks=await bridge('archiveBrowserTaskState');return tasks.length>count&&['completed','failed'].includes(tasks.at(-1).status)?tasks.at(-1):false},60000)
  assert.equal(task.status,'completed',JSON.stringify(task));assert.deepEqual(tree(output),tree(source))
  for(const [width,height] of [[1100,700],[760,520]]){
    await driver.manage().window().setRect({width,height});await shot(`sfx-1411-browser-${width}x${height}`)
    if(width===760){
      const action=await waitForElement('.browser-page > footer .browser-primary')
      await driver.wait(async()=>driver.executeScript(el=>{
        el.scrollIntoView({block:'center',behavior:'instant'})
        const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)
        return r.top>=0&&r.bottom<=innerHeight+1&&(hit===el||el.contains(hit))
      },action),10000,'SFX extraction action must remain reachable at the minimum window size')
      await shot('sfx-1411-browser-760x520-footer')
    }
  }
  await driver.manage().window().setRect({width:1100,height:700})
  for(const format of ['7Z','RAR']){
    const item=cases.find(item=>item.format===format&&item.mode==='plain')
    await bridge('queueDesktopDialogSelections',[item.file])
    await click('.browser-page > header .browser-primary')
    await driver.wait(async()=>{
      const picker=await driver.findElement(By.css('.browser-toolbar .browser-field button'))
      return (await picker.getAttribute('title'))===item.file && (await driver.findElements(By.css('.browser-summary'))).length===1
        && (await driver.findElement(By.css('.browser-summary')).getText()).includes(format)
    },30000)
    await shot(`sfx-1411-${format}-browse`)
  }
  for(const format of ['7Z','RAR']){
    const item=cases.find(item=>item.format===format&&item.mode==='headers')
    await fill('.browser-toolbar input[type="password"]','wrong-sfx-password')
    await bridge('queueDesktopDialogSelections',[item.file])
    await click('.browser-page > header .browser-primary')
    await waitForElement('.browser-empty')
    assert.equal((await driver.findElements(By.css('.browser-summary'))).length,0,'wrong header password must not expose an empty successful listing')
    await shot(`sfx-1411-${format}-wrong-header-password`)
    await fill('.browser-toolbar input[type="password"]',item.password)
    await click('.browser-toolbar button[title="重新读取"]')
    await waitForElement('.browser-summary')
    assert.match(await (await waitForElement('.browser-summary')).getText(),/已加密/)
    const target=path.join(root,`${format}-我改好了密码`);mkdirSync(target)
    await bridge('queueDesktopDialogSelections',[target]);await click('.browser-page > footer .output-target')
    const before=(await bridge('archiveBrowserTaskState')).length
    await click('.browser-page > footer .browser-primary')
    const task=await driver.wait(async()=>{const tasks=await bridge('archiveBrowserTaskState');return tasks.length>before&&['completed','failed'].includes(tasks.at(-1).status)?tasks.at(-1):false},60000)
    assert.equal(task.status,'completed',JSON.stringify(task));assert.deepEqual(tree(target),tree(source))
    await shot(`sfx-1411-${format}-corrected-and-extracted`)
  }
  // An ordinary program must fail inspection, not open a PE resource listing.
  const ordinary=path.join(root,'普通程序.exe');copyFileSync(bundledSevenZip,ordinary)
  await assert.rejects(()=>bridge('extractArchive',ordinary,path.join(root,'不能解压的程序'),null),/不是可读取|自解压归档/)
  await bridge('clearTasks')
  await click('[data-testid="nav-Decompress"]')
  await bridge('queueDesktopDialogSelections',[[cases[0].file,ordinary]])
  await click('[data-testid="dropzone-file"]')
  await driver.wait(async()=>(await bridge('archiveBrowserTaskState')).length===1,10000)
  const importText=await driver.findElement(By.css('body')).getText()
  assert.match(importText,/ZIP-plain.exe/)
  assert.match(importText,/不是可读取/)
  const imported=await bridge('archiveBrowserTaskState')
  assert.equal(imported[0].status,'pending')
  await shot('sfx-1411-decompression-import')
  const record={application,sha256:sha(readFileSync(application)),cases:results,ui:{preview:true,selectedExtraction:true,sevenZipAndRarHeaderPasswordCorrection:true,minimumWindowActionReachable:true,verifiedSfxImportOnly:true,screenshots},ordinaryExeRejected:true,fixtureMethod:'WinRAR/7-Zip/RAR real SFX creation; original source and independent extraction byte hashes'}
  writeFileSync(path.join(artifactDirectory,'sfx-1411-results.json'),JSON.stringify(record,null,2))
}

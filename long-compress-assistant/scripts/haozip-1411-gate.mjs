import assert from 'node:assert/strict'
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
const expectedPayload = '151a4929baf7f7d0e0626c3d865e26882ee45de330d868b6f26422df0c505900'
const base = '图解计算机基础（小林）'

// Download the two original attachments separately, never synthesize or edit
// their contents. Filename labels and source URLs are recorded in the audit.
export async function runHaozip1411Gate({driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application, root:projectRoot}) {
  const original = process.env.LONG_HAOZIP_FIXTURES || path.join(projectRoot, 'test-results/haozip-public-fixtures')
  const root = path.join(fixtureDirectory, '好压论坛收到的资料'); mkdirSync(root, {recursive:true})
  const output=path.join(root,'我指定的独立目录');mkdirSync(output)
  const globalOutput=path.join(root,'另一个全局目录');mkdirSync(globalOutput)
  const sourceHashes = ['8be787946b66587ad7ac9529d6780cdeee92bb516ad4f645c3e6aae7f2411117','ecc96ac9fb045bc779442ca1f67578b87967db455f2ab0a97898ef5ff728cde0']
  const members = [1,2].map((index) => {
    const name = `${base}.haozip0${index}.zip`
    const source = existsSync(path.join(original,name)) ? path.join(original,name) : path.join(original,`haozip0${index}.zip`)
    assert.equal(hash(source),sourceHashes[index-1],'must use the unmodified public original')
    const destination = path.join(root,name);copyFileSync(source,destination);return destination
  })
  const screenshots = [], records = []
  const invokeResult = (cmd,args) => driver.executeAsyncScript((cmd,args,done)=>{
    const ids=crypto.getRandomValues(new Uint32Array(2));const cleanup=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id))
    window['_'+ids[0]]=value=>{cleanup();done({ok:true,value})}
    window['_'+ids[1]]=error=>{cleanup();done({ok:false,error:String(error)})}
    window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]})
  },cmd,args)
  const invoke = async(cmd,args)=>{const result=await invokeResult(cmd,args);assert.ok(result.ok,result.error);return result.value}
  const click = async selector => {
    const el=await waitForElement(selector)
    await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el)
    await driver.wait(()=>driver.executeScript(el=>{const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !el.disabled&&(hit===el||el.contains(hit))},el),10000,`Control must be reachable: ${selector}`)
    await el.click()
  }
  const savedLocation=async(selector,directory)=>{
    await click(selector)
    const entries=await driver.findElements(By.css('.location-menu [role="menuitem"]'))
    let selected
    for(const entry of entries) if(await entry.getAttribute('title')===directory){selected=entry;break}
    assert.ok(selected,'the directory bookmarked through the file manager must be available')
    await selected.click()
  }
  const shot=async name=>{
    for(const toast of await driver.findElements(By.css('button[aria-label="关闭成功提示"]'))) await toast.click()
    writeFileSync(path.join(artifactDirectory,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push(name+'.png')
  }
  const verifyOutput = directory => {
    const files=readdirSync(directory,{withFileTypes:true})
    assert.equal(files.length,1,'the outer archive contains exactly one EPUB, not its embedded entries')
    assert.equal(files[0].name,base+'.epub');assert.equal(hash(path.join(directory,base+'.epub')),expectedPayload)
  }
  await bridge('clearTasks')
  for(const member of members) {
    console.log('[haozip-1411] native read/extract/diagnose/convert: '+path.basename(member))
    const group=await invoke('detect_split_archive',{path:member})
    assert.equal(group.is_complete,true);assert.equal(group.total_parts,2);assert.equal(group.first_part,members[0])
    const browse=await invoke('browse_archive',{filePath:member,password:null,browseId:crypto.randomUUID()})
    assert.equal(browse.totalFiles,1);assert.equal(browse.encrypted,false)
    const output=path.join(root,path.basename(member)+'-应用回解')
    await bridge('extractArchive',member,output,null);verifyOutput(output)
    const diagnosis=await bridge('diagnoseArchive',member,null)
    assert.equal(diagnosis.status,'healthy',JSON.stringify(diagnosis));assert.equal(diagnosis.integrityTested,true)
    const converted=path.join(root,path.basename(member)+'-转换.zip')
    await bridge('convertArchive',member,null,converted,{format:'zip'})
    const independent=converted+'-独立回解'
    const result=spawnSync(bundledSevenZip,['x','-y',`-o${independent}`,converted],{windowsHide:true,encoding:'utf8',timeout:120000})
    assert.ifError(result.error);assert.equal(result.status,0,result.stdout+result.stderr);verifyOutput(independent)
    records.push({member:path.basename(member),grouped:true,outerFiles:browse.totalFiles,extracted:true,diagnosedHealthy:true,convertedAndIndependentlyRead:true})
  }
  // "只收到文件名相似的两个卷，我先从第二个打开看看。"
  await driver.actions().keyDown(Key.CONTROL).sendKeys('b').keyUp(Key.CONTROL).perform()
  // Bookmark real directories through the existing file manager. The queue
  // picker substitution is not supported by the legacy native directory buttons.
  for(const directory of [output,globalOutput]) {
    const input=await waitForElement('[data-testid="file-manager-path-input-left"]')
    await input.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE,directory,Key.ENTER)
    await driver.wait(async()=>await input.getAttribute('value')===directory
      && await (await waitForElement('[data-testid="bookmark-current-left"]')).isEnabled(),10000)
    await click('[data-testid="bookmark-current-left"]')
    await driver.wait(async()=>await (await waitForElement('[data-testid="bookmark-current-left"]')).getAttribute('aria-pressed')==='true',10000)
  }
  await bridge('queueDesktopDialogSelections',[members[1]])
  await click('[data-testid="file-manager-open-archive"]')
  await waitForElement('.browser-summary')
  await waitForElement(`[data-entry-path="${base}.epub"]`)
  assert.equal((await driver.findElements(By.css('.browser-row:not(.directory)'))).length,1)
  const browserOutput=path.join(root,'我从末卷打开并解压');mkdirSync(browserOutput)
  await bridge('queueDesktopDialogSelections',[browserOutput]);await click('.browser-page > footer .output-target')
  const count=(await bridge('archiveBrowserTaskState')).length
  await click('.browser-page > footer .browser-primary')
  const task=await driver.wait(async()=>{const tasks=await bridge('archiveBrowserTaskState');return tasks.length>count&&['completed','failed'].includes(tasks.at(-1).status)?tasks.at(-1):false},60000)
  assert.equal(task.status,'completed',JSON.stringify(task));verifyOutput(browserOutput)
  await shot('haozip-1411-last-member-browser')

  // "两个都选进去，只要一个任务；输出目录和同名文件夹由我决定。"
  await bridge('clearTasks');await click('[data-testid="nav-Decompress"]')
  await bridge('queueDesktopDialogSelections',[[members[1],members[0],members[1]]]);await click('.decompress-view [data-testid="dropzone-file"]')
  await driver.wait(async()=>(await bridge('archiveBrowserTaskState')).length===1,10000)
  assert.match(await driver.findElement(By.css('body')).getText(),/2 个分卷/)
  await click('[data-testid^="toggle-archive-task-"]')
  console.log('[haozip-1411] real UI: unordered import, individual output and global changes')
  await savedLocation('[data-testid="decompression-config-panel"] [data-testid="saved-output-select"]',output)
  const subfolder=await waitForElement('.task-subfolder-option')
  if(await subfolder.getAttribute('aria-checked')!=='true') await click('.task-subfolder-option')
  await savedLocation('.global-config-controls [data-testid="saved-output-select"]',globalOutput)
  await driver.wait(async()=> (await (await waitForElement('[data-testid="extraction-output-path"]')).getAttribute('title'))===path.join(output,base),10000)
  assert.equal((await bridge('archiveBrowserTaskState'))[0].outputPath,output)
  for(const [width,height] of [[1100,700],[760,520]]) {
    await driver.manage().window().setRect({width,height})
    await driver.wait(()=>driver.executeScript(()=>{const el=document.querySelector('[data-testid="decompression-task-details"]');return el&&el.scrollWidth<=el.clientWidth+1}),10000)
    await shot(`haozip-1411-decompression-${width}x${height}`)
  }
  await driver.manage().window().setRect({width:1100,height:700})
  await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async()=>['completed','failed'].includes((await bridge('archiveBrowserTaskState'))[0]?.status),60000)
  assert.equal((await bridge('archiveBrowserTaskState'))[0].status,'completed');verifyOutput(path.join(output,base))
  assert.equal(readdirSync(globalOutput).length,0,'global changes must not rewrite an individual task')
  await shot('haozip-1411-queue-completed')

  console.log('[haozip-1411] real UI: conversion dedupe and retained default target')
  await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
  await (await driver.findElement(By.xpath('//button[contains(., "默认配置")]'))).click()
  await click('[data-testid="conversion-batch-fields"] [data-testid="conversion-format"]')
  await (await driver.findElement(By.xpath('//button[@role="option" and .//strong[contains(text(), "TAR.BR")]]'))).click()
  await click('[data-testid="conversion-batch-apply"]')
  await bridge('queueDesktopDialogSelections',[[members[1],members[0]]]);await click('[data-testid="conversion-choose-source"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="conversion-item"]'))).length===1,10000)
  const conversionRow=await waitForElement('[data-testid="conversion-row"]')
  if(await conversionRow.getAttribute('aria-expanded')!=='true') await click('[data-testid="conversion-row"]')
  assert.match(await (await waitForElement('[data-testid="conversion-item"] [data-testid="conversion-format"]')).getText(),/TAR.BR/)
  assert.doesNotMatch(await (await waitForElement('[data-testid="conversion-output"]')).getAttribute('value'),/haozip\d+_转换/)
  await click('[data-testid="conversion-item"] [data-testid="conversion-format"]')
  await (await driver.findElement(By.xpath('//button[@role="option" and .//strong[text()="ZIP"]]'))).click()
  const converted=path.join(root,'我合并的完整资料.zip')
  const field=await waitForElement('[data-testid="conversion-output"]')
  await field.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE,converted)
  for(const [width,height] of [[1100,700],[760,520]]) {
    await driver.manage().window().setRect({width,height})
    await driver.wait(()=>driver.executeScript(()=>{const el=document.querySelector('[data-testid="archive-conversion"]');return el&&el.scrollWidth<=el.clientWidth+1}),10000)
    await shot(`haozip-1411-conversion-${width}x${height}`)
  }
  await driver.manage().window().setRect({width:1100,height:700})
  await click('[data-testid="conversion-start"]');await waitForElement('[data-testid="conversion-result"]')
  const restored=converted+'-独立回解'
  const independent=spawnSync(bundledSevenZip,['x','-y',`-o${restored}`,converted],{windowsHide:true,encoding:'utf8',timeout:120000})
  assert.ifError(independent.error);assert.equal(independent.status,0,independent.stdout+independent.stderr);verifyOutput(restored)
  await shot('haozip-1411-conversion-completed')
  await click('[role="dialog"] button[aria-label="Close dialog"]');await click('[data-testid="nav-Decompress"]')

  // "缺首卷/缺终卷时告诉我缺的是什么，不要反复让我试密码。"
  const missing=[]
  for(const index of [0,1]) {
    console.log('[haozip-1411] real UI: missing member '+(index+1))
    await bridge('clearTasks');renameSync(members[index],members[index]+'.暂缺')
    try {
      await bridge('queueDesktopDialogSelections',[[members[1-index]]]);await click('.decompress-view [data-testid="dropzone-file"]')
      const expected=index===0?path.basename(members[0]):'终卷目录'
      await driver.wait(async()=>(await driver.findElement(By.css('body')).getText()).includes(expected),10000,`Missing-volume import must show ${expected}`)
      for(const alert of await driver.findElements(By.css('.toast-stack [role="alert"]'))) assert.doesNotMatch(await alert.getText(),/\[archive-inspection:/)
      assert.equal((await bridge('archiveBrowserTaskState')).length,0)
      const raw=await invokeResult('browse_archive',{filePath:members[1-index],password:null,browseId:crypto.randomUUID()})
      assert.equal(raw.ok,false);assert.match(raw.error,/^ARCHIVE_BROWSE_INCOMPLETE\|/)
      assert.ok(raw.error.split('|')[1].includes(expected),'the UI-visible segment must retain the actual missing-volume reason')
      await shot(index===0?'haozip-1411-missing-first':'haozip-1411-missing-terminal')
      for(const toast of await driver.findElements(By.css('button[aria-label="关闭错误提示"]'))) await toast.click()
      await driver.wait(async()=>(await driver.findElements(By.css('.toast-stack [role="alert"]'))).length===0,10000)
      await click('[data-testid="nav-ArchiveBrowser"]')
      await bridge('queueDesktopDialogSelections',[members[1-index]])
      await click((await driver.findElements(By.css('[data-testid="file-manager-open-archive"]'))).length
        ? '[data-testid="file-manager-open-archive"]' : '.browser-page > header .browser-primary')
      await waitForElement('.browser-empty')
      await driver.wait(async()=>{const alerts=await driver.findElements(By.css('.toast-stack [role="alert"]'));for(const alert of alerts) if((await alert.getText()).includes(expected)) return true;return false},10000,
        `Archive browser must visibly explain ${expected}`)
      await driver.wait(()=>driver.executeScript(()=>{const items=[...document.querySelectorAll('.toast-stack [role="alert"] p')];return items.length>0
        && items.every(el=>el.scrollWidth<=el.clientWidth+1)}),10000,'missing-volume reasons must wrap rather than be clipped')
      await shot(index===0?'haozip-1411-browser-missing-first':'haozip-1411-browser-missing-terminal')
      if(index===0) {
        await driver.manage().window().setRect({width:760,height:520})
        await driver.wait(()=>driver.executeScript(()=>{const items=[...document.querySelectorAll('.toast-stack [role="alert"] p')];return items.length>0
          && items.every(el=>el.scrollWidth<=el.clientWidth+1)}),10000)
        await shot('haozip-1411-browser-missing-first-760x520')
        await driver.manage().window().setRect({width:1100,height:700})
      }
      await click('[data-testid="nav-Decompress"]')
      missing.push({missing:path.basename(members[index]),queued:false,passwordNotRequested:true,reason:expected})
    } finally {renameSync(members[index]+'.暂缺',members[index])}
  }
  for(const toast of await driver.findElements(By.css('button[aria-label="关闭错误提示"]'))) await toast.click()
  await bridge('queueDesktopDialogSelections',[[members[1],members[0]]]);await click('.decompress-view [data-testid="dropzone-file"]')
  await driver.wait(async()=>(await bridge('archiveBrowserTaskState')).length===1,10000)
  assert.equal((await bridge('archiveBrowserTaskState'))[0].status,'pending')
  await shot('haozip-1411-supplied-volumes-reimported')
  members.forEach((member,index)=>assert.equal(hash(member),sourceHashes[index]))
  writeFileSync(path.join(artifactDirectory,'haozip-1411-results.json'),JSON.stringify({application,binarySha256:hash(application),source:'https://github.com/xiaolincoder/CS-Base/issues/210',sourceHashes,expectedPayloadSha256:expectedPayload,records,missing,ui:{lastMemberOpenAndExtract:true,unorderedDedupe:true,individualOutputPreserved:true,subfolderStem:true,minimumWindowNoDetailOverflow:true,conversionDedupeAndDefaultsPreserved:true,conversionCompleted:true,missingReasonVisibleInBrowser:true,reimportAfterSupplyingVolumes:true,screenshots},genuineFixtureScope:'one public unencrypted HaoZip forum ZIP set; encrypted/ZIP64 layout regressions are separately synthetic, not genuine HaoZip output'},null,2))
}

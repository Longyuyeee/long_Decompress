import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { readFileSync, writeFileSync, mkdirSync, readdirSync, existsSync, copyFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runZpaq150Gate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, application, root, bundledSevenZip }) {
  const original = path.join(root, 'src-tauri/tests/fixtures/zpaq')
  const manifest = JSON.parse(readFileSync(path.join(original, 'manifest.json'), 'utf8'))
  const sha = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const tree = dir => !existsSync(dir) ? [] : readdirSync(dir, { withFileTypes: true }).flatMap(e => e.isDirectory()
    ? tree(path.join(dir, e.name)).map(v => ({...v, path: e.name + '/' + v.path})) : [{path:e.name, sha256:sha(path.join(dir,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
  const expected = Object.entries(manifest.payload).map(([name,sha256])=>({path:'项目 文档/'+name,sha256})).sort((a,b)=>a.path.localeCompare(b.path))
  const work = path.join(fixtureDirectory, 'ZPAQ用户读取'); mkdirSync(work)
  const cases=[], screenshots=[], layouts=[]
  const invoke = (cmd,args) => driver.executeAsyncScript(`const [cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
  const ipc = async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
  const click = async selector => {const el=typeof selector==='string'?await waitForElement(selector):selector;await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el);await driver.wait(()=>driver.executeScript(el=>{const r=el.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !el.disabled&&(h===el||el.contains(h))},el),10000);await el.click()}
  const fill = async(selector,value)=>{const el=await waitForElement(selector);await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value) await el.sendKeys(value)}
  const sizes = async(name,anchor)=>{
    for(const [width,height] of [[1100,700],[760,520]]) {
      await driver.manage().window().setRect({width,height});await driver.wait(()=>driver.executeScript(w=>Math.abs(innerWidth-w)<=2,width),10000)
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      if(name==='zpaq-current-help' && !(await driver.findElements(By.css('[role="note"]'))).length) await click('[aria-label="ZPAQ 当前文件视图"]')
      await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement(anchor));await driver.sleep(200)
      const layout=await driver.executeScript(()=>({width:innerWidth,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('.browser-page,[role="note"],[role="dialog"]')].map(el=>({overflow:el.scrollWidth-el.clientWidth,rect:{left:el.getBoundingClientRect().left,right:el.getBoundingClientRect().right}}))}))
      assert.ok(layout.overflow<=1,JSON.stringify(layout));layout.panels.forEach(p=>assert.ok(p.overflow<=2,JSON.stringify(layout)))
      layouts.push({name,...layout});const file=`${name}-${width}x${height}.png`;screenshots.push(file);writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'))
    }
  }
  for(const [name,hash] of Object.entries(manifest.hashes)) {
    const source=path.join(original,name),password=name.includes('-key')?manifest.password:null;assert.equal(sha(source),hash)
    const directory=await ipc('browse_archive',{filePath:source,password});assert.equal(directory.format,'ZPAQ');assert.equal(directory.totalFiles,3)
    assert.ok(!directory.entries.some(e=>e.path.includes('已删除')||/^\d+\//.test(e.path)))
    await ipc('test_archive_integrity',{filePath:source,password})
    const report=await bridge('diagnoseArchive',source,password);assert.equal(report.status,'healthy');assert.equal(report.integrityTested,true)
    const output=path.join(work,name+'-完整解压');await bridge('extractArchive',source,output,password);assert.deepEqual(tree(output),expected)
    if(password) {const r=await invoke('test_archive_integrity',{filePath:source,password:'wrong'});assert.equal(r.ok,false);assert.match(r.error,/密码不匹配或文件头已损坏/)}
    const converted=path.join(work,name+'-转换.zip');await bridge('convertArchive',source,password,converted,{format:'zip',password:null})
    const r=spawnSync(bundledSevenZip,['x','-y','-o'+converted+'-独立回解',converted],{windowsHide:true,encoding:'utf8',timeout:30000});assert.equal(r.status,0,r.stdout+r.stderr);assert.deepEqual(tree(converted+'-独立回解'),expected)
    assert.equal(sha(source),hash);cases.push({name,currentFiles:3,wholeExtraction:true,integrity:true,convertedZipIndependentlyDecoded:true,originalUnchanged:true})
  }
  // Real browser entry, not a mocked component mount.
  await click('[data-testid="nav-ArchiveBrowser"]')
  await fill('[data-testid="file-manager-path-input-left"]',original);await (await waitForElement('[data-testid="file-manager-path-input-left"]')).sendKeys(Key.ENTER)
  const source=path.join(original,'journal.zpaq')
  const row=await driver.wait(async()=>{for(const row of await driver.findElements(By.css('.file-row'))) if(await row.getAttribute('data-path')===source)return row;return false},10000)
  assert.equal((await row.findElements(By.css('.pi-box'))).length,1);await driver.actions().doubleClick(row).perform()
  await waitForElement('[data-testid="zpaq-current-view"]');await sizes('zpaq-current-files','.browser-summary')
  await click('[aria-label="ZPAQ 当前文件视图"]');await waitForElement('[role="note"]');await sizes('zpaq-current-help','.browser-summary')
  await driver.actions().sendKeys(Key.ESCAPE).perform()
  // Enter folder, select one actual file, choose destination, extract through UI.
  const folder=await waitForElement('[data-entry-path="项目 文档/"]');await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),folder);await driver.actions().doubleClick(folder).perform()
  await waitForElement('[data-entry-path="项目 文档/说明.txt"]');await click('[data-entry-path="项目 文档/说明.txt"] .browser-checkbox')
  const selectedOutput=path.join(work,'用户只要说明文件');mkdirSync(selectedOutput)
  await bridge('queueDesktopDialogSelections',[selectedOutput]);await click('.output-target');await sizes('zpaq-selected-extraction','footer .browser-primary')
  await click('footer .browser-primary');await driver.wait(()=>existsSync(path.join(selectedOutput,'项目 文档/说明.txt')),30000)
  assert.deepEqual(tree(selectedOutput),expected.filter(e=>e.path.endsWith('/说明.txt')))
  // Open encrypted original via actual picker; correct the password in the page.
  await bridge('queueDesktopDialogSelections',[path.join(original,'journal-key.zpaq')]);await click('.browser-page header .browser-primary')
  await fill('.browser-toolbar input[type="password"]','wrong');await click('.browser-toolbar button[title="重新读取"]')
  await driver.wait(async()=>{const p=await driver.findElements(By.css('[data-testid="archive-browse-notice"]'));return p.length&&(await p[0].getText()).includes('密码')},10000)
  await sizes('zpaq-password-correction','.browser-toolbar')
  await fill('.browser-toolbar input[type="password"]',manifest.password);await click('.browser-toolbar button[title="重新读取"]');await waitForElement('[data-testid="zpaq-current-view"]')
  assert.match(await (await waitForElement('.browser-summary')).getText(),/已加密/);await sizes('zpaq-encrypted-current-files','.browser-summary')
  // A confirmed damaged payload must fail without publishing output.
  const damaged=path.join(work,'内容损坏.zpaq'),bytes=readFileSync(path.join(original,'stream.zpaq'))
  const prefix=Buffer.from(Array.from({length:64},(_,i)=>i*37&255)),offset=bytes.indexOf(prefix);assert.ok(offset>=0);bytes[offset+100]^=0x55;writeFileSync(damaged,bytes)
  const failedOutput=path.join(work,'失败不能留下文件');const detail=await failure('extractArchive',damaged,failedOutput,null);assert.match(detail,/ZPAQ|校验/);assert.deepEqual(tree(failedOutput),[])
  cases.push({actualFileManagerDoubleClick:true,actualSelectedExtraction:true,actualWrongThenCorrectPassword:true,damageNoPublishedOutput:true})
  await click('[data-testid="nav-Settings"]');await waitForElement('[data-testid="settings-password-extraction-formats"]')
  assert.match(await (await waitForElement('[data-testid="settings-password-extraction-formats"]')).getText(),/ZPAQ（只读）/)
  await sizes('zpaq-settings-password-support','[data-testid="settings-password-extraction-formats"]')
  for(const [name,hash] of Object.entries(manifest.hashes))assert.equal(sha(path.join(original,name)),hash)
  writeFileSync(path.join(artifactDirectory,'zpaq-150-results.json'),JSON.stringify({completedAt:new Date().toISOString(),application,binarySha256:sha(application),cases,screenshots,layouts,fixtureHashes:manifest.hashes,independentOutputDecoder:bundledSevenZip},null,2))
}

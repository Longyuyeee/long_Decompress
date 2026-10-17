import assert from 'node:assert/strict'
import { createHash, randomBytes } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { readFileSync, writeFileSync, mkdirSync, readdirSync, existsSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runZpaq151Gate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, application, root }) {
  const work = path.join(fixtureDirectory, 'ZPAQ创建用户场景'); mkdirSync(work)
  const official = process.env.LONG_OFFICIAL_ZPAQ || path.join(root, 'test-results/zpaq-150-tools/upstream/zpaq64.exe')
  const sha = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  assert.equal(sha(official), '7a94b4f1d6323a758c7b0b6344036f166bff0fd44f1c3c86f05b3688023496cb', 'use pristine official 7.15, not the product adapter')
  const tree = dir => !existsSync(dir) ? [] : readdirSync(dir, { withFileTypes: true }).flatMap(e => e.isDirectory()
    ? tree(path.join(dir, e.name)).map(v => ({ ...v, path: e.name + '/' + v.path })) : [{ path: e.name, sha256: sha(path.join(dir, e.name)) }]).sort((a,b) => a.path.localeCompare(b.path))
  const source = path.join(work, '项目 空格'); mkdirSync(path.join(source, '子目录/空目录'), { recursive: true })
  writeFileSync(path.join(source, '说明.txt'), '这是要分享给同事的中文资料。\r\n')
  writeFileSync(path.join(source, '空文件.txt'), '')
  writeFileSync(path.join(source, '子目录/数据.bin'), Buffer.from(Array.from({ length: 65536 }, (_, i) => i * 37 & 255)))
  const original = tree(source), cases = [], screenshots = [], layouts = []
  const password = '中文 密码!-钥匙'
  const options = { format: 'zpaq', level: 6, password: null, split_size: null, preserve_paths: true, delete_after: false, verify_after: true }
  const invoke = (cmd, args) => driver.executeAsyncScript(`const [cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`, cmd, args)
  const ipc = async (cmd,args) => { const r = await invoke(cmd,args); assert.equal(r.ok,true,r.error); return r.value }
  const decode = (archive, key, label) => {
    const output = path.join(work, label); mkdirSync(output)
    const result = spawnSync(official, ['x', archive, '-to', output, '-threads', '2', ...(key !== null ? ['-key', key] : [])], { windowsHide: true, encoding: 'utf8', timeout: 60000 })
    assert.ifError(result.error); assert.equal(result.status,0,result.stdout + result.stderr)
    return output
  }
  const click = async selector => { const el = typeof selector === 'string' ? await waitForElement(selector) : selector; await driver.executeScript(el => el.scrollIntoView({ block:'center', behavior:'instant' }),el); await driver.wait(() => driver.executeScript(el => { const r=el.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2); return !el.disabled && (h===el || el.contains(h)) },el),10000); await el.click() }
  const fill = async (selector,value) => { const el=await waitForElement(selector); await driver.wait(()=>el.isEnabled(),10000); await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el); await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE); if(value) await el.sendKeys(value) }
  const select = async (selector,label) => { await click(selector); await waitForElement('[role="option"]'); for(const el of await driver.findElements(By.css('[role="option"]'))) if((await el.getAttribute('textContent')).trim()===label) { await click(el); return } assert.fail('Missing option '+label) }
  const sizes = async (name,anchor) => {
    for(const [width,height] of [[1100,700],[760,520]]) {
      await driver.manage().window().setRect({width,height}); await driver.wait(()=>driver.executeScript(w=>Math.abs(innerWidth-w)<=2,width),10000)
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement(anchor)); await driver.sleep(200)
      const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panels:[...document.querySelectorAll('[role="dialog"],[data-testid="save-compression-profile-dialog"],[data-testid="compression-draft-details"]')].map(el=>({overflow:el.scrollWidth-el.clientWidth,left:el.getBoundingClientRect().left,right:el.getBoundingClientRect().right,top:el.getBoundingClientRect().top,bottom:el.getBoundingClientRect().bottom}))}))
      assert.ok(layout.overflow<=1,JSON.stringify(layout)); layout.panels.forEach(p=>assert.ok(p.overflow<=2 && p.left>=-1 && p.right<=width+1,JSON.stringify(layout)))
      if(name==='zpaq-save-profile') layout.panels.forEach(p=>assert.ok(p.top>=0 && p.bottom<=height,JSON.stringify(layout)))
      layouts.push({name,...layout}); const file=`${name}-${width}x${height}.png`; screenshots.push(file); writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'))
    }
  }
  // Real backend commands, independently decoded by the author's unmodified EXE.
  for(const key of [null,password]) {
    const label=key?'原生中文密码':'普通目录',archive=path.join(work,label+'.zpaq'),output=path.join(work,label+'-应用回解')
    await bridge('runArchiveRoundTrip',source,archive,output,'zpaq',key)
    assert.deepEqual(tree(path.join(output,path.basename(source))),original)
    const independent=decode(archive,key,label+'-官方回解')
    assert.deepEqual(tree(path.join(independent,path.basename(source))),original)
    assert.ok(existsSync(path.join(independent,path.basename(source),'子目录/空目录')))
    const directory=await ipc('browse_archive',{filePath:archive,password:key})
    assert.equal(directory.totalFiles,3); assert.equal(directory.encrypted,!!key)
    if(key) { const wrong=await invoke('browse_archive',{filePath:archive,password:'wrong'}); assert.equal(wrong.ok,false); assert.match(wrong.error,/密码|损坏/) }
    writeFileSync(path.join(artifactDirectory,key?'zpaq-151-native-key.zpaq':'zpaq-151-plain.zpaq'),readFileSync(archive))
    cases.push({created:label,archiveSha256:sha(archive),applicationRoundtrip:true,officialReadback:true,payloadHashes:true,emptyDirectory:true})
  }
  const input=path.join(root,'src-tauri/tests/fixtures/zpaq/journal-key.zpaq'),before=sha(input),converted=path.join(work,'已有密码ZPAQ改密.zpaq')
  await bridge('convertArchive',input,password,converted,{...options,password:'新的 中文 密码'})
  const expected=JSON.parse(readFileSync(path.join(root,'src-tauri/tests/fixtures/zpaq/manifest.json'),'utf8')).payload
  const convertedTree=tree(path.join(decode(converted,'新的 中文 密码','改密官方回解'),'项目 文档'))
  assert.deepEqual(convertedTree,Object.entries(expected).map(([path,sha256])=>({path,sha256})).sort((a,b)=>a.path.localeCompare(b.path)))
  assert.equal(sha(input),before); cases.push({nativeReencrypt:true,sourceUnchanged:true,officialReadback:true})
  const zipInput=path.join(work,'已有中文密码ZIP.zip')
  await bridge('convertArchive',input,password,zipInput,{format:'zip',password,verify_after:true})
  const zipBefore=sha(zipInput)
  // Confirm a running encoder can be cancelled, leaving neither publication nor
  // transaction files; delete-after is deliberately on to protect the source.
  const large=path.join(work,'取消任务源.bin'); writeFileSync(large,randomBytes(16*1024*1024)); const largeHash=sha(large)
  const cancelId='zpaq-cancel-'+Date.now(),cancelOutput=path.join(work,'取消不能发布.zpaq')
  await driver.executeScript(`const [taskId,file,outputPath,options]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));window.__zpaqCancelResult=null;const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=()=>{clean();window.__zpaqCancelResult={ok:true}};window['_'+ids[1]]=error=>{clean();window.__zpaqCancelResult={ok:false,error:String(error)}};window.__TAURI_IPC__({cmd:'compress_files',taskId,files:[file],outputPath,options,callback:ids[0],error:ids[1]});`,cancelId,large,cancelOutput,{...options,level:9,delete_after:true})
  await driver.wait(()=>readdirSync(work).some(name=>name.includes('long-compress')&&name.endsWith('取消不能发布.zpaq')),15000)
  await ipc('cancel_compression',{taskId:cancelId}); const cancelled=await driver.wait(()=>driver.executeScript(()=>window.__zpaqCancelResult),30000)
  assert.equal(cancelled.ok,false); assert.match(cancelled.error,/取消|cancel/i); assert.equal(sha(large),largeHash); assert.ok(!existsSync(cancelOutput))
  assert.ok(!readdirSync(work).some(name=>name.includes('long-compress'))); cases.push({realEncoderCancellation:true,sourcePreservedWithDeleteAfter:true,noPublishedOrStagedOutput:true})
  // User: choose a file, set ZPAQ and a password, save this reusable profile.
  await bridge('clearTasks'); await bridge('clearCompressionWorkspace'); await click('[data-testid="nav-Compress"]')
  await bridge('queueDesktopDialogSelections',[[path.join(source,'说明.txt')]]); await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
  await waitForElement('[data-testid="compression-draft-row"]')
  if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length) await click('[data-testid="compression-draft-row"] [data-testid="compression-archive-name"]')
  await select('[data-testid="compression-format-select"]','ZPAQ'); await fill('[data-testid="compression-password-input"]',password)
  const uiName='发给同事的中文资料_这个名字足够长也不能遮挡格式后缀',uiDir=path.join(work,'用户创建'); mkdirSync(uiDir)
  await fill('[data-testid="compression-filename-input"]',uiName); await fill('[data-testid="compression-output-path"]',uiDir)
  assert.equal(await (await waitForElement('[data-testid="compression-split-toggle"]')).isEnabled(),false)
  assert.equal(await (await waitForElement('[data-testid="compression-filename-extension"]')).getText(),'.zpaq')
  assert.equal((await driver.findElements(By.css('[data-testid="encrypted-7z-conversion-hint"]'))).length,0)
  await sizes('zpaq-create-inline','[data-testid="compression-format-select"]')
  await click('[data-testid="save-compression-profile"]'); await fill('[data-testid="compression-profile-name"]','分享中文密码ZPAQ')
  await sizes('zpaq-save-profile','[data-testid="save-compression-profile-dialog"]'); await click('[data-testid="confirm-save-compression-profile"]')
  const savedProfile=await driver.wait(async()=>(await ipc('get_compression_profiles',{})).find(p=>p.name==='分享中文密码ZPAQ')||false,10000)
  assert.equal(savedProfile.config.format,'zpaq'); assert.equal(savedProfile.config.password,password)
  await select('[data-testid="compression-format-select"]','ZIP'); await fill('[data-testid="compression-password-input"]','')
  await click('[data-testid="manage-compression-profiles"]'); await click(await driver.findElement(By.xpath('//button[contains(.,"返回选择配置")]')))
  const profileCard=await driver.wait(async()=>{for(const card of await driver.findElements(By.css('.profile-selector button')))if((await card.getText()).includes('分享中文密码ZPAQ'))return card;return false},10000)
  await click(profileCard); assert.match(await (await waitForElement('[data-testid="compression-format-select"]')).getText(),/ZPAQ/)
  assert.equal(await (await waitForElement('[data-testid="compression-password-input"]')).getAttribute('value'),password)
  await sizes('zpaq-profile-applied','[data-testid="compression-format-select"]')
  await driver.manage().window().setRect({width:1100,height:700}); await click('[data-testid="start-compression"]')
  const uiArchive=path.join(uiDir,uiName+'.zpaq')
  const saved=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.outputPath?.replaceAll('\\','/')===uiArchive.replaceAll('\\','/')&&['completed','failed'].includes(r.status))||false,60000)
  assert.equal(saved.status,'completed',JSON.stringify(saved)); assert.equal(saved.executionConfig.settings.format,'zpaq'); assert.equal(saved.executionConfig.passwordRequired,true)
  assert.equal(saved.executionConfig.settings.password,undefined)
  assert.deepEqual(tree(decode(uiArchive,password,'用户创建官方回解')),[{path:'说明.txt',sha256:sha(path.join(source,'说明.txt'))}])
  cases.push({actualPickerAndCompressionControls:true,savedProfileNativePassword:true,historySnapshotFormat:true,officialReadback:true})
  // User: repeat this completed compression, in another directory, without
  // modifying the old history record or restoring a password from history.
  await bridge('clearCompressionWorkspace'); await click('[data-testid="nav-History"]')
  const historyRow=await driver.wait(async()=>{for(const row of await driver.findElements(By.css('[data-testid="history-record-row"]')))if((await row.getText()).includes(uiName))return row;return false},10000)
  await click(await historyRow.findElement(By.css('[data-testid="history-compression-retry"]')))
  await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement('[data-testid="compression-restore-notice"]'))
  await driver.wait(async()=>/已带入上次/.test(await (await waitForElement('[data-testid="compression-restore-notice"]')).getText()),10000)
  assert.equal(await (await waitForElement('[data-testid="history-compression-draft"] [data-testid="compression-password-input"]')).getAttribute('value'),'')
  await click('[data-testid="compression-retry-create"]'); assert.match(await (await waitForElement('[data-testid="history-compression-draft"] [role="alert"]')).getText(),/重新填写本次密码/)
  const retryDir=path.join(work,'再次压缩'); mkdirSync(retryDir); await fill('[data-testid="compression-retry-output"]',retryDir)
  await fill('[data-testid="history-compression-draft"] [data-testid="compression-password-input"]',password)
  await sizes('zpaq-history-rerun','[data-testid="compression-retry-create"]'); await click('[data-testid="compression-retry-create"]')
  assert.deepEqual(readdirSync(retryDir),[]); await click('[data-testid="history-compression-draft"] a'); await click('[data-testid="start-compression"]')
  const retried=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.retryOfTaskId===saved.id&&['completed','failed'].includes(r.status))||false,60000)
  assert.equal(retried.status,'completed',JSON.stringify(retried)); assert.equal(retried.executionConfig.settings.format,'zpaq')
  assert.deepEqual((await bridge('taskHistory')).find(r=>r.id===saved.id),saved)
  assert.deepEqual(tree(decode(retried.outputPath,password,'历史再做官方回解')),[{path:'说明.txt',sha256:sha(path.join(source,'说明.txt'))}])
  cases.push({actualHistoryRerun:true,requiresPasswordAgain:true,createsDraftOnly:true,oldRecordUnchanged:true,officialReadback:true})
  // User: convert an existing archive to ZPAQ through the real dialog.
  await click('[data-testid="open-archive-conversion"]'); await bridge('queueDesktopDialogSelections',[[zipInput]]); await click('[data-testid="conversion-choose-source"]')
  await waitForElement('[data-testid="conversion-item"]'); if(!(await driver.findElements(By.css('[data-testid="conversion-details"]'))).length)await click('[data-testid="conversion-row"]')
  await select('[data-testid="conversion-format"]','ZPAQ'); await fill('[data-testid="conversion-source-password"]',password); await fill('[data-testid="conversion-password"]','分享新密码')
  const uiConverted=path.join(work,'用户转换.zpaq'); await fill('[data-testid="conversion-output"]',uiConverted)
  assert.equal(await (await waitForElement('[data-testid="conversion-split"]')).isEnabled(),false)
  await sizes('zpaq-conversion','[data-testid="conversion-format"]'); await click('[data-testid="conversion-start"]')
  await driver.wait(()=>existsSync(uiConverted),60000); await waitForElement('[data-testid="conversion-result"]')
  assert.deepEqual(tree(path.join(decode(uiConverted,'分享新密码','用户转换官方回解'),'项目 文档')),convertedTree)
  assert.equal(sha(input),before); assert.equal(sha(zipInput),zipBefore); cases.push({actualConversionControls:true,sourceFormat:'AES ZIP',nativePasswordOutput:true,officialReadback:true,sourceUnchanged:true})
  await driver.actions().sendKeys(Key.ESCAPE).perform(); await click('[data-testid="nav-Settings"]')
  await sizes('zpaq-settings','[data-testid="settings-zpaq-creation"]')
  await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement('[data-testid="settings-password-extraction-formats"]'))
  await driver.wait(async()=>/ZPAQ/.test(await (await waitForElement('[data-testid="settings-password-extraction-formats"]')).getText()),10000)
  const support=await (await waitForElement('[data-testid="settings-password-extraction-formats"]')).getText(); assert.equal(support.match(/ZPAQ/g)?.length,1); assert.ok(!support.includes('只读'))
  assert.deepEqual(tree(source),original)
  writeFileSync(path.join(artifactDirectory,'zpaq-151-results.json'),JSON.stringify({completedAt:new Date().toISOString(),application,binarySha256:sha(application),officialToolSha256:sha(official),sourcePayload:original,cases,screenshots,layouts},null,2))
}

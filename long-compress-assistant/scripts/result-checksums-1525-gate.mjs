import assert from 'node:assert/strict'
import {createHash,randomBytes,randomUUID} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync,truncateSync,unlinkSync,renameSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runResultChecksums1525Gate({driver,bridge,waitForElement,artifactDirectory,application,root}) {
  const base=path.join(root,'test-results/result-checksums-1525');mkdirSync(base,{recursive:true})
  const hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p)),progress=path.join(base,'desktop-progress.json'),binarySha256=sha(application)
  const state=existsSync(progress)?JSON.parse(readFileSync(progress,'utf8')):{version:'1.5.25',binarySha256,sourceCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8',windowsHide:true}).trim(),work:path.join(base,'ui-'+randomUUID()),ui:[],native:[],screenshots:[],layouts:[]}
  const polishOnly=process.argv.includes('--result-checksums-1525-legacy-polish-only')
  if(polishOnly){
    assert.equal(state.ui.length,6,'Retain the completed six flows; only missing-member copy changed')
    state.originalBinarySha256??=state.binarySha256
    for(const entry of [...state.ui,...state.layouts])entry.binarySha256??=state.originalBinarySha256
    state.latestBinarySha256=binarySha256;state.resumedScope='Only legacy missing-member copy and busy label; six passed flows retained with their original program identity'
  }else assert.equal(state.binarySha256,binarySha256,'Only resume evidence from the same program')
  const work=state.work;mkdirSync(work,{recursive:true})
  const owned=p=>{assert.ok(path.resolve(p).startsWith(path.resolve(work)+path.sep));return p}
  const save=()=>writeFileSync(progress,JSON.stringify(state,null,2))
  const done=(kind,data={})=>{state.ui.push({kind,binarySha256,...data});save();console.log('result-checksums-1525: '+kind)}
  const passed=kind=>state.ui.some(row=>row.kind===kind)
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const result=await invoke(cmd,args);assert.equal(result.ok,true,result.error);return result.value}
  const click=async target=>{
    for(const t of await driver.findElements(By.css('.toast-item button'))){try{if(await t.isDisplayed())await t.click()}catch{}}
    const e=typeof target==='string'?await waitForElement(target):target
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&r.height>0&&(hit===e||e.contains(hit))},e),15000)
    await e.click()
  }
  const fill=async(selector,value)=>{const e=await waitForElement(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const choose=async(selector,label)=>{await click(selector);for(const option of await driver.findElements(By.css('[role="option"]'))){const strong=await option.findElements(By.css('strong'));if((await(strong[0]||option).getAttribute('textContent')).trim()===label){await click(option);return}}throw Error('Missing '+label)}
  const sizes=async(name,anchor)=>{
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement(anchor))
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const geometry=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,details:[...document.querySelectorAll('.checksum-calculation-details,.checksum-export-receipt,.checksum-source-summary')].map(e=>({overflow:e.scrollWidth-e.clientWidth}))}))
      assert.ok(geometry.overflow<=1&&geometry.details.every(row=>row.overflow<=1),JSON.stringify(geometry))
      const file='checksums1525-'+name+(polishOnly?'-polished':'')+'-'+width+'x'+height+'.png';writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));state.screenshots.push(file);state.layouts.push({name,binarySha256,...geometry});save()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  const statuses=()=>driver.executeScript(()=>[...document.querySelectorAll('[data-testid="checksum-calculation-entry"]')].map(e=>e.dataset.status))
  const settled=expected=>driver.wait(async()=>JSON.stringify(await statuses())===JSON.stringify(expected)&&await(await waitForElement('[data-testid="checksum-calculate-start"]')).isEnabled(),45000)
  const verifySettled=()=>driver.wait(async()=>await(await waitForElement('[data-testid="checksum-verification"]')).getAttribute('aria-busy')==='false'&&(await(await waitForElement('[data-testid="checksum-heading"]')).getText()).includes('均匹配'),45000)
  const exportTo=async file=>{await bridge('queueDesktopDialogSelections',[file]);await click('[data-testid="checksum-export"]');await driver.wait(async()=>await(await waitForElement('[data-testid="checksum-export"]')).isEnabled(),15000)}
  const independentManifest=(file,expected)=>{
    const text=readFileSync(file,'utf8'),actual=text.trimEnd().split('\n').map(line=>{const match=/^([0-9a-f]{64}) \*(.+)$/i.exec(line);assert.ok(match,line);return{sha256:match[1],name:match[2]}})
    assert.equal(actual.length,expected.length)
    for(const entry of expected){const relative=path.relative(path.dirname(file),entry.path).replaceAll('\\','/');assert.ok(!relative.startsWith('../')&&!path.isAbsolute(relative));assert.deepEqual(actual.find(row=>row.name===relative),{sha256:entry.sha256,name:relative})}
    return actual
  }
  await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
  if(!state.creation){
    const source=owned(path.join(work,'用于分享的真实数据.bin'));writeFileSync(source,randomBytes(2*1024*1024+1337))
    await bridge('clearCompressionWorkspace');await bridge('clearTasks');await bridge('clearTaskHistory');await click('[data-testid="nav-Compress"]')
    await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length)await click('[data-testid="compression-draft-row"]')
    await choose('[data-testid="compression-format-select"]','7Z')
    const split=await waitForElement('[data-testid="compression-split-toggle"]');if(!await split.isSelected())await click(split)
    await fill('[data-testid="compression-split-size"]','1');await fill('[data-testid="compression-output-path"]',work);await fill('[data-testid="compression-filename-input"]','分享整组')
    await click('[data-testid="start-compression"]')
    const record=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.outputPath?.includes('分享整组')&&['completed','failed'].includes(row.status))||false,60000)
    assert.equal(record.status,'completed',JSON.stringify(record));assert.ok(record.archiveOutput.split&&record.archiveOutput.files.length>=3)
    state.creation=record;save()
  }
  const published=state.creation.archiveOutput,expected=published.files.map(file=>({path:file.path,sha256:sha(file.path)}))
  if(!passed('completed-result-whole-group')){
    const stale=owned(published.primaryPath.replace(/\.\d+$/,'.999'));writeFileSync(stale,'stale unrelated same-prefix output')
    if(!(await driver.findElements(By.css('[data-testid="compression-draft-details"] [data-testid="result-generate-checksums"]'))).length){
      await click('[data-testid="nav-History"]');await click('[data-testid="history-record-row"] [data-testid="result-generate-checksums"]')
    }else await click('[data-testid="compression-draft-details"] [data-testid="result-generate-checksums"]')
    assert.match(await(await waitForElement('[data-testid="checksum-source-summary"]')).getText(),new RegExp('整组 '+expected.length+' 个文件'))
    assert.equal((await statuses()).length,0,'Opening a result must not start hashing')
    await click('[data-testid="checksum-source-toggle"]');assert.equal((await driver.findElements(By.css('.checksum-source-paths li'))).length,expected.length)
    await sizes('whole-group-ready','[data-testid="checksum-source-summary"]')
    await click('[data-testid="checksum-calculate-start"]');await settled(expected.map(()=>'success'))
    const digests=await driver.executeScript(()=>[...document.querySelectorAll('.checksum-main>code')].map(e=>e.textContent));assert.deepEqual(digests,expected.map(row=>row.sha256))
    const manifest=owned(path.join(work,'整组分卷.sha256'));await exportTo(manifest);independentManifest(manifest,expected)
    await sizes('exported','[data-testid="checksum-export-receipt"]')
    await click('[data-testid="checksum-verify-exported"]');await verifySettled()
    assert.equal((await driver.findElements(By.css('[data-testid="checksum-entry"]'))).length,expected.length)
    unlinkSync(stale);done('completed-result-whole-group',{actualVolumes:expected.length,staleSamePrefixExcluded:true,noImplicitCalculation:true,manifest,independentSha256:true})
  }
  if(!passed('history-result-whole-group')){
    await click('[data-testid="nav-History"]');await click('[data-testid="history-record-row"] [data-testid="result-generate-checksums"]')
    assert.match(await(await waitForElement('[data-testid="checksum-source-summary"]')).getText(),new RegExp('整组 '+expected.length+' 个文件'))
    await click('[data-testid="checksum-source-toggle"]')
    const actual=await driver.executeScript(()=>[...document.querySelectorAll('.checksum-source-paths li')].map(e=>e.textContent))
    assert.deepEqual(actual,published.files.map(file=>file.path));assert.equal((await statuses()).length,0)
    done('history-result-whole-group',{actualVolumes:expected.length,noImplicitCalculation:true})
  }
  if(!passed('nested-same-name-and-save-guidance')){
    await click('[data-testid="nav-FileIntegrity"]')
    const files=['甲/很长的目录/同名 文件.txt','乙/很长的目录/同名 文件.txt'].map((name,index)=>{const file=owned(path.join(work,name));mkdirSync(path.dirname(file),{recursive:true});writeFileSync(file,'不同内容 '+index+'\n');return file})
    await bridge('queueDesktopDialogSelections',[files]);await click('[data-testid="checksum-calculate-choose"]');await click('[data-testid="checksum-calculate-start"]');await settled(['success','success'])
    await click((await driver.findElements(By.css('[data-testid="checksum-calculation-toggle"]')))[1]);await sizes('same-name-inline','[data-testid="checksum-calculation-details"]')
    const incorrect=owned(path.join(work,'甲','不应覆盖.sha256'));writeFileSync(incorrect,'existing user manifest')
    await exportTo(incorrect);assert.match(await(await waitForElement('[data-testid="checksum-export-error"]')).getText(),/共同上级目录/);assert.equal(readFileSync(incorrect,'utf8'),'existing user manifest')
    await sizes('save-guidance','[data-testid="checksum-export-error"]')
    const manifest=owned(path.join(work,'同名分目录.sha256'));await exportTo(manifest);independentManifest(manifest,files.map(file=>({path:file,sha256:sha(file)})))
    await click('[data-testid="checksum-verify-exported"]');await verifySettled()
    await click('[data-testid="checksum-entry"][data-line="2"] [data-testid="checksum-row"]');await sizes('relative-manifest-verified','[data-testid="checksum-entry"][data-line="2"]')
    done('nested-same-name-and-save-guidance',{manifest,distinctRelativeNames:true,failedExportPreservedExisting:true,existingVerifierMatched:true})
  }
  if(!passed('partial-export-and-continue')){
    await click('.integrity-mode-switch button:first-child')
    const first=owned(path.join(work,'已完成.txt')),large=owned(path.join(work,'读取中.bin')),last=owned(path.join(work,'尚未读取.txt'))
    writeFileSync(first,'first completed observation');writeFileSync(last,'last unfinished observation')
    const bytes=2*1024*1024*1024,block=Buffer.alloc(8*1024*1024),digest=createHash('sha256');writeFileSync(large,'');truncateSync(large,bytes)
    for(let offset=0;offset<bytes;offset+=block.length)digest.update(block)
    const checks=[sha(first),digest.digest('hex'),sha(last)]
    await bridge('queueDesktopDialogSelections',[[first,large,last]]);await click('[data-testid="checksum-calculate-choose"]');await click('[data-testid="checksum-calculate-start"]')
    const live=await driver.wait(async()=>{const p=await driver.findElements(By.css('[data-testid="checksum-progress"]'));if(!p.length)return false;const fileBytes=Number(await p[0].getAttribute('data-file-bytes'));return fileBytes>0&&fileBytes<bytes&&(await p[0].getText()).includes(path.basename(large))?{fileBytes,totalBytes:Number(await p[0].getAttribute('data-total-bytes'))}:false},20000)
    await click('[data-testid="checksum-stop"]');await settled(['success','interrupted','pending'])
    const manifest=owned(path.join(work,'仅已完成.sha256'));unlinkSync(first)
    await exportTo(manifest);independentManifest(manifest,[{path:first,sha256:checks[0]}])
    assert.match(await(await waitForElement('[data-testid="checksum-export-receipt"]')).getText(),/其余 2 项未写入/)
    await sizes('partial-export','[data-testid="checksum-export-receipt"]')
    await click('[data-testid="checksum-calculate-continue"]');await settled(['success','success','success']);assert.ok(!existsSync(first))
    assert.deepEqual(await driver.executeScript(()=>[...document.querySelectorAll('.checksum-main>code')].map(e=>e.textContent)),checks)
    const complete=owned(path.join(work,'接续后完整.sha256'));await exportTo(complete);independentManifest(complete,[first,large,last].map((file,index)=>({path:file,sha256:checks[index]})))
    writeFileSync(first,'first completed observation');await click('[data-testid="checksum-verify-exported"]');await verifySettled()
    unlinkSync(large);done('partial-export-and-continue',{live,partialManifest:manifest,completeManifest:complete,completedFileNotReopened:true,interruptedDigestExcluded:true,independentHashesMatched:true})
  }
  if(polishOnly||!passed('legacy-history-resolves-members')){
    const legacy={...state.creation,id:'legacy-'+randomUUID(),name:'旧记录整组',archiveOutput:null,completedAt:new Date().toISOString()}
    await bridge('seedHistoryRecords',[legacy])
    await click('[data-testid="nav-History"]')
    const findLegacy=async()=>{for(const row of await driver.findElements(By.css('[data-testid="history-record-row"]'))){if((await row.getText()).includes('旧记录整组'))return row}throw Error('legacy record not shown')}
    const withdrawn=owned(published.files[1].path),backup=owned(withdrawn+'.owned-original');renameSync(withdrawn,backup)
    try{const row=await driver.wait(()=>findLegacy(),10000);await click(await row.findElement(By.css('[data-testid="result-generate-checksums"]')));await driver.wait(async()=>(await row.getText()).includes('补齐'),15000);await sizes('legacy-missing-guidance','[data-testid="history-record-row"]')}finally{renameSync(backup,withdrawn)}
    await click(await(await findLegacy()).findElement(By.css('[data-testid="result-generate-checksums"]')))
    assert.match(await(await waitForElement('[data-testid="checksum-source-summary"]')).getText(),new RegExp('整组 '+expected.length+' 个文件'))
    done(polishOnly?'legacy-guidance-polished':'legacy-history-resolves-members',{basedOnActualPublishedRecord:true,missingMemberGuidance:true,allActualMembers:true})
  }
  if(!passed('conversion-result-closes-dialog')){
    await click('[data-testid="nav-Compress"]');await click('[data-testid="open-archive-conversion"]')
    await bridge('queueDesktopDialogSelections',[[published.primaryPath]]);await click('[data-testid="conversion-choose-source"]')
    const row=await waitForElement('[data-testid="conversion-row"]');if(await row.getAttribute('aria-expanded')!=='true')await click(row)
    await choose('[data-testid="conversion-format"]','ZIP')
    const output=owned(path.join(work,'转换后分享.zip'));await fill('[data-testid="conversion-output"]',output);await click('[data-testid="conversion-start"]')
    const record=await driver.wait(async()=>(await bridge('taskHistory')).find(row=>row.outputPath?.replaceAll('\\','/')===output.replaceAll('\\','/')&&['completed','failed'].includes(row.status))||false,60000)
    assert.equal(record.status,'completed',JSON.stringify(record));await click('[data-testid="conversion-result"] [data-testid="result-generate-checksums"]')
    await driver.wait(async()=>(await driver.findElements(By.css('[role="dialog"]'))).length===0,10000)
    assert.match(await(await waitForElement('[data-testid="checksum-source-summary"]')).getText(),/整组 1 个文件/)
    assert.equal((await statuses()).length,0);await sizes('conversion-handoff','[data-testid="checksum-source-summary"]')
    done('conversion-result-closes-dialog',{actualConversion:true,dialogClosed:true,noImplicitCalculation:true})
  }
  state.completedAt=new Date().toISOString();save();writeFileSync(path.join(artifactDirectory,'result-checksums-1525-results.json'),JSON.stringify(state,null,2))
}

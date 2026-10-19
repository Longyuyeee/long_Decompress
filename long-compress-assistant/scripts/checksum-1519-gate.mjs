import assert from 'node:assert/strict'
import {execFileSync} from 'node:child_process'
import {existsSync,mkdirSync,readFileSync,writeFileSync,renameSync,rmdirSync,readdirSync} from 'node:fs'
import {createHash,randomUUID} from 'node:crypto'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runChecksum1519Gate({driver,bridge,waitForElement,artifactDirectory,application,root}) {
  const base=path.join(root,'test-results/checksum-1519');mkdirSync(base,{recursive:true})
  const hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(readFileSync(p))
  const work=path.join(base,'ui-'+randomUUID()),owned=p=>{assert.ok(path.resolve(p).startsWith(work+path.sep));return p}
  execFileSync('python',[path.join(root,'scripts/prepare-checksum-1519.py'),work],{windowsHide:true})
  const original=JSON.parse(readFileSync(owned(path.join(work,'originals.json')),'utf8'))
  const parts=original.parts,originalBytes=parts.map(p=>readFileSync(owned(p))),manifestBytes=readFileSync(original.manifest)
  assert.equal(parts.length,3)
  const result={version:'1.5.19',binarySha256:sha(application),original,native:[],ui:[],screenshots:[],layouts:[]}
  const record=()=>writeFileSync(path.join(artifactDirectory,'checksum-1519-results.json'),JSON.stringify(result,null,2))
  const click=async target=>{
    for(const toast of await driver.findElements(By.css('.toast-item button'))){try{if(await toast.isDisplayed())await toast.click()}catch{}}
    const e=typeof target==='string'?await waitForElement(target):target
    await driver.wait(()=>driver.executeScript(e=>{e.scrollIntoView({block:'center',behavior:'instant'});const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !e.disabled&&r.width>0&&r.height>0&&(hit===e||e.contains(hit))},e),15000)
    await e.click()
  }
  const fill=async(selector,value)=>{const e=await waitForElement(selector);await driver.wait(async()=>await e.isDisplayed()&&await e.isEnabled(),15000);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
  const invoke=(cmd,args)=>driver.executeAsyncScript('const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,"_"+id));window["_"+ids[0]]=value=>{clean();done({ok:true,value})};window["_"+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});',cmd,args)
  const ipc=async(cmd,args)=>{const r=await invoke(cmd,args);assert.equal(r.ok,true,r.error);return r.value}
  const row=line=>`[data-testid="checksum-entry"][data-line="${line}"]`
  const statuses=()=>driver.executeScript(()=>[...document.querySelectorAll('[data-testid="checksum-entry"]')].map(e=>e.dataset.status))
  const settled=async expected=>{await driver.wait(async()=>await(await waitForElement('[data-testid="checksum-verification"]')).getAttribute('aria-busy')==='false'&&JSON.stringify(await statuses())===JSON.stringify(expected),30000)}
  const expand=async line=>{const e=await waitForElement(row(line)+' [data-testid="checksum-row"]');if(await e.getAttribute('aria-expanded')!=='true')await click(e)}
  const sizes=async(name,anchor)=>{
    for(const[width,height]of[[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});const e=await waitForElement(anchor)
      await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e)
      await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const geometry=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,digests:[...document.querySelectorAll('.checksum-details')].map(e=>{const cols=e.querySelectorAll('.checksum-digests>div');return{left:cols[0].getBoundingClientRect().width,right:cols[1].getBoundingClientRect().width,overflow:e.scrollWidth-e.clientWidth}})}))
      assert.ok(geometry.overflow<=1,JSON.stringify(geometry));for(const cols of geometry.digests)assert.ok(Math.abs(cols.left-cols.right)<=1&&cols.overflow<=1,JSON.stringify(cols))
      const file=`checksum1519-${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'))
      result.screenshots.push(file);result.layouts.push({name,...geometry});record()
    }await driver.manage().window().setRect({width:1100,height:700})
  }
  await driver.manage().setTimeouts({script:90000});await driver.manage().window().setRect({width:1100,height:700})
  for(const extension of ['sfv','md5','sha256']){
    const r=await ipc('verify_checksum_file',{checksumPath:owned(path.join(work,'独立小清单.'+extension))})
    assert.equal(r.valid,true);assert.equal(r.entries[0].status,'matched');assert.ok(r.entries[0].actualChecksum);result.native.push({algorithm:extension,fullContentMatches:true,manifestId:r.manifestId})
  }
  const unreadable=await ipc('verify_checksum_file',{checksumPath:original.manifest,entryLines:[4]})
  assert.deepEqual(unreadable.entries.map(e=>e.status),['not_checked','not_checked','not_checked','unreadable']);assert.equal(unreadable.valid,false)
  result.native.push({kind:'selective-only',report:unreadable});record()
  const backup=owned(parts[1]+'.owned-original')
  try {
    await bridge('clearTasks');await click('[data-testid="nav-Decompress"]')
    await bridge('queueDesktopDialogSelections',[[parts[0]]]);await click('[data-testid="dropzone-file"]')
    const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000)
    const firstOutput=owned(path.join(work,'首次输出'));mkdirSync(firstOutput)
    await bridge('queueDesktopDialogSelections',[[firstOutput]])
    await click(await driver.findElement(By.xpath('//*[contains(@class,"global-config-controls")]//button[.//i[contains(@class,"pi-folder-open")]]')))
    renameSync(parts[1],backup)
    await click('[data-testid="start-decompression-queue"]')
    const failed=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.id===task.id&&h.status==='failed')||false,30000)
    assert.match(failed.errorMessage,/incomplete|缺少|分卷|source-missing/);assert.deepEqual(readdirSync(firstOutput),[])
    const corrupt=Buffer.from(originalBytes[2]);corrupt[Math.floor(corrupt.length/2)]^=1;writeFileSync(parts[2],corrupt)

    await click('[data-testid="nav-FileIntegrity"]');await click('[data-testid="checksum-verify-mode"]')
    await bridge('queueDesktopDialogSelections',[[original.manifest]]);await click('[data-testid="checksum-choose"]')
    await settled(['matched','missing','mismatch','unreadable']);await expand(3)
    await sizes('file-causes',row(3)+' .checksum-digests')
    await click('[data-testid="checksum-copy-pending"]')
    await driver.wait(()=>driver.executeScript(()=>document.body.innerText.includes('已复制到剪贴板')),10000)
    result.ui.push({kind:'import-and-pending',fourDistinctStatuses:true,realClipboardWriteSucceeded:true});record()

    renameSync(backup,parts[1]);writeFileSync(parts[2],originalBytes[2])
    // Change an unselected, previously matched volume to prove no hidden rehash.
    const changedFirst=Buffer.from(originalBytes[0]);changedFirst[100]^=1;writeFileSync(parts[0],changedFirst)
    await click('[data-testid="checksum-recheck-problems"]');await settled(['matched','matched','matched','unreadable'])
    assert.notEqual(sha(parts[0]),original.partSha256[path.basename(parts[0])]);writeFileSync(parts[0],originalBytes[0])
    await expand(4);rmdirSync(owned(original.blocked));writeFileSync(original.blocked,Buffer.from('Hello, World!'))
    await click(row(4)+' input[type="checkbox"]');await click('[data-testid="checksum-recheck-selected"]');await settled(['matched','matched','matched','matched'])
    assert.match(await(await waitForElement('[data-testid="checksum-heading"]')).getText(),/均匹配/)
    await sizes('selective-restored',row(4)+' .checksum-digests')
    result.ui.push({kind:'selective-restoration',unselectedMatchedVolumeNotRehashed:true,unreadableFixedSeparately:true});record()

    writeFileSync(original.manifest,Buffer.concat([manifestBytes,Buffer.from('a'.repeat(64)+'  新加入的卷.zip.004\n')]))
    await click('[data-testid="checksum-recheck-4"]');await settled(['not_checked','not_checked','not_checked','matched','not_checked'])
    assert.match(await(await waitForElement('[data-testid="checksum-notice"]')).getText(),/旧结果已作废/)
    assert.match(await(await waitForElement('[data-testid="checksum-heading"]')).getText(),/未检查/)
    await sizes('manifest-changed','[data-testid="checksum-notice"]')
    result.ui.push({kind:'manifest-change',oldMatchesInvalidated:true,uncheckedNeverPassed:true});record()
    writeFileSync(original.manifest,manifestBytes);await click('[data-testid="checksum-recheck-all"]');await settled(['matched','matched','matched','matched'])

    await expand(1);await click(row(1)+' [data-testid="checksum-diagnose"]')
    await driver.wait(async()=>/完整性正常/.test(await(await waitForElement('[data-testid="diagnostic-report"]')).getText()),30000)
    await click('[data-testid="checksum-verify-mode"]');await settled(['matched','matched','matched','matched'])
    result.ui.push({kind:'diagnosis-return',actualArchiveIntegrityHealthy:true,observationsRetained:true});record()

    await click('[data-testid="checksum-go-tasks"]');await click('[data-testid="retry-batch-failures"]')
    const output=owned(path.join(work,'补齐后新任务输出'));mkdirSync(output)
    await fill('[role="dialog"] [data-testid="retry-output"]',output)
    const sub=await waitForElement('[role="dialog"] [data-testid="retry-subfolder"]');if(await sub.isSelected())await click(sub)
    await sizes('retry-new-task','[role="dialog"]')
    await click('[role="dialog"] [data-testid="create-retry-drafts"]')
    await driver.wait(async()=>(await(await waitForElement('[role="dialog"]')).getText()).includes('已创建新任务'),15000)
    assert.deepEqual(readdirSync(output),[])
    await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[contains(.,"返回队列确认并启动")]')))
    const retry=(await bridge('extractionRecoveryTasks')).find(t=>t.retryOfTaskId===task.id);assert.equal(retry.status,'pending')
    await click('[data-testid="start-decompression-queue"]')
    const completed=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.id===retry.id&&['completed','failed'].includes(h.status))||false,30000);assert.equal(completed.status,'completed',JSON.stringify(completed))
    for(const[name,expected]of Object.entries(original.expectedFiles))assert.equal(sha(path.join(output,name)),expected)
    assert.ok(existsSync(path.join(output,original.emptyDirectory)));assert.deepEqual((await bridge('taskHistory')).find(h=>h.id===task.id),failed)
    result.ui.push({kind:'failed-task-continuation',pendingBeforeExplicitStart:true,fullContentHashes:true,emptyDirectory:true,oldFailureUnchanged:true});result.completedAt=new Date().toISOString();record()
  }finally{
    if(existsSync(backup))renameSync(backup,parts[1]);for(let i=0;i<parts.length;i++)writeFileSync(parts[i],originalBytes[i]);writeFileSync(original.manifest,manifestBytes)
  }
}

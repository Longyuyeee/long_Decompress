import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {readFileSync,writeFileSync,mkdirSync,readdirSync,existsSync,statSync,openSync,readSync,closeSync,copyFileSync} from 'node:fs'
import path from 'node:path'
import {By} from 'selenium-webdriver'

export async function runRar156Gate({driver,bridge,waitForElement,fixtureDirectory,artifactDirectory,application,root}) {
  const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
  const base=path.join(root,'test-results/rar-156-originals'),originals=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8'))
  const work=path.join(fixtureDirectory,'RAR实际进度');mkdirSync(work,{recursive:true})
  const cases=[],screenshots=[],layouts=[]
  const options={preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,create_subdirectory:false,file_filter:null,conflict_policy:'rename',enable_bruteforce:false,bruteforce_wordlists:[]}
  const invoke=(cmd,args)=>driver.executeAsyncScript(`const[cmd,args,done]=arguments,ids=crypto.getRandomValues(new Uint32Array(2));const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error:String(error)})};window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`,cmd,args)
  const click=async selector=>{const el=typeof selector==='string'?await waitForElement(selector):selector;await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),el);await driver.wait(()=>el.isDisplayed(),10000);await el.click()}
  async function sizes(name,anchor) {
    for(const [width,height] of [[1100,700],[760,520]]){
      await driver.manage().window().setRect({width,height});await driver.wait(()=>driver.executeScript(w=>Math.abs(innerWidth-w)<=2,width),10000)
      await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),await waitForElement(anchor));await driver.sleep(250)
      const layout=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth}));assert.ok(layout.overflow<=1,JSON.stringify(layout));layouts.push({name,...layout})
      const file=`${name}-${width}x${height}.png`;writeFileSync(path.join(artifactDirectory,file),Buffer.from(await driver.takeScreenshot(),'base64'));screenshots.push(file)
    }
  }
  // Observe real backend events only; no seeded tasks, times or progress.
  await driver.executeAsyncScript(`const done=arguments[0],ids=crypto.getRandomValues(new Uint32Array(3));window.__rar156Progress=[];window['_'+ids[0]]=()=>done();window['_'+ids[1]]=error=>done({error:String(error)});window['_'+ids[2]]=event=>window.__rar156Progress.push({...event.payload,observedAt:Date.now()});window.__TAURI_IPC__({cmd:'tauri',__tauriModule:'Event',message:{cmd:'listen',event:'task-progress',windowLabel:null,handler:ids[2]},callback:ids[0],error:ids[1]});`)
  const events=id=>driver.executeScript(id=>window.__rar156Progress.filter(e=>e.task_id===id),id)
  for(const c of originals.cases){
    console.log('[rar-156] original',c.name)
    const taskId='rar156-'+randomUUID(),outputPath=path.join(work,c.name)
    const result=await invoke('extract_file',{taskId,filePath:path.join(base,c.name,c.archive),outputPath,password:c.password,options})
    assert.equal(result.ok,true,result.error);assert.equal(sha(path.join(outputPath,'payload.bin')),c.expected.sha256)
    const progress=(await events(taskId)).filter(e=>e.stage!=='password-attempt'&&e.processed_bytes>0)
    assert.ok(progress.length>0);assert.ok(progress.some(e=>e.processed_bytes===c.expected.bytes))
    assert.ok(progress.every(e=>e.processed_bytes<=c.expected.bytes&&!e.output_bytes_estimated))
    assert.ok(progress.filter(e=>e.progress<1).every(e=>e.progress<=0.99+1e-6))
    for(const m of c.members)assert.equal(sha(path.join(base,c.name,m.name)),m.sha256)
    cases.push({original:c.name,sourceMembers:c.members,expected:c.expected,actualBytes:progress.map(e=>e.processed_bytes),progress:progress.map(e=>e.progress),noOutputLengthEstimate:true,outputSha256:c.expected.sha256})
  }
  // Decoding every byte is not success: a genuine late CRC fault must fail
  // without a 100% committed event or published output.
  const plain=originals.cases.find(c=>c.name==='single-plain'),broken=path.join(work,'传输损坏.rar')
  const bytes=readFileSync(path.join(base,plain.name,plain.archive));bytes[Math.floor(bytes.length/2)]^=0x40;writeFileSync(broken,bytes)
  const brokenId='rar156-crc-'+randomUUID(),brokenOut=path.join(work,'损坏不发布')
  const failed=await invoke('extract_file',{taskId:brokenId,filePath:broken,outputPath:brokenOut,password:null,options})
  assert.equal(failed.ok,false);assert.ok(!(await events(brokenId)).some(e=>e.progress===1));assert.ok(!existsSync(brokenOut)||readdirSync(brokenOut).length===0)
  assert.ok(!readdirSync(work).some(n=>n.startsWith('.long-extract-')))
  cases.push({lateCrcFailure:true,finalError:failed.error,noFalseCompletion:true,noPublishedOrStagedOutput:true})
  const controlBase=path.join(root,'test-results/sfx-155-control'),control=JSON.parse(readFileSync(path.join(controlBase,'manifest.json'),'utf8'))
  const known=readFileSync(path.join(controlBase,'source/payload.bin')),blockSize=256*1024
  function staged(parent){
    for(const name of readdirSync(parent).filter(n=>n.startsWith('.long-extract-'))){
      const file=path.join(parent,name,'payload.bin');if(!existsSync(file))continue
      const logicalBytes=statSync(file).size,fd=openSync(file,'r');let lo=0,hi=Math.floor(logicalBytes/blockSize)
      try{while(lo<hi){const mid=Math.ceil((lo+hi)/2),offset=mid*blockSize-512,b=Buffer.alloc(512);if(readSync(fd,b,0,512,offset)===512&&b.equals(known.subarray(offset,offset+512)))lo=mid;else hi=mid-1}}finally{closeSync(fd)}
      return{file,actualPrefixBytes:lo*blockSize,logicalBytes}
    }return null
  }
  await click('[data-testid="nav-Decompress"]')
  for(const cancel of [false,true]){
    await driver.manage().window().setRect({width:1100,height:700});await bridge('clearTasks')
    const target=path.join(work,cancel?'取消不发布':'暂停恢复完成');mkdirSync(target)
    for(const m of control.members)copyFileSync(path.join(controlBase,m.name),path.join(target,m.name))
    await bridge('queueDesktopDialogSelections',[[path.join(target,control.archive)]]);await click('[data-testid="dropzone-file"]')
    const task=await driver.wait(async()=>(await bridge('extractionRecoveryTasks'))[0]||false,15000,'Imported real SFX queue task')
    // Receive the original files in an owned directory and use the user's
    // normal "same directory" flow, without a native folder-dialog mock.
    await click(`[data-task-id="${task.id}"]`)
    assert.equal(path.resolve(task.output),path.resolve(target))
    const subfolder=await driver.findElements(By.css('.task-subfolder-option'))
    if(await subfolder[0].getAttribute('aria-checked')==='true')await click(subfolder[0])
    // Collapse the details so the real row controls stay in view.
    await click(`[data-task-id="${task.id}"]`)
    await click('[data-testid="start-decompression-queue"]')
    try { await driver.wait(()=>{const p=staged(work);return p?.actualPrefixBytes>1024*1024?p:false},20000,'Actual known data before pause',25) }
    catch(error){writeFileSync(path.join(artifactDirectory,'rar156-control-failure.json'),JSON.stringify({tasks:await bridge('extractionRecoveryTasks'),history:await bridge('taskHistory'),events:await events(task.id),work,target,entries:readdirSync(work)},null,2));throw error}
    await click(`[data-testid="pause-archive-task-${task.id}"]`)
    const pauseStarted=Date.now();await driver.sleep(150)
    assert.equal(await bridge('taskStatus',task.id),'paused')
    const first=staged(work);assert.ok(first&&first.actualPrefixBytes>1024*1024&&first.actualPrefixBytes<known.length,JSON.stringify(first))
    const samples=await events(task.id),last=samples.filter(e=>e.processed_bytes>0).at(-1)
    assert.ok(last&&last.processed_bytes>0&&last.processed_bytes<known.length,JSON.stringify(last))
    assert.equal(first.logicalBytes,known.length,'Real preallocation must be present for this regression')
    assert.ok(last.processed_bytes<=first.actualPrefixBytes+4*1024*1024,'Decoded count cannot jump to unwritten allocation')
    assert.ok(last.processed_bytes>=first.actualPrefixBytes,'Pause flushes the latest decoded count, not an old throttled sample')
    for(const el of await driver.findElements(By.css('[aria-label="关闭成功提示"]')))await click(el)
    if(!cancel)await sizes('rar156-paused-actual-progress',`[data-task-id="${task.id}"]`)
    await driver.sleep(1000);assert.equal(staged(work).actualPrefixBytes,first.actualPrefixBytes)
    if(cancel){
      await click(`[data-testid="stop-archive-task-${task.id}"]`)
      await driver.wait(async()=>await bridge('taskStatus',task.id)==='cancelled',30000)
      await driver.wait(()=>!readdirSync(work).some(n=>n.startsWith('.long-extract-')),30000)
      assert.deepEqual(readdirSync(target).sort(),control.members.map(m=>m.name).sort())
      cases.push({actualQueueCancel:true,actualKnownBytesBeforePause:first.actualPrefixBytes,logicalBytes:first.logicalBytes,reportedBytes:last.processed_bytes,pauseStopsWriting:true,cancelWhilePaused:true,noStagedOrPublishedOutput:true})
    }else{
      await driver.manage().window().setRect({width:1100,height:700})
      await click(`[data-testid="resume-archive-task-${task.id}"]`);const pausedMs=Date.now()-pauseStarted
      await driver.wait(async()=>['completed','failed'].includes(await bridge('taskStatus',task.id)),60000)
      assert.equal(await bridge('taskStatus',task.id),'completed');assert.equal(sha(path.join(target,'payload.bin')),control.expected[0].sha256)
      const history=await driver.wait(async()=>(await bridge('taskHistory')).find(h=>h.id===task.id)||false,15000)
      const wallMs=Date.parse(history.completedAt)-Date.parse(history.startedAt),excludedMs=wallMs-history.durationMs
      assert.ok(excludedMs>=pausedMs-250&&excludedMs<=pausedMs+250,JSON.stringify({history,pausedMs,wallMs,excludedMs}))
      assert.equal(history.processedBytes,known.length)
      await sizes('rar156-completed-output',`[data-task-id="${task.id}"]`)
      await click('[data-testid="nav-History"]');await sizes('rar156-real-history','[data-testid="history-record-row"]');await click('[data-testid="nav-Decompress"]')
      cases.push({actualQueuePauseResumeComplete:true,actualKnownBytesBeforePause:first.actualPrefixBytes,logicalBytes:first.logicalBytes,reportedBytes:last.processed_bytes,reportedProgress:last.progress,pauseStopsWriting:true,pausedMs,wallMs,historyDurationMs:history.durationMs,excludedMs,outputSha256:control.expected[0].sha256})
    }
  }
  for(const m of control.members)assert.equal(sha(path.join(controlBase,m.name)),m.sha256)
  writeFileSync(path.join(artifactDirectory,'rar-156-results.json'),JSON.stringify({completedAt:new Date().toISOString(),application,binarySha256:sha(application),originals,control,cases,screenshots,layouts,events:await driver.executeScript(()=>window.__rar156Progress)},null,2)+'\n')
}

import assert from 'node:assert/strict'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,copyFileSync,existsSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import {createHash,randomBytes} from 'node:crypto'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runArchiveRecoveryUpgradeGate({driver,bridge,waitForElement,fixtureDirectory,artifactDirectory,bundledSevenZip,application,forwardContextAction}) {
  const tool=process.env.LONG_RAR_FIXTURE_TOOL
  assert.ok(tool && existsSync(tool),'Provide the isolated RAR fixture tool')
  const root=path.join(fixtureDirectory,'archive-recovery-148'); mkdirSync(root)
  const source=path.join(root,'原始资料'); mkdirSync(source)
  const payload=randomBytes(180000), text=Buffer.from('ZIP 分卷与 RAR 恢复卷应能继续完成解压。')
  writeFileSync(path.join(source,'payload.bin'),payload); writeFileSync(path.join(source,'说明.txt'),text)
  const password='中文 recovery + !', hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex')
  const command=(binary,args,cwd=source)=>{const r=spawnSync(binary,args,{cwd,windowsHide:true,timeout:60000,encoding:'utf8'});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr)}
  const click=async selector=>{const el=await waitForElement(selector);await driver.wait(()=>el.isEnabled(),15000);await driver.executeScript(el=>el.scrollIntoView({block:'center'}),el);await el.click()}
  const fill=async(selector,value)=>{const el=await waitForElement(selector);await driver.wait(()=>el.isEnabled(),15000);await driver.executeScript(el=>el.scrollIntoView({block:'center'}),el);await el.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await el.sendKeys(value)}
  const shot=async(name,selector)=>{
    for(const close of await driver.findElements(By.css('.toast-item button'))){try{await close.click()}catch{}}
    const el=await waitForElement(selector);await driver.executeScript(el=>el.scrollIntoView({block:'start'}),el);await new Promise(resolve=>setTimeout(resolve,500))
    const bounds=await driver.executeScript(el=>({client:el.clientWidth,scroll:el.scrollWidth}),el);assert.ok(bounds.scroll<=bounds.client+1,name+' overflows')
    writeFileSync(path.join(artifactDirectory,name+'.png'),Buffer.from(await driver.takeScreenshot(),'base64'))
  }
  // Use the real compression button and the new persisted option.
  const automatic=path.join(root,'应用创建');mkdirSync(automatic)
  await bridge('clearTasks');await bridge('clearCompressionWorkspace')
  await bridge('seedRarRecoveryDraft',path.join(source,'payload.bin'),automatic,{filename:'应用保护',password,encryptFilenames:true,level:0,splitArchive:true,splitSize:'0.0625',rarRecoveryVolumes:2})
  await click('[data-testid="nav-Compress"]')
  for(const width of [1100,760]){await driver.manage().window().setRect({width,height:700});await shot('recovery-options-'+width,'[data-testid="rar-recovery-settings"]')}
  await bridge('queueDesktopConfirmations',[true]);await click('[data-testid="start-compression"]')
  const created=await driver.wait(async()=>{const tasks=await bridge('compressionVerificationTaskState');if(tasks.some(t=>t.status==='failed'))throw Error(JSON.stringify(tasks));return tasks.length&&tasks.every(t=>t.status==='completed')?tasks[0]:false},60000)
  const savedHistory=await driver.wait(async()=>(await bridge('taskHistory')).find(item=>item.outputPath===created.outputPath&&item.executionConfig?.kind==='archive-compression')||false,15000)
  assert.equal(savedHistory.executionConfig.settings.rarRecoveryVolumes,2,'history must retain the recovery-volume option')
  const rarParts=readdirSync(automatic).filter(name=>/\.part\d+\.rar$/i.test(name)).sort((a,b)=>a.localeCompare(b,undefined,{numeric:true})).map(name=>path.join(automatic,name))
  const automaticRevs=readdirSync(automatic).filter(name=>/\.rev$/i.test(name));assert.ok(rarParts.length>1);assert.equal(automaticRevs.length,2)
  const originalRar=rarParts.map(file=>({file,hash:hash(file)}));command(tool,['t','-idq',`-p${password}`,rarParts[0]])
  console.log('[recovery-148] actual compression UI created data and recovery volumes')

  // A healthy existing set can acquire recovery files without recompression.
  await bridge('clearTasks');forwardContextAction('--repair-archive',[rarParts[1]])
  await waitForElement('.archive-password');await fill('.archive-password',password);await click('[data-testid="start-archive-diagnosis"]')
  await click('[data-testid="open-create-recovery-volumes"]')
  const additional=path.join(root,'补充恢复卷');mkdirSync(additional)
  await fill('[data-testid="create-recovery-directory"]',additional)
  await click('[data-testid="create-recovery-count"]')
  await driver.findElement(By.xpath('//button[@role="option" and contains(.,"2 个恢复卷")]')).click()
  await shot('recovery-create-dialog-760','[data-testid="create-recovery-volumes-dialog"]')
  await click('[data-testid="confirm-create-recovery"]');await waitForElement('[data-testid="created-recovery-volumes"]')
  const newRevs=readdirSync(additional);assert.equal(newRevs.length,2);assert.ok(newRevs.every(name=>name.endsWith('.rev')))
  await shot('recovery-created-760','[data-testid="create-recovery-volumes-dialog"]')
  await driver.findElement(By.xpath('//div[@role="dialog"]//button[normalize-space(.)="完成"]')).click()
  originalRar.forEach(item=>assert.equal(hash(item.file),item.hash))
  const missingRar=path.join(root,'缺失首卷');mkdirSync(missingRar)
  for(const file of rarParts.slice(1))copyFileSync(file,path.join(missingRar,path.basename(file)))
  for(const file of newRevs)copyFileSync(path.join(additional,file),path.join(missingRar,file))
  const selectedRev=path.join(missingRar,newRevs[0]), missingFirst=path.join(missingRar,path.basename(rarParts[0]))

  // Real 7-Zip local records, relocated to standard ZIP disks. Independent
  // 7-Zip verifies this layout before its central directory is removed.
  const cases=[]
  for(const mode of ['plain','AES256','ZipCrypto']) {
    const folder=path.join(root,'ZIP-'+mode);mkdirSync(folder)
    const single=path.join(folder,'完整.zip'), secret=mode==='plain'?null:password
    command(bundledSevenZip,['a','-tzip','-mm=Copy',...(secret?[`-mem=${mode}`,`-p${secret}`]:[]),single,'.\\*'])
    const bytes=readFileSync(single),end=bytes.length-22,central=bytes.readUInt32LE(end+16),limit=65536
    assert.equal(bytes.readUInt32LE(end),0x06054b50)
    const body=Buffer.concat([Buffer.from([80,75,7,8]),bytes.subarray(0,central)]),records=[]
    for(let at=central;at<end;) {
      const length=46+bytes.readUInt16LE(at+28)+bytes.readUInt16LE(at+30)+bytes.readUInt16LE(at+32),record=Buffer.from(bytes.subarray(at,at+length))
      assert.equal(record.readUInt32LE(0),0x02014b50)
      const old=record.readUInt32LE(42), absolute=old+4
      assert.equal(bytes.readUInt16LE(old+6)&8,0,'external encrypted fixtures must declare local sizes')
      record.writeUInt16LE(Math.floor(absolute/limit),34);record.writeUInt32LE(absolute%limit,42);records.push(record);at+=length
    }
    const directory=Buffer.concat(records),footer=Buffer.from(bytes.subarray(end)),lastDisk=Math.floor((body.length+directory.length+footer.length-1)/limit)
    assert.equal(Math.floor(body.length/limit),lastDisk,'fixture directory must fit its final disk')
    footer.writeUInt16LE(lastDisk,4);footer.writeUInt16LE(lastDisk,6);footer.writeUInt16LE(records.length,8);footer.writeUInt32LE(body.length%limit,16)
    const complete=Buffer.concat([body,directory,footer]),parts=[]
    for(let at=0,index=1;at<complete.length;at+=limit,index++){const file=path.join(folder,at+limit>=complete.length?'分包.zip':`分包.z${String(index).padStart(2,'0')}`);writeFileSync(file,complete.subarray(at,at+limit));parts.push(file)}
    command(bundledSevenZip,['t',parts.at(-1),...(secret?[`-p${secret}`]:[])])
    const damaged=Buffer.from(complete);damaged.fill(0,body.length)
    for(let index=0,at=0;index<parts.length;index++){const size=readFileSync(parts[index]).length;writeFileSync(parts[index],damaged.subarray(at,at+size));at+=size}
    cases.push({mode,parts,secret})
  }
  const streamed=path.join(root,'流式ZIP.zip')
  const python="import io,zipfile,sys\nclass Sink(io.BytesIO):\n def seek(self,*args): raise io.UnsupportedOperation()\ns=Sink()\nwith zipfile.ZipFile(s,'w',compression=zipfile.ZIP_DEFLATED) as z:\n z.write(sys.argv[1],arcname='payload.bin')\nopen(sys.argv[2],'wb').write(s.getvalue())"
  command(process.env.LONG_PYTHON_FIXTURE_TOOL||'python',['-c',python,path.join(source,'payload.bin'),streamed])
  command(bundledSevenZip,['t',streamed])
  const streamBytes=readFileSync(streamed),streamEnd=streamBytes.length-22
  assert.equal(streamBytes.readUInt16LE(6)&8,8);writeFileSync(streamed,streamBytes.subarray(0,streamBytes.readUInt32LE(streamEnd+16)))
  const healthy=path.join(root,'正常.zip');command(bundledSevenZip,['a','-tzip',healthy,'.\\*'])
  const missing=path.join(root,'缺卷ZIP');mkdirSync(missing)
  const plain=cases.find(item=>item.mode==='plain');copyFileSync(plain.parts.at(-1),path.join(missing,'分包.zip'))
  const aes=cases.find(item=>item.mode==='AES256'),crypto=cases.find(item=>item.mode==='ZipCrypto')
  const originals=[...cases.flatMap(item=>item.parts),streamed,...readdirSync(missingRar).map(name=>path.join(missingRar,name))].map(file=>({file,hash:hash(file)}))
  await bridge('clearTasks')
  forwardContextAction('--repair-archive',[aes.parts[0],aes.parts.at(-1),selectedRev,path.join(missingRar,path.basename(rarParts[1])),plain.parts.at(-1),crypto.parts.at(-1),streamed,healthy,path.join(missing,'分包.zip')])
  await waitForElement('[data-testid="archive-recovery-batch"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-batch-row"]'))).length===7,15000)
  await click('[data-testid="batch-diagnose"]')
  await driver.wait(async()=>!await driver.findElement(By.css('[data-testid="batch-diagnose"]')).getAttribute('disabled'),30000)
  const batchRows=()=>driver.findElements(By.css('[data-testid="archive-batch-row"]'))
  const rows=await batchRows();assert.equal(rows.length,7)
  await rows[0].findElement(By.css('[data-testid="archive-batch-expand"]')).click()
  await rows[0].findElement(By.xpath('.//button[normalize-space(.)="查看完整诊断"]')).click()
  await waitForElement('[data-testid="diagnostic-report"]')
  assert.equal(await driver.findElement(By.css('[data-testid="archive-recovery-batch"]')).isDisplayed(),false)
  await driver.findElement(By.xpath('//button[contains(.,"返回批量列表")]')).click()
  assert.equal((await batchRows()).length,7,'inspection must retain the batch results')
  for(let index=0;index<rows.length;index++){const checkbox=await rows[index].findElement(By.css('input[type="checkbox"]'));if((await checkbox.isSelected())!==[0,1,3].includes(index))await checkbox.click()}
  await click('[data-testid="batch-recovery-config"]')
  const config=await waitForElement('[data-testid="archive-batch-config-dialog"]'),toggles=await config.findElements(By.css('input[type="checkbox"]'))
  await toggles[0].click();await fill('[data-testid="archive-batch-shared-password"]',password)
  const saved=path.join(root,'批量恢复'),extracted=path.join(root,'统一解压');mkdirSync(saved);mkdirSync(extracted)
  await fill('[data-testid="archive-batch-output"]',saved);await toggles[1].click();await fill('input[placeholder="留空则使用各自原包目录"]',extracted)
  await shot('batch-config-dialog-760','[data-testid="archive-batch-config-dialog"]')
  await click('[data-testid="archive-batch-config-apply"]');await click('[data-testid="batch-diagnose"]')
  await driver.wait(async()=>!await driver.findElement(By.css('[data-testid="batch-diagnose"]')).getAttribute('disabled'),30000)
  for(const checkbox of await driver.findElements(By.css('[data-testid="archive-batch-row"] > div > input[type="checkbox"]'))){if(!await checkbox.isSelected())await checkbox.click()}
  for(const width of [1100,760]){await driver.manage().window().setRect({width,height:700});await shot('batch-diagnosis-'+width,'[data-testid="archive-recovery-batch"]')}
  await bridge('queueDesktopDialogSelections',[saved]);await click('[data-testid="batch-repair"]')
  await driver.wait(async()=>(await driver.findElements(By.css('[data-testid="archive-batch-result"]'))).length===5,60000)
  await driver.wait(async()=>!await driver.findElement(By.css('[data-testid="batch-diagnose"]')).getAttribute('disabled'),15000)
  await shot('batch-recovery-results-760','[data-testid="archive-recovery-batch"]')
  await driver.manage().window().setRect({width:760,height:520});await driver.executeScript(()=>document.documentElement.style.zoom='1.25')
  await shot('batch-recovery-small-125','[data-testid="archive-recovery-batch"]');await driver.executeScript(()=>document.documentElement.style.zoom='')
  await driver.manage().window().setRect({width:1100,height:700})
  await click('[data-testid="batch-queue-recovered"]')
  const queued=await bridge('extractionRecoveryTasks');assert.equal(queued.length,5);assert.equal(queued.filter(t=>t.hasPassword).length,3);assert.equal(queued.filter(t=>t.output.startsWith(extracted)).length,3)
  const encryptedZipDestinations=queued.filter(t=>t.hasPassword&&t.source.endsWith('.zip')).map(t=>t.output);assert.equal(new Set(encryptedZipDestinations).size,2,'same-name encrypted ZIP results need separate extraction directories')
  await click('[data-testid="nav-Decompress"]');await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async()=>{const tasks=await bridge('extractionRecoveryTasks');if(tasks.some(t=>t.status==='failed'))throw Error(JSON.stringify(tasks));return tasks.every(t=>t.status==='completed')},60000)
  const walk=folder=>readdirSync(folder,{withFileTypes:true}).flatMap(entry=>entry.isDirectory()?walk(path.join(folder,entry.name)):[path.join(folder,entry.name)])
  for(const task of queued){const files=walk(task.output);assert.ok(files.some(file=>path.basename(file)==='payload.bin'&&readFileSync(file).equals(payload)),'payload not found: '+task.source)}
  originals.forEach(item=>assert.equal(hash(item.file),item.hash));originalRar.forEach(item=>assert.equal(hash(item.file),item.hash));assert.equal(existsSync(missingFirst),false)
  writeFileSync(path.join(artifactDirectory,'archive-recovery-148-results.json'),JSON.stringify({application,binarySha256:hash(application),externalToolSha256:hash(tool),uiCompressionCreatedRecoveryVolumes:true,existingRarRecoveryCreation:true,standardZipLostDirectoryModes:cases.map(item=>item.mode),streamedDeflateRescue:true,selectedMembers:9,groupedArchives:7,recoveredGroups:5,queuedAndExtracted:5,passwordAndOutputRetained:true,originalsUnchanged:true,nativePickerVerified:false},null,2))
}

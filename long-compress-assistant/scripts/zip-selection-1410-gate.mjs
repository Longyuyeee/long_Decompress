import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, existsSync, readdirSync, openSync, closeSync, ftruncateSync, createReadStream } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { createHash, randomBytes } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runZipSelection1410Gate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application, root: projectRoot }) {
  const root = path.join(fixtureDirectory, 'zip-selection-1410'); mkdirSync(root)
  const command = (tool, args, options = {}) => {
    const result = spawnSync(tool, args, { encoding: 'utf8', windowsHide: true, timeout: 300_000, ...options })
    assert.ifError(result.error); assert.equal(result.status, 0, `${args.join(' ')}: ${result.stdout}\n${result.stderr}`)
    return result.stdout
  }
  const fill = async (selector, value) => {
    const el = await waitForElement(selector)
    await el.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (value) await el.sendKeys(value)
  }
  const click = async selector => {
    const el = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), el)
    await driver.wait(async () => driver.executeScript(el => {
      const r = el.getBoundingClientRect(), hit = document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)
      return !el.disabled && (hit === el || el.contains(hit)) && !el.getAnimations({subtree:true}).some(a=>a.playState==='running')
    }, el), 10000)
    await el.click()
  }
  const navigateBrowser = async () => {
    if ((await driver.findElements(By.css('.browser-page'))).length) return
    // Use the product's documented Ctrl+B shortcut, not injected route state.
    await driver.actions().keyDown(Key.CONTROL).sendKeys('b').keyUp(Key.CONTROL).perform()
    await waitForElement('[data-testid="file-manager-open-archive"]')
  }
  const open = async (file, output) => {
    await navigateBrowser()
    if ((await driver.findElements(By.css('.browser-page'))).length) await fill('.browser-toolbar input[type="password"]', '')
    await bridge('queueDesktopDialogSelections', [file, output])
    const localChooser = await driver.findElements(By.css('[data-testid="file-manager-open-archive"]'))
    await click(localChooser.length ? '[data-testid="file-manager-open-archive"]' : '.browser-page > header .browser-primary')
    await driver.wait(async () => {
      const chooser = await driver.findElement(By.css('.browser-toolbar .browser-field button'))
      return await chooser.getAttribute('title') === file && (await driver.findElements(By.css('.browser-summary'))).length === 1
    }, 30_000)
    await click('.browser-page > footer .output-target')
    await driver.wait(async () => (await driver.findElement(By.css('.output-target')).getText()).includes(output), 10000)
  }
  const choose = async query => {
    await fill('.browser-search input', '.txt')
    await driver.wait(async () => (await driver.findElements(By.css('.browser-row:not(.directory)'))).length === 3, 10000)
    const all = await waitForElement('.browser-table-head .browser-checkbox')
    if (!(await all.getAttribute('class')).includes('checked')) await click('.browser-table-head .browser-checkbox')
    await click('.browser-table-head .browser-checkbox')
    await fill('.browser-search input', query)
    await driver.wait(async () => (await driver.findElements(By.css('.browser-row:not(.directory)'))).length === 1, 10000)
    await click('.browser-row:not(.directory) .browser-checkbox')
    assert.match(await (await waitForElement('.browser-page > footer')).getText(), /已选择\s+1\s*\/\s*3/)
  }
  const extract = async expected => {
    const before = (await bridge('archiveBrowserTaskState')).length
    await click('.browser-page > footer .browser-primary')
    const task = await driver.wait(async () => {
      const tasks = await bridge('archiveBrowserTaskState')
      return tasks.length > before && ['completed','failed','cancelled'].includes(tasks.at(-1).status) ? tasks.at(-1) : false
    }, 60_000)
    assert.equal(task.status, expected, JSON.stringify(task))
    return task
  }
  const results = [], screenshots = []
  const shot = async name => { const file = path.join(artifactDirectory, `${name}.png`); writeFileSync(file, Buffer.from(await driver.takeScreenshot(), 'base64')); screenshots.push(path.basename(file)) }
  await bridge('clearTasks')
  for (const encryption of ['AES256', 'ZipCrypto']) {
    const source = path.join(root, encryption); mkdirSync(source)
    mkdirSync(path.join(source,'甲方资料')); mkdirSync(path.join(source,'乙方资料'))
    writeFileSync(path.join(source,'公开说明.txt'), '我只需要这份公开说明\n')
    writeFileSync(path.join(source,'甲方资料','合同.txt'), '甲方合同，使用甲方密码\n')
    writeFileSync(path.join(source,'乙方资料','报价.txt'), randomBytes(180_000))
    const file = path.join(root, `${encryption}-两种密码混合.zip`)
    command(bundledSevenZip, ['a','-tzip',file,'公开说明.txt'], {cwd:source})
    command(bundledSevenZip, ['a','-tzip',`-mem=${encryption}`,'-palpha-password',file,'甲方资料'], {cwd:source})
    command(bundledSevenZip, ['a','-tzip',`-mem=${encryption}`,'-pbeta-password',file,'乙方资料'], {cwd:source})
    const original = createHash('sha256').update(readFileSync(file)).digest('hex')
    const output = path.join(root, `${encryption}-公开文件`); mkdirSync(output)
    await open(file, output); await choose('公开说明')
    await click('.browser-row:not(.directory) .preview-trigger')
    const preview = await waitForElement('.preview-text')
    await driver.wait(async () => (await preview.getText()).includes('我只需要这份公开说明'), 10000)
    await shot(`zip-selection-${encryption}-public-preview`)
    await click('.preview-close')
    await extract('completed')
    assert.equal(readFileSync(path.join(output,'公开说明.txt'),'utf8'), '我只需要这份公开说明\n')
    assert.equal(readdirSync(output).length, 1, 'unselected encrypted files must never be materialized or published')
    await shot(`zip-selection-${encryption}-public`)

    const alphaOutput = path.join(root, `${encryption}-合同`); mkdirSync(alphaOutput)
    await bridge('queueDesktopDialogSelections',[alphaOutput]); await click('.output-target')
    await choose('合同'); await fill('.browser-toolbar input[type="password"]','wrong-password')
    const wrong = await extract('failed'); assert.match(wrong.error, /密码/)
    await shot(`zip-selection-${encryption}-wrong-password`)
    await fill('.browser-toolbar input[type="password"]','alpha-password')
    await extract('completed')
    assert.equal(readFileSync(path.join(alphaOutput,'甲方资料','合同.txt'),'utf8'), '甲方合同，使用甲方密码\n')
    assert.equal(existsSync(path.join(alphaOutput,'乙方资料')), false)
    assert.equal(existsSync(path.join(alphaOutput,'公开说明.txt')), false)
    assert.equal((await bridge('archiveBrowserTaskState')).filter(t=>t.status==='failed').length >= 1, true, 'the original failure must remain visible in history')
    if (encryption === 'AES256') {
      await bridge('seedVaultPassword','乙方密码','beta-password')
      await bridge('seedVaultPassword','甲方密码','alpha-password')
      const automatic = path.join(root,'保险箱只解锁甲方'); mkdirSync(automatic)
      await bridge('queueDesktopDialogSelections',[automatic]); await click('.output-target')
      await fill('.browser-toolbar input[type="password"]',''); await extract('completed')
      assert.equal(readFileSync(path.join(automatic,'甲方资料','合同.txt'),'utf8'),'甲方合同，使用甲方密码\n')
      for (const [width,height] of [[1100,700],[760,520]]) {
        await driver.manage().window().setRect({width,height})
        await driver.wait(()=>driver.executeScript('return document.querySelector(".browser-page").getAnimations({subtree:true}).every(a=>a.playState!=="running")'),5000)
        const layout = await driver.executeScript('const p=document.querySelector(".browser-page");return {width:p.clientWidth,scroll:p.scrollWidth}')
        assert.ok(layout.scroll <= layout.width+1,JSON.stringify(layout))
        await shot(`zip-selection-workspace-${width}x${height}`)
        // The existing narrow layout deliberately scrolls. Check the user's
        // real action: scroll to the footer and reach the extraction button.
        const footer = await waitForElement('.browser-page > footer .browser-primary')
        await driver.executeScript(el=>el.scrollIntoView({block:'center',behavior:'instant'}),footer)
        const reachable=await driver.executeScript(el=>{const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return r.top>=0&&r.bottom<=innerHeight+1&&(hit===el||el.contains(hit))},footer)
        assert.ok(reachable,'narrow-window extraction action must be reachable by scrolling')
        const obscured = await driver.executeScript('const a=document.querySelector("[data-testid=return-to-file-manager]").getBoundingClientRect(),b=document.querySelector(".browser-search").getBoundingClientRect();return a.top<b.bottom&&a.bottom>b.top&&a.left<b.right&&a.right>b.left')
        assert.equal(obscured,false,'return-to-files action must not float over the search field after scrolling')
        await shot(`zip-selection-footer-${width}x${height}`)
        if(width===760 && process.env.LONG_ZIP1410_VIEW_PAUSE==='1') await new Promise(resolve=>setTimeout(resolve,30000))
      }
      await driver.manage().window().setRect({width:1100,height:700})
    }
    // A numeric set with mixed entry passwords must retain the same behavior.
    const bytes=readFileSync(file), first=path.join(root, `${encryption}-mixed.zip.001`)
    for(let offset=0,index=1;offset<bytes.length;offset+=65536,index++) writeFileSync(path.join(root,`${encryption}-mixed.zip.${String(index).padStart(3,'0')}`),bytes.subarray(offset,offset+65536))
    const numericOutput=path.join(root,`${encryption}-numeric-selected`);mkdirSync(numericOutput)
    await open(first,numericOutput);await choose('合同');await fill('.browser-toolbar input[type="password"]','alpha-password');await extract('completed')
    assert.equal(readFileSync(path.join(numericOutput,'甲方资料','合同.txt'),'utf8'),'甲方合同，使用甲方密码\n')
    assert.equal(createHash('sha256').update(readFileSync(file)).digest('hex'),original)
    results.push({encryption,publicWithoutPassword:true,wrongPasswordRejected:true,correctedSelectionCompleted:true,vaultSelection:encryption==='AES256',numericMixedPasswordSelection:true,originalUnchanged:true})
  }

  const exported = path.join(root, 'zstd-fixtures')
  console.log('[zip-selection] exporting encrypted Zstandard fixtures for independent verification')
  command('cargo',['test','--manifest-path','src-tauri/Cargo.toml','--lib','export_zip_1410_compatibility_fixtures','--','--ignored','--nocapture'],{cwd:projectRoot,env:{...process.env,LONG_ZIP1410_FIXTURE_ROOT:exported}})
  const manifest=JSON.parse(readFileSync(path.join(exported,'manifest.json'),'utf8'))
  for (const item of manifest.cases) {
    command(bundledSevenZip,['t',`-p${item.password}`,item.file])
    const output=path.join(exported,`out-${item.encryption}-${item.layout}`)
    await bridge('extractArchive',item.file,output,item.password)
    assert.deepEqual(readFileSync(path.join(output,...item.entry.split('/'))),readFileSync(path.join(exported,'expected.bin')))
    results.push({codec:'Zstandard',encryption:item.encryption,layout:item.layout,producer:manifest.producer,independentSevenZipVerified:true,applicationFullExtraction:true})
  }

  // Preserve the old CLI route's legacy ZIP coverage when ZIP now shares the
  // native selected-entry reader. These are upstream fixed archives, not data
  // relabelled with a different method number.
  const entries = folder => readdirSync(folder,{withFileTypes:true}).flatMap(e=>e.isDirectory()?entries(path.join(folder,e.name)).map(f=>path.join(e.name,f)):[e.name]).sort()
  const hashes = {
    'shrink.zip':'04d2b9534d3d0a07ae2fda191a464b32bae516a4b9471be29120755431faddf4',
    'reduce.zip':'bd76c104ed775b189a1ebf25f1f5d7f4a1cff42e01ef66d2af570ddba6f8d2f6',
    'implode.zip':'36ebf1dc4833767728e1cabb99aba83137931638a6b07754d437a3adefc7984a',
    'first.txt':'7fa9e80fcfc8ef32d3e08d88b85730803da855affea2d1ec51f08a4b01f171e7',
  }
  const pinned = async (name,url) => {
    const cached=path.join(process.env.LONG_ZIP1410_LEGACY_FIXTURES || path.join(projectRoot,'test-results','zip1410-legacy-probe'),name)
    let bytes
    if(existsSync(cached)) bytes=readFileSync(cached)
    else {const response=await fetch(url,{signal:AbortSignal.timeout(30000)});assert.equal(response.status,200,url);bytes=Buffer.from(await response.arrayBuffer())}
    assert.equal(createHash('sha256').update(bytes).digest('hex'),hashes[name],`pinned upstream fixture: ${name}`)
    return bytes
  }
  for(const codec of ['shrink','reduce','implode']) {
    const url=`https://raw.githubusercontent.com/zip-rs/zip2/fd7c9b1ec5c06107ffbce435a30478d8ba19a41c/tests/data/legacy/${codec}.zip`
    const bytes=await pinned(`${codec}.zip`,url),file=path.join(root,`${codec}.zip`)
    writeFileSync(file,bytes)
    const expected=path.join(root,`${codec}-independent`),output=path.join(root,`${codec}-native`)
    await bridge('extractArchive',file,output,null)
    let count
    if(codec==='reduce') {
      // 7-Zip 26.04 explicitly reports Unsupported Method for Reduce. Use
      // the upstream test's independently published original bytes instead.
      const rawUrl='https://raw.githubusercontent.com/zip-rs/zip2/fd7c9b1ec5c06107ffbce435a30478d8ba19a41c/tests/data/folder/first.txt'
      assert.deepEqual(entries(output),['first.txt'])
      assert.deepEqual(readFileSync(path.join(output,'first.txt')),await pinned('first.txt',rawUrl))
      count=1
    } else {
      command(bundledSevenZip,['t',file]);command(bundledSevenZip,['x','-y',`-o${expected}`,file])
      const expectedNames=entries(expected);assert.ok(expectedNames.length);count=expectedNames.length
      assert.deepEqual(entries(output),expectedNames)
      for(const name of expectedNames) assert.deepEqual(readFileSync(path.join(output,name)),readFileSync(path.join(expected,name)))
    }
    results.push({codec,sourceUrl:url,sha256:createHash('sha256').update(bytes).digest('hex'),independentSevenZipVerified:codec!=='reduce',publishedOriginalBytesVerified:codec==='reduce',applicationFullExtraction:true,files:count,passwordVariantsTested:false})
  }

  if (process.env.LONG_ZIP1410_REAL_ZIP64 === '1') {
    const large=path.join(root,'real-zip64');mkdirSync(large)
    const length=2**32+12345, largeFile=path.join(large,'大文件.bin')
    const fd=openSync(largeFile,'wx');ftruncateSync(fd,length);closeSync(fd)
    writeFileSync(path.join(large,'末尾说明.txt'),'大文件之后的说明不能丢失\n')
    const hash=async file=>{const h=createHash('sha256');for await(const chunk of createReadStream(file))h.update(chunk);return h.digest('hex')}
    const originalHash=await hash(largeFile)
    for(const encryption of ['AES256','ZipCrypto']) {
      console.log(`[zip-selection] real >4GiB ZIP64 ${encryption}: external creation, full verification and application extraction`)
      const archive=path.join(root,`real-zip64-${encryption}.zip`),password='large-zip-password'
      command(bundledSevenZip,['a','-tzip','-mm=Copy',`-mem=${encryption}`,`-p${password}`,'-v1g',archive,'大文件.bin','末尾说明.txt'],{cwd:large})
      command(bundledSevenZip,['t',`-p${password}`,`${archive}.001`])
      const output=path.join(root,`real-zip64-${encryption}-out`)
      // Full application extraction checks the large entry as well as the entry after it.
      await bridge('extractArchive',`${archive}.001`,output,password)
      assert.equal(await hash(path.join(output,'大文件.bin')),originalHash)
      assert.equal(readFileSync(path.join(output,'末尾说明.txt'),'utf8'),'大文件之后的说明不能丢失\n')
      results.push({realZip64:true,inputBytes:length,encryption,layout:'numeric',independentProducer:'7-Zip 26.04 Copy ZIP',independentFullVerification:true,applicationFullExtraction:true,largePayloadSha256:originalHash,followingEntryPreserved:true})
    }
  }
  await shot('zip-selection-final-workspace')
  const evidence={version:'1.4.10',binarySha256:createHash('sha256').update(readFileSync(application)).digest('hex'),results,screenshots,complete:true}
  writeFileSync(path.join(artifactDirectory,'zip-selection-1410-results.json'),JSON.stringify(evidence,null,2))
  console.log('[zip-selection] all requested user workflows and compatibility cases passed')
}

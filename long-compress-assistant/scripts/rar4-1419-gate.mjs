import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runRar41419Gate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, application, root, forwardContextAction }) {
  const original = path.join(root, 'tests/fixtures/archives/rar4-recovery-1419')
  const manifest = JSON.parse(readFileSync(path.join(original, 'manifest.json'), 'utf8'))
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const tree = directory => readdirSync(directory, { withFileTypes: true }).flatMap(e => e.isDirectory()
    ? tree(path.join(directory, e.name)).map(p => ({ ...p, path: e.name + '/' + p.path }))
    : [{ path: e.name, sha256: hash(path.join(directory, e.name)) }]).sort((a, b) => a.path.localeCompare(b.path))
  const expected = manifest.source.map(p => ({ path: p.name.replaceAll('\\', '/'), sha256: p.sha256 })).sort((a,b) => a.path.localeCompare(b.path))
  const password = manifest.publicPassword
  const tool = path.resolve(process.env.LONG_RAR_FIXTURE_TOOL || path.join(root, 'test-results/archive-upgrade-tools/winrar/Rar.exe'))
  const cases = [], screenshots = []
  const work = path.join(fixtureDirectory, 'RAR4旧格式恢复'); mkdirSync(work)
  const copyCase = (name, suffix) => {
    const dest = path.join(work, name + '-' + suffix); mkdirSync(dest)
    for (const file of readdirSync(path.join(original, name))) copyFileSync(path.join(original, name, file), path.join(dest, file))
    return dest
  }
  const independent = (source, secret, destination) => {
    mkdirSync(destination)
    const r = spawnSync(tool, ['x', '-idq', '-y', '-cfg-', secret ? '-p' + secret : '-p-', source, destination + path.sep], { windowsHide: true, encoding: 'utf8', timeout: 30000 })
    assert.ifError(r.error); assert.equal(r.status, 0, r.stdout + r.stderr)
    assert.deepEqual(tree(destination), expected)
    assert.deepEqual(readdirSync(path.join(destination, '中文目录/空目录')), [])
  }
  for (const style of ['single', 'old', 'modern']) for (const mode of ['none', 'content', 'headers']) {
    const name = `${style}-${mode}`, dir = copyCase(name, '损坏后另存')
    for (const f of readdirSync(dir)) if (f.endsWith('.rev')) unlinkSync(path.join(dir, f))
    const first = path.join(dir, style === 'modern' ? '原始归档.part1.rar' : '原始归档.rar')
    const damaged = style === 'single' ? first : path.join(dir, style === 'old' ? '原始归档.r00' : '原始归档.part2.rar')
    const bytes = readFileSync(damaged); bytes[3000] ^= 0x55; writeFileSync(damaged, bytes)
    const before = tree(dir), secret = mode === 'none' ? undefined : password
    const report = await bridge('diagnoseArchive', first, secret); assert.equal(report.canRepair, true)
    const destination = path.join(work, name + '-修复'); mkdirSync(destination)
    const repaired = await bridge('repairRar', first, destination, secret)
    assert.equal(repaired.verified, true); assert.equal(repaired.outputEncrypted, mode !== 'none')
    independent(repaired.outputPath, secret, path.join(work, name + '-独立回解'))
    const extracted = path.join(work, name + '-应用回解'); mkdirSync(extracted)
    await bridge('extractArchive', repaired.outputPath, extracted, secret); assert.deepEqual(tree(extracted), expected)
    assert.deepEqual(tree(dir), before); cases.push({ name, kind: 'recovery-record', originalUnchanged: true, independentAndApplicationContentMatch: true })
  }
  for (const style of ['old', 'modern']) for (const mode of ['none', 'content', 'headers']) {
    const name = `${style}-${mode}`, dir = copyCase(name, '缺末卷')
    const tail = path.join(dir, style === 'old' ? '原始归档.r02' : '原始归档.part4.rar'); unlinkSync(tail)
    const rev = path.join(dir, style === 'old' ? '原始归档1.rev' : '原始归档.part1.rev')
    const secret = mode === 'none' ? undefined : password, before = tree(dir)
    const report = await bridge('diagnoseArchive', rev, 'wrong'); assert.equal(report.status, 'missing_volume'); assert.equal(report.canRepair, true)
    const destination = path.join(work, name + '-补末'); mkdirSync(destination)
    const r = await bridge('repairRar', rev, destination, secret)
    assert.equal(r.outputFiles.length, 4); independent(r.outputPath, secret, path.join(work, name + '-补末回解'))
    assert.equal(existsSync(tail), false); assert.deepEqual(tree(dir), before)
    cases.push({ name, kind: 'missing-tail', structuralErrorBeforePassword: true, independentContentMatch: true, originalUnchanged: true })
  }
  const click = async selector => {
    const el = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block:'center', behavior:'instant' }), el)
    await driver.wait(() => el.isEnabled(), 10000); await el.click()
  }
  const fill = async (selector, value) => {
    const el = await waitForElement(selector); await driver.wait(() => el.isEnabled(), 10000)
    await driver.executeScript(el => el.scrollIntoView({block:'center', behavior:'instant'}), el)
    await el.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE, value)
  }
  const sizes = async (name, anchor) => {
    for (const [width, height] of [[1100,700],[760,520]]) {
      await driver.manage().window().setRect({width,height})
      await driver.wait(() => driver.executeScript(w => Math.abs(innerWidth-w)<3, width),10000)
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
      const el = await waitForElement(anchor)
      await driver.executeScript(el => el.scrollIntoView({block:'center', behavior:'instant'}), el); await driver.sleep(200)
      await driver.executeAsyncScript('const done=arguments[arguments.length-1]; requestAnimationFrame(()=>requestAnimationFrame(done))')
      const overflow = await driver.executeScript(() => document.documentElement.scrollWidth-innerWidth); assert.ok(overflow<=1)
      const file = `${name}-${width}x${height}.png`; screenshots.push(file)
      writeFileSync(path.join(artifactDirectory,file), Buffer.from(await driver.takeScreenshot(),'base64'))
    }
    await driver.manage().window().setRect({width:1100,height:700})
  }
  // User receives an encrypted old-style set without its first disk.
  const dir = copyCase('old-headers', '用户缺首卷'); unlinkSync(path.join(dir,'原始归档.rar'))
  const selected = path.join(dir, '原始归档.r00'), kept = tree(dir)
  const output = path.join(work,'用户指定解压目录'); mkdirSync(output)
  await bridge('clearTasks'); await failure('extractArchive',selected,output,password)
  const [failed] = await bridge('extractionRecoveryTasks'); assert.equal(failed.status,'failed')
  await click('[data-testid="nav-Decompress"]'); await click(`[data-task-id="${failed.id}"]`)
  await fill('[data-testid="retry-output"]',output); await fill('[data-testid="retry-password"]',password)
  await click('[data-testid="retry-rescue"]'); await waitForElement('[data-testid="start-archive-repair"]')
  await sizes('rar4-missing-first','[data-testid="start-archive-repair"]')
  const destination = path.join(work,'用户另存修复包'); mkdirSync(destination)
  await bridge('queueDesktopDialogSelections',[destination]); await click('[data-testid="start-archive-repair"]')
  const panel = await waitForElement('[data-testid="repair-result"]')
  assert.match(await panel.getText(),/保留原加密方式与密码/)
  const recovered = await panel.findElement(By.css('p')).getText()
  independent(recovered,password,path.join(work,'用户结果独立回解'))
  await sizes('rar4-repaired-result','[data-testid="queue-recovered-archive"]')
  await click('[data-testid="queue-recovered-archive"]')
  const queued = await driver.wait(async()=> (await bridge('extractionRecoveryTasks')).find(t=>t.status==='pending')||false,10000)
  assert.equal(queued.retryOfTaskId,failed.id); assert.equal(queued.output,output); assert.ok(queued.hasPassword)
  await click('[data-testid="nav-Decompress"]'); await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async()=>await bridge('taskStatus',queued.id)==='completed',30000)
  const files = tree(output)
  for (const p of expected) assert.ok(files.some(f=>f.path.endsWith(p.path)&&f.sha256===p.sha256),JSON.stringify({files,p}))
  assert.equal((await bridge('extractionRecoveryTasks')).find(t=>t.id===failed.id).status,'failed')
  assert.deepEqual(tree(dir),kept); assert.equal(existsSync(path.join(dir,'原始归档.rar')),false)
  // The real healthy RAR4 page exposes recovery-volume creation as before.
  const fresh = copyCase('old-headers','用户新建恢复卷')
  for (const f of readdirSync(fresh)) if(f.endsWith('.rev')) unlinkSync(path.join(fresh,f))
  const freshBefore = tree(fresh)
  forwardContextAction('--repair-archive', [path.join(fresh,'原始归档.rar')])
  await fill('.archive-password',password); await click('[data-testid="start-archive-diagnosis"]')
  await click('[data-testid="open-create-recovery-volumes"]')
  const saved = path.join(work,'用户创建的恢复卷'); mkdirSync(saved)
  await fill('[data-testid="create-recovery-directory"]',saved)
  await sizes('rar4-create-recovery','[data-testid="confirm-create-recovery"]')
  await click('[data-testid="confirm-create-recovery"]')
  await waitForElement('[data-testid="created-recovery-volumes"]')
  assert.equal(readdirSync(saved).filter(f=>f.endsWith('.rev')).length,1)
  assert.deepEqual(tree(fresh),freshBefore)
  await sizes('rar4-created-recovery','[data-testid="created-recovery-volumes"]')
  await driver.actions().sendKeys(Key.ESCAPE).perform()
  const invalid = copyCase('old-headers','损坏恢复卷')
  unlinkSync(path.join(invalid,'原始归档.rar')); unlinkSync(path.join(invalid,'原始归档.r00'))
  for (const f of readdirSync(invalid)) if(f.endsWith('.rev')) writeFileSync(path.join(invalid,f),'short')
  forwardContextAction('--repair-archive',[path.join(invalid,'原始归档.r01')])
  await driver.wait(async()=> (await (await waitForElement('[data-testid="diagnostic-report"]')).getText()).includes('已忽略无效恢复卷'),10000)
  assert.equal((await driver.findElements(By.css('[data-testid="start-archive-repair"]'))).length,0)
  await sizes('rar4-invalid-recovery','[data-testid="diagnostic-report"] .diagnostic-issue')
  writeFileSync(path.join(artifactDirectory,'rar4-1419-results.json'),JSON.stringify({completedAt:new Date().toISOString(),application,binarySha256:hash(application),tool,toolSha256:hash(tool),cases,screenshots,userFailureRepairQueueExtraction:true,originalFailureRetained:true,modifiedOutputAndPasswordRetained:true,userCreatedRar4RecoveryVolume:true,originalFilesUnchanged:true},null,2))
}

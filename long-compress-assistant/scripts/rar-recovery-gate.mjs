import assert from 'node:assert/strict'
import { mkdirSync, readFileSync, writeFileSync, readdirSync, unlinkSync, existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { randomBytes, createHash } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runRarRecoveryGate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application, forwardContextAction }) {
  const tool = process.env.LONG_RAR_FIXTURE_TOOL
  assert.ok(tool && existsSync(tool), 'Provide the external RAR fixture tool without installing or bundling it')
  const root = path.join(fixtureDirectory, 'rar-recovery-147'); mkdirSync(root)
  const source = path.join(root, '资料'); mkdirSync(source)
  const payload = randomBytes(180_000); writeFileSync(path.join(source, 'payload.bin'), payload)
  const text = Buffer.from('RAR 修复后应保留中文说明与输出配置。'); writeFileSync(path.join(source, '说明.txt'), text)
  const password = '中文 recovery + !'
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const command = (binary, args, cwd = source) => {
    const result = spawnSync(binary, args, { cwd, windowsHide: true, timeout: 60_000, encoding: 'utf8' })
    assert.ifError(result.error); assert.equal(result.status, 0, result.stdout + result.stderr)
  }
  const click = async selector => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center' }), element); await element.click()
  }
  const fill = async (selector, value) => {
    const element = await waitForElement(selector); await driver.wait(() => element.isEnabled(), 15000); await driver.executeScript(el => el.scrollIntoView({block:'center'}), element); await element.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE, value)
  }
  const shot = async (name, selector) => {
    for (const button of await driver.findElements(By.css('.toast-item button'))) { try { await button.click() } catch {} }
    const element = await waitForElement(selector); await driver.executeScript(el => el.scrollIntoView({ block: 'start' }), element)
    await new Promise(resolve => setTimeout(resolve, 500))
    const bounds = await driver.executeScript(el => ({client:el.clientWidth,scroll:el.scrollWidth}), element)
    assert.ok(bounds.scroll <= bounds.client + 1, name + ' overflow')
    writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  // The application's conversion path, not the external tool, must create the recovery record.
  const zip = path.join(root, '来源.zip')
  command(bundledSevenZip, ['a', '-tzip', zip, '.\\*'])
  const rar = path.join(root, '应用生成.rar')
  await bridge('convertArchive', zip, null, rar, { format:'rar', level:1, password,
    split_size:null, encrypt_filenames:true, rar_recovery_percent:5,
    preserve_paths:true, delete_after:false, verify_after:true, allow_insecure_password_cli:true })
  const damaged = readFileSync(rar); assert.ok(damaged.length > 4000); damaged[3000] ^= 0x55; writeFileSync(rar, damaged)
  const sourceHash = hash(rar)
  const recoveredAt = path.join(root, '修复结果'); mkdirSync(recoveredAt)
  await bridge('clearTasks')
  const failedOutput = path.join(root, '原输出'); mkdirSync(failedOutput)
  const damagedFailure = await failure('extractArchive', rar, failedOutput, password)
  assert.match(damagedFailure, /ambiguous-data/, 'damaged encrypted RAR must not be called a proven wrong password')
  const [failed] = await bridge('extractionRecoveryTasks'); assert.equal(failed.status,'failed')
  await click('[data-testid="nav-Decompress"]'); await click(`[data-task-id="${failed.id}"]`)
  await waitForElement('[data-testid="inline-extraction-recovery"]')
  const editedOutput = path.join(root,'修改后的输出'); mkdirSync(editedOutput)
  await fill('[data-testid="retry-output"]',editedOutput); await fill('[data-testid="retry-password"]',password)
  await driver.manage().window().setRect({width:760,height:700})
  await shot('rar-failure-inline-760','[data-testid="inline-extraction-recovery"]')
  await click('[data-testid="retry-rescue"]'); await waitForElement('[data-testid="start-archive-repair"]')
  await bridge('queueDesktopDialogSelections',[recoveredAt]); await click('[data-testid="start-archive-repair"]')
  const resultPanel = await waitForElement('[data-testid="repair-result"]')
  assert.ok((await resultPanel.getText()).includes('保留原加密方式与密码'))
  const repaired = {outputPath:await resultPanel.findElement(By.css('p')).getText()}
  command(tool, ['t', '-idq', `-p${password}`, repaired.outputPath])
  await click('[data-testid="queue-recovered-archive"]')
  const retry = await driver.wait(async () => (await bridge('extractionRecoveryTasks')).find(task => task.status==='pending') || false,10000)
  assert.equal(retry.output,editedOutput); assert.equal(retry.retryOfTaskId,failed.id); assert.ok(retry.hasPassword)
  await click('[data-testid="nav-Decompress"]'); await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async () => await bridge('taskStatus',retry.id)==='completed',30000)
  const wrongAt = path.join(root, '错误密码'); mkdirSync(wrongAt)
  await failure('repairRar', rar, wrongAt, 'wrong')
  assert.deepEqual(readdirSync(wrongAt), []); assert.equal(hash(rar), sourceHash)
  console.log('[rar-recovery] application-created recovery record, inline correction, repair and extraction passed')

  // Send the actual Explorer CLI flag to the running isolated instance.
  await bridge('clearTasks')
  const archive = path.join(root, '分卷.rar')
  command(tool, ['a', '-idq', '-y', '-cfg-', '-m0', '-v60000b', `-hp${password}`, archive, '.\\*'])
  const first = path.join(root, '分卷.part1.rar')
  command(tool, ['rv2', '-idq', '-y', '-cfg-', `-p${password}`, first])
  const rev = path.join(root, '分卷.part1.rev')
  unlinkSync(first)
  const originals = readdirSync(root).filter(name => /^分卷\.part\d+\.(rar|rev)$/.test(name)).map(name => ({name,hash:hash(path.join(root,name))}))
  forwardContextAction('--repair-archive', [rev, rar])
  await waitForElement('[data-testid="repair-source-selection"]')
  await fill('.archive-password', password)
  await click('[data-testid="start-archive-diagnosis"]')
  await driver.wait(async () => (await driver.findElement(By.css('[data-testid="diagnostic-report"]')).getText()).includes('恢复卷补齐'), 15_000)
  for (const width of [1100, 760]) {
    await driver.manage().window().setRect({width,height:700})
    await shot('rar-recovery-diagnosis-' + width, '[data-testid="archive-diagnostic-panel"]')
  }
  const uiSaved = path.join(root, '页面补卷'); mkdirSync(uiSaved)
  await bridge('queueDesktopDialogSelections', [uiSaved])
  await click('[data-testid="start-archive-repair"]')
  await waitForElement('[data-testid="repair-result"]')
  await shot('rar-recovery-result-760', '[data-testid="repair-result"]')
  await click('[data-testid="queue-recovered-archive"]')
  const queued = await driver.wait(async () => (await bridge('extractionRecoveryTasks')).find(task => task.status === 'pending') || false, 10_000)
  assert.ok(queued.source.endsWith('分卷.part1.rar') && queued.hasPassword)
  await click('[data-testid="nav-Decompress"]'); await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async () => await bridge('taskStatus', queued.id) === 'completed', 30_000)
  const files = []
  const walk = directory => { for (const entry of readdirSync(directory,{withFileTypes:true})) { const file=path.join(directory,entry.name); if(entry.isDirectory())walk(file);else files.push(file) } }
  walk(queued.output)
  assert.ok(files.some(file => path.basename(file)==='payload.bin' && readFileSync(file).equals(payload)))
  assert.ok(files.some(file => path.basename(file)==='说明.txt' && readFileSync(file).equals(text)))
  assert.equal(existsSync(first), false)
  for (const file of originals) assert.equal(hash(path.join(root,file.name)),file.hash)
  assert.equal(hash(rar),sourceHash)
  await bridge('clearTasks')
  await bridge('seedRarRecoveryDraft', path.join(source,'payload.bin'), root)
  await click('[data-testid="nav-Compress"]')
  await waitForElement('[data-testid="rar-recovery-settings"]')
  for (const width of [1100,760]) {
    await driver.manage().window().setRect({width,height:700})
    await shot('rar-recovery-settings-' + width, '[data-testid="rar-recovery-settings"]')
  }
  writeFileSync(path.join(artifactDirectory,'rar-recovery-results.json'),JSON.stringify({
    application, binarySha256:hash(application), externalToolSha256:hash(tool),
    appCreatedRecoveryRecord:true, encryptedHeadersAndChinesePassword:true,
    inlineFailureRepairAndOutputRetention:true, damagedRarDoesNotAssertWrongPassword:true,
    wrongPasswordNoOutput:true, explorerCliAndMultiSelection:true,
    revMissingFirstRestoredAndExtracted:true, originalFilesUnchanged:true,
  },null,2))
}

import assert from 'node:assert/strict'
import { writeFileSync, readFileSync, mkdirSync } from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { By, Key } from 'selenium-webdriver'
import { runPasswordSplitCreationGate } from './password-split-creation-gate.mjs'

export async function runStandardZipCreateDesktopGate(context) {
  const { driver, callDesktopBridge: bridge, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip } = context
  let aes
  for (const settings of [{ password: '' }, { password: 'aes-utf8-password' },
    { password: '中文密码 + !测试', zipEncoding: 'windows_local' },
    { zipEncryption: 'zipcrypto', password: '中文密码 + !测试', zipEncoding: 'windows_local' }]) {
    const result = await runPasswordSplitCreationGate({ ...context, ...settings, zipSplitMode: 'standard' })
    if (settings.password && !settings.zipEncryption) aes = result
  }
  const root = path.join(fixtureDirectory, 'standard-zip-conversion')
  mkdirSync(root)
  const fill = async (id, value) => {
    const input = await waitForElement(`[data-testid="${id}"]`)
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', input)
    await input.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (value) await input.sendKeys(value)
  }
  await (await waitForElement('[data-testid="open-archive-conversion"]')).click()
  await bridge('queueDesktopDialogSelections', [aes.parts[0]])
  await (await waitForElement('[data-testid="conversion-choose-source"]')).click()
  await (await waitForElement('[data-testid="conversion-row"]')).click()
  await (await waitForElement('[data-testid="conversion-format"]')).click()
  await driver.findElement(By.xpath('//button[@role="option" and .//strong[normalize-space(.)="ZIP"]]')).click()
  const convertedPath = path.join(root, '改密标准分卷.zip')
  await fill('conversion-output', convertedPath)
  await fill('conversion-source-password', aes.password)
  await fill('conversion-password', 'changed-password')
  await fill('conversion-split', '1')
  const choice = await waitForElement('[data-testid="conversion-zip-split-mode"]')
  await choice.click()
  await driver.findElement(By.xpath('//button[@role="option" and contains(.,"标准 ZIP")]')).click()
  for (const width of [1100, 760]) {
    await driver.manage().window().setRect({ width, height: 760 })
    await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', choice)
    await choice.click()
    await driver.wait(() => driver.executeScript('return [...document.querySelectorAll("[role=listbox]")].every(menu => menu.getAnimations({subtree:true}).every(animation => animation.playState !== "running"))'), 5000)
    writeFileSync(path.join(artifactDirectory, `standard-zip-conversion-${width}.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
    await choice.click()
  }
  await (await waitForElement('[data-testid="conversion-start"]')).click()
  const record = await driver.wait(async () => (await bridge('taskHistory')).find(r => r.outputPath === convertedPath), 60000)
  assert.equal(record.status, 'completed', record.errorMessage)
  assert.equal(record.executionConfig.zipSplitMode, 'standard')
  assert.equal(record.archiveOutput.primaryPath, convertedPath)
  const independent = spawnSync(bundledSevenZip, ['t', '-pchanged-password', convertedPath], { encoding: 'utf8', windowsHide: true })
  assert.equal(independent.status, 0, independent.stdout + independent.stderr)
  const oldPassword = spawnSync(bundledSevenZip, ['t', `-p${aes.password}`, convertedPath], { encoding: 'utf8', windowsHide: true })
  assert.notEqual(oldPassword.status, 0)
  const numeric = await bridge('convertArchive', convertedPath, 'changed-password', path.join(root, '数字换卷.zip'), { format: 'zip', split_size: 1048576, zip_split_mode: 'numeric' })
  const merged = await bridge('convertArchive', numeric.primaryPath, null, path.join(root, '去密合卷.zip'), { format: 'zip' })
  assert.equal(merged.split, false)
  const restored = path.join(root, '最终还原')
  await bridge('extractArchive', merged.primaryPath, restored, null)
  assert.deepEqual(readFileSync(path.join(restored, path.basename(aes.folder), '数据.bin')), aes.payload)
  await driver.findElement(By.css('[role="dialog"] button[aria-label="Close dialog"]')).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="archive-conversion"]'))).length === 0, 10000)
  // Confirm the newly added choice on the actual profile page, including native
  // persistence and the portable export/import path (not a mocked store).
  await (await waitForElement('[data-testid="open-global-compression-settings"]')).click()
  const manage = await waitForElement('[data-testid="manage-compression-profiles"]')
  await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', manage)
  await manage.click()
  const create = await driver.findElement(By.css('.profile-manager button:has(.pi-plus)'))
  await create.click()
  await (await waitForElement('[role="dialog"] form input[maxlength="50"]')).sendKeys('标准 ZIP 分享')
  const split = await driver.findElement(By.xpath('//form//label[.//strong[normalize-space(.)="分卷压缩"]]/input'))
  await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', split)
  await split.click()
  const profileChoice = await waitForElement('[data-testid="profile-zip-split-mode"]')
  await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', profileChoice)
  await profileChoice.click()
  await driver.findElement(By.xpath('//button[@role="option" and contains(.,"标准 ZIP")]')).click()
  writeFileSync(path.join(artifactDirectory, 'standard-zip-profile.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  await (await waitForElement('[role="dialog"] form button[type="submit"]')).click()
  const invoke = async (cmd, args) => {
    const result = await driver.executeAsyncScript(`const [cmd,args,done]=arguments;const ids=crypto.getRandomValues(new Uint32Array(2));
      const clean=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
      window['_'+ids[0]]=value=>{clean();done({ok:true,value})};window['_'+ids[1]]=error=>{clean();done({ok:false,error})};
      window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});`, cmd, args)
    assert.ok(result.ok, JSON.stringify(result.error)); return result.value
  }
  const profile = await driver.wait(async () => (await invoke('get_compression_profiles', {})).find(p => p.name === '标准 ZIP 分享'), 10000)
  assert.equal(profile.config.extra_params.zip_split_mode, 'standard')
  const filePath = path.join(root, '分享.longtask.json')
  const template = await invoke('export_task_template', { profileId: profile.id, filePath })
  assert.equal(template.compression.zipSplitMode, 'standard')
  const preview = await invoke('preview_task_template', { filePath })
  const importedId = await invoke('import_task_template', { filePath, expectedSha256: preview.contentSha256 })
  const imported = (await invoke('get_compression_profiles', {})).find(p => p.id === importedId)
  assert.equal(imported.config.extra_params.zip_split_mode, 'standard')
  writeFileSync(path.join(artifactDirectory, 'standard-zip-create.json'), JSON.stringify({ buildKind: 'isolated-release-desktop', nativePickerVerified: false,
    createPlainAesZipCrypto: true, independentSevenZipVerified: true, uiPasswordChange: true, standardToNumericToMerged: true,
    payloadPreserved: true, historyVolumeModePreserved: true, profileAndPortableTemplateModePreserved: true }, null, 2))
}

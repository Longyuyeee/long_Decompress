import assert from 'node:assert/strict'
import { createHash, randomBytes, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runPurposePresets1541Gate({ driver, bridge, waitForElement, artifactDirectory, application, root, bundledSevenZip }) {
  const base = path.join(root, 'test-results/purpose-presets-1541'), progress = path.join(base, 'desktop-progress.json')
  mkdirSync(base, { recursive: true })
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const state = existsSync(progress) ? JSON.parse(readFileSync(progress, 'utf8')) : { version: '1.5.41', binarySha256: hash(application), work: path.join(base, randomUUID()), checks: [], layouts: [], screenshots: [] }
  assert.equal(state.binarySha256, hash(application), 'Use a new evidence directory when changing the application binary')
  const work = state.work; mkdirSync(work, { recursive: true })
  const save = () => writeFileSync(progress, JSON.stringify(state, null, 2))
  const done = (kind, evidence = {}) => { state.checks.push({ kind, ...evidence }); save(); console.log('purpose1541: ' + kind) }
  const passed = kind => state.checks.some(row => row.kind === kind)
  // Scope repeated shared controls to the foreground dialog, and ignore clipped/hidden controls.
  const find = selector => driver.wait(async () => {
    try {
    const dialogs = await driver.findElements(By.css('[role="dialog"]'))
    for (const dialog of dialogs.reverse()) {
      if (!await dialog.isDisplayed()) continue
      const matches = await dialog.findElements(By.css(selector))
      for (const element of matches.reverse()) if (await element.isDisplayed()) return element
      return false
    }
    const matches = await driver.findElements(By.css(selector))
    for (const element of matches.reverse()) if (await element.isDisplayed()) return element
    return false
    } catch (error) { if (error.name === 'StaleElementReferenceError') return false; throw error }
  }, 15000)
  const click = async selector => {
    const element = typeof selector === 'string' ? await find(selector) : selector
    await driver.executeScript(e => { e.scrollIntoView({ block: 'center', behavior: 'instant' }); e.click() }, element)
  }
  const fill = async (selector, value) => { const e = await find(selector); await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), e); await e.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE); if (value) await e.sendKeys(value) }
  const complete = predicate => driver.wait(async () => (await bridge('taskHistory')).find(row => predicate(row) && ['completed', 'failed'].includes(row.status)) || false, 90000)
  const closeConversion = async () => click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  const draft = () => driver.executeScript(() => JSON.parse(localStorage.getItem('long-pending-archive-drafts-v1')))
  async function screenshots(name, anchor) {
    for (const [width, height] of [[1100, 700], [760, 520]]) {
      await driver.manage().window().setRect({ width, height })
      await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), await find(anchor))
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
      const layout = await driver.executeScript(() => {
        const picker = [...document.querySelectorAll('[data-testid="purpose-picker"]')].find(e => e.offsetHeight > 0)
        const details = document.querySelector('[data-testid="compression-draft-details"]')
        return { width: innerWidth, height: innerHeight, overflow: document.documentElement.scrollWidth - innerWidth, pickerOverflow: picker ? picker.scrollWidth - picker.clientWidth : 0, columns: details ? [...details.children].map(e => e.getBoundingClientRect().width) : [] }
      })
      assert.ok(layout.overflow <= 1 && layout.pickerOverflow <= 1, JSON.stringify(layout))
      if (name === 'inline-preview' && width === 1100) assert.ok(Math.abs(layout.columns[0] - layout.columns[1]) <= 1, JSON.stringify(layout))
      const file = `purpose1541-${name}-${width}x${height}.png`
      writeFileSync(path.join(artifactDirectory, file), Buffer.from(await driver.takeScreenshot(), 'base64'))
      state.layouts.push({ name, ...layout }); state.screenshots.push(file); save()
    }
    await driver.manage().window().setRect({ width: 1100, height: 700 })
  }
  function independent(output, password, label) {
    const destination = path.join(work, label)
    execFileSync(bundledSevenZip, ['x', '-y', ...(password ? ['-p' + password] : []), '-o' + destination, output.primaryPath], { windowsHide: true, maxBuffer: 4 * 1024 * 1024 })
    const walk = dir => readdirSync(dir, { withFileTypes: true }).flatMap(e => e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)])
    const files = walk(destination)
    assert.equal(files.length, 1); assert.equal(hash(files[0]), hash(state.source))
    return { allHashes: true, members: output.files.length, primaryPath: output.primaryPath }
  }
  if (!state.source) { state.source = path.join(work, '用户方案与长名称显示检查-' + '资料'.repeat(12) + '.bin'); writeFileSync(state.source, randomBytes(2 * 1024 * 1024 + 4096)); save() }

  if (!passed('inline-confirmation-and-encrypted-create')) {
    await bridge('clearCompressionWorkspace'); await bridge('clearTasks'); await click('[data-testid="nav-Compress"]')
    await bridge('queueDesktopDialogSelections', [[state.source]])
    await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    await click('[data-testid="compression-archive-name"]')
    assert.match(await (await find('[data-testid="compression-format-select"]')).getAttribute('textContent'), /ZIP/)
    await click('[data-testid="purpose-smaller"]')
    assert.match(await (await find('[data-testid="compression-format-select"]')).getAttribute('textContent'), /ZIP/)
    await screenshots('inline-preview', '[data-testid="purpose-preview"]')
    await click('[data-testid="purpose-encrypted"]')
    assert.equal(await (await find('[data-testid="purpose-apply"]')).isEnabled(), false)
    await fill('[data-testid="purpose-password"]', 'Purpose1541!'); await click('[data-testid="purpose-apply"]')
    assert.match(await (await find('[data-testid="compression-format-select"]')).getAttribute('textContent'), /7Z/)
    assert.equal(await (await find('[data-testid="compression-encrypt-filenames"]')).isSelected(), true)
    await fill('[data-testid="compression-output-path"]', work)
    await fill('[data-testid="compression-filename-input"]', '用途加密-' + randomUUID())
    await click('[data-testid="start-compression"]')
    const record = await complete(row => row.executionConfig?.settings?.format === '7z')
    assert.equal(record.status, 'completed', JSON.stringify(record))
    const evidence = independent(record.archiveOutput, 'Purpose1541!', 'encrypted-output')
    const listing = execFileSync(bundledSevenZip, ['l', '-slt', '-pPurpose1541!', record.archiveOutput.primaryPath], { windowsHide: true, encoding: 'utf8' })
    assert.match(listing, /Encrypted = \+/)
    state.encryptedArchive = record.archiveOutput.primaryPath
    await screenshots('inline-result', '[data-testid="compression-draft-details"]')
    done('inline-confirmation-and-encrypted-create', evidence)
  }

  if (!passed('multi-select-cancel-and-standard-zip')) {
    await bridge('clearCompressionWorkspace'); await bridge('clearTasks')
    await bridge('queueDesktopDialogSelections', [[state.source]])
    await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    await click('[data-testid="compression-group-checkbox"]')
    assert.equal((await driver.findElements(By.css('[data-testid="purpose-picker"]'))).length, 0, 'multi-select must not spread configuration on the page')
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]')
    await click('[data-testid="purpose-split"]'); await fill('[data-testid="purpose-split-size"]', '1'); await click('[data-testid="purpose-apply"]')
    await screenshots('batch-modal', '[data-testid="batch-config-fields"]')
    await click('[data-testid="batch-config-cancel"]')
    await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]')
    assert.equal(await (await find('[data-testid="batch-config-apply"]')).isEnabled(), false)
    await click('[data-testid="purpose-split"]'); await fill('[data-testid="purpose-split-size"]', '1'); await click('[data-testid="purpose-apply"]')
    await click('[data-testid="batch-config-apply"]')
    await driver.wait(async () => (await driver.findElements(By.css('[role="dialog"]'))).length === 0, 10000)
    await driver.wait(async () => (await draft())?.compression?.files?.[0]?.settings?.zipSplitMode === 'standard', 10000)
    const saved = (await draft()).compression.files[0].settings
    assert.equal(saved.splitSize, '1'); assert.equal(saved.splitArchive, true); assert.equal(saved.zipMethod, 'deflate')
    await click('[data-testid="compression-archive-name"]')
    await fill('[data-testid="compression-output-path"]', work)
    await fill('[data-testid="compression-filename-input"]', '用途分包-' + randomUUID())
    await click('[data-testid="start-compression"]')
    const record = await complete(row => row.executionConfig?.settings?.format === 'zip')
    assert.equal(record.status, 'completed', JSON.stringify(record))
    assert.ok(record.archiveOutput.files.length >= 3); assert.ok(record.archiveOutput.files.some(file => /\.z01$/i.test(file.path)))
    done('multi-select-cancel-and-standard-zip', independent(record.archiveOutput, '', 'split-output'))
  }

  if (!passed('conversion-compatible-share-preserves-input-password')) {
    await bridge('clearTasks'); await click('[data-testid="nav-Compress"]'); await click('[data-testid="open-archive-conversion"]')
    await bridge('queueDesktopDialogSelections', [[state.encryptedArchive]]); await click('[data-testid="conversion-choose-source"]')
    const row = (await driver.findElements(By.css('[data-testid="conversion-row"]'))).at(-1)
    if (await row.getAttribute('aria-expanded') !== 'true') await click(row)
    await fill('[data-testid="conversion-source-password"]', 'Purpose1541!')
    await click('[data-testid="purpose-share"]'); await screenshots('conversion-preview', '[data-testid="purpose-preview"]')
    await click('[data-testid="purpose-apply"]')
    assert.equal(await (await find('[data-testid="conversion-source-password"]')).getAttribute('value'), 'Purpose1541!')
    await fill('[data-testid="conversion-output"]', path.join(work, '兼容分享-' + randomUUID() + '.zip'))
    await click('[data-testid="conversion-start"]')
    const record = await complete(row => row.executionConfig?.format === 'zip')
    assert.equal(record.status, 'completed', JSON.stringify(record))
    done('conversion-compatible-share-preserves-input-password', independent(record.archiveOutput, '', 'conversion-output'))
    await closeConversion()
  }

  if (!passed('conversion-bulk-cancel-and-default-inheritance')) {
    await click('[data-testid="nav-Compress"]')
    await click('[data-testid="open-archive-conversion"]')
    await bridge('queueDesktopDialogSelections', [[state.encryptedArchive]]); await click('[data-testid="conversion-choose-source"]')
    await click('[data-testid="conversion-batch-config"]')
    await click('[data-testid="purpose-smaller"]'); await click('[data-testid="purpose-apply"]')
    await screenshots('conversion-bulk', '[data-testid="conversion-batch-fields"]')
    const cancels = await driver.findElements(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="取消"]')); await click(cancels.at(-1))
    const defaults = await driver.findElement(By.xpath('//button[normalize-space(.)="默认配置"]')); await click(defaults)
    await click('[data-testid="purpose-smaller"]'); await click('[data-testid="purpose-apply"]'); await click('[data-testid="conversion-batch-apply"]')
    await driver.wait(async () => (await driver.findElements(By.css('[data-testid="conversion-batch-fields"]'))).length === 0, 10000)
    const rows = await driver.findElements(By.css('[data-testid="conversion-row"]')); const row = rows.at(-1)
    if (await row.getAttribute('aria-expanded') !== 'true') await click(row)
    assert.match(await (await find('[data-testid="conversion-format"]')).getAttribute('textContent'), /7Z/)
    done('conversion-bulk-cancel-and-default-inheritance')
    await closeConversion()
  }
  if (!passed('global-save-cancel-and-natural-row-inheritance')) {
    await bridge('clearCompressionWorkspace'); await bridge('clearTasks'); await click('[data-testid="nav-Compress"]')
    const second = path.join(work, '第二条保持继承.txt'); writeFileSync(second, 'purpose inheritance')
    await bridge('queueDesktopDialogSelections', [[state.source, second]])
    await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
    const openGlobal = async () => click(await driver.findElement(By.xpath('//button[contains(.,"全局设置")]')))
    await openGlobal(); await click('[data-testid="purpose-smaller"]'); await click('[data-testid="purpose-apply"]')
    await click('[data-testid="cancel-global-compression-settings"]')
    await openGlobal(); assert.match(await (await find('[data-testid="compression-format-select"]')).getAttribute('textContent'), /ZIP/)
    await click('[data-testid="purpose-smaller"]'); await click('[data-testid="purpose-apply"]')
    await screenshots('global-config', '[data-testid="purpose-picker"]')
    console.log('purpose1541: saving global scheme')
    await click('[data-testid="save-global-compression-settings"]')
    await driver.wait(async () => (await driver.findElements(By.css('[role="dialog"]'))).length === 0, 10000)
    await driver.wait(async () => (await draft())?.compression?.defaults?.format === '7z', 10000)
    let saved = (await draft()).compression
    assert.ok(saved.files.every(file => !file.settings))
    await click((await driver.findElements(By.css('[data-testid="compression-archive-name"]')))[0])
    await click('[data-testid="purpose-share"]'); await click('[data-testid="purpose-apply"]')
    await driver.wait(async () => (await draft()).compression.files.some(file => file.settings?.format === 'zip'), 10000)
    saved = (await draft()).compression
    assert.equal(saved.defaults.format, '7z'); assert.equal(saved.files.filter(file => !!file.settings).length, 1)
    await openGlobal(); await click('[data-testid="purpose-share"]'); await click('[data-testid="purpose-apply"]')
    console.log('purpose1541: saving global scheme')
    await click('[data-testid="save-global-compression-settings"]')
    await driver.wait(async () => (await draft()).compression.defaults.format === 'zip', 10000)
    saved = (await draft()).compression
    assert.equal(saved.files.filter(file => !!file.settings).length, 1)
    done('global-save-cancel-and-natural-row-inheritance', { inheritedDrafts: 1, independentDrafts: 1 })
  }
  save(); console.log('purpose1541: complete ' + progress)
}

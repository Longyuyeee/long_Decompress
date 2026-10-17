import assert from 'node:assert/strict'
import { mkdirSync, readFileSync, writeFileSync, copyFileSync, existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
const names = Array.from({ length: 5 }, (_, index) => `中文目录/${index}-资料.txt`)

function splitZip(input, output) {
  const bytes = readFileSync(input), end = bytes.length - 22
  assert.equal(bytes.readUInt32LE(end), 0x06054b50)
  const central = bytes.readUInt32LE(end + 16), entries = []
  for (let at = central; at < end;) {
    assert.equal(bytes.readUInt32LE(at), 0x02014b50)
    const length = 46 + bytes.readUInt16LE(at + 28) + bytes.readUInt16LE(at + 30) + bytes.readUInt16LE(at + 32)
    entries.push({ at: at + 4, offset: bytes.readUInt32LE(at + 42) + 4 })
    at += length
  }
  assert.equal(entries.length, 5)
  const bounds = [0, entries[1].offset + 150, entries[3].offset, entries[4].offset + 150]
  const joined = Buffer.concat([Buffer.from([0x50, 0x4b, 0x07, 0x08]), bytes])
  for (const entry of entries) {
    const disk = bounds.findLastIndex(start => start <= entry.offset)
    joined.writeUInt16LE(disk, entry.at + 34)
    joined.writeUInt32LE(entry.offset - bounds[disk], entry.at + 42)
  }
  joined.writeUInt16LE(3, end + 4 + 4); joined.writeUInt16LE(3, end + 4 + 6)
  joined.writeUInt16LE(5, end + 4 + 8); joined.writeUInt32LE(central + 4 - bounds[3], end + 4 + 16)
  const parts = bounds.map((start, index) => {
    const file = index === 3 ? output : output.replace(/\.zip$/i, `.z0${index + 1}`)
    writeFileSync(file, joined.subarray(start, bounds[index + 1] ?? joined.length)); return file
  })
  return parts
}

export async function runPartialZip1413Gate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application, forwardContextAction }) {
  const root = process.env.LONG_PARTIAL_ZIP_RESUME_ROOT || path.join(fixtureDirectory, 'partial-zip-1413')
  const batchOnly = process.env.LONG_PARTIAL_ZIP_UI_FROM === 'batch'
  const previous = batchOnly ? JSON.parse(readFileSync(path.join(root, 'initial-ui-results.json'), 'utf8')) : null
  if (previous) { assert.equal(previous.uiRescuedAndExtracted, true); assert.equal(previous.binarySha256, hash(application)) }
  const manifestPath = path.join(root, 'manifest.json'), results = [], screenshots = previous ? previous.screenshots.slice(0, 4) : []
  const sevenZip = (args, cwd = root) => {
    const result = spawnSync(bundledSevenZip, args, { cwd, windowsHide: true, timeout: 60000, encoding: 'utf8' })
    assert.ifError(result.error); assert.equal(result.status, 0, result.stdout + result.stderr)
  }
  if (!existsSync(manifestPath)) {
    mkdirSync(root, { recursive: true }); mkdirSync(path.join(root, '原件/中文目录'), { recursive: true })
    names.forEach((name, index) => writeFileSync(path.join(root, '原件', name), `中文资料 ${index}：缺卷时保留完整文件。\n`.repeat(200)))
    const cases = []
    for (const mode of ['plain', 'AES256', 'ZipCrypto']) {
      const folder = path.join(root, mode); mkdirSync(folder)
      const password = mode === 'plain' ? null : 'rescue + password!'
      const original = path.join(folder, '外部原包.zip')
      sevenZip(['a', '-tzip', '-mx=0', '-mm=Copy', '-mcu=on', ...(password ? [`-p${password}`, `-mem=${mode}`] : []), original, ...names], path.join(root, '原件'))
      const healthy = path.join(folder, '完整卷组.zip'), parts = splitZip(original, healthy)
      sevenZip(['t', ...(password ? [`-p${password}`] : []), healthy])
      for (const missing of [0, 1]) {
        const directory = path.join(folder, `缺卷${missing + 1}`); mkdirSync(directory)
        const input = path.join(directory, '完整卷组.zip')
        parts.forEach((file, index) => { if (index !== missing) copyFileSync(file, path.join(directory, path.basename(file))) })
        cases.push({ mode, missing, input, password, expected: missing === 0 ? [2, 3, 4] : [0, 3, 4] })
      }
    }
    writeFileSync(manifestPath, JSON.stringify({ cases, source: names.map(name => ({ name, sha256: hash(path.join(root, '原件', name)) })) }, null, 2))
  }
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
  const verify = (output, expected, password) => {
    sevenZip(['t', ...(password ? [`-p${password}`] : []), output])
    const directory = output + '-独立回解'
    sevenZip(['x', '-y', ...(password ? [`-p${password}`] : []), `-o${directory}`, output])
    for (let index = 0; index < names.length; index++) {
      const file = path.join(directory, names[index])
      assert.equal(existsSync(file), expected.includes(index), names[index])
      if (expected.includes(index)) assert.equal(hash(file), manifest.source[index].sha256)
    }
  }
  if (!process.env.LONG_PARTIAL_ZIP_UI_ONLY) {
    for (const [index, item] of manifest.cases.entries()) {
      const before = hash(item.input), report = await bridge('diagnoseArchive', item.input, item.password)
      assert.equal(report.status, 'partial_recovery', JSON.stringify(report)); assert.equal(report.canRepair, true)
      assert.equal(report.totalFiles, 3); assert.equal(report.integrityTested, false)
      if (item.password) {
        const required = await bridge('diagnoseArchive', item.input, null)
        assert.equal(required.status, 'password_required'); assert.equal(required.canRepair, false)
        const wrong = path.join(root, `错误密码-${index}.zip`)
        await failure('repairZip', item.input, wrong, 'wrong'); assert.equal(existsSync(wrong), false)
      }
      const output = path.join(root, `命令恢复-${index}.zip`), result = await bridge('repairZip', item.input, output, item.password)
      assert.equal(result.recoveredFiles, 3); assert.equal(result.partial, true); assert.equal(result.directoryRebuilt, false)
      assert.deepEqual([...result.recoveredEntries].sort(), item.expected.map(i => names[i]).sort())
      verify(output, item.expected, item.password); assert.equal(hash(item.input), before)
      results.push({ mode: item.mode, missing: item.missing + 1, recovered: result.recoveredEntries, independentReadback: true, originalUnchanged: true })
    }
    const noEnd = path.join(root, '无终卷'); mkdirSync(noEnd)
    const part = path.join(noEnd, '缺失.z01'); copyFileSync(path.join(root, 'plain/完整卷组.z01'), part)
    const unavailable = await bridge('diagnoseArchive', part, null)
    assert.equal(unavailable.status, 'missing_volume'); assert.equal(unavailable.canRepair, false)
    writeFileSync(path.join(root, 'native-results.json'), JSON.stringify({ binarySha256: hash(application), results, missingTerminalRejected: true }, null, 2))
    console.log('[partial-zip-1413] six native missing-disk cases and independent readback passed')
  }
  const click = async selector => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await driver.wait(() => driver.executeScript(el => { const r = el.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return !el.disabled && (hit === el || el.contains(hit)) }, element), 10000)
    await element.click()
  }
  const fill = async (selector, value) => {
    const element = await waitForElement(selector); await driver.executeScript(el => el.scrollIntoView({ block: 'center' }), element)
    await element.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE); if (value) await element.sendKeys(value)
  }
  const shot = async (name, selector) => {
    const element = await waitForElement(selector); await driver.executeScript(el => el.scrollIntoView({ block: 'start' }), element)
    await driver.sleep(350)
    const bounds = await driver.executeScript(el => ({ client: el.clientWidth, scroll: el.scrollWidth, viewport: innerWidth, page: document.documentElement.scrollWidth }), element)
    assert.ok(bounds.scroll <= bounds.client + 1 && bounds.page <= bounds.viewport + 1, JSON.stringify(bounds))
    writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64')); screenshots.push({ name: name + '.png', bounds })
  }
  const item = manifest.cases.find(item => item.mode === 'AES256' && item.missing === 1)
  if (!batchOnly) {
  await bridge('clearTasks'); const originalOutput = path.join(root, '用户解压目录'); mkdirSync(originalOutput, { recursive: true })
  await failure('extractArchive', item.input, originalOutput, item.password)
  const [failed] = await bridge('extractionRecoveryTasks'); assert.equal(failed.status, 'failed')
  await click('[data-testid="nav-Decompress"]'); await click(`[data-task-id="${failed.id}"]`)
  await fill('[data-testid="retry-password"]', item.password); await click('[data-testid="retry-rescue"]')
  await waitForElement('[data-testid="start-archive-repair"]')
  for (const width of [1100, 760]) {
    await driver.manage().window().setRect({ width, height: width === 760 ? 520 : 700 })
    await shot(`partial-zip-diagnosis-${width}`, '[data-testid="diagnostic-report"]')
  }
  const output = path.join(root, `页面恢复-${Date.now()}.zip`)
  await bridge('queueDesktopDialogSelections', [output]); await click('[data-testid="start-archive-repair"]'); await waitForElement('[data-testid="repair-result"]')
  assert.match(await driver.findElement(By.css('[data-testid="repair-result"]')).getAttribute('textContent'), /部分恢复/)
  verify(output, item.expected, item.password)
  await shot('partial-zip-result-folded-760', '[data-testid="repair-result"]')
  await click('[data-testid="recovery-outcome-missing"] summary'); await shot('partial-zip-result-details-760', '[data-testid="recovery-outcome-missing"]')
  await click('[data-testid="queue-recovered-archive"]')
  const queued = await driver.wait(async () => (await bridge('extractionRecoveryTasks')).find(task => task.status === 'pending') || false, 10000)
  assert.equal(queued.output, originalOutput); assert.equal(queued.retryOfTaskId, failed.id); assert.ok(queued.hasPassword)
  await click('[data-testid="nav-Decompress"]'); await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async () => await bridge('taskStatus', queued.id) === 'completed', 30000)
  const restored = queued.extractToSubfolder ? path.join(originalOutput, path.basename(output, '.zip')) : originalOutput
  for (const index of item.expected) assert.equal(hash(path.join(restored, names[index])), manifest.source[index].sha256)
  }
  // The existing batch page retains inline details and modal-only multi-edit.
  const batchItems = [manifest.cases[1], item]
  forwardContextAction('--repair-archive', batchItems.map(item => item.input))
  await waitForElement('[data-testid="archive-recovery-batch"]'); await driver.manage().window().setRect({ width: 760, height: 520 }); await click('[data-testid="batch-recovery-config"]')
  const dialog = await waitForElement('[data-testid="archive-batch-config-dialog"]')
  await dialog.findElements(By.css('input[type="checkbox"]')).then(async boxes => boxes[0].click())
  await fill('[data-testid="archive-batch-shared-password"]', item.password)
  const batchOutput = path.join(root, '批量恢复'); mkdirSync(batchOutput, { recursive: true }); await fill('[data-testid="archive-batch-output"]', batchOutput)
  await shot('partial-zip-batch-modal-760', '[data-testid="archive-batch-config-dialog"]'); await click('[data-testid="archive-batch-config-apply"]')
  await click('[data-testid="batch-diagnose"]'); await driver.wait(async () => { const buttons = await driver.findElements(By.css('[data-testid="batch-repair"]')); return buttons.length > 0 && await buttons[0].isEnabled() }, 30000)
  await click('[data-testid="batch-repair"]'); await driver.wait(async () => (await driver.findElements(By.css('[data-testid="archive-batch-result"]'))).length === 2, 30000)
  await shot('partial-zip-batch-results-760', '[data-testid="archive-recovery-batch"]')
  await click('[data-testid="archive-batch-result"] [data-testid="recovery-outcome-recovered"] summary')
  await driver.executeScript(() => document.documentElement.style.fontSize = '125%')
  await shot('partial-zip-batch-file-list-760-125', '[data-testid="archive-batch-result"] [data-testid="recovery-outcome-recovered"]')
  await driver.executeScript(() => document.documentElement.style.fontSize = '')
  await driver.manage().window().setRect({ width: 1100, height: 700 }); await shot('partial-zip-batch-results-1100', '[data-testid="archive-recovery-batch"]')
  writeFileSync(path.join(artifactDirectory, 'partial-zip-1413-results.json'), JSON.stringify({ root, binarySha256: hash(application), native: JSON.parse(readFileSync(path.join(root, 'native-results.json'), 'utf8')), nativeReused: !!process.env.LONG_PARTIAL_ZIP_UI_ONLY, singleUiReused: batchOnly, uiRescuedAndExtracted: true, passwordAndDestinationPreserved: true, batchModalAndRepair: true, screenshots }, null, 2))
}

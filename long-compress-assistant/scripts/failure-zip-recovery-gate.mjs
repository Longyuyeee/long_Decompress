import assert from 'node:assert/strict'
import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { createHash, randomBytes } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runFailureZipRecoveryGate({ driver, bridge, failure, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application }) {
  const root = path.join(fixtureDirectory, 'failure-zip-recovery'); mkdirSync(root)
  mkdirSync(artifactDirectory, { recursive: true })
  const source = path.join(root, '别人发来的资料'); mkdirSync(source)
  const good = Buffer.from('完整的中文说明应该被救回，原包与失败配置需要保留。')
  writeFileSync(path.join(source, '完整说明.txt'), good)
  writeFileSync(path.join(source, 'broken.bin'), randomBytes(180_000))
  mkdirSync(path.join(source, '空目录'))
  const password = 'fixture-rescue + !'
  const sha256 = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const sevenZip = (args, cwd = source, success = true) => {
    const result = spawnSync(bundledSevenZip, args, { cwd, windowsHide: true, encoding: 'utf8' })
    assert.ifError(result.error)
    if (success) assert.equal(result.status, 0, result.stdout + result.stderr)
    else assert.notEqual(result.status, 0)
  }
  const fill = async (selector, value) => {
    const input = await waitForElement(selector)
    await input.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE, value)
  }
  const click = async selector => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center' }), element)
    await element.click()
  }
  const screenshot = async (name, selector) => {
    for (const close of await driver.findElements(By.css('.toast-item button'))) {
      try { await close.click() } catch { /* Toast may already be leaving. */ }
    }
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'start' }), element)
    await new Promise(resolve => setTimeout(resolve, 550))
    const geometry = await driver.executeScript(el => ({ client: el.clientWidth, scroll: el.scrollWidth }), element)
    assert.ok(geometry.scroll <= geometry.client + 1, `${name}: horizontal overflow`)
    writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  await driver.executeScript(() => {
    document.documentElement.classList.remove('light')
    document.documentElement.classList.add('dark')
  })
  const records = []
  let healthyAes, lostAes
  for (const mode of ['AES256', 'ZipCrypto']) {
    const healthy = path.join(root, `完整-${mode}.zip`)
    sevenZip(['a', '-tzip', '-mm=Copy', `-mem=${mode}`, `-p${password}`, healthy, '.\\*'])
    const bytes = readFileSync(healthy)
    const end = bytes.length - 22
    assert.equal(bytes.readUInt32LE(end), 0x06054b50)
    const central = bytes.readUInt32LE(end + 16)
    let at = central, corrupt = false
    while (at < end && bytes.readUInt32LE(at) === 0x02014b50) {
      const name = bytes.subarray(at + 46, at + 46 + bytes.readUInt16LE(at + 28)).toString('utf8')
      if (name === 'broken.bin') {
        const local = bytes.readUInt32LE(at + 42)
        const data = local + 30 + bytes.readUInt16LE(local + 26) + bytes.readUInt16LE(local + 28)
        assert.equal(bytes.readUInt16LE(local + 6) & 8, 0, 'external fixture must declare its local sizes')
        bytes[data + 50] ^= 1; corrupt = true
      }
      at += 46 + bytes.readUInt16LE(at + 28) + bytes.readUInt16LE(at + 30) + bytes.readUInt16LE(at + 32)
    }
    assert.ok(corrupt)
    const lost = path.join(root, `目录丢失-${mode}.zip`)
    const localBytes = bytes.subarray(0, central); writeFileSync(lost, localBytes)
    const parts = []
    for (let offset = 0, index = 1; offset < localBytes.length; offset += 65536, index++) {
      const part = path.join(root, `目录丢失-${mode}.zip.${String(index).padStart(3, '0')}`)
      writeFileSync(part, localBytes.subarray(offset, offset + 65536)); parts.push(part)
    }
    for (const [route, input] of [['single', lost], ['numeric', parts[1]]]) {
      const originals = [lost, ...parts].map(file => ({ file, sha256: sha256(file) }))
      const report = await bridge('diagnoseArchive', input, password)
      assert.equal(report.status, 'directory_lost'); assert.ok(report.canRepair && report.encrypted)
      const output = path.join(root, `救回-${mode}-${route}.zip`)
      assert.match(await failure('repairZip', input, output, 'wrong'), /密码|完整文件|complete file/i)
      assert.equal(existsSync(output), false)
      const result = await bridge('repairZip', input, output, password)
      assert.ok(result.verified && result.outputEncrypted && result.partial && result.directoryRebuilt)
      assert.equal(result.recoveredFiles, 1)
      sevenZip(['t', output, `-p${password}`]); sevenZip(['t', output, '-pwrong'], source, false)
      const restored = path.join(root, `独立回解-${mode}-${route}`)
      sevenZip(['x', output, `-p${password}`, `-o${restored}`, '-y'])
      assert.deepEqual(readFileSync(path.join(restored, '完整说明.txt')), good)
      assert.equal(existsSync(path.join(restored, 'broken.bin')), false)
      originals.forEach(item => assert.equal(sha256(item.file), item.sha256))
      records.push({ mode, route, originalSha256: sha256(lost), outputSha256: sha256(output), recoveredFiles: result.recoveredFiles, outputEncrypted: true, partial: true, originalPreserved: true, independentExtraction: true })
    }
    if (mode === 'AES256') { healthyAes = healthy; lostAes = lost }
  }

  // Start from a real engine failure, then use the actual inline rescue flow.
  await bridge('clearTasks')
  const originalOutput = path.join(root, '原输出'); mkdirSync(originalOutput)
  await failure('extractArchive', lostAes, originalOutput, password)
  const [failed] = await bridge('extractionRecoveryTasks')
  assert.equal(failed.status, 'failed')
  await click('[data-testid="nav-Decompress"]')
  await click(`[data-task-id="${failed.id}"]`)
  await waitForElement('[data-testid="inline-extraction-recovery"]')
  const editedOutput = path.join(root, '修改后的输出'); mkdirSync(editedOutput)
  await fill('[data-testid="retry-output"]', editedOutput)
  await fill('[data-testid="retry-password"]', password)
  for (const width of [1100, 760]) {
    await driver.manage().window().setRect({ width, height: 700 })
    await screenshot(`failure-recovery-inline-${width}`, '[data-testid="inline-extraction-recovery"]')
  }
  await click('[data-testid="retry-rescue"]')
  await waitForElement('[data-testid="recovery-task-context"]')
  await waitForElement('[data-testid="repair-password-note"]')
  const uiOutput = path.join(root, '页面救回的新包.zip')
  await bridge('queueDesktopDialogSelections', [uiOutput])
  const repairButton = await driver.findElement(By.xpath('//button[contains(.,"选择位置并修复")]'))
  await driver.executeScript(el => el.scrollIntoView({ block: 'center' }), repairButton); await repairButton.click()
  await waitForElement('[data-testid="repair-result"]')
  await screenshot('failure-recovery-result-760', '[data-testid="repair-result"]')
  await click('[data-testid="queue-recovered-archive"]')
  const queued = await driver.wait(async () => (await bridge('extractionRecoveryTasks')).find(task => task.status === 'pending') || false, 10_000)
  assert.equal(queued.output, editedOutput); assert.equal(queued.retryOfTaskId, failed.id); assert.ok(queued.hasPassword)
  await click('[data-testid="nav-Decompress"]')
  await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async () => await bridge('taskStatus', queued.id) === 'completed', 30_000)
  const restored = queued.extractToSubfolder ? path.join(editedOutput, path.basename(uiOutput, '.zip')) : editedOutput
  assert.deepEqual(readFileSync(path.join(restored, '完整说明.txt')), good)
  assert.equal(existsSync(path.join(restored, 'broken.bin')), false)

  // Restore a missing member, then only run the newly queued correction.
  await bridge('clearTasks')
  const healthyBytes = readFileSync(healthyAes), healthyParts = []
  for (let offset = 0, index = 1; offset < healthyBytes.length; offset += 65536, index++) {
    const part = path.join(root, `补卷.zip.${String(index).padStart(3, '0')}`)
    writeFileSync(part, healthyBytes.subarray(offset, offset + 65536)); healthyParts.push(part)
  }
  const missingBytes = readFileSync(healthyParts[1]); unlinkSync(healthyParts[1])
  const volumeOutput = path.join(root, '补卷输出'); mkdirSync(volumeOutput)
  await failure('extractArchive', healthyParts[0], volumeOutput, password)
  const [volumeFailure] = await bridge('extractionRecoveryTasks')
  await click(`[data-task-id="${volumeFailure.id}"]`)
  await driver.wait(async () => {
    const elements = await driver.findElements(By.css('[data-testid="retry-missing-volumes"]'))
    return elements.length > 0 && (await elements[0].getText()).includes(path.basename(healthyParts[1]))
  }, 10_000, 'The expanded correction must show the missing volume name')
  await screenshot('failure-recovery-missing-volumes-760', '[data-testid="inline-extraction-recovery"]')
  writeFileSync(healthyParts[1], missingBytes)
  await fill('[data-testid="retry-password"]', password)
  await click('[data-testid="create-retry-drafts"]')
  const retry = await driver.wait(async () => (await bridge('extractionRecoveryTasks')).find(task => task.status === 'pending') || false, 10_000)
  assert.ok(retry); assert.equal(retry.output, volumeOutput); assert.equal(retry.retryOfTaskId, volumeFailure.id)
  await click('[data-testid="start-decompression-queue"]')
  await driver.wait(async () => await bridge('taskStatus', retry.id) === 'completed', 30_000)
  const volumeRestored = retry.extractToSubfolder ? path.join(volumeOutput, '补卷') : volumeOutput
  assert.deepEqual(readFileSync(path.join(volumeRestored, '完整说明.txt')), good)
  assert.equal(sha256(path.join(volumeRestored, 'broken.bin')), sha256(path.join(source, 'broken.bin')))
  writeFileSync(path.join(artifactDirectory, 'failure-zip-recovery-results.json'), JSON.stringify({
    testKind: 'real-windows-webview2-failure-and-lost-directory-password-zip',
    binarySha256: sha256(application), records,
    realFailedTaskToRescueToExtraction: true, editedOutputPreserved: true,
    realMissingVolumeToCorrectionToExtraction: true, passed: true,
  }, null, 2))
}

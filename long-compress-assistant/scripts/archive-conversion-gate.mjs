import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { randomBytes, createHash } from 'node:crypto'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runArchiveConversionGate({ driver, waitForElement, bridge, failure, fixtureDirectory, artifactDirectory, bundledSevenZip, productionIpc }) {
  if (productionIpc) {
    bridge = async (method, ...args) => {
      const taskId = `installed-conversion-${Date.now()}-${Math.random().toString(16).slice(2)}`
      if (method === 'convertArchive') return productionIpc('convert_archive', {
        taskId, sourcePath: args[0], sourcePassword: args[1], outputPath: args[2],
        options: { level: 3, preserve_paths: true, verify_after: true, delete_after: false, ...args[3] },
      })
      if (method === 'extractArchive') return productionIpc('extract_file', {
        taskId, filePath: args[0], outputPath: args[1], password: args[2],
        options: { preserve_paths: true, create_subdirectory: false, delete_after: false },
      })
      if (method === 'taskHistory') return productionIpc('list_task_history', { limit: 500 })
      throw new Error(`Unsupported production adapter: ${method}`)
    }
    failure = async (...args) => { try { await bridge(...args) } catch (error) { return String(error) } throw new Error('Expected failure') }
  }
  const fill = async (testId, value) => {
    const input = await waitForElement(`[data-testid="${testId}"]`)
    await input.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (value) await input.sendKeys(value)
    assert.equal(await input.getAttribute('value'), value)
  }
  const root = path.join(fixtureDirectory, 'conversion')
  const source = path.join(root, '原始文件')
  mkdirSync(path.join(source, '中文目录', '空目录'), { recursive: true })
  const payload = randomBytes(160000)
  writeFileSync(path.join(source, '中文目录', '内容.bin'), payload)
  writeFileSync(path.join(source, '说明.txt'), '转换应保留内容与目录，不是把旧包再次打包。')
  const run = (args, ok = true) => {
    const result = spawnSync(bundledSevenZip, args, { cwd: source, encoding: 'utf8', windowsHide: true })
    if (ok) assert.equal(result.status, 0, result.stdout + result.stderr)
    else assert.notEqual(result.status, 0, 'Old password unexpectedly accepted')
  }
  const original = path.join(root, '原包.zip')
  run(['a', '-tzip', '-pold-password', '-mem=AES256', original, '.\\*'])
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const originalHash = hash(original)
  const target = path.join(root, '改密分卷.7z')
  const result = await bridge('convertArchive', original, 'old-password', target, {
    format: '7z', password: 'new-password', encrypt_filenames: true, split_size: 65536,
  })
  assert.equal(result.split, true)
  assert.ok(result.files.length > 1)
  run(['t', result.primaryPath, '-pold-password'], false)
  const restored = path.join(root, '还原')
  run(['x', result.primaryPath, '-pnew-password', `-o${restored}`, '-y'])
  assert.deepEqual(readFileSync(path.join(restored, '中文目录', '内容.bin')), payload)
  assert.deepEqual(readFileSync(path.join(restored, '说明.txt')), readFileSync(path.join(source, '说明.txt')))
  assert.ok(existsSync(path.join(restored, '中文目录', '空目录')))
  assert.deepEqual(readdirSync(restored).sort(), ['中文目录', '说明.txt'].sort())
  const plain = path.join(root, '去密合卷.zip')
  const merged = await bridge('convertArchive', result.primaryPath, 'new-password', plain, { format: 'zip', password: null })
  assert.equal(merged.split, false)
  const plainRestored = path.join(root, '去密还原')
  run(['x', plain, `-o${plainRestored}`, '-y'])
  assert.deepEqual(readFileSync(path.join(plainRestored, '中文目录', '内容.bin')), payload)
  assert.ok(existsSync(path.join(plainRestored, '中文目录', '空目录')))
  assert.deepEqual(readFileSync(path.join(plainRestored, '说明.txt')), readFileSync(path.join(source, '说明.txt')))
  const formatChecks = []
  for (const format of ['tar', 'tar.gz', 'tar.bz2', 'tar.xz', 'tar.zst', 'wim']) {
    const output = path.join(root, `格式兼容.${format}`)
    await bridge('convertArchive', original, 'old-password', output, { format, password: null })
    run(['t', output])
    const destination = path.join(root, `还原-${format}`)
    await bridge('extractArchive', output, destination, null)
    assert.deepEqual(readFileSync(path.join(destination, '中文目录', '内容.bin')), payload)
    assert.deepEqual(readFileSync(path.join(destination, '说明.txt')), readFileSync(path.join(source, '说明.txt')))
    assert.ok(existsSync(path.join(destination, '中文目录', '空目录')))
    formatChecks.push(format)
  }
  const splitZip = await bridge('convertArchive', original, 'old-password', path.join(root, '换卷.zip'), { format: 'zip', split_size: 65536 })
  await bridge('convertArchive', splitZip.primaryPath, null, path.join(root, 'ZIP分卷转7Z.7z'), { format: '7z' })
  run(['t', path.join(root, 'ZIP分卷转7Z.7z')])
  const wrongOutput = path.join(root, '错误密码.7z')
  assert.match(await failure('convertArchive', original, 'wrong-password', wrongOutput, { format: '7z' }), /转换读取原包失败/)
  assert.equal(existsSync(wrongOutput), false)
  assert.match(await failure('convertArchive', original, 'old-password', original, { format: 'zip' }), /不能覆盖/)
  const corrupt = path.join(root, '损坏.zip')
  writeFileSync(corrupt, readFileSync(original).subarray(0, 50))
  const corruptOutput = path.join(root, '损坏转换.7z')
  assert.match(await failure('convertArchive', corrupt, null, corruptOutput, { format: '7z' }), /转换读取原包失败/)
  assert.equal(existsSync(corruptOutput), false)
  assert.equal(hash(original), originalHash)
  assert.deepEqual(readdirSync(root).filter(name => name.startsWith('.long-convert-')), [])
  await (await waitForElement('[data-testid="nav-Compress"]')).click()
  await (await waitForElement('[data-testid="open-archive-conversion"]')).click()
  const uiOutput = path.join(root, '页面改密.7z')
  if (productionIpc) {
    await fill('conversion-source', original)
    await fill('conversion-output', uiOutput)
  } else {
    await bridge('queueDesktopDialogSelections', [original, uiOutput])
    await (await waitForElement('[data-testid="conversion-choose-source"]')).click()
    await (await waitForElement('[data-testid="conversion-choose-output"]')).click()
  }
  await (await waitForElement('[data-testid="conversion-source-password"]')).sendKeys('old-password')
  await (await waitForElement('[data-testid="conversion-password"]')).sendKeys('ui-new-password')
  await (await waitForElement('[data-testid="conversion-hidden-names"]')).click()
  for (const size of [{ width: 1280, height: 900 }, { width: 1100, height: 760 }]) {
    await driver.manage().window().setRect(size)
    await driver.wait(async () => {
      const rect = await (await waitForElement('[data-testid="archive-conversion"]')).getRect()
      return rect.width > 0 && rect.x >= 0
    }, 5000)
    writeFileSync(path.join(artifactDirectory, `archive-conversion-${size.width}.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  await (await waitForElement('[data-testid="conversion-start"]')).click()
  let record
  await driver.wait(async () => {
    record = (await bridge('taskHistory')).find(item => item.executionConfig?.kind === 'archive-conversion' && item.outputPath === uiOutput)
    return !!record
  }, 60000)
  assert.equal(record.status, 'completed', record.errorMessage)
  assert.ok(record.archiveOutput)
  assert.doesNotMatch(JSON.stringify(record), /old-password|ui-new-password/)
  run(['t', uiOutput, '-pui-new-password'])
  const uiRestored = path.join(root, '页面还原')
  run(['x', uiOutput, '-pui-new-password', `-o${uiRestored}`, '-y'])
  assert.deepEqual(readFileSync(path.join(uiRestored, '中文目录', '内容.bin')), payload)
  writeFileSync(path.join(artifactDirectory, 'archive-conversion-completed.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  await driver.findElement(By.css('[role="dialog"] button[aria-label="Close dialog"]')).click()
  const historyBeforeRetry = await bridge('taskHistory')
  await (await waitForElement('[data-testid="nav-History"]')).click()
  await (await waitForElement('[data-testid="history-conversion-retry"]')).click()
  assert.equal(await (await waitForElement('[data-testid="conversion-source-password"]')).getAttribute('value'), '')
  assert.equal(await (await waitForElement('[data-testid="conversion-password"]')).getAttribute('value'), '')
  assert.equal(await (await waitForElement('[data-testid="conversion-output"]')).getAttribute('value'), '')
  assert.deepEqual(await bridge('taskHistory'), historyBeforeRetry)
  writeFileSync(path.join(artifactDirectory, 'archive-conversion-retry.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  const retryOutput = path.join(root, '再次转换.7z')
  await (await waitForElement('[data-testid="conversion-source-password"]')).sendKeys('old-password')
  await (await waitForElement('[data-testid="conversion-password"]')).sendKeys('retry-password')
  await (await waitForElement('[data-testid="conversion-output"]')).sendKeys(retryOutput)
  await (await waitForElement('[data-testid="conversion-start"]')).click()
  let retried
  await driver.wait(async () => {
    retried = (await bridge('taskHistory')).find(item => item.outputPath === retryOutput)
    return !!retried
  }, 60000)
  assert.equal(retried.status, 'completed', retried.errorMessage)
  assert.equal(retried.retryOfTaskId, record.id)
  assert.deepEqual((await bridge('taskHistory')).find(item => item.id === record.id), record)
  run(['t', retryOutput, '-pretry-password'])
  const largeSource = path.join(root, '取消源.bin')
  writeFileSync(largeSource, randomBytes(96 * 1048576))
  const largeArchive = path.join(root, '取消原包.zip')
  run(['a', '-tzip', '-mx=0', largeArchive, largeSource])
  const largeHash = hash(largeArchive)
  const cancelOutput = path.join(root, '取消新包.7z')
  for (const [testId, value] of [['conversion-source', largeArchive], ['conversion-output', cancelOutput], ['conversion-password', 'cancel-password']]) {
    await fill(testId, value)
  }
  await (await waitForElement('[data-testid="conversion-start"]')).click()
  await (await waitForElement('[data-testid="conversion-cancel"]')).click()
  let cancelled
  await driver.wait(async () => {
    cancelled = (await bridge('taskHistory')).find(item => item.outputPath === cancelOutput)
    return !!cancelled
  }, 60000)
  assert.equal(cancelled.status, 'cancelled', cancelled.errorMessage)
  assert.doesNotMatch(await (await waitForElement('[data-testid="task-summary-label"]')).getText(), /进行中/)
  assert.equal(existsSync(cancelOutput), false)
  assert.equal(hash(largeArchive), largeHash)
  assert.deepEqual(readdirSync(root).filter(name => name.startsWith('.long-convert-')), [])
  writeFileSync(path.join(artifactDirectory, 'archive-conversion-cancelled.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  const writingCancelOutput = path.join(root, '写入中取消.7z')
  await fill('conversion-output', writingCancelOutput)
  await fill('conversion-password', 'cancel-password')
  await (await waitForElement('[data-testid="conversion-start"]')).click()
  await driver.wait(async () => /步骤 2\/2/.test(await (await waitForElement('[data-testid="archive-conversion"]')).getText()), 60000)
  await (await waitForElement('[data-testid="conversion-cancel"]')).click()
  await driver.wait(async () => {
    cancelled = (await bridge('taskHistory')).find(item => item.outputPath === writingCancelOutput)
    return !!cancelled
  }, 60000)
  assert.equal(cancelled.status, 'cancelled', cancelled.errorMessage)
  assert.doesNotMatch(await (await waitForElement('[data-testid="task-summary-label"]')).getText(), /进行中/)
  assert.equal(existsSync(writingCancelOutput), false)
  assert.equal(hash(largeArchive), largeHash)
  assert.deepEqual(readdirSync(root).filter(name => name.startsWith('.long-convert-')), [])
  writeFileSync(path.join(artifactDirectory, 'archive-conversion-writing-cancelled.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  await fill('conversion-source', original)
  await fill('conversion-source-password', 'old-password')
  await (await waitForElement('[data-testid="conversion-format"]')).click()
  await driver.findElement(By.xpath('//button[@role="option" and normalize-space(.)="ZIP"]')).click()
  const zipUiOutput = path.join(root, '页面传统兼容.zip')
  await fill('conversion-output', zipUiOutput)
  await fill('conversion-password', '中文密码 + !测试')
  await (await waitForElement('[data-testid="conversion-zip-encryption"]')).click()
  await driver.findElement(By.xpath('//button[@role="option" and contains(.,"ZipCrypto")]')).click()
  await (await waitForElement('[data-testid="conversion-zip-encoding"]')).click()
  await driver.findElement(By.xpath('//button[@role="option" and contains(.,"Windows 兼容")]')).click()
  writeFileSync(path.join(artifactDirectory, 'archive-conversion-zip-settings.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
  await (await waitForElement('[data-testid="conversion-start"]')).click()
  let zipRecord
  await driver.wait(async () => {
    zipRecord = (await bridge('taskHistory')).find(item => item.outputPath === zipUiOutput)
    return !!zipRecord
  }, 60000)
  assert.equal(zipRecord.status, 'completed', zipRecord.errorMessage)
  assert.equal(zipRecord.executionConfig.zipEncryption, 'zipcrypto')
  assert.equal(zipRecord.executionConfig.zipPasswordEncoding, 'windows_local')
  run(['t', zipUiOutput, '-p中文密码 + !测试'])
  writeFileSync(path.join(artifactDirectory, 'archive-conversion.json'), JSON.stringify({
    buildKind: productionIpc ? 'installed-production' : 'isolated-desktop-e2e', nativePickerVerified: false,
    sourcePreserved: true, contentVerified: true, emptyDirectoryPreserved: true,
    passwordChanged: true, resplit: true, mergedAndDecrypted: true, wrongPasswordRejected: true,
    stagingCleaned: true, corruptInputRejected: true, uiExecutionVerified: true, historyRetryDraftVerified: true,
    historyRetryExecutionVerified: true, actualCancellationVerified: true,
    additionalTargetFormats: formatChecks, numericZipInputVerified: true,
    writingCancellationVerified: true, zipCryptoChineseUiConversionVerified: true,
  }, null, 2))
}

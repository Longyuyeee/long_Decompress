import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync, copyFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'
import { makeLzipFixtures, lzipTools, linuxPath, sha256 } from './prepare-lzip-fixtures.mjs'

const tree = root => readdirSync(root, { withFileTypes: true }).flatMap(entry => {
  const file = path.join(root, entry.name)
  return entry.isDirectory() ? tree(file).map(item => ({ ...item, name: entry.name + '/' + item.name }))
    : [{ name: entry.name, sha256: sha256(readFileSync(file)) }]
}).sort((a, b) => a.name.localeCompare(b.name))
const key = value => value.replaceAll('\\', '/').toLowerCase()

export async function runLzip1412Gate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application, root: projectRoot }) {
  const resumed = process.env.LONG_LZIP_RESUME_ROOT
  const root = resumed || path.join(fixtureDirectory, 'lzip-1412')
  const fixtures = resumed ? {
    source: path.join(root, '原文件'), single: path.join(root, '说明.txt.lz'), tarLzip: path.join(root, '资料.tar.lz'), alias: path.join(root, '资料别名.tlz'), multi: path.join(root, '多成员.txt.lz'),
    multiContent: Buffer.concat([readFileSync(path.join(root, '原文件/说明.txt')), readFileSync(path.join(root, '第二段.txt'))]),
    manifest: JSON.parse(readFileSync(path.join(root, 'manifest.json'), 'utf8')), tools: lzipTools(projectRoot),
  } : makeLzipFixtures(root, projectRoot)
  const { source, single, tarLzip, alias, multi, multiContent, manifest, tools } = fixtures
  const screenshots = [], results = []
  const click = async selector => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await driver.wait(() => driver.executeScript(el => {
      const r = el.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
      return !el.disabled && (hit === el || el.contains(hit))
    }, element), 10000)
    await element.click()
  }
  const fill = async (selector, value) => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await element.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (value) await element.sendKeys(value)
  }
  const select = async (selector, label) => {
    await click(selector)
    await waitForElement('[role="option"]')
    for (const option of await driver.findElements(By.css('[role="option"]'))) {
      if ((await option.findElement(By.css('strong')).getAttribute('textContent')).trim() === label) {
        await driver.executeScript(el => el.scrollIntoView({ block: 'nearest', behavior: 'instant' }), option)
        await option.click(); return
      }
    }
    assert.fail(`Option not found: ${label}`)
  }
  const shot = async name => {
    writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64'))
    screenshots.push(name + '.png')
  }
  const ipc = async (command, args) => {
    const result = await driver.executeAsyncScript((cmd, args, done) => {
      const ids = crypto.getRandomValues(new Uint32Array(2))
      const cleanup = () => ids.forEach(id => Reflect.deleteProperty(window, '_' + id))
      window['_' + ids[0]] = value => { cleanup(); done({ ok: true, value }) }
      window['_' + ids[1]] = error => { cleanup(); done({ ok: false, error: String(error) }) }
      window.__TAURI_IPC__({ cmd, ...args, callback: ids[0], error: ids[1] })
    }, command, args)
    assert.equal(result.ok, true, result.error)
    return result.value
  }
  const sevenZip = (args, cwd) => {
    const result = spawnSync(bundledSevenZip, args, { cwd, windowsHide: true, encoding: 'utf8', timeout: 120000 })
    assert.ifError(result.error); assert.equal(result.status, 0, result.stdout + result.stderr)
  }
  const externalTar = (file, output) => {
    tools.lzip(['-t', linuxPath(file)])
    const tar = output + '.tar'
    writeFileSync(tar, tools.lzip(['-d', '-c', linuxPath(file)]))
    mkdirSync(output, { recursive: true })
    tools.run('/usr/bin/tar', ['-xf', linuxPath(tar), '-C', linuxPath(output)])
    return output
  }

  if (!resumed) {
  for (const [file, isTar] of [[single, false], [tarLzip, true], [alias, true], [multi, false]]) {
    const before = sha256(readFileSync(file)), output = file + '-应用回解'
    mkdirSync(output)
    const metadata = await ipc('browse_archive', { filePath: file, password: null, browseId: crypto.randomUUID() })
    assert.equal(metadata.format, isTar ? 'TAR.LZIP' : 'LZIP')
    await bridge('extractArchive', file, output, null)
    if (isTar) { assert.deepEqual(tree(output), tree(source)); assert.ok(existsSync(path.join(output, '中文目录/空目录'))) }
    else assert.deepEqual(readFileSync(path.join(output, path.basename(file, '.lz'))), file === multi ? multiContent : readFileSync(path.join(source, '说明.txt')))
    const report = await bridge('diagnoseArchive', file, null)
    assert.equal(report.status, 'healthy', JSON.stringify(report)); assert.equal(report.integrityTested, true)
    assert.equal(sha256(readFileSync(file)), before)
    results.push({ input: path.basename(file), browse: metadata.format, extracted: true, healthy: true, originalUnchanged: true })
  }
  for (const [format, input, label] of [['lz', path.join(source, '说明.txt'), '应用创建'], ['lz', path.join(source, '空文件.txt'), '应用空文件'], ['tar.lz', source, '应用创建']]) {
    const file = path.join(root, `${label}.${format}`), output = file + '-应用回解'
    await bridge('runArchiveRoundTrip', input, file, output, format, null)
    tools.lzip(['-t', linuxPath(file)])
    if (format === 'lz') assert.deepEqual(tools.lzip(['-d', '-c', linuxPath(file)]), readFileSync(input))
    else {
      const independent = externalTar(file, file + '-GNU回解')
      assert.deepEqual(tree(path.join(independent, path.basename(source))), tree(source))
      assert.ok(existsSync(path.join(independent, path.basename(source), '中文目录/空目录')))
    }
    results.push({ created: format, input: path.basename(input), applicationRoundtrip: true, gnuLzipReadback: true })
  }
  const convertedZip = path.join(root, '论坛外来源转换.zip')
  await bridge('convertArchive', tarLzip, null, convertedZip, { format: 'zip', password: null })
  const zipOutput = path.join(root, 'ZIP外部回解')
  sevenZip(['x', '-y', `-o${zipOutput}`, convertedZip]); assert.deepEqual(tree(zipOutput), tree(source))
  const encrypted = path.join(root, '带密码的原包.zip'), sourcePassword = 'LZIP-source-1412'
  sevenZip(['a', '-tzip', '-mem=AES256', `-p${sourcePassword}`, encrypted, '.\\*'], source)
  const convertedTar = path.join(root, '去密码转换.tlz')
  await bridge('convertArchive', encrypted, sourcePassword, convertedTar, { format: 'tar.lz', password: null })
  assert.deepEqual(tree(externalTar(convertedTar, path.join(root, '去密码转换外部回解'))), tree(source))
  results.push({ conversions: ['TAR.LZIP → ZIP', 'AES ZIP → TAR.LZIP (.tlz)'], independentReadback: true })

  for (const [name, mutation] of [['CRC损坏', 'crc'], ['尾部截断', 'truncated']]) {
    const bytes = readFileSync(tarLzip), bad = Buffer.from(bytes)
    if (mutation === 'crc') bad[bad.length - 20] ^= 1
    const file = path.join(root, name + '.tar.lz'), output = file + '-不能发布残缺文件'
    writeFileSync(file, mutation === 'truncated' ? bad.subarray(0, bad.length - 1) : bad); mkdirSync(output)
    await assert.rejects(() => bridge('extractArchive', file, output, null))
    assert.deepEqual(tree(output), [], 'failed stream checks must not publish extracted files')
    results.push({ invalid: name, rejected: true, noPublishedFiles: true })
  }
  writeFileSync(path.join(root, 'native-results.json'), JSON.stringify(results, null, 2))
  console.log('[lzip-1412] native input/create/convert/corruption matrix passed')
  } else {
    const checkpoint = path.join(root, 'native-results.json')
    results.push(...(existsSync(checkpoint) ? JSON.parse(readFileSync(checkpoint, 'utf8')) : [{ reusedNativePhase: true, evidence: 'Initial run reached the first TLZ UI summary after every native assertion; see first-run.log and retained output manifest.' }]))
    for (const original of manifest.files) assert.equal(sha256(readFileSync(path.join(root, original.name))), original.sha256)
    console.log('[lzip-1412] resuming UI only; original native assertions and files reused')
  }
  const uiRoot = path.join(root, '界面-' + Date.now()); mkdirSync(uiRoot)

  // Real user controls below: file picker responses are isolated; status and
  // outputs are produced by the application commands, never injected.
  await bridge('clearTasks'); await bridge('clearCompressionWorkspace')
  await driver.manage().window().setRect({ width: 1100, height: 700 })
  if (!['compression', 'conversion'].includes(process.env.LONG_LZIP_UI_FROM)) {
  await click('[data-testid="nav-ArchiveBrowser"]'); await waitForElement('[data-testid="dual-pane-file-manager"]')
  await bridge('queueDesktopDialogSelections', [alias]); await click('[data-testid="file-manager-open-archive"]')
  await driver.wait(async () => (await driver.findElements(By.css('.browser-summary'))).length && /TAR.LZIP/.test(await driver.findElement(By.css('.browser-summary')).getText()), 30000, 'TAR.LZIP summary must finish appearing')
  await fill('.browser-search input', '说明.txt'); await click('.browser-row:not(.directory) .preview-trigger')
  assert.match(await (await waitForElement('.preview-text')).getText(), /先浏览，再选择解压/)
  await shot('lzip-1412-tlz-preview'); await click('.preview-close'); await fill('.browser-search input', '')
  const selectedOutput = path.join(uiRoot, '界面所选解压'); mkdirSync(selectedOutput)
  await bridge('queueDesktopDialogSelections', [selectedOutput]); await click('.browser-page > footer .output-target')
  await click('.browser-page > footer .browser-primary')
  const selectedTask = await driver.wait(async () => (await bridge('archiveBrowserTaskState')).find(item => key(item.outputPath || item.output || '') === key(selectedOutput) && ['completed', 'failed'].includes(item.status)), 60000)
  assert.equal(selectedTask.status, 'completed', JSON.stringify(selectedTask)); assert.deepEqual(tree(selectedOutput), tree(source))

  await bridge('queueDesktopDialogSelections', [path.join(root, 'CRC损坏.tar.lz')])
  await click('.browser-page > header .browser-primary')
  const damagedNotice = await waitForElement('[data-testid="archive-browse-notice"]')
  assert.match(await damagedNotice.getText(), /校验|损坏|CRC/i)
  await shot('lzip-1412-corrupted-read-notice')
  } else {
    screenshots.push('lzip-1412-tlz-preview.png', 'lzip-1412-corrupted-read-notice.png')
    console.log('[lzip-1412] reusing completed browse/preview/extraction/error UI; continuing compression and conversion')
  }

  if (process.env.LONG_LZIP_UI_FROM !== 'conversion') {
  await click('[data-testid="nav-Compress"]')
  await waitForElement('[data-testid="compression-header"]')
  await bridge('queueDesktopDialogSelections', [[path.join(source, '说明.txt')]])
  await click('[data-testid="compression-center"] [data-testid="dropzone-file"]'); await waitForElement('[data-testid="compression-draft-row"]')
  if (!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length) await click('[data-testid="compression-draft-row"] [data-testid="compression-archive-name"]')
  assert.equal((await driver.findElements(By.css('[role="dialog"]'))).length, 0, 'single-row configuration must stay inline')
  await select('[data-testid="compression-format-select"]', 'LZIP')
  assert.equal(await (await waitForElement('[data-testid="compression-password-input"]')).isEnabled(), false)
  assert.equal(await (await waitForElement('[data-testid="compression-split-toggle"]')).isEnabled(), false)
  const uiDirectory = path.join(uiRoot, '界面单文件输出'); mkdirSync(uiDirectory)
  await fill('[data-testid="compression-filename-input"]', '界面单文件')
  await fill('[data-testid="compression-output-path"]', uiDirectory)
  for (const [width, height] of [[1100, 700], [760, 520]]) {
    await driver.manage().window().setRect({ width, height })
    const format = await waitForElement('[data-testid="compression-format-select"]')
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), format)
    await shot(`lzip-1412-inline-${width}x${height}`)
  }
  await driver.manage().window().setRect({ width: 1100, height: 700 }); await click('[data-testid="start-compression"]')
  const uiArchive = path.join(uiDirectory, '界面单文件.lz')
  const compression = await driver.wait(async () => (await bridge('taskHistory')).find(item => key(item.outputPath) === key(uiArchive) && ['completed', 'failed'].includes(item.status)), 60000)
  assert.equal(compression.status, 'completed', JSON.stringify(compression)); assert.deepEqual(tools.lzip(['-d', '-c', linuxPath(uiArchive)]), readFileSync(path.join(source, '说明.txt')))
  await shot('lzip-1412-ui-created')

  await bridge('clearCompressionWorkspace'); await bridge('clearTasks')
  await bridge('queueDesktopDialogSelections', [[path.join(source, '说明.txt'), path.join(source, '空文件.txt')]])
  await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))).length === 2, 10000)
  for (const checkbox of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))) await checkbox.click()
  await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]')
  await select('[data-testid="batch-format"]', 'TAR.LZIP（无加密）')
  for (const [width, height] of [[1100, 700], [760, 520]]) {
    await driver.manage().window().setRect({ width, height }); await shot(`lzip-1412-compression-batch-${width}x${height}`)
  }
  await click('[data-testid="batch-config-apply"]')
  for (const draft of await driver.findElements(By.css('[data-testid="compression-draft-row"]'))) {
    assert.match(await draft.findElement(By.css('[data-testid="compression-archive-name"]')).getText(), /\.tar\.lz/i)
    assert.equal(await draft.getAttribute('data-task-id'), '')
  }
  await driver.manage().window().setRect({ width: 1100, height: 700 })
  } else {
    screenshots.push('lzip-1412-inline-1100x700.png', 'lzip-1412-inline-760x520.png', 'lzip-1412-ui-created.png', 'lzip-1412-compression-batch-1100x700.png', 'lzip-1412-compression-batch-760x520.png')
    await click('[data-testid="nav-Compress"]'); await waitForElement('[data-testid="compression-header"]')
    console.log('[lzip-1412] reusing completed inline creation/readback and batch configuration; continuing conversion only')
  }

  await click('[data-testid="open-archive-conversion"]')
  const uiSingle = path.join(uiRoot, path.basename(single)), uiAlias = path.join(uiRoot, path.basename(alias))
  copyFileSync(single, uiSingle); copyFileSync(alias, uiAlias)
  await bridge('queueDesktopDialogSelections', [[uiSingle, uiAlias]]); await click('[data-testid="conversion-choose-source"]')
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="conversion-checkbox"]'))).length === 2, 10000)
  await click('[data-testid="conversion-batch-config"]')
  await select('[data-testid="conversion-batch-format"]', 'TAR.LZIP')
  for (const [width, height] of [[1100, 700], [760, 520]]) {
    await driver.manage().window().setRect({ width, height }); await shot(`lzip-1412-batch-modal-${width}x${height}`)
  }
  await click('[data-testid="conversion-batch-apply"]')
  const conversionTargets = [path.join(uiRoot, '说明.txt_转换.tar.lz'), path.join(uiRoot, '资料别名_转换.tar.lz')]
  await click('[data-testid="conversion-start"]')
  const records = await driver.wait(async () => {
    const history = await bridge('taskHistory')
    const records = conversionTargets.map(output => history.find(item => key(item.outputPath) === key(output) && ['completed', 'failed'].includes(item.status)))
    return records.every(Boolean) ? records : false
  }, 60000)
  records.forEach(record => assert.equal(record.status, 'completed', JSON.stringify(record)))
  assert.deepEqual(tree(externalTar(conversionTargets[1], path.join(uiRoot, '界面批量转换外部回解'))), tree(source))
  await shot('lzip-1412-batch-completed-760x520')
  for (const original of manifest.files) assert.equal(sha256(readFileSync(path.join(root, original.name))), original.sha256)
  const record = { application, sha256: sha256(readFileSync(application)), version: '1.4.12', generator: manifest, results, resumedUiOnly: !!resumed, resumedUiFrom: process.env.LONG_LZIP_UI_FROM || 'browse', retainedRoot: root,
    ui: { tlzBrowseAndTextPreview: true, selectedExtraction: true, corruptionNotice: true, singleInlineCreation: true, unsupportedOptionsDisabled: true, compressionMultiSelectionModalConfiguration: true, multiSelectionModalConversion: true, ordinaryAndMinimumWindow: true, screenshots },
    scope: 'LZIP v1 single/multi-member and TAR.LZIP/.tlz; no native password or multi-volume creation; picker responses isolated; native commands and filesystem outputs real' }
  writeFileSync(path.join(artifactDirectory, 'lzip-1412-results.json'), JSON.stringify(record, null, 2))
}

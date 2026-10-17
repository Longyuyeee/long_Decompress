import assert from 'node:assert/strict'
import { mkdirSync, readFileSync, writeFileSync, readdirSync, existsSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
const tree = root => readdirSync(root, { withFileTypes: true }).flatMap(entry => entry.isDirectory()
  ? tree(path.join(root, entry.name)).map(item => ({ ...item, name: entry.name + '/' + item.name }))
  : [{ name: entry.name, sha256: hash(path.join(root, entry.name)) }]).sort((a, b) => a.name.localeCompare(b.name))
const key = value => value.replaceAll('\\', '/').toLowerCase()

export async function runZipOutput1414Gate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, bundledSevenZip, application }) {
  const resumed = process.env.LONG_ZIP_OUTPUT_RESUME_ROOT
  const root = resumed || path.join(fixtureDirectory, 'zip-output-1414')
  mkdirSync(root, { recursive: true })
  const source = path.join(root, '原文件'), password = 'ZIP-output + 1414!', results = [], screenshots = [], geometry = []
  const sevenZip = (args, cwd) => {
    const result = spawnSync(bundledSevenZip, ['-sccUTF-8', ...args], { cwd, windowsHide: true, encoding: 'utf8', timeout: 120000 })
    assert.ifError(result.error); assert.equal(result.status, 0, result.stdout + result.stderr)
    return result.stdout
  }
  const ipc = async (command, args) => {
    const result = await driver.executeAsyncScript((cmd, args, done) => {
      const ids = crypto.getRandomValues(new Uint32Array(2)), cleanup = () => ids.forEach(id => Reflect.deleteProperty(window, '_' + id))
      window['_' + ids[0]] = value => { cleanup(); done({ ok: true, value }) }
      window['_' + ids[1]] = error => { cleanup(); done({ ok: false, error: String(error) }) }
      window.__TAURI_IPC__({ cmd, ...args, callback: ids[0], error: ids[1] })
    }, command, args)
    assert.equal(result.ok, true, `${command}: ${result.error}`); return result.value
  }
  const verify = (input, output, expected, secret, method) => {
    const listing = sevenZip(['l', '-slt', `-p${secret || '-'}`, input])
    if (method) assert.match(listing, new RegExp(`^Method = .*${method}`, 'mi'))
    sevenZip(['x', '-y', `-p${secret || '-'}`, `-o${output}`, input])
    assert.deepEqual(tree(output), expected)
    return listing
  }
  if (process.env.LONG_ZIP_OUTPUT_DIAG === '1') {
    const diagnostic = path.join(artifactDirectory, 'xz-stream-diagnostic'); mkdirSync(diagnostic, { recursive: true })
    for (const [name, payload] of [['empty', Buffer.alloc(0)], ['small', Buffer.from('small text'.repeat(120))]]) {
      const file = path.join(diagnostic, name + '.txt'), output = path.join(diagnostic, name + '.zipx')
      writeFileSync(file, payload)
      await ipc('compress_files', { taskId: 'diag-' + name, files: [file], outputPath: output,
        options: { format: 'zipx', level: 6, preserve_paths: true, verify_after: false } })
      const tested = spawnSync(bundledSevenZip, ['t', output], { windowsHide: true, encoding: 'utf8' })
      console.log('[xz-diagnostic]', name, tested.status, tested.stdout, tested.stderr)
    }
    return
  }
  const checkpoint = path.join(root, 'native-results.json'), progress = path.join(root, 'native-progress.json')
  if (!resumed || !existsSync(checkpoint)) {
    if (existsSync(progress)) results.push(...JSON.parse(readFileSync(progress, 'utf8')))
    mkdirSync(path.join(source, '中文目录/空目录'), { recursive: true })
    writeFileSync(path.join(source, '中文目录/说明.txt'), '归档方法、密码和分卷应当在整个流程中保持一致。\n'.repeat(120))
    writeFileSync(path.join(source, '空文件.txt'), '')
    let seed = 1414
    writeFileSync(path.join(source, '内容.bin'), Buffer.from(Array.from({ length: 260000 }, () => {
      seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5; return seed & 255
    })))
    const original = tree(source), expected = original.map(item => ({ ...item, name: '原文件/' + item.name }))
    for (const format of ['zip', 'zipx']) for (const encryption of ['none', 'aes256', 'zipcrypto']) for (const split of ['none', 'numeric', 'standard']) {
      const label = `${format}-${encryption}-${split}`, output = path.join(root, label + '.' + format)
      if (results.some(result => result.label === label)) { console.log(`[zip-output-1414] reuse ${label}`); continue }
      const options = { format, level: 6, zip_method: 'zstd', password: encryption === 'none' ? null : password,
        zip_encryption: encryption === 'none' ? 'aes256' : encryption, zip_password_encoding: 'utf8',
        zip_split_mode: split === 'none' ? 'numeric' : split, split_size: split === 'none' ? null : 65536,
        preserve_paths: true, verify_after: true, delete_after: false }
      if (format === 'zipx' && split === 'standard') {
        // This historical gate retains its original 15 positive combinations.
        // The v1.4.18 genuine ZIPX disk/WinRAR gate supersedes the old rejection;
        // 7-Zip cannot independently decode the native .zxNN names used here.
        results.push({ label, coveredBy: 'standard-zipx-1418-gate.mjs', notExecutedByThisHistoricalGate: true });
        writeFileSync(progress, JSON.stringify(results, null, 2)); continue
      }
      const published = await ipc('compress_files', { taskId: 'zip-output-' + label, files: [source], outputPath: output, options })
      assert.equal(published.split, split !== 'none')
      const method = format === 'zipx' ? 'XZ' : 'ZSTD'
      verify(published.primaryPath, path.join(root, label + '-外部回解'), expected, options.password, method)
      const restored = path.join(root, label + '-应用回解')
      await bridge('extractArchive', published.primaryPath, restored, options.password)
      assert.deepEqual(tree(restored), expected)
      assert.ok(existsSync(path.join(restored, '原文件/中文目录/空目录')))
      results.push({ label, method, files: published.files.map(file => ({ ...file, sha256: hash(file.path) })), applicationReadback: true, independentReadback: true })
      writeFileSync(progress, JSON.stringify(results, null, 2))
      console.log(`[zip-output-1414] ${label} passed`)
    }
    const originalZip = path.join(root, '外部密码原包.zip'), sourcePassword = 'original + password!'
    if (!existsSync(originalZip)) sevenZip(['a', '-tzip', '-mem=AES256', `-p${sourcePassword}`, originalZip, '.\\*'], source)
    for (const [format, split] of [['zip', 'standard'], ['zipx', 'numeric']]) {
      if (results.some(result => result.conversion === `AES ZIP → ${format}`)) continue
      const output = path.join(root, `改密转换.${format}`)
      const published = await bridge('convertArchive', originalZip, sourcePassword, output, {
        format, level: 6, zip_method: 'zstd', password, zip_encryption: 'zipcrypto', zip_split_mode: split, split_size: 65536,
      })
      verify(published.primaryPath, output + '-外部回解', original, password, format === 'zip' ? 'ZSTD' : 'XZ')
      results.push({ conversion: `AES ZIP → ${format}`, changedPassword: true, split, independentReadback: true })
      writeFileSync(progress, JSON.stringify(results, null, 2))
    }
    const zipxSource = path.join(root, 'zipx-aes256-numeric.zipx.001'), converted = path.join(root, 'ZIPX转7Z.7z')
    const output = await bridge('convertArchive', zipxSource, password, converted, { format: '7z', level: 3, password: null })
    verify(output.primaryPath, converted + '-外部回解', expected, null)
    results.push({ conversion: 'AES XZ ZIPX 数字卷 → 7Z', independentReadback: true })
    assert.deepEqual(tree(source), original)
    writeFileSync(path.join(root, 'native-results.json'), JSON.stringify({ results, original, engine: sevenZip(['i']).split('\n').find(line => /7-Zip/.test(line)) }, null, 2))
  } else {
    results.push(...JSON.parse(readFileSync(path.join(root, 'native-results.json'), 'utf8')).results)
    console.log('[zip-output-1414] reusing completed native outputs; continuing UI only')
  }

  const click = async selector => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await driver.wait(() => driver.executeScript(el => { const r = el.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return !el.disabled && (hit === el || el.contains(hit)) }, element), 10000)
    await element.click()
  }
  const fill = async (selector, value) => {
    const element = await waitForElement(selector)
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await element.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (String(value)) await element.sendKeys(String(value))
  }
  const select = async (selector, label) => {
    await click(selector); await waitForElement('[role="option"]')
    for (const option of await driver.findElements(By.css('[role="option"]'))) {
      if ((await option.findElement(By.css('strong')).getAttribute('textContent')).trim() === label) {
        await driver.executeScript(el => el.scrollIntoView({ block: 'nearest', behavior: 'instant' }), option); await option.click(); return
      }
    }
    assert.fail('Missing option ' + label)
  }
  const shot = async name => {
    await driver.wait(() => driver.executeScript(() => document.getAnimations().every(animation =>
      animation.playState !== 'running' || animation.effect?.getComputedTiming().iterations === Infinity)), 5000)

    const metrics = await driver.executeScript(() => ({ width: innerWidth, height: innerHeight,
      overflow: document.documentElement.scrollWidth - innerWidth,
      columns: [...document.querySelectorAll('.compression-detail-grid')].map(grid => [...grid.children].map(child => child.getBoundingClientRect().width)),
      panels: [...document.querySelectorAll('.compression-config-panel,.compression-execution-panel')].map(panel => ({ width: panel.clientWidth, overflow: panel.scrollWidth - panel.clientWidth, scrollable: panel.scrollHeight > panel.clientHeight })) }))
    assert.ok(metrics.overflow <= 1, JSON.stringify(metrics))
    metrics.columns.forEach(columns => assert.ok(Math.abs(columns[0] - columns[1]) <= 1, JSON.stringify(metrics)))
    metrics.panels.forEach(panel => assert.ok(panel.overflow <= 2, JSON.stringify(metrics)))
    geometry.push({ name, ...metrics })
    writeFileSync(path.join(artifactDirectory, name + '.png'), Buffer.from(await driver.takeScreenshot(), 'base64')); screenshots.push(name + '.png')
  }
  const sizes = async (prefix, anchor) => {
    for (const [width, height] of [[1100, 700], [760, 520]]) {
      await driver.manage().window().setRect({ width, height })
      if (anchor) await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), await waitForElement(anchor))
      await shot(`${prefix}-${width}x${height}`)
    }
  }
  const historyFor = output => driver.wait(async () => (await bridge('taskHistory')).find(item => key(item.outputPath) === key(output) && ['completed', 'failed'].includes(item.status)), 60000)
  const ui = path.join(root, '界面-' + Date.now()); mkdirSync(ui)
  const uiFile = path.join(ui, '界面资料.bin'); writeFileSync(uiFile, Buffer.concat(Array.from({ length: 5 }, () => readFileSync(path.join(source, '内容.bin')))))
  await bridge('clearTasks'); await bridge('clearCompressionWorkspace'); await click('[data-testid="nav-Compress"]')
  await driver.manage().window().setRect({ width: 1100, height: 700 })
  const uiFrom = process.env.LONG_ZIP_OUTPUT_UI_FROM || 'compression'
  if (uiFrom === 'compression') {
  await bridge('queueDesktopDialogSelections', [[uiFile]]); await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
  if (!(await driver.findElements(By.css('[data-testid="compression-draft-details"]'))).length) await click('[data-testid="compression-draft-row"] [data-testid="compression-archive-name"]')
  assert.equal((await driver.findElements(By.css('[role="dialog"]'))).length, 0)
  await select('[data-testid="compression-zip-method"]', 'Zstandard')
  await fill('[data-testid="compression-password-input"]', password)
  await fill('[data-testid="compression-filename-input"]', '界面Zstd')
  await fill('[data-testid="compression-output-path"]', ui)
  await click('[data-testid="compression-split-toggle"]'); await fill('[data-testid="compression-split-size"]', '1')
  await select('[data-testid="compression-zip-split-mode"]', '标准 ZIP · .z01/.zip')
  await sizes('zip-output-inline', '[data-testid="compression-zip-method"]')
  await driver.manage().window().setRect({ width: 1100, height: 700 }); await click('[data-testid="start-compression"]')
  const created = path.join(ui, '界面Zstd.zip'), record = await historyFor(created)
  assert.equal(record.status, 'completed', JSON.stringify(record)); assert.equal(record.executionConfig.settings.zipMethod, 'zstd')
  verify(created, path.join(ui, '界面Zstd外部回解'), [{ name: path.basename(uiFile), sha256: hash(uiFile) }], password, 'ZSTD')
  await sizes('zip-output-completed', '.compression-execution-panel')
  await driver.executeScript(() => document.documentElement.style.fontSize = '125%')
  await shot('zip-output-completed-760-125'); await driver.executeScript(() => document.documentElement.style.fontSize = '')

  } else {
    screenshots.push(...['zip-output-inline-1100x700', 'zip-output-inline-760x520', 'zip-output-completed-1100x700', 'zip-output-completed-760x520', 'zip-output-completed-760-125'].map(name => name + '.png'))
    console.log('[zip-output-1414] reuse completed single-row creation and layouts')
  }
  if (uiFrom !== 'conversion') {
  await bridge('clearTasks'); await bridge('clearCompressionWorkspace')
  await bridge('queueDesktopDialogSelections', [[uiFile, path.join(source, '空文件.txt')]])
  await click('[data-testid="compression-center"] [data-testid="dropzone-file"]')
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))).length === 2, 10000)
  for (const checkbox of await driver.findElements(By.css('[data-testid="compression-group-checkbox"]'))) await checkbox.click()
  await click('[data-testid="batch-config-bar"] button[aria-haspopup="dialog"]')
  await select('[data-testid="batch-format"]', 'ZIPX（XZ）')
  await sizes('zip-output-compression-batch')
  await click('[data-testid="batch-config-apply"]')
  for (const row of await driver.findElements(By.css('[data-testid="compression-draft-row"]'))) assert.match(await row.getText(), /\.zipx/)
  await click('[data-testid="compression-draft-row"] [data-testid="compression-archive-name"]')
  await driver.wait(async () => {
    const method = await waitForElement('[data-testid="compression-zip-method"]')
    await driver.executeScript(el => el.scrollIntoView({ block: 'center', behavior: 'instant' }), method)
    return /XZ/.test(await method.getText())
  }, 10000)
  await sizes('zipx-output-inline', '[data-testid="compression-zip-method"]')

  }
  await click('[data-testid="open-archive-conversion"]')
  await bridge('queueDesktopDialogSelections', [[path.join(root, 'zip-none-none.zip'), path.join(root, 'zipx-none-none.zipx')]])
  await click('[data-testid="conversion-choose-source"]')
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="conversion-checkbox"]'))).length === 2, 10000)
  await click('[data-testid="conversion-batch-config"]'); await select('[data-testid="conversion-batch-format"]', 'ZIPX（XZ）')
  await sizes('zip-output-conversion-batch'); await click('[data-testid="conversion-batch-apply"]')
  await click('[data-testid="conversion-start"]')
  for (const stem of ['zip-none-none', 'zipx-none-none']) {
    const output = path.join(root, stem + '_转换.zipx'), converted = await historyFor(output)
    assert.equal(converted.status, 'completed', JSON.stringify(converted))
    verify(output, output + '-外部回解', JSON.parse(readFileSync(path.join(root, 'native-results.json'), 'utf8')).original.map(item => ({ ...item, name: '原文件/' + item.name })), null, 'XZ')
  }
  await shot('zip-output-conversion-completed-760')
  writeFileSync(path.join(artifactDirectory, 'zip-output-1414-results.json'), JSON.stringify({ version: '1.4.14', application, sha256: hash(application), retainedRoot: root,
    engine: bundledSevenZip, results, screenshots, geometry, uiResumedFrom: uiFrom,
    scope: 'ZIP Zstandard: no/AES256/ZipCrypto × ordinary/numeric/standard disks; ZIPX XZ: no/AES256/ZipCrypto × ordinary/numeric disks; standard ZIPX is tested separately by the v1.4.18 gate; real commands, publication, verification, application and independent readback; inline and modal UI with 50/50 columns; isolated picker responses' }, null, 2))
}

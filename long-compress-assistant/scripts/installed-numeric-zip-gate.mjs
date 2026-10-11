import assert from 'node:assert/strict'
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
import path from 'node:path'
import { By } from 'selenium-webdriver'
import { createHmac, pbkdf2Sync } from 'node:crypto'

// Production IPC supplies fixtures, a drop event and persisted-history reads.
// Extraction startup, password entry, retry confirmation and conflict policy use the UI.
export async function runInstalledNumericZipGate({ driver, fixtureRoot, evidenceRoot, run, sevenZip, waitForElement, record, utf8 = false }) {
  const scenario = utf8 ? 'utf8' : 'ascii'
  const folder = path.join(fixtureRoot, `numeric-password-${scenario}`)
  mkdirSync(folder)
  const source = path.join(folder, '内容.txt')
  const archiveName = utf8 ? '安装态中文密码数字卷' : '安装态密码数字卷'
  const archive = path.join(folder, `${archiveName}.zip`)
  const password = utf8 ? '中文密码 + !测试' : 'installed secret + !'
  const payload = '安装版密码数字卷用户操作验证\n'.repeat(100)
  writeFileSync(source, payload)
  const ipc = async (command, args) => {
    const result = await driver.executeAsyncScript(`
      const [cmd,args,done]=arguments;
      const ids=crypto.getRandomValues(new Uint32Array(2));
      const cleanup=()=>ids.forEach(id=>Reflect.deleteProperty(window,'_'+id));
      window['_'+ids[0]]=value=>{cleanup();done({ok:true,value})};
      window['_'+ids[1]]=error=>{cleanup();done({ok:false,error:String(error)})};
      window.__TAURI_IPC__({cmd,...args,callback:ids[0],error:ids[1]});
    `, command, args)
    assert.ok(result.ok, result.error)
    return result.value
  }
  if (utf8) {
    // Fixture creation invokes the installed production command, not the test bridge.
    await ipc('compress_files', {
      taskId: `installed-utf8-fixture-${Date.now()}`, files: [source], outputPath: archive,
      options: { format: 'zip', level: 6, password, zip_password_encoding: 'utf8', split_size: null,
        preserve_paths: true, delete_after: false, allow_insecure_password_cli: false },
    })
  } else {
    run(sevenZip, ['a', '-tzip', '-mem=AES256', `-p${password}`, archive, source], 'numeric ZIP AES fixture')
  }
  const bytes = readFileSync(archive), parts = []
  if (utf8) {
    // Independently authenticate the single-entry AES-256 fixture with UTF-8
    // password bytes so a writer/reader encoding fallback cannot fake this gate.
    const end = bytes.lastIndexOf(Buffer.from([0x50, 0x4b, 5, 6]))
    assert.ok(end >= 0)
    const central = bytes.readUInt32LE(end + 16)
    assert.equal(bytes.readUInt32LE(central), 0x02014b50)
    const compressedSize = bytes.readUInt32LE(central + 20)
    const local = bytes.readUInt32LE(central + 42)
    assert.equal(bytes.readUInt16LE(local + 8), 99)
    const start = local + 30 + bytes.readUInt16LE(local + 26) + bytes.readUInt16LE(local + 28)
    const encrypted = bytes.subarray(start, start + compressedSize)
    const keys = pbkdf2Sync(Buffer.from(password, 'utf8'), encrypted.subarray(0, 16), 1000, 66, 'sha1')
    assert.deepEqual(encrypted.subarray(16, 18), keys.subarray(64))
    assert.deepEqual(createHmac('sha1', keys.subarray(32, 64)).update(encrypted.subarray(18, -10)).digest().subarray(0, 10), encrypted.subarray(-10))
  }
  for (let offset = 0, index = 1; offset < bytes.length; offset += 37, index++) {
    const part = `${archive}.${String(index).padStart(3, '0')}`
    writeFileSync(part, bytes.subarray(offset, offset + 37))
    parts.push(part)
  }
  assert.ok(parts.length > 2)
  const history = async () => (await ipc('list_task_history_page', { limit: 100, query: { text: parts[0] } })).records
  await (await waitForElement('[data-testid="nav-Decompress"]')).click()
  await waitForElement('[data-testid="decompression-header"]')
  await ipc('tauri', { __tauriModule: 'Event', message: { cmd: 'emit', event: 'tauri://file-drop', windowLabel: null, payload: [parts[0]] } })
  const start = async () => (await driver.findElement(By.xpath('//header[@data-testid="decompression-header"]//button[contains(., "开始解压队列")]'))).click()
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="task-row"]')))
    .reduce(async (found, row) => (await found) || (await row.getText()).includes(archiveName), Promise.resolve(false)), 15_000)
  // Empty queues do not render queue-level controls; import before locating them.
  const recycle = await waitForElement('[data-testid="global-recycle-source-switch"]')
  if (await recycle.getAttribute('aria-checked') === 'true') await recycle.click()
  await start()
  const input = await waitForElement('.task-password-cell input')
  const failed = await driver.wait(async () => (await history()).find(row => row.status === 'failed'), 30_000)
  await input.sendKeys(password)
  assert.equal(await input.getAttribute('value'), password)
  await (await driver.findElement(By.css('.task-password-cell button:last-child'))).click()
  const panel = await waitForElement('[data-testid="failed-extraction-retry"]')
  const output = path.join(folder, '输出')
  mkdirSync(output)
  writeFileSync(path.join(output, '内容.txt'), '已有内容，不可覆盖')
  const outputInput = await panel.findElement(By.css('[data-testid="retry-output"]'))
  await outputInput.clear()
  await outputInput.sendKeys(output)
  const subfolder = await panel.findElement(By.css('[data-testid="retry-subfolder"]'))
  if (await subfolder.isSelected()) await subfolder.click()
  await (await waitForElement('[data-testid="create-retry-drafts"]')).click()
  await driver.wait(async () => (await panel.getText()).includes('已创建新任务'), 30_000)
  assert.equal(existsSync(path.join(output, '内容 (1).txt')), false)
  await (await driver.findElement(By.xpath('//button[contains(., "返回队列确认并启动")]'))).click()
  await start()
  const rename = await waitForElement('[data-testid="conflict-rename"]')
  assert.equal((await history()).some(row => row.retryOfTaskId === failed.id), false)
  await driver.manage().window().setRect({ width: 920, height: 620 })
  await driver.wait(async () => driver.executeScript(() => {
    const button = document.querySelector('[data-testid="conflict-rename"]')
    const r = button.getBoundingClientRect()
    return r.bottom <= innerHeight && button.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2))
  }), 10_000)
  writeFileSync(path.join(evidenceRoot, `installed-numeric-${scenario}-conflict.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
  await rename.click()
  const completed = await driver.wait(async () => (await history()).find(row => row.retryOfTaskId === failed.id), 60_000)
  assert.equal(completed.status, 'completed', JSON.stringify(completed))
  assert.equal(readFileSync(path.join(output, '内容.txt'), 'utf8'), '已有内容，不可覆盖')
  assert.equal(readFileSync(path.join(output, '内容 (1).txt'), 'utf8'), payload)
  assert.ok(parts.every(part => existsSync(part)))
  await driver.navigate().refresh()
  await waitForElement('main h1')
  const reloaded = await history()
  assert.deepEqual(reloaded.find(row => row.id === failed.id), failed)
  assert.deepEqual(reloaded.find(row => row.id === completed.id), completed)
  await (await waitForElement('[data-testid="nav-History"]')).click()
  await (await waitForElement('[data-testid="history-search"]')).sendKeys(archiveName)
  await driver.wait(async () => (await driver.findElements(By.css('[data-testid="history-record-row"]'))).length === 2, 15_000)
  writeFileSync(path.join(evidenceRoot, `installed-numeric-${scenario}-history.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
  record(`installed numeric ZIP ${scenario} password retry and conflict`, `AES ZIP, ${utf8 ? 'UTF-8 Chinese (independent HMAC verified)' : 'ASCII'} password with spaces, ${parts.length} volumes; UI retry/rename; exact payload and original preserved; history retained after WebView reload (not process restart)`)
}

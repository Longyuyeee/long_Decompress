import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, openSync, writeSync, closeSync, existsSync, unlinkSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { By } from 'selenium-webdriver'

export async function runChecksum1522Gate({ driver, bridge, waitForElement, artifactDirectory, application, root }) {
  const base = path.join(root, 'test-results/checksum-1522'); mkdirSync(base, { recursive: true })
  const statePath = path.join(base, 'desktop-progress.json')
  const binarySha256 = createHash('sha256').update(readFileSync(application)).digest('hex')
  const state = existsSync(statePath) ? JSON.parse(readFileSync(statePath, 'utf8')) : { version: '1.5.22', binarySha256, work: mkdtempSync(path.join(tmpdir(), 'long-checksum-1522-')), ui: [], screenshots: [], layouts: [] }
  assert.equal(state.binarySha256, binarySha256, 'Do not reuse results from a different program')
  const save = () => writeFileSync(statePath, JSON.stringify(state, null, 2))
  const first = path.join(state.work, '先完成的 中文.txt'), large = path.join(state.work, '正在读取的大文件.bin'), last = path.join(state.work, '稍后读取.txt'), manifest = path.join(state.work, '校验清单.md5')
  if (!state.fixture) {
    const block = Buffer.alloc(2 * 1024 * 1024, 0x67), digest = createHash('md5'), fd = openSync(large, 'w')
    try { for (let i = 0; i < 2048; i++) { writeSync(fd, block); digest.update(block) } } finally { closeSync(fd) }
    state.fixture = { largeBytes: block.length * 2048, largeMd5: digest.digest('hex') }; save()
  }
  const known = '65a8e27d8879283831b664bd8b7f0ad4'
  const original = `${known}  ${path.basename(first)}\n${state.fixture.largeMd5}  ${path.basename(large)}\n${known}  ${path.basename(last)}\n`
  writeFileSync(first, 'Hello, World!'); writeFileSync(last, 'Hello, World!'); writeFileSync(manifest, original)
  const click = async target => {
    for (const t of await driver.findElements(By.css('.toast-item button'))) { try { if (await t.isDisplayed()) await t.click() } catch {} }
    const e = typeof target === 'string' ? await waitForElement(target) : target
    await driver.wait(() => driver.executeScript(e => { e.scrollIntoView({ block: 'center', behavior: 'instant' }); const r = e.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return !e.disabled && r.width > 0 && (hit === e || e.contains(hit)) }, e), 15000)
    await e.click()
  }
  const sizes = async (name, anchor) => {
    for (const [width, height] of [[1100, 700], [760, 520]]) {
      await driver.manage().window().setRect({ width, height }); const e = await waitForElement(anchor)
      await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), e)
      const geometry = await driver.executeScript(() => ({ width: innerWidth, height: innerHeight, overflow: document.documentElement.scrollWidth - innerWidth, details: [...document.querySelectorAll('.checksum-details')].map(e => { const c = e.querySelectorAll('.checksum-digests>div'); return { overflow: e.scrollWidth - e.clientWidth, left: c[0].getBoundingClientRect().width, right: c[1].getBoundingClientRect().width } }) }))
      assert.ok(geometry.overflow <= 1, JSON.stringify(geometry)); for (const d of geometry.details) assert.ok(d.overflow <= 1 && Math.abs(d.left - d.right) <= 1, JSON.stringify(d))
      const file = `checksum1522-${name}-${width}x${height}.png`; writeFileSync(path.join(artifactDirectory, file), Buffer.from(await driver.takeScreenshot(), 'base64'))
      state.screenshots.push(file); state.layouts.push({ name, ...geometry }); save()
    }
    await driver.manage().window().setRect({ width: 1100, height: 700 })
  }
  const visibleProgress = async () => driver.wait(async () => {
    for (const e of await driver.findElements(By.css('[data-testid="checksum-progress"]'))) {
      if (await e.isDisplayed() && Number(await e.getAttribute('data-file-bytes')) > 0 && (await e.getText()).includes(path.basename(large))) {
        const bytes = Number(await e.getAttribute('data-file-bytes')); assert.ok(bytes < state.fixture.largeBytes); return { fileBytes: bytes, totalBytes: Number(await e.getAttribute('data-total-bytes')) }
      }
    } return false
  }, 20000)
  const statuses = selector => driver.executeScript(s => [...document.querySelectorAll(s)].map(e => e.dataset.status), selector)
  // No state/progress injection: only the normal file picker seam and actual buttons are used.
  await driver.manage().setTimeouts({ script: 180000 }); await driver.manage().window().setRect({ width: 1100, height: 700 })
  await click('[data-testid="nav-FileIntegrity"]')
  if (!state.ui.some(r => r.kind === 'manifest-stop-continue')) {
    await click('[data-testid="checksum-verify-mode"]'); await bridge('queueDesktopDialogSelections', [manifest]); await click('[data-testid="checksum-choose"]')
    const progress = await visibleProgress()
    const live = 'checksum1522-live-manifest.png'; writeFileSync(path.join(artifactDirectory, live), Buffer.from(await driver.takeScreenshot(), 'base64')); state.screenshots.push(live)
    await click('[data-testid="checksum-stop"]')
    const rows = '[data-testid="checksum-entry"]'
    await driver.wait(async () => JSON.stringify(await statuses(rows)) === JSON.stringify(['matched', 'interrupted', 'not_checked']) && await (await waitForElement('[data-testid="checksum-verification"]')).getAttribute('aria-busy') === 'false', 15000)
    await click('[data-testid="checksum-entry"][data-line="2"] [data-testid="checksum-row"]'); await sizes('stopped', '[data-testid="checksum-notice"]')
    unlinkSync(first) // A completed observation must not be secretly rehashed by Continue.
    await click('[data-testid="checksum-continue"]')
    await driver.wait(async () => JSON.stringify(await statuses(rows)) === JSON.stringify(['matched', 'matched', 'matched']) && await (await waitForElement('[data-testid="checksum-verification"]')).getAttribute('aria-busy') === 'false', 90000)
    assert.ok(!existsSync(first)); assert.match(await (await waitForElement('[data-testid="checksum-heading"]')).getText(), /均匹配/)
    await sizes('continued', '.checksum-digests'); writeFileSync(first, 'Hello, World!')
    state.ui.push({ kind: 'manifest-stop-continue', progress, stoppedStatuses: ['matched', 'interrupted', 'not_checked'], independentDigestMatched: true, completedFileNotReopened: true }); save()
  }
  if (!state.ui.some(r => r.kind === 'changed-manifest')) {
    // A prior result is needed even on an explicitly scoped script continuation.
    if (!(await driver.findElements(By.css('[data-testid="checksum-report"]'))).length) { await click('[data-testid="checksum-verify-mode"]'); await bridge('queueDesktopDialogSelections', [manifest]); await click('[data-testid="checksum-choose"]'); await driver.wait(async () => await (await waitForElement('[data-testid="checksum-verification"]')).getAttribute('aria-busy') === 'false', 90000) }
    writeFileSync(manifest, original + `${known}  新增但未读取.txt\n`)
    const rowButton = await waitForElement('[data-testid="checksum-entry"][data-line="2"] [data-testid="checksum-row"]')
    if (await rowButton.getAttribute('aria-expanded') !== 'true') await click(rowButton)
    await click('[data-testid="checksum-recheck-2"]')
    await driver.wait(async () => JSON.stringify(await statuses('[data-testid="checksum-entry"]')) === JSON.stringify(['not_checked', 'not_checked', 'not_checked', 'not_checked']), 15000)
    assert.match(await (await waitForElement('[data-testid="checksum-notice"]')).getText(), /旧结果已作废/)
    await sizes('changed-manifest', '[data-testid="checksum-notice"]')
    state.ui.push({ kind: 'changed-manifest', oldObservationsInvalidated: true, oldLineSelectionDidNotReadNewManifest: true }); save()
  }
  if (!state.ui.some(r => r.kind === 'calculate-stop-continue')) {
    await click(await driver.findElement(By.css('.integrity-mode-switch button:first-child')))
    await click(await driver.findElement(By.xpath('//div[contains(@class,"algorithm-options")]//button[.//strong[text()="MD5"]]')))
    await bridge('queueDesktopDialogSelections', [[first, large, last]]); await click('[data-testid="checksum-calculate-choose"]'); await click('[data-testid="checksum-calculate-start"]')
    const progress = await visibleProgress(); await click('[data-testid="checksum-stop"]')
    await driver.wait(async () => JSON.stringify(await statuses('[data-testid="checksum-calculation-entry"]')) === JSON.stringify(['success', 'interrupted', 'pending']) && await (await waitForElement('[data-testid="checksum-calculate-continue"]')).isEnabled(), 15000)
    await sizes('calculate-stopped', '[data-testid="checksum-calculate-continue"]')
    unlinkSync(first); await click('[data-testid="checksum-calculate-continue"]')
    await driver.wait(async () => JSON.stringify(await statuses('[data-testid="checksum-calculation-entry"]')) === JSON.stringify(['success', 'success', 'success']), 90000)
    const actual = await driver.executeScript(() => [...document.querySelectorAll('[data-testid="checksum-calculation-entry"] code')].map(e => e.textContent))
    assert.deepEqual(actual, [known, state.fixture.largeMd5, known]); assert.ok(!existsSync(first))
    await sizes('calculate-completed', '.checksum-results'); writeFileSync(first, 'Hello, World!')
    state.ui.push({ kind: 'calculate-stop-continue', progress, completedFileNotReopened: true, allDigestsMatchIndependentMd5: true }); save()
  }
  state.completedAt = new Date().toISOString(); save()
  writeFileSync(path.join(artifactDirectory, 'checksum-1522-results.json'), JSON.stringify(state, null, 2))
  // Only the one large fixture owned by this run is removed after successful evidence capture.
  assert.ok(large.startsWith(state.work + path.sep)); unlinkSync(large)
}

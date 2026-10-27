import { chromium } from '@playwright/test'
import assert from 'node:assert/strict'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'

// Headless Vue/CSS replay of real native reports/results. No desktop input,
// real filesystem writes, or claim that these IPC fixtures run native recovery.
const base = 'test-results/tar-lzo-gnu-178'
const native = JSON.parse(readFileSync(`${base}/native.json`, 'utf8'))
const diagnostics = JSON.parse(readFileSync(`${base}/diagnostic-records.json`, 'utf8'))
const diagnosis = { ...diagnostics.partial, filePath: native.source, lzopPrefix: native.report }
mkdirSync(base, { recursive: true })
const browser = await chromium.launch({ headless: true })
const checks = [], layouts = [], errors = []
try {
  const page = await browser.newPage({ viewport: { width: 1100, height: 700 } })
  page.on('pageerror', error => errors.push(error.message))
  await page.addInitScript(({ native, diagnosis }) => {
    window.__gnuNative = native
    window.__gnuDiagnosis = diagnosis
    window.__gnuCalls = []
    window.__gnuHistory = []
    window.__TAURI_IPC__ = message => {
      window.__gnuCalls.push(message)
      let value
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'take_archive_opening_reminder') value = false
      if (message.cmd === 'take_pending_context_actions') value = []
      if (message.cmd === 'list_task_history_page') value = { records: window.__gnuHistory, nextCursor: null }
      if (message.cmd === 'check_app_update') value = { shouldUpdate: false }
      if (message.cmd === 'diagnose_archive') value = window.__gnuDiagnosis
      if (message.cmd === 'scan_lzop_recovery') value = native.report
      if (message.cmd === 'recover_lzop_members') {
        if (message.revision !== native.report.revision || message.selected.length !== native.report.entries.length) throw Error('UI submitted a different recovery snapshot or scope')
        value = native.result
      }
      if (message.cmd === 'save_task_history') window.__gnuHistory.push(message.record)
      window[`_${message.callback}`]?.(value)
    }
  }, { native, diagnosis })
  await page.goto(`${process.env.TAR_LZO_GNU_UI_URL || 'http://127.0.0.1:1440'}/#/integrity`, { waitUntil: 'domcontentloaded', timeout: 60000 })
  await page.evaluate(async () => {
    const { useAppStore } = await import('/src/stores/app.ts')
    const app = useAppStore()
    app.theme = 'obsidian'; app.accentColor = '#d9b76f'
    app.openArchiveForDiagnosis({ path: window.__gnuNative.source, taskId: 'gnu-original-failure' })
  })
  await page.getByTestId('open-lzop-recovery').waitFor()
  await page.getByTestId('open-lzop-recovery').click()
  const dialog = page.getByRole('dialog')
  await page.getByTestId('lzop-recovery-members').waitFor()
  assert.match(await page.getByTestId('lzop-recovery-count').innerText(), /6 个完整文件.*2 个目录/)
  assert.equal(await dialog.locator('.stop-detail').getAttribute('open'), null)
  assert.equal(await dialog.locator('[data-recovery-member]').count(), 8)
  const longEntry = native.report.entries.find(entry => entry.name.includes('-notes.txt'))
  const name = dialog.locator(`[data-recovery-member="${longEntry.id}"] span`)
  assert.equal(await name.getAttribute('title'), longEntry.name)
  assert.ok(!await dialog.innerText().then(text => text.includes('@LongLink')))
  const shot = async name => {
    await page.evaluate(() => Promise.all(document.getAnimations().filter(animation => animation.effect?.getTiming().iterations !== Infinity).map(animation => animation.finished.catch(() => {}))))
    await page.screenshot({ path: `${base}/${name}.png` })
  }
  for (const [width, height] of [[1100, 700], [760, 520]]) {
    await page.setViewportSize({ width, height })
    await page.getByTestId('lzop-recovery-confirm').scrollIntoViewIfNeeded()
    await shot(`gnu-selection-${width}x${height}`)
    const layout = await dialog.evaluate(el => {
      const box = element => { const rect = element.getBoundingClientRect(); return { top: rect.top, bottom: rect.bottom, left: rect.left, right: rect.right } }
      return { width: innerWidth, height: innerHeight, overflow: el.scrollWidth - el.clientWidth,
        box: box(el.querySelector('.app-dialog')), button: box(el.querySelector('[data-testid="lzop-recovery-confirm"]')) }
    })
    assert.ok(layout.overflow <= 1 && layout.box.left >= 0 && layout.box.right <= width + 1, JSON.stringify(layout))
    assert.ok(layout.button.top >= 0 && layout.button.bottom <= height, JSON.stringify(layout))
    layouts.push(layout)
  }
  if (!process.argv.includes('--layout-only')) {
  const filter = dialog.getByRole('textbox', { name: '筛选可取回文件' })
  await filter.fill('-notes.txt')
  assert.equal(await dialog.locator('[data-recovery-member]').count(), 1)
  assert.equal(await name.innerText(), longEntry.name)
  await filter.fill('')
  const directory = native.report.entries.find(entry => entry.isDirectory && !entry.name.endsWith('/empty'))
  const child = native.report.entries.find(entry => entry.name.endsWith('/readme.txt') && !entry.name.startsWith('other/'))
  await dialog.locator(`[data-recovery-member="${directory.id}"] input`).uncheck()
  assert.equal(await dialog.locator(`[data-recovery-member="${child.id}"] input`).isChecked(), false)
  await dialog.locator(`[data-recovery-member="${directory.id}"] input`).check()
  assert.equal(await dialog.locator(`[data-recovery-member="${child.id}"] input`).isChecked(), true)
  await page.getByTestId('lzop-recovery-confirm').click()
  await page.getByTestId('lzop-recovery-result').waitFor()
  await page.waitForFunction(() => window.__gnuHistory.length === 1)
  assert.match(await page.getByTestId('lzop-recovery-result').innerText(), /已部分取回 6 个文件、2 个目录/)
  await shot('gnu-result-760x520')
  const history = await page.evaluate(() => window.__gnuHistory[0])
  assert.equal(history.executionConfig.kind, 'archive-recovery')
  assert.equal(history.retryOfTaskId, 'gnu-original-failure')
  assert.equal(history.executionConfig.recoveredFiles, 6)
  assert.equal(history.executionConfig.recoveredDirectories, 2)
  checks.push('real native GNU prefix values replay through existing diagnosis, long-name filtering, folder selection, result and partial-history UI')
  checks.push('wide and narrow dialogs keep long names contained and footer reachable without a new page')
  } else {
    const child = native.report.entries.find(entry => entry.name.endsWith('/readme.txt') && !entry.name.startsWith('other/'))
    const row = dialog.locator(`[data-recovery-member="${child.id}"]`)
    assert.equal(await row.locator('.member-copy strong').innerText(), 'readme.txt')
    assert.equal(await row.locator('.member-copy').getAttribute('title'), child.name)
    checks.push('updated filename-first presentation keeps basename and full-path tooltip intact at both viewport sizes')
  }
  assert.deepEqual(errors, [])
  console.log(JSON.stringify({ checks, layouts, errors }))
} finally {
  writeFileSync(`${base}/${process.argv.includes('--layout-only') ? 'ui-layout' : 'ui'}.json`, JSON.stringify({ fixtureNativeIPC: true, nativeReportsReplayed: true, checks, layouts, errors }, null, 2))
  await browser.close()
}

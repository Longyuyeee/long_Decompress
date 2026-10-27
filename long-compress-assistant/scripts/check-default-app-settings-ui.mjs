import { chromium } from '@playwright/test'
import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync } from 'node:fs'

// Real Vue/CSS interaction; Windows association IPC is an explicit fixture.
// Native registry and URI activation are checked separately; this does not set defaults.
const output = 'test-results/default-app-settings'
mkdirSync(output, { recursive: true })
const browser = await chromium.launch({ headless: true })
const checks = [], layouts = [], errors = []
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
  page.on('pageerror', error => errors.push(error.message))
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'platform', { configurable: true, value: 'Win32' })
    window.__formats = ['zip','7z','rar','zipx','tar','gz','bz2','xz','zst','tgz','tbz2','txz','lz','lzo','lz4','br','cab','arj','alz','egg','zpaq']
    window.__defaults = ['rar','lz','lzo','lz4','br','egg','zpaq']
    window.__associationCalls = []
    window.__settingsLaunches = 0
    window.__TAURI_IPC__ = message => {
      let value
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'list_task_history_page') value = { records: [], nextCursor: null }
      if (message.cmd === 'take_pending_context_actions') value = []
      if (message.cmd === 'take_archive_opening_reminder') value = false
      if (message.cmd === 'archive_file_associations') value = window.__formats.map(extension => ({ extension: '.' + extension, registered: true, isDefault: window.__defaults.includes(extension) }))
      if (message.cmd === 'configure_archive_file_associations') window.__associationCalls.push([...message.extensions])
      if (message.cmd === 'open_default_archive_apps') window.__settingsLaunches++
      window[`_${message.callback}`]?.(value)
    }
  })
  await page.goto(`${process.env.DEFAULT_APP_SETTINGS_UI_URL || 'http://127.0.0.1:1439'}/#/settings?section=behavior`, { waitUntil: 'domcontentloaded', timeout: 60000 })
  await page.locator('[data-settings-section="behavior"]').click()
  await page.getByTestId('archive-opening-settings').scrollIntoViewIfNeeded()
  await page.evaluate(async () => { const { useAppStore } = await import('/src/stores/app.ts'); const app = useAppStore(); app.theme = 'obsidian'; app.accentColor = '#d9b76f' })
  for (const button of await page.getByRole('button', { name: /^关闭.*提示$/ }).all()) await button.click()
  await page.getByTestId('archive-association-configure').click()
  const dialog = page.getByRole('dialog')
  const save = page.getByTestId('archive-association-save')
  await dialog.waitFor()
  assert.equal(await dialog.locator('input:checked').count(), 7)
  assert.equal(await dialog.locator('input:disabled').count(), 0)
  assert.equal(await page.getByRole('button', { name: '更改 / 取消默认…', exact: true }).count(), 0)
  const shot = async name => {
    await page.evaluate(() => Promise.all(document.getAnimations().filter(animation => animation.effect?.getTiming().iterations !== Infinity).map(animation => animation.finished.catch(() => {}))))
    await page.screenshot({ path: `${output}/${name}.png` })
  }
  if (process.argv.includes('--scroll-only')) {
    await page.setViewportSize({ width: 760, height: 520 })
    await page.getByTestId('archive-association-select-all').click()
    await save.click()
    await page.waitForFunction(() => window.__settingsLaunches === 1)
    const notice = page.getByTestId('archive-association-notice')
    await notice.scrollIntoViewIfNeeded()
    const layout = await dialog.evaluate(el => {
      const body = el.querySelector('.min-h-0'), last = el.querySelector('input[value=".zpaq"]'), notice = el.querySelector('[data-testid="archive-association-notice"]'), save = el.querySelector('[data-testid="archive-association-save"]')
      const box = element => { const rect = element.getBoundingClientRect(); return { top: rect.top, bottom: rect.bottom } }
      return { scrollTop: body.scrollTop, body: box(body), last: box(last), notice: box(notice), save: box(save), height: innerHeight }
    })
    assert.ok(layout.scrollTop > 0, JSON.stringify(layout))
    assert.ok(layout.last.top >= layout.body.top && layout.last.bottom <= layout.body.bottom, JSON.stringify(layout))
    assert.ok(layout.notice.top >= layout.body.top && layout.notice.bottom <= layout.body.bottom + 1, JSON.stringify(layout))
    assert.ok(layout.save.top >= 0 && layout.save.bottom <= layout.height, JSON.stringify(layout))
    layouts.push(layout)
    await shot('small-window-scrolled-bottom')
    checks.push('760x520 body scroll reaches ZPAQ and confirmation notice while the fixed footer remains visible')
  } else {
  await shot('effective-defaults-wide')
  await page.getByTestId('archive-association-select-all').click()
  assert.equal(await dialog.locator('input:checked').count(), 21)
  await save.click()
  await page.waitForFunction(() => window.__settingsLaunches === 1)
  assert.equal((await page.evaluate(() => window.__associationCalls[0])).length, 21)
  assert.match(await page.getByTestId('archive-association-notice').innerText(), /14 种/)
  checks.push('all 21 types selected in one click; actual seven defaults kept distinct from 14 pending changes')
  for (const [width, height] of [[1440, 900], [1100, 700], [760, 520]]) {
    await page.setViewportSize({ width, height })
    await shot(`all-selected-pending-${width}x${height}`)
    const layout = await dialog.evaluate(el => {
      const box = element => { const rect = element.getBoundingClientRect(); return { top: rect.top, bottom: rect.bottom, left: rect.left, right: rect.right } }
      const button = el.querySelector('[data-testid="archive-association-save"]')
      return { width: innerWidth, height: innerHeight, button: box(button), overflow: el.scrollWidth - el.clientWidth, grid: box(el.querySelector('.association-formats')), body: box(el.querySelector('.min-h-0')) }
    })
    assert.ok(layout.button.top >= 0 && layout.button.bottom <= height, JSON.stringify(layout))
    assert.ok(layout.overflow <= 1, JSON.stringify(layout))
    layouts.push(layout)
  }
  checks.push('wide and small windows retain visible footer and contained scroll area')
  await page.evaluate(() => { window.__defaults = [...window.__formats]; dispatchEvent(new Event('focus')) })
  await page.waitForFunction(() => document.querySelector('[data-testid="archive-association-notice"]')?.textContent.includes('全部生效'))
  await dialog.locator('input[value=".zip"]').uncheck()
  await save.click()
  await page.waitForFunction(() => window.__settingsLaunches === 2)
  assert.ok((await dialog.locator('label').filter({ has: page.locator('input[value=".zip"]') }).innerText()).includes('待更换'))
  await page.evaluate(() => { window.__defaults = window.__formats.filter(extension => extension !== 'zip'); dispatchEvent(new Event('focus')) })
  await page.waitForFunction(() => document.querySelector('[data-testid="archive-association-notice"]')?.textContent.includes('全部生效'))
  await save.click()
  await dialog.waitFor({ state: 'hidden' })
  checks.push('a current default can be unchecked; Windows confirmation refreshes without losing the desired selection')
  await page.getByTestId('archive-association-configure').click()
  await page.getByTestId('archive-association-clear-all').click()
  await save.click()
  await page.waitForFunction(() => window.__settingsLaunches === 3)
  assert.deepEqual(await page.evaluate(() => window.__associationCalls.at(-1)), [])
  await shot('cancel-all-pending-760x520')
  await page.evaluate(() => { window.__defaults = []; dispatchEvent(new Event('focus')) })
  await page.waitForFunction(() => document.querySelector('[data-testid="archive-association-notice"]')?.textContent.includes('全部生效'))
  await save.click()
  await dialog.waitFor({ state: 'hidden' })
  checks.push('empty selection is accepted and cancellation completion reflects the effective Windows state')
  }
  assert.deepEqual(errors, [])
  console.log(JSON.stringify({ checks, layouts, errors }))
} finally {
  writeFileSync(`${output}/${process.argv.includes('--scroll-only') ? 'scroll-result' : 'result'}.json`, JSON.stringify({ fixtureNativeIPC: true, checks, layouts, errors }, null, 2))
  await browser.close()
}

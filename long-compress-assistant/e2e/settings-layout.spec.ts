import { test, expect, type Page } from '@playwright/test'

// Real Vue page/CSS; IPC only is stubbed, so tests never register OS integration.
async function openSettings(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem('app-theme', 'light')
    const win = window as any
    win.__settingsCalls = []
    win.__TAURI_IPC__ = (message: any) => {
      win.__settingsCalls.push(message.cmd)
      let result
      if (message.cmd === 'load_app_settings') result = JSON.stringify({ theme: 'light', autoCheckUpdates: false })
      else if (message.cmd === 'get_archive_engine_capabilities') result = { available: true, fullEngine: true, version: '26.02', formats: [], browseExtensions: [], nestedExtensions: [], boundedPreviewFormats: [], imagePreviewExtensions: [], textPreviewExtensions: [], message: 'Full engine ready' }
      else if (message.cmd === 'check_rar_compression_support') result = { available: false, message: '未检测到本机 RAR 编码器' }
      else if (message.cmd === 'check_auto_start') result = false
      else if (message.cmd === 'is_context_menu_registered') result = true
      else if (message.cmd === 'tauri' && message.__tauriModule === 'App') result = '1.4.1'
      else if (message.cmd === 'load_history' || message.cmd === 'take_pending_context_actions') result = []
      win[`_${message.callback}`]?.(result)
    }
  })
  await page.goto('/#/settings')
  await expect(page.locator('.settings-view')).toBeVisible()
  await expect(page.locator('.settings-theme-options button')).toHaveCount(6)
  await expect(page.locator('.settings-accent-options button')).toHaveCount(13)
  await page.evaluate(() => document.fonts.ready)
  await expect(page.locator('.settings-view')).toHaveCSS('opacity', '1')
  await page.waitForTimeout(750) // allow route/theme transitions to settle before visual capture
}

async function checkLayout(page: Page) {
  const overflow = await page.locator('.settings-scroll').evaluate(root => {
    const elements = [root, ...root.querySelectorAll('section, button, select')]
    return elements.filter(el => el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 2)
      .map(el => ({ element: el.tagName, text: el.textContent?.trim().slice(0, 80), width: el.clientWidth, scroll: el.scrollWidth }))
  })
  expect(overflow).toEqual([])
}

for (const scenario of [
  { name: 'default-light', width: 1280, height: 1000, dark: false },
  { name: 'default-dark', width: 1280, height: 800, dark: true },
  { name: 'narrow-light', width: 900, height: 700, dark: false },
  { name: 'narrow-dark', width: 900, height: 700, dark: true },
]) {
  test(`settings layout ${scenario.name}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: scenario.width, height: scenario.height })
    await openSettings(page)
    if (scenario.dark) {
      const dark = page.locator('.settings-theme-options button').filter({ hasText: '暗色 (Dark)' })
      await dark.click()
      await expect(dark).toHaveAttribute('aria-pressed', 'true')
    }
    await page.waitForTimeout(750)
    await checkLayout(page)
    await page.screenshot({ path: testInfo.outputPath('appearance.png') })
    for (const [name, selector] of [
      ['accessibility', '.accessibility-settings'],
      ['performance', '.settings-performance-grid'],
      ['behavior', '.settings-behavior'],
      ['update', '.settings-content > section:nth-last-of-type(2)'],
      ['formats', '.settings-content > section:last-of-type'],
    ]) {
      await page.locator(selector).evaluate(el => el.scrollIntoView({ block: 'start' }))
      await page.screenshot({ path: testInfo.outputPath(`${name}.png`) })
    }
    await page.getByRole('button', { name: '重置为默认值', exact: true }).scrollIntoViewIfNeeded()
    await expect(page.getByRole('button', { name: '重置为默认值', exact: true })).toBeInViewport()
    const calls = await page.evaluate(() => (window as any).__settingsCalls)
    expect(calls).not.toContain('set_auto_start')
  })
}

test('settings controls stay usable in English and at large scale', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1100, height: 760 })
  await openSettings(page)
  await page.getByRole('button', { name: 'English', exact: true }).click()
  await checkLayout(page)
  await page.screenshot({ path: testInfo.outputPath('english.png') })
  await page.locator('.settings-language button').first().click()
  const smart = page.getByTestId('smart-directory-switch')
  await expect(smart).toHaveAttribute('aria-checked', 'false')
  await smart.click()
  await expect(smart).toHaveAttribute('aria-checked', 'true')
  await smart.click()
  const parallel = page.getByTestId('parallel-zip-switch')
  await expect(parallel).toHaveAttribute('aria-checked', 'false')
  await parallel.click()
  await expect(parallel).toHaveAttribute('aria-checked', 'true')
  await parallel.click()
  const reduce = page.getByRole('switch', { name: '减少动画', exact: true })
  await reduce.click()
  await expect(reduce).toHaveAttribute('aria-checked', 'true')
  await page.locator('.accessibility-font button').last().click()
  await expect(page.locator('.accessibility-font button').last()).toHaveAttribute('aria-pressed', 'true')
  const scale = page.locator('.settings-scale input')
  await scale.fill('200')
  await scale.dispatchEvent('change')
  await checkLayout(page)
  await page.locator('.settings-appearance').evaluate(el => el.scrollIntoView({ block: 'start' }))
  await page.screenshot({ path: testInfo.outputPath('large-scale.png') })
  await scale.fill('100')
  await scale.dispatchEvent('change')
  await page.locator('.accessibility-font button').first().click()
  await page.getByRole('button', { name: '重置为默认值', exact: true }).click()
  await expect(page.locator('.modal-no-glass')).toBeVisible()
  await page.locator('.modal-no-glass button').last().click()
  await expect(page.locator('.modal-no-glass')).toHaveCount(0)
})

import { test, expect, type Page } from '@playwright/test'

// Real application and compiled CSS; native IPC only is substituted.
// No update is downloaded and no Windows settings or desktop are changed.
async function openSettings(page: Page, theme: string) {
  await page.addInitScript(theme => {
    if (!localStorage.getItem('app-theme')) localStorage.setItem('app-theme', theme)
    const win = window as any
    win.__TAURI_IPC__ = (message: any) => {
      let result: any
      if (message.cmd === 'load_app_settings') result = JSON.stringify({ theme, autoCheckUpdates: false })
      else if (message.cmd === 'get_archive_engine_capabilities') result = { available: true, fullEngine: true, formats: [], browseExtensions: [], nestedExtensions: [], boundedPreviewFormats: [], imagePreviewExtensions: [], textPreviewExtensions: [], message: 'ready' }
      else if (message.cmd === 'load_history' || message.cmd === 'take_pending_context_actions') result = []
      else if (message.cmd === 'is_context_menu_registered') result = true
      else if (message.cmd === 'check_app_update') result = { shouldUpdate: true, manifest: { version: '9.9.9', date: '2026-11-06', body: 'UI regression fixture' } }
      else if (message.cmd === 'install_app_update') return // hold in preparing; no actual update
      else if (message.cmd === 'tauri' && message.__tauriModule === 'App') result = '1.7.40'
      win[`_${message.callback}`]?.(result)
    }
  }, theme)
  await page.goto('/#/settings?section=accessibility', { waitUntil: 'domcontentloaded', timeout: 60000 })
  await expect(page.getByRole('switch', { name: '减少动画', exact: true })).toBeVisible()
}

async function expectFrameInsideViewport(page: Page) {
  const bounds = await page.evaluate(() => {
    const frame = document.querySelector('.main-container')!.getBoundingClientRect()
    const sidebar = document.querySelector('.app-sidebar')!.getBoundingClientRect()
    return { rootMargin: getComputedStyle(document.documentElement).marginLeft, left: frame.left, top: frame.top,
      right: frame.right, bottom: frame.bottom, sidebarLeft: sidebar.left, width: innerWidth, height: innerHeight }
  })
  expect(bounds.rootMargin).toBe('0px')
  expect(Math.abs(bounds.left)).toBeLessThanOrEqual(1)
  expect(Math.abs(bounds.top)).toBeLessThanOrEqual(1)
  expect(Math.abs(bounds.sidebarLeft)).toBeLessThanOrEqual(1)
  expect(bounds.right).toBeLessThanOrEqual(bounds.width + 1)
  expect(bounds.bottom).toBeLessThanOrEqual(bounds.height + 1)
}

for (const scenario of [
  { name: 'wide-light', width: 1920, height: 1040, theme: 'light' },
  { name: 'compact-dark', width: 1024, height: 720, theme: 'dark' },
  { name: 'small-obsidian', width: 920, height: 620, theme: 'obsidian' },
]) {
  test(`reduced motion remains reversible and survives restart: ${scenario.name}`, async ({ page }, info) => {
    test.setTimeout(90000)
    await page.setViewportSize(scenario)
    await openSettings(page, scenario.theme)
    const toggle = page.getByRole('switch', { name: '减少动画', exact: true })
    await expectFrameInsideViewport(page)
    await toggle.click()
    await expect(toggle).toHaveAttribute('aria-checked', 'true')
    await expectFrameInsideViewport(page)
    await expect(toggle).toBeInViewport({ ratio: 1 })
    // A saved true value must remain usable without clearing any user settings.
    await page.reload({ waitUntil: 'domcontentloaded' })
    await expect(toggle).toHaveAttribute('aria-checked', 'true')
    await expectFrameInsideViewport(page)
    await expect(toggle).toBeInViewport({ ratio: 1 })
    expect(await toggle.locator('.settings-toggle-knob').evaluate(el => getComputedStyle(el).transform)).not.toBe('none')
    await page.screenshot({ path: info.outputPath('reduced-motion-settings.png') })
    await toggle.click()
    await expect(toggle).toHaveAttribute('aria-checked', 'false')
    await expectFrameInsideViewport(page)
    await toggle.click()
    await page.locator('.settings-reset-action').click()
    const reset = page.locator('.settings-view .modal-no-glass')
    await expect(reset).toBeInViewport({ ratio: 1 })
    await reset.getByRole('button', { name: '取消', exact: true }).click()
    await expect(reset).toHaveCount(0)
    await page.locator('.app-sidebar button').filter({ hasText: '压缩中心' }).click()
    await expect(page).toHaveURL(/#\/compress$/)
    await expectFrameInsideViewport(page)
    await page.locator('.app-sidebar button').filter({ hasText: '解压中心' }).click()
    await expect(page).toHaveURL(/#\/decompress$/)
    await expectFrameInsideViewport(page)
  })
}

test('unknown update length stays visible under app and system reduced motion', async ({ page }, info) => {
  test.setTimeout(90000)
  await page.setViewportSize({ width: 1024, height: 720 })
  await openSettings(page, 'light')
  await page.getByRole('switch', { name: '减少动画', exact: true }).click()
  await page.locator('[data-settings-section="updates"]').click()
  await page.getByRole('button', { name: /立即检查更新/ }).click()
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('button', { name: '下载并安装', exact: true }).click()
  const fill = dialog.locator('.update-progress-track > .indeterminate')
  await expect(fill).toBeVisible()
  const expectStaticProgress = async () => {
    await expectFrameInsideViewport(page)
    await expect(dialog.locator('.app-dialog')).toBeInViewport({ ratio: 1 })
    const geometry = await fill.evaluate(el => {
      const track = el.parentElement!.getBoundingClientRect(), fill = el.getBoundingClientRect(), style = getComputedStyle(el)
      return { left: fill.left, right: fill.right, trackLeft: track.left, trackRight: track.right, width: fill.width, animation: style.animationName }
    })
    expect(geometry.animation).toBe('none')
    expect(geometry.width).toBeGreaterThan(0)
    expect(geometry.left).toBeGreaterThan(geometry.trackLeft)
    expect(geometry.right).toBeLessThan(geometry.trackRight)
  }
  await expectStaticProgress()
  await page.screenshot({ path: info.outputPath('reduced-motion-update.png') })
  await page.emulateMedia({ reducedMotion: 'reduce' })
  await page.evaluate(async () => {
    const modulePath = '/src/stores/app.ts'
    const { useAppStore } = await import(/* @vite-ignore */ modulePath)
    const store = useAppStore()
    store.updateSettings({ accessibility: { ...store.settings.accessibility, reduceMotion: false } })
  })
  await expect(page.locator('html')).not.toHaveClass(/reduce-motion/)
  await expectStaticProgress()
  await page.emulateMedia({ reducedMotion: 'no-preference' })
  await expect(fill).not.toHaveCSS('animation-name', 'none')
  await expectFrameInsideViewport(page)
})


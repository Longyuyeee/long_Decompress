import { test, expect } from '@playwright/test'

for (const width of [1280, 920]) {
  test(`Windows menu location and repair fit the settings page at ${width}px`, async ({ page }, info) => {
    test.setTimeout(90000)
    await page.setViewportSize({ width, height: 720 })
    await page.addInitScript(() => {
      const win = window as any
      win.__menuCalls = []
      win.__TAURI_IPC__ = (message: any) => {
        win.__menuCalls.push(message.cmd)
        let result: any
        if (message.cmd === 'load_app_settings') result = JSON.stringify({ theme: 'dark', autoCheckUpdates: false })
        else if (message.cmd === 'get_archive_engine_capabilities') result = { available: true, fullEngine: true, formats: [], browseExtensions: [], nestedExtensions: [], boundedPreviewFormats: [], imagePreviewExtensions: [], textPreviewExtensions: [], message: 'ready' }
        else if (message.cmd === 'is_context_menu_registered' || message.cmd === 'register_context_menu') result = true
        else if (message.cmd === 'context_menu_location') result = 'classic'
        else if (message.cmd === 'check_auto_start') result = false
        else if (message.cmd === 'archive_file_associations') result = [{ extension: '.zip', registered: true, isDefault: true }, { extension: '.7z', registered: true, isDefault: false }]
        else if (message.cmd === 'load_history' || message.cmd === 'take_pending_context_actions') result = []
        else if (message.cmd === 'tauri' && message.__tauriModule === 'App') result = '1.7.41'
        win[`_${message.callback}`]?.(result)
      }
    })
    await page.goto('/#/settings?section=behavior', { waitUntil: 'domcontentloaded', timeout: 60000 })
    const location = page.getByTestId('context-menu-location')
    await expect(location).toContainText('显示更多选项')
    const repair = page.getByTestId('repair-context-menu')
    await repair.scrollIntoViewIfNeeded()
    await expect(repair).toBeInViewport({ ratio: 1 })
    await page.evaluate(() => { (window as any).__menuCalls = [] })
    await repair.click()
    await expect(repair).toBeEnabled()
    const calls = await page.evaluate(() => (window as any).__menuCalls as string[])
    expect(calls.filter(name => name === 'register_context_menu')).toHaveLength(1)
    expect(calls).not.toContain('unregister_context_menu')
    await expect(location).toBeInViewport({ ratio: 1 })
    const overflow = await location.locator('..').evaluate(el => ({ left: el.getBoundingClientRect().left, right: el.getBoundingClientRect().right, viewport: innerWidth, overflow: el.scrollWidth - el.clientWidth }))
    expect(overflow.left).toBeGreaterThanOrEqual(0)
    expect(overflow.right).toBeLessThanOrEqual(overflow.viewport)
    expect(overflow.overflow).toBeLessThanOrEqual(1)
    const closeToast = page.locator('.toast-stack button')
    if (await closeToast.count()) { await closeToast.last().click(); await expect(closeToast).toHaveCount(0) }
    await page.screenshot({ path: info.outputPath(`menu-settings-${width}.png`) })
  })
}

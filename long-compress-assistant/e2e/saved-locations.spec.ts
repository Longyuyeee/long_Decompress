import { expect, test } from '@playwright/test'

test('bookmark a folder and reopen it in the other pane after a restart', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 920, height: 620 })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'file_manager_locations') value = [
        { name: '资料', path: 'C:/资料', kind: 'home' }, { name: '输出盘', path: 'D:/', kind: 'drive' },
      ]
      if (message.cmd === 'list_files') value = []
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/browser')
  await expect(page.getByTestId('file-manager-path-input-left')).toHaveValue('C:/资料')
  await page.getByTestId('bookmark-current-left').click()
  await expect(page.getByTestId('bookmark-current-left')).toHaveAttribute('aria-pressed', 'true')
  await page.reload()
  await expect(page.getByTestId('bookmark-current-left')).toHaveAttribute('aria-pressed', 'true')
  await page.getByTestId('file-manager-path-input-right').focus()
  await expect(page.getByTestId('saved-location-right')).toContainText('C:/资料')
  await page.screenshot({ path: testInfo.outputPath('bookmarks-920.png'), animations: 'disabled' })
  await page.getByTestId('saved-location-right').click()
  await expect(page.getByTestId('file-manager-path-input-right')).toHaveValue('C:/资料')
  await expect(page.getByTestId('file-manager-path-input-left')).toHaveValue('C:/资料')
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    for (const pane of await page.locator('.file-pane').all()) {
      expect(await pane.evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1)
    }
  }
})

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
      if (message.cmd === 'inspect_extraction_destination') value = 'empty'
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
  await page.goto('/#/decompress')
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    ;(await import(path)).useTaskStore().addTask({ id: 'saved-output', type: 'decompression', name: '资料.zip', sourceFiles: ['E:/资料.zip'], outputPath: 'E:/', configurationMode: 'global' })
  })
  await page.getByTestId('saved-output-select').selectOption('C:/资料')
  await expect.poll(() => page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    return (await import(path)).useTaskStore().tasks.find((task: any) => task.id === 'saved-output')?.outputPath
  })).toBe('C:/资料')
  await page.goto('/#/compress')
  await page.evaluate(async () => {
    const path = '/src/stores/compression.ts'
    ;(await import(path)).useCompressionStore().addFile({ name: '资料.txt', path: 'E:/资料.txt', size: 4096, type: 'file', isDirectory: false })
  })
  await page.getByRole('button', { name: '展开压缩详情' }).click()
  await page.getByTestId('compression-advanced-options').click()
  await page.getByTestId('saved-output-select').selectOption('C:/资料')
  await expect(page.getByTestId('compression-output-path')).toHaveValue('C:/资料')
  await page.waitForTimeout(450)
  await page.getByTestId('compression-output-path').scrollIntoViewIfNeeded()
  await page.screenshot({ path: testInfo.outputPath('saved-compression-output.png') })
})

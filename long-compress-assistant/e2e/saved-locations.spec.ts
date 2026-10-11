import { expect, test } from '@playwright/test'

test('long saved paths fit the viewport and remain keyboard accessible', async ({ page }, info) => {
  await page.addInitScript(() => {
    localStorage.setItem('long-saved-locations-v1', JSON.stringify({
      bookmarks: ['F:/下载/项目资料与设计文件/这是一个很长的中文目录名称用于确认省略展示与完整路径可访问'],
      recent: Array.from({ length: 20 }, (_, index) => `D:/项目/${index}/历史输出目录`),
    }))
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      const value = message.cmd === 'load_app_settings' ? '{}' : message.cmd === 'inspect_extraction_destination' ? 'unavailable' : undefined
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/decompress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    ;(await import(path)).useTaskStore().addTask({ id: 'long-location', type: 'decompression', name: '资料.zip', sourceFiles: ['E:/资料.zip'], outputPath: 'E:/', configurationMode: 'global' })
  })
  const trigger = page.getByTestId('saved-output-select')
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await trigger.click()
    const menu = page.getByRole('menu', { name: '常用目录' })
    await expect(menu).toBeVisible()
    const box = await menu.boundingBox()
    expect(box!.x).toBeGreaterThanOrEqual(0)
    expect(box!.y).toBeGreaterThanOrEqual(0)
    expect(box!.x + box!.width).toBeLessThanOrEqual(width)
    expect(box!.y + box!.height).toBeLessThanOrEqual(620)
    expect(await menu.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
    await expect(menu.getByRole('menuitem').first()).toHaveAttribute('title', /这是一个很长的中文目录/)
    await page.screenshot({ path: info.outputPath(`long-locations-${width}.png`) })
    await page.keyboard.press('End')
    await expect(menu.getByRole('menuitem').last()).toBeFocused()
    await page.keyboard.press('Home')
    await expect(menu.getByRole('menuitem').first()).toBeFocused()
    await page.keyboard.press('Escape')
    await expect(trigger).toBeFocused()
    await trigger.press('Enter')
    await page.locator('h1').click({ position: { x: 1, y: 1 } })
    await expect(menu).toHaveCount(0)
  }
  await trigger.click()
  await page.getByRole('menuitem').first().click()
  await expect(page.getByRole('status')).toContainText('保留原输出位置')
})

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
  await page.getByTestId('saved-output-select').click()
  await expect(page.getByRole('menu', { name: '常用目录' })).toBeVisible()
  await page.screenshot({ path: testInfo.outputPath('decompress-location-menu.png') })
  await page.getByRole('menuitem', { name: 'C:/资料', exact: true }).first().click()
  await expect.poll(() => page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    return (await import(path)).useTaskStore().tasks.find((task: any) => task.id === 'saved-output')?.outputPath
  })).toBe('C:/资料')
  await page.setViewportSize({ width: 920, height: 620 })
  await page.goto('/#/compress')
  await page.evaluate(async () => {
    const path = '/src/stores/compression.ts'
    ;(await import(path)).useCompressionStore().addFile({ name: '资料.txt', path: 'E:/资料.txt', size: 4096, type: 'file', isDirectory: false })
  })
  await page.getByRole('button', { name: '展开压缩详情' }).click()
  await expect(page.getByTestId('compression-advanced-options')).toHaveCount(0)
  await page.getByTestId('saved-output-select').click()
  await expect(page.getByRole('menu', { name: '常用目录' })).toBeVisible()
  await page.screenshot({ path: testInfo.outputPath('compress-location-menu.png') })
  await page.keyboard.press('Escape')
  await expect(page.getByTestId('saved-output-select')).toBeFocused()
  await page.getByTestId('saved-output-select').press('ArrowDown')
  await page.getByRole('menuitem', { name: 'C:/资料', exact: true }).first().click()
  await expect(page.getByTestId('compression-output-path')).toHaveValue('C:/资料')
  await page.waitForTimeout(450)
  await page.getByTestId('compression-output-path').scrollIntoViewIfNeeded()
  await page.screenshot({ path: testInfo.outputPath('saved-compression-output.png') })
})

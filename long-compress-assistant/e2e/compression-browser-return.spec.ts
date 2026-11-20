import { expect, test } from '@playwright/test'

for (const source of ['left', 'right'] as const) {
  test(`quick packing from the ${source} pane restores the source and locates the published file`, async ({ page }, info) => {
    await page.setViewportSize({ width: source === 'left' ? 1180 : 760, height: 680 })
    const destination = source === 'left' ? 'right' : 'left'
    const sourcePath = source === 'left' ? 'C:/项目/当前' : 'D:/交付/当前'
    const destinationPath = source === 'left' ? 'D:/交付/当前' : 'C:/项目/当前'
    const publishedPath = destinationPath + '/资料 (2).zip'
    await page.addInitScript(({ publishedPath, destinationPath, sourcePath }) => {
      const desktop = window as unknown as Record<string, any>
      let published = false
      window.__TAURI_IPC__ = (message: Record<string, any>) => {
        const reply = desktop[`_${message.callback}`]
        switch (message.cmd) {
          case 'file_manager_locations':
            return reply?.([{ path: 'C:/项目', name: '项目', kind: 'home' }, { path: 'D:/交付', name: '交付', kind: 'drive' }])
          case 'list_files': {
            const path = message.path
            if (!path.endsWith('/当前')) return reply?.([{ path: path + '/当前', name: '当前', size: 0, isDir: true }])
            const files = [{ path: path + '/资料.txt', name: '资料.txt', size: 128, isDir: false }]
            if (path === sourcePath) {
              for (let index = 1; index <= 24; index++) {
                const name = `交付文档 ${index}.txt`
                files.push({ path: path + '/' + name, name, size: index * 1024, isDir: false })
              }
            }
            if (published && path === destinationPath) files.push({ path: publishedPath, name: '资料 (2).zip', size: 256, isDir: false })
            return reply?.(files)
          }
          case 'compress_files':
            desktop.requestedPackPath = message.outputPath
            desktop.finishPack = () => {
              published = true
              reply?.({ primaryPath: publishedPath, files: [{ path: publishedPath, bytes: 256 }], totalBytes: 256, split: false })
            }
            return
          case 'load_app_settings':
            return reply?.('{}')
          case 'preflight_operation_resources':
            return reply?.({ canStart: true, warnings: [], summary: '', issues: [] })
          case 'take_pending_context_actions':
          case 'load_history':
            return reply?.([])
          default:
            return reply?.(undefined)
        }
      }
    }, { publishedPath, destinationPath, sourcePath })
    await page.goto('/#/browser')
    for (const side of ['left', 'right']) {
      await page.locator(`[data-file-pane="${side}"]`).getByText('当前', { exact: true }).dblclick()
    }
    const sourceRow = page.locator(`[data-file-pane="${source}"] [data-entry-path="${sourcePath}/资料.txt"]`)
    await sourceRow.click()
    const sourceList = page.locator(`[data-file-pane="${source}"] .file-list`)
    const scrollTop = await sourceList.evaluate(element => element.scrollTop)
    expect(scrollTop).toBeGreaterThan(0)
    await page.getByTitle('压缩到另一栏').click()
    await expect(page.getByRole('heading', { name: '压缩中心', exact: true })).toBeVisible()
    await page.waitForFunction(() => typeof Reflect.get(window, 'finishPack') === 'function')
    expect(await page.evaluate(() => Reflect.get(window, 'requestedPackPath')?.replace(/\\/g, '/'))).toBe(destinationPath + '/资料.zip')
    const locate = page.getByTestId('compression-locate-in-browser')
    await expect(locate).toHaveCount(0)
    await page.getByTestId('compression-return-to-browser').click()
    await expect(sourceRow).toHaveClass(/selected/)
    await expect.poll(() => sourceList.evaluate(element => element.scrollTop)).toBe(scrollTop)
    await expect(page.getByTestId(`file-manager-path-input-${destination}`)).toHaveValue(destinationPath)
    await page.evaluate(() => Reflect.get(window, 'finishPack')())
    await page.getByRole('button', { name: /压缩中心/ }).click()
    await expect(locate).toBeVisible()

    const result = page.getByTestId('result-actions').filter({ has: locate })
    const more = result.getByRole('button', { name: '更多结果操作' })
    await more.scrollIntoViewIfNeeded()
    const bounds = await result.boundingBox()
    await more.click()
    const menu = page.getByRole('menu', { name: '更多结果操作' })
    await expect(menu.getByRole('menuitem', { name: '复制路径' })).toBeInViewport()
    await expect(menu.getByRole('menuitem', { name: '生成校验清单' })).toBeInViewport()
    expect(await result.boundingBox()).toEqual(bounds)
    await page.screenshot({ path: info.outputPath('result-menu.png'), animations: 'disabled' })
    await page.keyboard.press('Escape')
    await expect(menu).toHaveCount(0)
    await expect(more).toBeFocused()
    await locate.click()
    const output = page.locator(`[data-file-pane="${destination}"] [data-entry-path="${publishedPath}"]`)
    await expect(output).toHaveClass(/selected/)
    await expect(sourceRow).toHaveClass(/selected/)
    await expect(page.getByTestId('file-manager-notice')).toContainText('已定位 资料 (2).zip')
    await page.screenshot({ path: info.outputPath('browser-return.png'), animations: 'disabled' })
  })
}

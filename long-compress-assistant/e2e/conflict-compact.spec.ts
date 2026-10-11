import { test, expect } from '@playwright/test'

test('conflict choices stay in view without scrolling and success notices stay behind the dialog', async ({ page }, info) => {
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      ;(window as any)[`_${message.callback}`]?.(message.cmd === 'load_app_settings' ? '{}' : undefined)
    }
  })
  await page.goto('/#/decompress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const taskPath = '/src/stores/task.ts', uiPath = '/src/stores/ui.ts'
    const store = (await import(taskPath)).useTaskStore()
    store.addTask({ id: 'conflict', name: '资料.zip.001', type: 'decompression', sourceFiles: ['D:/资料.zip.001'], outputPath: 'D:/out' })
    store.tasks[0].conflicts.push({ taskId: 'conflict', fileName: '资料.txt', sourcePath: 'D:/资料.zip.001', destPath: 'D:/' + '很长的输出目录/'.repeat(12) + '资料.txt', sourceSize: 2048, destSize: 1024, sourceModified: 0, destModified: 1700000000000 })
    ;(await import(uiPath)).useUIStore().toasts.push({ id: 'success', type: 'success', message: '已创建新任务，请确认后开始', duration: 0 })
  })
  const dialog = page.getByRole('dialog')
  await expect(dialog).toBeVisible()
  for (const [width, height] of [[920, 620], [760, 620], [560, 480]]) {
    await page.setViewportSize({ width, height })
    await page.waitForTimeout(650)
    for (const action of ['rename', 'overwrite', 'skip']) {
      const button = page.getByTestId(`conflict-${action}`)
      await expect(button).toBeInViewport({ ratio: 1 })
      expect(await button.evaluate(el => {
        const r = el.getBoundingClientRect()
        return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2))
      })).toBe(true)
    }
    expect(await dialog.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
    expect(await page.locator('.toast-stack').evaluate(el => Number(getComputedStyle(el).zIndex))).toBeLessThan(await dialog.evaluate(el => Number(getComputedStyle(el).zIndex)))
    await page.screenshot({ path: info.outputPath(`conflict-${width}.png`) })
    const unknownTime = dialog.getByText('时间未知', { exact: true })
    await unknownTime.scrollIntoViewIfNeeded()
    await expect(unknownTime).toBeInViewport({ ratio: 1 })
    await expect(page.getByTestId('conflict-rename')).toBeInViewport({ ratio: 1 })
  }
  await page.evaluate(async () => {
    const path = '/src/stores/ui.ts'
    ;(await import(path)).useUIStore().toasts.push({ id: 'error', type: 'error', message: '提交失败，请重试', duration: 0 })
  })
  await expect(page.getByRole('alert')).toContainText('提交失败')
  expect(await page.locator('.toast-stack').evaluate(el => Number(getComputedStyle(el).zIndex))).toBeGreaterThan(await dialog.evaluate(el => Number(getComputedStyle(el).zIndex)))
})

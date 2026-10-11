import { test, expect } from '@playwright/test'

test('split history retains source workload rather than copied archive size', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 620 })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      if (message.cmd === 'open_in_explorer' && (window as any).__locateFailure) {
        ;(window as any)[`_${message.error}`]?.((window as any).__locateFailure)
        return
      }
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'list_task_history_page') value = { records: [(window as any).__splitRecord], nextCursor: null }
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/compress')
  await page.waitForSelector('h1')
  const metrics = await page.evaluate(async () => {
    const path = '/src/types/taskHistory.ts'
    const { createTaskHistoryRecord } = await import(path)
    const record = createTaskHistoryRecord({
      id: 'split-history', name: '资料备份.zip（分卷）', type: 'compression', status: 'completed', progress: 100,
      sourceFiles: ['C:/资料'], outputPath: 'D:/资料备份.zip', conflicts: [], logs: [],
      processedBytes: 1048576, totalBytes: 1048576, outputBytes: 1048576,
      compressionSourceProgress: { processedBytes: 10485760, totalBytes: 10485760 },
      archiveOutput: { primaryPath: 'D:/资料备份.zip.001', totalBytes: 1048576, split: true,
        files: [{ path: 'D:/资料备份.zip.001', bytes: 524288 }, { path: 'D:/资料备份.zip.002', bytes: 524288 }] },
    })
    ;(window as any).__splitRecord = record
    window.location.hash = '#/history'
    return record.metrics
  })
  expect(metrics).toMatchObject({ inputBytes: 10485760, outputBytes: 1048576, savingsRatio: 0.9 })
  const row = page.getByTestId('history-record-row')
  await expect(row).toHaveCount(1)
  await expect(row).toContainText('10.0 MiB')
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await page.waitForTimeout(350)
    await page.screenshot({ path: info.outputPath(`split-history-${width}.png`) })
  }
  await row.click()
  await page.getByRole('button', { name: /查看产物/ }).click()
  const dialog = page.getByRole('dialog', { name: '压缩产物', exact: true })
  await expect(dialog).toContainText('共 2 卷')
  await expect(dialog).toContainText('D:/资料备份.zip.002')
  await page.evaluate(() => { (window as any).__locateFailure = 'EXPLORER_PATH_UNAVAILABLE:not-found: missing' })
  await dialog.getByRole('button', { name: '定位首卷' }).click()
  await expect(dialog.getByRole('alert')).toContainText('已移动或删除')
  await page.evaluate(() => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {
      writeText: async (text: string) => { (window as any).__copiedArchivePath = text },
    } })
  })
  await dialog.getByRole('button', { name: '复制路径' }).click()
  await expect(dialog.getByRole('button', { name: '已复制' })).toBeVisible()
  expect(await page.evaluate(() => (window as any).__copiedArchivePath)).toBe('D:/资料备份.zip.001')
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await page.waitForTimeout(350)
    await page.screenshot({ path: info.outputPath(`history-manifest-${width}.png`) })
  }
  await page.evaluate(() => { (window as any).__locateFailure = 'EXPLORER_PATH_UNAVAILABLE:permission-denied: denied' })
  await dialog.getByRole('button', { name: '定位首卷' }).click()
  await expect(dialog.getByRole('alert')).toContainText('没有权限')
  await page.evaluate(() => { (window as any).__locateFailure = null })
  await dialog.getByRole('button', { name: '定位首卷' }).click()
  await expect(dialog.getByRole('alert')).toHaveCount(0)
  await dialog.getByRole('button', { name: '关闭产物清单' }).click()
  await expect(dialog).toHaveCount(0)
})

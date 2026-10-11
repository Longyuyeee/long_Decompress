import { test, expect } from '@playwright/test'

test('split history retains source workload rather than copied archive size', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 620 })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
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
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await page.waitForTimeout(350)
    await page.screenshot({ path: info.outputPath(`history-manifest-${width}.png`) })
  }
  await dialog.getByRole('button', { name: '关闭产物清单' }).click()
  await expect(dialog).toHaveCount(0)
})

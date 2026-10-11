import { test, expect } from '@playwright/test'

test('incomplete ZIP volume history explains recovery without a password suggestion', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 620 })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'list_task_history_page') value = { records: [(window as any).__volumeRecord], nextCursor: null }
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/compress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const path = '/src/types/taskHistory.ts'
    const { createTaskHistoryRecord } = await import(path)
    ;(window as any).__volumeRecord = createTaskHistoryRecord({
      id: 'incomplete-volume', name: '资料.zip.001', type: 'decompression', status: 'failed', progress: 0,
      sourceFiles: ['D:/资料.zip.001'], outputPath: 'D:/output', conflicts: [], logs: [],
      error: '[archive-inspection:incomplete] 无法读取完整 ZIP 卷组：Missing ZIP volumes: D:/资料.zip.002',
    })
    // The save command classifies the error before returning persisted history.
    ;(window as any).__volumeRecord.failure = {
      schemaVersion: 1, category: 'incomplete', stage: 'inspection', evidence: 'recorded-marker',
    }
    window.location.hash = '#/history'
  })
  await page.getByTestId('history-record-row').click()
  const reason = page.getByRole('region', { name: '最终失败原因', exact: true })
  await expect(reason).toContainText('卷组不完整或无效 · 归档检测')
  await expect(reason).toContainText('Missing ZIP volumes: D:/资料.zip.002')
  const guidance = page.getByText('卷组不完整或无效：先检查分卷', { exact: true })
  await expect(guidance).toBeVisible()
  await expect(page.getByText(/反复输入密码不能补齐分卷/)).toBeVisible()
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await guidance.scrollIntoViewIfNeeded()
    await page.waitForTimeout(400)
    await page.screenshot({ path: info.outputPath(`incomplete-history-${width}.png`) })
  }
})

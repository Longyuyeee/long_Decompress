import { expect, test } from '@playwright/test'

test('history statistics stay in a modal without losing filters or selected retries', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 620 })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'list_task_history_page') {
        const rows = Array.from({ length: 12 }, (_, index) => ({
          id: `stat-${index}`, name: `项目资料-${index}.zip`, taskType: 'compression', workloadKind: 'archive',
          status: index % 2 ? 'failed' : 'completed', sourcePaths: ['C:/项目'], outputPath: 'D:/项目.zip',
          completedAt: new Date().toISOString(), durationMs: 1000, processedBytes: 512, totalBytes: 1024, logs: [],
        }))
        value = { records: rows.filter(row => !message.query?.text || row.name.includes(message.query.text)), nextCursor: null }
      }
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/history')
  await expect(page.getByTestId('history-record-row')).toHaveCount(12)
  await expect(page.getByTestId('history-kpis')).toHaveCount(0)
  await expect(page.getByTestId('history-retention-policy')).toHaveCount(0)
  await page.getByTestId('history-search').fill('项目')
  await expect(page.getByTestId('history-record-row')).toHaveCount(12)
  await page.getByTestId('history-select-retry').first().check()
  await page.screenshot({ path: info.outputPath('history-clean-920.png') })
  const trigger = page.getByTestId('history-open-statistics')
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await trigger.click()
    const scroll = await page.locator('.history-view').evaluate(el => el.scrollTop)
    const modal = page.getByRole('dialog', { name: '统计与趋势' })
    await expect(modal).toBeVisible()
    await expect(modal).toContainText('当前显示的 12 条')
    await expect(modal.getByTestId('history-kpis')).toContainText('50%')
    await page.waitForTimeout(350)
    await page.screenshot({ path: info.outputPath(`history-statistics-${width}.png`) })
    const body = modal.getByTestId('history-statistics-content')
    expect(await body.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
    const scroller = modal.locator('.custom-scrollbar')
    await scroller.hover()
    await page.mouse.wheel(0, 600)
    await expect.poll(() => scroller.evaluate(el => el.scrollTop)).toBeGreaterThan(0)
    await page.screenshot({ path: info.outputPath(`history-statistics-scrolled-${width}.png`) })
    await page.keyboard.press('Shift+Tab')
    await expect(modal.getByRole('button', { name: '关闭统计' })).toBeFocused()
    await page.keyboard.press('Tab')
    await expect(modal.getByRole('button', { name: '关闭统计' })).toBeFocused()
    await page.keyboard.press('Escape')
    await expect(modal).toHaveCount(0)
    await expect(trigger).toBeFocused()
    await expect(page.getByTestId('history-search')).toHaveValue('项目')
    await expect(page.getByTestId('history-select-retry').first()).toBeChecked()
    expect(await page.locator('.history-view').evaluate(el => el.scrollTop)).toBe(scroll)
  }
  await page.getByTestId('history-search').fill('没有这个任务')
  await expect(page.getByTestId('history-record-row')).toHaveCount(0)
  await trigger.click()
  const empty = page.getByRole('dialog', { name: '统计与趋势' })
  await expect(empty).toContainText('当前显示的 0 条')
  await page.waitForTimeout(350)
  await page.screenshot({ path: info.outputPath('history-statistics-empty.png') })
  await empty.getByRole('button', { name: '关闭统计' }).click()
  await expect(empty).toHaveCount(0)
})

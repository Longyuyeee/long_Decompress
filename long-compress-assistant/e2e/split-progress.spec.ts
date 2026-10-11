import { test, expect } from '@playwright/test'

test('split stage is visible without claiming task completion', async ({ page }, info) => {
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      ;(window as any)[`_${message.callback}`]?.(message.cmd === 'load_app_settings' ? '{}' : undefined)
    }
  })
  await page.setViewportSize({ width: 920, height: 620 })
  await page.goto('/#/compress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const compressionPath = '/src/stores/compression.ts'
    const taskPath = '/src/stores/task.ts'
    const compression = (await import(compressionPath)).useCompressionStore()
    const tasks = (await import(taskPath)).useTaskStore()
    compression.addFile({ name: '旅行照片与视频备份.zip', path: 'C:/旅行照片与视频备份.zip', size: 104857600, type: 'file', isDirectory: false })
    tasks.addTask({ id: 'split-preview', name: '旅行照片与视频备份.zip', type: 'compression', sourceFiles: ['C:/旅行照片与视频备份.zip'], outputPath: 'C:/输出/备份.zip' })
    compression.selectedFiles[0].taskId = 'split-preview'
    Object.assign(tasks.tasks[0], { status: 'compressing', stage: 'Splitting', progress: 42, processedBytes: 44040192, totalBytes: 104857600, outputBytes: 44040192, speed: '10 MB/s', etaSeconds: 6 })
    tasks.tasks[0].compressionSourceProgress = { processedBytes: 209715200, totalBytes: 209715200 }
  })
  const cell = page.getByTestId('compression-status-progress').first()
  await expect(cell).toContainText('正在分卷')
  await expect(cell).toContainText('42.00%')
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await page.waitForTimeout(400)
    expect(await cell.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
    expect(await cell.locator(':scope > div').nth(1).evaluate(el => el.getBoundingClientRect().width)).toBeGreaterThan(80)
    await page.screenshot({ path: info.outputPath(`split-${width}.png`) })
  }
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const store = (await import(path)).useTaskStore()
    Object.assign(store.tasks[0], { stage: 'Verifying', progress: 100, processedBytes: 104857600, outputBytes: 104857600, speed: undefined, etaSeconds: undefined })
  })
  await expect(cell).toContainText('正在校验')
  await expect(cell).not.toContainText('已完成')
  await page.getByRole('button', { name: '展开压缩详情', exact: true }).click()
  const metrics = page.getByTestId('compression-live-metrics')
  await expect(metrics).toContainText('200 MB')
  await expect(metrics).toContainText('完成后统计')
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await metrics.scrollIntoViewIfNeeded()
    await page.waitForTimeout(400)
    await page.screenshot({ path: info.outputPath(`split-detail-${width}.png`) })
  }
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const store = (await import(path)).useTaskStore()
    Object.assign(store.tasks[0], { status: 'completed', outputBytes: 104857600 })
  })
  await expect(metrics).toContainText('节省 50.00%')
  await expect(page.getByTestId('compression-draft-execution')).toContainText('已完成')
  await page.screenshot({ path: info.outputPath('split-detail-completed.png') })
})

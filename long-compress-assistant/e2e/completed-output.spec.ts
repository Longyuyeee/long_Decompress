import { expect, test } from '@playwright/test'

test('open a batch result and recover from a missing directory in a small window', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 760, height: 520 })
  await page.addInitScript(() => {
    ;(window as any).openedOutputs = []
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'inspect_extraction_destination') value = message.path.includes('已移动') ? 'missing' : 'occupied'
      if (message.cmd === 'open_in_explorer') (window as any).openedOutputs.push(message.path)
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/decompress')
  await expect(page.getByRole('heading', { name: '解压中心', exact: true })).toBeVisible()
  await page.evaluate(async () => {
    const taskModule = '/src/stores/task.ts'
    const batchModule = '/src/stores/decompressionBatch.ts'
    const { useTaskStore } = await import(taskModule)
    const { useDecompressionBatchStore } = await import(batchModule)
    const store = useTaskStore()
    for (const id of ['trip', 'moved']) store.addTask({ id, name: id === 'trip' ? '旅行照片与视频的完整备份.zip' : '已经移动的资料.zip', type: 'decompression', sourceFiles: ['D:/source.zip'], outputPath: 'D:/output' })
    useDecompressionBatchStore().begin(['trip', 'moved'])
    store.tasks.forEach((task: any) => {
      task.outputPath = task.id === 'trip' ? 'D:/家庭相册/旅行/需要完整显示的很长目录名称/2026年十月假期照片与视频' : 'D:/已移动的资料'
      task.status = 'completed'
    })
    // Clearing completed rows must not lose this run's result access.
    store.tasks = []
  })
  await page.getByTestId('show-batch-results').click()
  const dialog = page.getByRole('dialog', { name: '本批成功结果', exact: true })
  const cards = dialog.getByTestId('completed-extraction-output')
  await expect(cards).toHaveCount(2)
  await cards.first().getByRole('button', { name: '打开输出目录' }).click()
  expect(await page.evaluate(() => (window as any).openedOutputs)).toEqual(['D:/家庭相册/旅行/需要完整显示的很长目录名称/2026年十月假期照片与视频'])
  await cards.last().getByRole('button', { name: '打开输出目录' }).click()
  await expect(cards.last().getByRole('alert')).toContainText('可能已被移动或删除')
  expect(await page.evaluate(() => (window as any).openedOutputs.length)).toBe(1)
  for (const card of await cards.all()) expect(await card.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
  await cards.last().getByRole('alert').scrollIntoViewIfNeeded()
  await page.screenshot({ path: testInfo.outputPath('batch-results-light.png'), animations: 'disabled' })
  await page.evaluate(async () => {
    const module = '/src/stores/app.ts'
    const { useAppStore } = await import(module)
    useAppStore().theme = 'dark'
  })
  await page.screenshot({ path: testInfo.outputPath('batch-results-dark.png'), animations: 'disabled' })
  await dialog.getByRole('button', { name: '关闭成功结果', exact: true }).click()
  await expect(dialog).toHaveCount(0)
})

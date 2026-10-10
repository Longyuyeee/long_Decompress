import { expect, test } from '@playwright/test'

test('batch results keep failures scoped across navigation and fit compact windows', async ({ page }, testInfo) => {
  await page.goto('/')
  await page.waitForURL('**/#/decompress')
  // Synthetic task state verifies UI only, not actual extraction or persistence.
  await page.evaluate(async () => {
    const taskModule = '/src/stores/task.ts'
    const batchModule = '/src/stores/decompressionBatch.ts'
    const { useTaskStore } = await import(taskModule)
    const { useDecompressionBatchStore } = await import(batchModule)
    const tasks = useTaskStore()
    const ids = Array.from({ length: 20 }, (_, i) => `batch-ui-${i}`)
    for (const id of [...ids, 'older-failure']) tasks.addTask({
      id, name: `${id}.zip`, type: 'decompression', sourceFiles: [`C:/fixtures/${id}.zip`], outputPath: 'C:/output',
    })
    useDecompressionBatchStore().begin(ids)
    for (const task of tasks.tasks) {
      task.status = ids.indexOf(task.id) >= 0 && ids.indexOf(task.id) < 17 ? 'completed' : 'failed'
      if (task.status === 'failed') task.error = '[archive-inspection:damaged] ZIP 文件损坏或不完整，请核对来源：Could not find EOCD'
    }
  })
  const summary = page.getByTestId('decompression-batch-summary')
  await expect(summary).toContainText('成功 17 · 失败 3')
  await page.getByTestId('show-batch-failures').click()
  await expect(page.getByTestId('decompression-batch-config-mode')).toHaveCount(0)
  await expect(page.getByRole('button', { name: '清理已完成', exact: true })).toHaveCount(0)
  await expect(page.getByTestId('task-row')).toHaveCount(3)
  await page.getByTestId('nav-Compress').click()
  await page.getByTestId('nav-Decompress').click()
  await expect(summary).toContainText('成功 17 · 失败 3')
  await expect(page.getByTestId('task-row')).toHaveCount(3)
  for (const size of [{ width: 920, height: 620 }, { width: 760, height: 520 }]) {
    await page.setViewportSize(size)
    await expect(summary).toBeVisible()
    expect(await summary.evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1)
  }
  await page.screenshot({ path: testInfo.outputPath('batch-summary-760.png') })
  await page.evaluate(async () => {
    const path = '/src/stores/ui.ts'
    const { useUIStore } = await import(path)
    useUIStore().showToast('error', '后台任务解压失败：文件损坏或不完整', 60000)
  })
  await page.getByTestId('retry-batch-failures').click()
  const corrections = page.getByTestId('failed-extraction-retry')
  await expect(corrections.locator('article')).toHaveCount(3)
  await expect(corrections.locator('details').first().locator('p')).not.toBeVisible()
  await corrections.locator('summary').first().click()
  await expect(corrections.locator('details').first().locator('p')).toContainText('Could not find EOCD')
  await corrections.locator('summary').first().click()
  expect(await page.locator('.toast-stack').evaluate(element => Number(getComputedStyle(element).zIndex))).toBeLessThan(
    await page.getByRole('dialog').evaluate(element => Number(getComputedStyle(element).zIndex)),
  )
  await expect(page.getByTestId('create-retry-drafts')).toContainText('3 项')
  await corrections.locator('article > label input').last().uncheck()
  await expect(page.getByTestId('create-retry-drafts')).toContainText('2 项')
  expect(await corrections.evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1)
  await expect(page.getByTestId('create-retry-drafts')).toBeInViewport()
  await corrections.locator('article').first().scrollIntoViewIfNeeded()
  await page.screenshot({ path: testInfo.outputPath('batch-retry-760.png') })
  await page.getByRole('dialog').getByRole('button', { name: '关闭', exact: true }).click()
  await page.getByTestId('show-all-decompression-tasks').click()
  await expect(page.getByTestId('task-row')).toHaveCount(21)
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const { useTaskStore } = await import(path)
    useTaskStore().tasks = []
  })
  await page.getByTestId('show-batch-failures').click()
  await expect(page.getByTestId('batch-failures-cleared')).toBeVisible()
  await expect(summary).toContainText('失败 3')
})

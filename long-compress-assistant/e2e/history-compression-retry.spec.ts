import { test, expect } from '@playwright/test'

test('retry a saved archive with its old parameters as an editable, non-running draft', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 700 })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'get_file_info') value = { name: message.path.split('/').pop(), size: 42, is_dir: !message.path.endsWith('.jpg') }
      if (message.cmd === 'list_task_history_page') value = { records: [{
        id: 'original-archive', name: '照片合集', taskType: 'compression', workloadKind: 'archive', status: 'failed',
        sourcePaths: ['C:/a.jpg', 'C:/b.jpg'], outputPath: 'D:/out/照片合集.zip',
        completedAt: '2026-10-10T00:00:00Z', durationMs: 2, processedBytes: 0, totalBytes: 0, logs: [],
        errorMessage: '输出空间不足',
        executionConfig: { schemaVersion: 1, kind: 'archive-compression', outputDirectory: 'D:/out', passwordRequired: false,
          settings: { format: 'zip', level: 8, filename: '照片合集', splitArchive: false, splitSize: '100',
            keepStructure: true, verifyAfter: true, createSolidArchive: false, fastPack: true } },
      }], nextCursor: null }
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/history')
  await page.getByTestId('history-compression-retry').click()
  const draft = page.getByTestId('history-compression-draft')
  await expect(draft).toContainText('已带入上次执行参数')
  await expect(draft.getByTestId('compression-fast-pack')).toHaveCount(0)
  await expect(draft.locator('input[type="range"]')).toHaveValue('8')
  await page.getByTestId('compression-retry-output').fill('E:/新目录')
  await page.setViewportSize({ width: 760, height: 620 })
  await draft.scrollIntoViewIfNeeded()
  expect(await draft.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
  await page.screenshot({ path: info.outputPath('retry-settings.png') })
  await page.getByTestId('compression-retry-create').click()
  await expect(draft).toContainText('草稿已创建')
  await page.getByRole('link', { name: '前往压缩中心确认并启动' }).click()
  await expect(page.getByTestId('compression-group-row')).toHaveCount(1)
  await expect(page.getByTestId('compression-fast-pack')).toHaveCount(0)
  await expect(page.getByText('快速打包', { exact: false })).toHaveCount(0)
  await page.getByTestId('compression-advanced-options').click()
  await page.getByTestId('compression-output-path').scrollIntoViewIfNeeded()
  await page.screenshot({ path: info.outputPath('compression-without-fast-pack.png') })
  const state = await page.evaluate(async () => {
    const compressionModule = '/src/stores/compression.ts'
    const taskModule = '/src/stores/task.ts'
    const store = (await import(compressionModule)).useCompressionStore()
    return { groups: store.groups, tasks: (await import(taskModule)).useTaskStore().tasks }
  })
  expect(state.groups[0]).toMatchObject({ retryOfTaskId: 'original-archive', outputPath: 'E:/新目录', settings: { fastPack: false, level: 8, deleteAfter: false } })
  expect(state.groups[0].files).toHaveLength(2)
  expect(state.tasks).toHaveLength(0)
})

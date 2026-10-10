import { expect, test } from '@playwright/test'

test('find an old task and go directly to its extraction settings', async ({ page }, testInfo) => {
  // UI fixture only; the SQLite query is exercised by the Rust history tests.
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'preview_extraction_output') value = message.outputPath + (message.createSubdirectory ? '/去年旅行照片' : '')
      if (message.cmd === 'inspect_extraction_destination') value = 'occupied'
      if (message.cmd === 'list_task_history_page') {
        const old = { id: 'old-trip', name: '去年旅行照片.zip', taskType: 'decompression', workloadKind: 'archive',
          status: 'completed', sourcePaths: ['D:/旅行/去年旅行照片.zip'], outputPath: 'D:/相册',
          executionConfig: { schemaVersion: 1, kind: 'archive-extraction', outputDirectory: 'D:/相册', extractToSubfolder: false },
          completedAt: '2025-05-01T12:00:00Z', durationMs: 2300, processedBytes: 1024, totalBytes: 1024, logs: [] }
        value = { records: !message.query?.text || old.name.includes(message.query.text) ? [old] : [], nextCursor: null }
      }
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/history')
  await expect(page.getByTestId('history-record-row')).toContainText('去年旅行照片.zip')
  await expect(page.getByTestId('history-range-filter')).toHaveValue('all')
  await page.getByTestId('history-search').fill('不存在')
  await expect(page.getByTestId('history-empty')).toContainText('已查找全部保存的历史')
  await page.getByTestId('history-search').fill('旅行')
  await expect(page.getByTestId('history-record-row')).toHaveCount(1)
  await page.setViewportSize({ width: 920, height: 620 })
  await page.screenshot({ path: testInfo.outputPath('history-search.png') })
  await page.getByTestId('history-retry').click()
  await expect(page.getByTestId('history-draft-source')).toHaveValue('D:/旅行/去年旅行照片.zip')
  await expect(page.getByTestId('history-config-restored')).toContainText('已带入')
  await expect(page.getByTestId('history-draft-output')).toHaveValue('D:/相册')
  await expect(page.getByTestId('history-draft-subfolder')).not.toBeChecked()
  await expect(page.getByTestId('extraction-output-path')).toHaveText('D:/相册')
  await page.getByTestId('history-draft-subfolder').check()
  await expect(page.getByTestId('extraction-output-path')).toHaveText('D:/相册/去年旅行照片')
  await page.getByTestId('history-draft-output').fill('E:/新相册')
  await expect(page.getByTestId('extraction-output-path')).toHaveText('E:/新相册/去年旅行照片')
  await expect(page.getByTestId('extraction-destination-notice')).toContainText('目录已有内容')
  await expect(page.getByTestId('history-draft-select-output')).toBeInViewport()
  await page.getByTestId('extraction-output-preview').scrollIntoViewIfNeeded()
  await page.screenshot({ path: testInfo.outputPath('history-retry.png') })
  await page.goto('/#/decompress')
  await expect(page.getByTestId('history-detail')).toHaveCount(0)
  await expect(page.getByRole('heading', { name: '解压中心', exact: true })).toBeVisible()
  await page.evaluate(async () => {
    const module = '/src/stores/task.ts'
    const { useTaskStore } = await import(module)
    useTaskStore().addTask({ id: 'preview-pending', name: '去年旅行照片.zip', type: 'decompression',
      sourceFiles: ['D:/旅行/去年旅行照片.zip'], outputPath: 'E:/新相册', extractToSubfolder: true, configurationMode: 'individual' })
  })
  await expect(page.getByTestId('extraction-output-preview')).toHaveCount(0)
  await page.evaluate(async () => {
    const module = '/src/stores/task.ts'
    const { useTaskStore } = await import(module)
    useTaskStore().addTask({ id: 'preview-second', name: '去年旅行照片.zip', type: 'decompression',
      sourceFiles: ['D:/备份/去年旅行照片.zip'], outputPath: 'E:/新相册', extractToSubfolder: true, configurationMode: 'individual' })
  })
  const firstRow = page.getByTestId('task-row').first()
  expect(await firstRow.evaluate(row => row.scrollHeight - row.clientHeight)).toBeLessThanOrEqual(1)
  await expect(firstRow.getByRole('button', { name: '查看任务详情', exact: true })).toBeInViewport()
  expect(await firstRow.evaluate(row => row.scrollWidth - row.clientWidth)).toBeLessThanOrEqual(1)
  await page.screenshot({ path: testInfo.outputPath('queue-preview.png'), animations: 'disabled' })
  await firstRow.getByRole('button', { name: '查看任务详情', exact: true }).click()
  await expect(page.getByRole('dialog')).toHaveCount(0)
  await expect(page.getByTestId('extraction-output-path')).toHaveText('E:/新相册/去年旅行照片')
  await expect(page.getByTestId('extraction-shared-destination')).toContainText('2 项任务')
  await page.getByTestId('extraction-output-preview').scrollIntoViewIfNeeded()
  await page.screenshot({ path: testInfo.outputPath('output-inline.png') })
  await firstRow.getByRole('button', { name: '关闭任务详情', exact: true }).click()
  await expect(page.getByRole('dialog')).toHaveCount(0)
  await expect(page.getByTestId('extraction-output-preview')).toHaveCount(0)
})


test('inline compression edits keep their settings while untouched rows follow defaults', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 920, height: 620 })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'check_rar_compression_support') value = { available: false }
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/compress')
  await page.evaluate(async () => {
    const module = '/src/stores/compression.ts'
    const { useCompressionStore } = await import(module)
    for (const name of ['one.txt', 'two.txt']) useCompressionStore().addFile({ name, path: 'D:/' + name, size: 10, type: 'file', isDirectory: false })
  })
  const row = page.getByTestId('compression-draft-row').first()
  await row.click()
  await expect(page.getByRole('dialog')).toHaveCount(0)
  await expect(row.locator('input[type="range"]')).toBeVisible()
  await page.evaluate(async () => {
    const module = '/src/stores/compression.ts'
    const { useCompressionStore } = await import(module)
    useCompressionStore().globalSettings.level = 4
  })
  await expect(row.locator('input[type="range"]')).toHaveValue('4')
  expect(await page.evaluate(async () => {
    const module = '/src/stores/compression.ts'
    const { useCompressionStore } = await import(module)
    return useCompressionStore().selectedFiles[0].settings
  })).toBeUndefined()
  await row.locator('input[type="range"]').fill('8')
  await page.evaluate(async () => {
    const module = '/src/stores/compression.ts'
    const { useCompressionStore } = await import(module)
    useCompressionStore().globalSettings.level = 2
    useCompressionStore().globalOutputPath = 'E:/later'
  })
  await expect(row.locator('input[type="range"]')).toHaveValue('8')
  expect(await page.evaluate(async () => {
    const module = '/src/stores/compression.ts'
    const { useCompressionStore } = await import(module)
    const store = useCompressionStore()
    return [store.getEffectiveOutputPath(store.selectedFiles[0].outputPath), store.getEffectiveSettings(store.selectedFiles[1].settings).level]
  })).toEqual(['', 2])
  await page.setViewportSize({ width: 760, height: 520 })
  expect(await row.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
  await page.screenshot({ path: testInfo.outputPath('compression-inline.png'), animations: 'disabled' })
})

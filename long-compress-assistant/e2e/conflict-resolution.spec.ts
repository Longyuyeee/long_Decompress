import { expect, test } from '@playwright/test'

test('skip just this file, then overwrite the remaining files without changing future defaults', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 760, height: 520 })
  // IPC responses and task state are synthetic; this is a browser interaction test.
  await page.addInitScript(() => {
    ;(window as any).conflictCalls = []
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'resolve_extraction_conflict') {
        ;(window as any).conflictCalls.push(message)
        setTimeout(() => (window as any)[`_${message.callback}`]?.('C:/输出'), 200)
        return
      }
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/decompress')
  await expect(page.getByRole('heading', { name: '解压中心', exact: true })).toBeVisible()
  await page.evaluate(async () => {
    const tasksPath = '/src/stores/task.ts'
    const appPath = '/src/stores/app.ts'
    const { useTaskStore } = await import(tasksPath)
    const { useAppStore } = await import(appPath)
    useAppStore().settings.conflictPolicy = 'ask'
    const store = useTaskStore()
    store.addTask({ id: 'conflict-ui', name: '项目资料.zip', type: 'decompression', sourceFiles: ['C:/项目资料.zip'], outputPath: 'C:/输出' })
    store.tasks.find((task: any) => task.id === 'conflict-ui').conflicts = ['保留旧版.txt', '更新报告.txt', '更新照片.jpg'].map(fileName => ({
      taskId: 'conflict-ui', fileName, sourcePath: 'C:/项目资料.zip', destPath: `C:/输出/${fileName}`,
      sourceSize: 2048, destSize: 1024, sourceModified: 0, destModified: 1700000000000,
    }))
  })
  const dialog = page.getByRole('dialog')
  await expect(dialog).toContainText('保留旧版.txt')
  await expect(dialog).toContainText('时间未知')
  await page.getByTestId('conflict-skip').click()
  await expect(dialog).toContainText('更新报告.txt')
  expect(await page.evaluate(() => (window as any).conflictCalls)).toEqual([])
  await page.getByTestId('conflict-apply-remaining').check()
  expect(await dialog.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
  await page.screenshot({ path: testInfo.outputPath('conflict-760.png'), animations: 'disabled' })
  await page.getByTestId('conflict-overwrite').click()
  await expect(dialog).toHaveCount(0)
  const calls = await page.evaluate(() => (window as any).conflictCalls)
  expect(calls).toHaveLength(1)
  expect(calls[0]).toMatchObject({
    taskId: 'conflict-ui', fallbackAction: 'overwrite',
    resolutions: [
      { destPath: 'C:/输出/保留旧版.txt', action: 'skip' },
      { destPath: 'C:/输出/更新报告.txt', action: 'overwrite' },
    ],
  })
  expect(await page.evaluate(async () => {
    const path = '/src/stores/app.ts'
    return (await import(path)).useAppStore().settings.conflictPolicy
  })).toBe('ask')
})

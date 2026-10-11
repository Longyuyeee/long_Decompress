import { test, expect } from '@playwright/test'

test('completion immediately settles progress even during animation', async ({ page }, info) => {
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      ;(window as any)[`_${message.callback}`]?.(message.cmd === 'load_app_settings' ? '{}' : undefined)
    }
  })
  await page.goto('/#/decompress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const store = (await import(path)).useTaskStore()
    store.addTask({ id: 'finishing', name: '真实目录.zip', type: 'compression', sourceFiles: [], outputPath: '' })
    store.updateTaskStatus('finishing', 'compressing')
    store.tasks.find(t => t.id === 'finishing')!.progress = 66
  })
  await expect(page.getByTestId('summary-progress-value')).toHaveText('66%')
  const snapshot = await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const vue = '/node_modules/.vite/deps/vue.js'
    const store = (await import(path)).useTaskStore()
    store.updateTaskStatus('finishing', 'completed')
    await (await import(vue)).nextTick()
    return {
      value: document.querySelector('[data-testid="summary-progress-value"]')?.textContent,
      duration: getComputedStyle(document.querySelector('[data-testid="summary-progress-ring"]')!).transitionDuration,
    }
  })
  expect(snapshot).toEqual({ value: '100%', duration: '0s' })
  await expect(page.getByTestId('global-progress-summary')).toHaveAttribute('title', '全部完成')
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await page.screenshot({ path: info.outputPath(`completed-${width}.png`) })
  }
})

test('mixed final outcomes remain visible without a success summary', async ({ page }, info) => {
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      ;(window as any)[`_${message.callback}`]?.(message.cmd === 'load_app_settings' ? '{}' : undefined)
    }
  })
  await page.goto('/#/decompress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const store = (await import(path)).useTaskStore()
    for (const [id, status] of [['done', 'completed'], ['failed', 'failed'], ['cancelled', 'cancelled']]) {
      store.addTask({ id, name: `${id}.zip`, type: 'decompression', sourceFiles: [], outputPath: '' })
      const task = store.tasks.find(t => t.id === id)!
      task.status = status as typeof task.status
      task.progress = status === 'completed' ? 100 : 0
    }
  })
  const label = page.getByTestId('task-summary-label')
  await expect(label).toHaveText('1 项失败')
  await expect(page.getByTestId('summary-failed-icon')).toBeVisible()
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    if (width > 840) await expect(label).toBeVisible()
    else await expect(label).toBeHidden()
    await expect(page.getByTestId('global-progress-bar')).toBeVisible()
    await expect(page.getByTestId('global-progress-summary')).toHaveAttribute('title', '1 项失败')
    await page.waitForTimeout(500) // Capture the settled responsive/sidebar transition.
    await page.screenshot({ path: info.outputPath(`summary-${width}.png`) })
  }
  await page.setViewportSize({ width: 920, height: 620 })
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const store = (await import(path)).useTaskStore()
    store.tasks.find(t => t.id === 'failed')!.status = 'completed'
  })
  await expect(label).toHaveText('1 项取消')
  await expect(page.getByTestId('summary-cancelled-icon')).toBeVisible()
})

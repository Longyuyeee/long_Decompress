import { expect, test } from '@playwright/test'

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    ;(window as any).__resultCalls = []
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let result: unknown
      if (message.cmd === 'load_app_settings') result = '{}'
      if (message.cmd === 'inspect_extraction_destination') result = 'occupied'
      if (message.cmd === 'preview_extraction_output') result = `${message.outputPath || 'D:/输出'}/项目资料`
      if (message.cmd === 'open_in_explorer') (window as any).__resultCalls.push(message.path)
      ;(window as any)[`_${message.callback}`]?.(result)
    }
  })
})

test('batch edits use a dialog while single rows stay inline and other configuration is preserved', async ({ page }, info) => {
  await page.setViewportSize({ width: 1100, height: 760 })
  await page.goto('/#/compress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const modulePath = '/src/stores/compression.ts'
    const store = (await import(modulePath)).useCompressionStore()
    for (let index = 0; index < 3; index++) store.addFile({ name: `项目资料与设计文件_${index}.txt`, path: `C:/项目资料/${index}.txt`, size: 1024, type: 'file', isDirectory: false })
    store.updateFileSettings('C:/项目资料/0.txt', { ...store.globalSettings, level: 2, password: 'first', filename: '我的文件' })
    const group = store.createGroup(['C:/项目资料/1.txt'])
    store.updateGroupSettings(group, { ...store.globalSettings, level: 4, password: 'second' })
    store.groups[0].expanded = false
  })
  await page.getByTestId('compression-group-checkbox').first().click()
  await page.getByTestId('compression-draft-group-checkbox').click()
  const bar = page.getByTestId('batch-config-bar')
  await expect(bar).toContainText('已选 2 项')
  await expect(page.getByRole('dialog')).toHaveCount(0)
  await expect(bar.getByTestId('batch-config-fields')).toHaveCount(0)
  expect(await bar.evaluate(element => element.getBoundingClientRect().height)).toBeLessThan(80)
  await page.getByRole('button', { name: /修改所选配置/ }).click()
  const dialog = page.getByRole('dialog', { name: '修改所选压缩配置' })
  await expect(dialog).toBeVisible()
  await expect(dialog.getByTestId('batch-config-apply')).toBeDisabled()
  await page.getByTestId('batch-level').click()
  await page.getByRole('option', { name: '9 · 最小体积', exact: true }).click()
  await dialog.getByTestId('batch-config-cancel').click()
  await expect(dialog).toHaveCount(0)
  expect(await page.evaluate(async () => {
    const path = '/src/stores/compression.ts'
    return (await import(path)).useCompressionStore().selectedFiles[0].settings?.level
  })).toBe(2)
  await page.getByRole('button', { name: /修改所选配置/ }).click()
  await expect(dialog.getByTestId('batch-level')).toContainText('保持各自设置')
  await page.getByTestId('batch-level').click()
  await page.getByRole('option', { name: '9 · 最小体积', exact: true }).click()
  await dialog.getByTestId('batch-config-apply').click()
  await expect(dialog).toHaveCount(0)
  const state = await page.evaluate(async () => {
    const modulePath = '/src/stores/compression.ts'
    const store = (await import(modulePath)).useCompressionStore()
    return { files: store.selectedFiles, groups: store.groups }
  })
  expect(state.files[0].settings).toMatchObject({ level: 9, password: 'first', filename: '我的文件' })
  expect(state.groups[0].settings).toMatchObject({ level: 9, password: 'second' })
  expect(state.files[1].settings).toBeUndefined()
  await page.getByTestId('compression-draft-row').first().click()
  await expect(page.getByTestId('compression-draft-details')).toHaveCount(1)
  await expect(page.getByRole('dialog')).toHaveCount(0)
  // Viewing the expanded row must not rewrite settings inherited by another row.
  await page.getByTestId('compression-draft-row').last().click()
  expect(await page.evaluate(async () => {
    const modulePath = '/src/stores/compression.ts'
    return (await import(modulePath)).useCompressionStore().selectedFiles[1].settings
  })).toBeUndefined()
  await page.evaluate(async () => {
    const path = '/src/stores/app.ts'
    ;(await import(path)).useAppStore().setSuccess(null)
  })
  for (const width of [1100, 760]) {
    await page.setViewportSize({ width, height: 760 })
    await bar.scrollIntoViewIfNeeded()
    expect(await bar.evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1)
    await page.screenshot({ path: info.outputPath(`compression-batch-${width}.png`) })
    await bar.getByRole('button', { name: /修改所选配置/ }).click()
    await dialog.evaluate(async element => {
      await Promise.all(element.getAnimations({ subtree: true }).filter(animation => animation.effect?.getComputedTiming().iterations !== Infinity).map(animation => animation.finished.catch(() => undefined)))
    })
    await expect(dialog).not.toHaveClass(/fade-enter-active/)
    await expect(dialog.locator('.modal-no-glass')).not.toHaveClass(/pop-enter-active/)
    await page.screenshot({ path: info.outputPath(`compression-batch-dialog-${width}.png`) })
    await dialog.getByTestId('batch-config-cancel').click()
  }
})

test('extraction batch edits use a dialog without filling the list and result folders are opened once', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 700 })
  await page.goto('/#/decompress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const store = (await import(path)).useTaskStore()
    for (let index = 0; index < 3; index++) store.addTask({ id: `selected-${index}`, name: `项目资料_${index}.zip`, type: 'decompression', sourceFiles: [`C:/资料/${index}.zip`], outputPath: `D:/各自目录/${index}`, password: `password-${index}`, extractToSubfolder: true, configurationMode: 'global' })
  })
  await page.getByTestId('task-row').first().locator('input[type="checkbox"], button[role="checkbox"]').first().click()
  await expect(page.getByRole('dialog')).toHaveCount(0)
  expect(await page.getByTestId('batch-config-bar').evaluate(element => element.getBoundingClientRect().height)).toBeLessThan(80)
  await page.getByRole('button', { name: /修改所选配置/ }).click()
  const dialog = page.getByRole('dialog', { name: '修改所选解压配置' })
  await expect(dialog).toBeVisible()
  await page.getByTestId('batch-subfolder').click()
  await page.getByRole('option', { name: '直接放入输出目录', exact: true }).click()
  await page.getByTestId('batch-config-apply').click()
  await expect(dialog).toHaveCount(0)
  expect(await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const task = (await import(path)).useTaskStore().tasks[0]
    return { outputPath: task.outputPath, password: task.password, subfolder: task.extractToSubfolder }
  })).toEqual({ outputPath: 'D:/各自目录/0', password: 'password-0', subfolder: false })
  await page.evaluate(async () => {
    const path = '/src/stores/app.ts'
    ;(await import(path)).useAppStore().setSuccess(null)
  })
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 700 })
    expect(await page.getByTestId('batch-config-bar').evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1)
    await page.screenshot({ path: info.outputPath(`decompression-batch-${width}.png`) })
    await page.getByRole('button', { name: /修改所选配置/ }).click()
    await dialog.evaluate(async element => {
      await Promise.all(element.getAnimations({ subtree: true }).filter(animation => animation.effect?.getComputedTiming().iterations !== Infinity).map(animation => animation.finished.catch(() => undefined)))
    })
    await expect(dialog).not.toHaveClass(/fade-enter-active/)
    await expect(dialog.locator('.modal-no-glass')).not.toHaveClass(/pop-enter-active/)
    await page.screenshot({ path: info.outputPath(`decompression-batch-dialog-${width}.png`) })
    await dialog.getByTestId('batch-config-cancel').click()
  }
  await page.evaluate(async () => {
    const path = '/src/stores/task.ts'
    const tasks = (await import(path)).useTaskStore().tasks
    tasks[0].status = 'completed'; tasks[0].outputPath = 'D:/同一目录'
    tasks[1].status = 'completed'; tasks[1].outputPath = 'd:/同一目录/'
  })
  await page.locator('.progress-summary').click()
  const results = page.getByTestId('batch-result-actions')
  await expect(results).toContainText('打开目录（1）')
  await results.getByRole('button', { name: '打开目录（1）', exact: true }).click()
  expect(await page.evaluate(() => (window as any).__resultCalls)).toEqual(['D:/同一目录'])
  await expect(page.locator('.progress-panel').getByTestId('result-actions')).toHaveCount(2)
  await page.screenshot({ path: info.outputPath('unified-results-760.png') })
})

test('Explorer request opens a password/split draft without starting conversion', async ({ page }, info) => {
  await page.addInitScript(() => {
    let sent = false
    const ipc = window.__TAURI_IPC__
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      if (message.cmd !== 'take_pending_context_actions') { ipc(message); return }
      const result = sent ? [] : [{ action: 'context-reconfigure-archive', files: ['C:/用户资料/原压缩包.zip.001', 'C:/用户资料/另一个压缩包.7z'] }]
      sent = true
      ;(window as any)[`_${message.callback}`]?.(result)
    }
  })
  await page.setViewportSize({ width: 760, height: 760 })
  await page.goto('/')
  await expect(page.getByRole('dialog')).toBeVisible()
  await expect(page.getByTestId('conversion-context-source')).toContainText('原压缩包.zip.001')
  await expect(page.getByTestId('conversion-format')).toContainText('ZIP')
  await expect(page.getByTestId('conversion-context-source')).toBeVisible()
  await expect(page.getByTestId('conversion-source-password')).toHaveValue('')
  await expect(page.getByTestId('conversion-password')).toHaveValue('')
  await expect(page.getByTestId('conversion-output')).toHaveValue('')
  expect(await page.evaluate(async () => { const path = '/src/stores/task.ts'; return (await import(path)).useTaskStore().tasks.length })).toBe(0)
  await page.getByRole('dialog').evaluate(async element => {
    const animations = element.getAnimations({ subtree: true }).filter(animation => animation.effect?.getComputedTiming().iterations !== Infinity)
    await Promise.all(animations.map(animation => animation.finished.catch(() => undefined)))
  })
  await page.screenshot({ path: info.outputPath('explorer-conversion-760.png') })
})

import { expect, test } from '@playwright/test'
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    ;(window as any).__conversionCalls = []
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      let result: unknown
      if (message.cmd === 'load_app_settings') result = '{}'
      if (message.cmd === 'get_file_info') result = { name: message.path.split(/[\\/]/).pop(), size: 1024, is_dir: false }
      if (message.cmd === 'check_rar_compression_support') result = { available: true }
      if (message.cmd === 'detect_split_archive' && /卷组/.test(message.path)) result = {
        is_split: true, archive_format: 'rar', first_part: 'C:/用户资料/卷组.part01.rar',
        parts: ['C:/用户资料/卷组.part01.rar', 'C:/用户资料/卷组.part02.rar'], is_complete: true, missing_parts: [],
      }
      if (message.cmd === 'convert_archive') {
        ;(window as any).__conversionCalls.push({ source: message.sourcePath, output: message.outputPath })
        result = { primaryPath: message.outputPath, files: [{ path: message.outputPath, bytes: 1024 }], totalBytes: 1024, split: false }
      }
      ;(window as any)[`_${message.callback}`]?.(result)
    }
  })
})
test('RAR groups retain inline configuration and new TAR codecs work in the batch dialog', async ({ page }, info) => {
  await page.setViewportSize({ width: 1100, height: 820 })
  await page.goto('/#/compress'); await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const path = '/src/stores/archiveConversion.ts', store = (await import(path)).useArchiveConversionStore()
    store.openFromExplorer(['C:/用户资料/卷组.part02.rar', 'C:/用户资料/卷组.part01.rar', 'C:/用户资料/普通资料.zip'], 'reconfigure')
  })
  const dialog = page.getByRole('dialog', { name: '压缩包转换', exact: true })
  await expect(dialog.getByTestId('conversion-row')).toHaveCount(2)
  await dialog.getByTestId('conversion-row').first().click()
  const details = dialog.getByTestId('conversion-details').first()
  await expect(details).toContainText('已检测到 RAR 编码器')
  await expect(details.getByTestId('conversion-split')).toBeEnabled()
  await expect(details.getByTestId('conversion-hidden-names')).toBeVisible()
  await expect(page.getByRole('dialog')).toHaveCount(1)
  await page.screenshot({ path: info.outputPath('rar-inline-volume-settings.png') })
  await page.getByTestId('conversion-batch-config').click()
  const batch = page.getByRole('dialog', { name: '修改所选转换配置', exact: true })
  await batch.getByTestId('conversion-batch-format').click()
  await page.getByRole('option', { name: 'TAR.BR（Brotli）', exact: true }).click()
  await batch.getByTestId('conversion-batch-apply').click()
  await expect(batch).toHaveCount(0)
  await expect(details.getByTestId('conversion-format')).toContainText('TAR.BR')
  await expect(details.getByTestId('conversion-password')).toBeDisabled()
  await expect(details.getByTestId('conversion-split')).toBeEnabled()
  await expect(details.getByTestId('conversion-output')).toHaveValue('C:/用户资料/卷组_调整.tar.br')
  await page.setViewportSize({ width: 760, height: 820 })
  expect(await dialog.locator('.modal-no-glass').evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1)
  await page.screenshot({ path: info.outputPath('tar-brotli-inline-settings-760.png') })
  await dialog.getByTestId('conversion-start').click()
  await expect(page.getByTestId('batch-result-actions')).toContainText('已完成结果 · 2 项')
  expect(await page.evaluate(() => (window as any).__conversionCalls[0])).toEqual({ source: 'C:/用户资料/卷组.part01.rar', output: 'C:/用户资料/卷组_调整.tar.br' })
})
test('conversion rows expand inline, batch edits are modal, then selected items generate separate results', async ({ page }, info) => {
  await page.setViewportSize({ width: 1100, height: 780 })
  await page.goto('/#/compress'); await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const path = '/src/stores/archiveConversion.ts', store = (await import(path)).useArchiveConversionStore()
    store.openFromExplorer(['C:/用户资料/设计稿与项目文档.zip', 'C:/用户资料/发布资源.zip', 'C:/用户资料/备用文件.zip'], 'convert')
    store.items[2].selected = false
  })
  const dialog = page.getByRole('dialog', { name: '压缩包转换', exact: true })
  await expect(page.getByTestId('conversion-details')).toHaveCount(0)
  await page.getByTestId('conversion-row').first().click()
  await expect(page.getByTestId('conversion-details')).toHaveCount(1)
  await expect(page.getByRole('dialog')).toHaveCount(1)
  await page.getByTestId('conversion-output').fill('D:/成品/单独名称.7z')
  await page.getByTestId('conversion-batch-config').click()
  const batch = page.getByRole('dialog', { name: '修改所选转换配置' })
  await expect(batch).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(batch).toHaveCount(0)
  await expect(dialog).toBeVisible()
  await page.getByTestId('conversion-batch-config').click()
  await page.getByTestId('conversion-batch-level').click(); await page.getByRole('option', { name: '9', exact: true }).click()
  await page.setViewportSize({ width: 760, height: 780 })
  await expect(batch).not.toHaveClass(/fade-enter-active/)
  await expect(batch.locator('.modal-no-glass')).not.toHaveClass(/pop-enter-active/)
  await page.screenshot({ path: info.outputPath('conversion-batch-dialog-760.png') })
  await batch.getByRole('button', { name: '取消', exact: true }).click()
  await page.getByTestId('conversion-batch-config').click()
  await expect(page.getByTestId('conversion-batch-level')).toContainText('保持各自设置')
  await page.getByTestId('conversion-batch-level').click(); await page.getByRole('option', { name: '9', exact: true }).click()
  await page.getByTestId('conversion-batch-apply').click()
  await expect(batch).toHaveCount(0)
  await expect(page.getByTestId('conversion-output')).toHaveValue('D:/成品/单独名称.7z')
  for (const width of [1100, 760]) {
    await page.setViewportSize({ width, height: 780 })
    await expect(dialog).not.toHaveClass(/fade-enter-active/)
    await expect(dialog.locator('.modal-no-glass')).not.toHaveClass(/pop-enter-active/)
    expect(await dialog.locator('.modal-no-glass').evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1)
    await page.screenshot({ path: info.outputPath('conversion-' + width + '.png') })
  }
  await page.getByTestId('conversion-start').click()
  await expect(page.getByTestId('batch-result-actions')).toContainText('已完成结果 · 2 项')
  expect(await page.evaluate(() => (window as any).__conversionCalls)).toEqual([
    { source: 'C:/用户资料/设计稿与项目文档.zip', output: 'D:/成品/单独名称.7z' },
    { source: 'C:/用户资料/发布资源.zip', output: 'C:/用户资料/发布资源_转换.7z' },
  ])
  await dialog.getByRole('button', { name: '关闭', exact: true }).click()
})
test('pending work survives reload and requires an explicit restore without starting any job', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 760 })
  await page.goto('/#/compress'); await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const compPath = '/src/stores/compression.ts', taskPath = '/src/stores/task.ts', convPath = '/src/stores/archiveConversion.ts', draftsPath = '/src/stores/pendingDrafts.ts'
    const compression = (await import(compPath)).useCompressionStore()
    compression.addFile({ path: 'C:/项目/设计资料.txt', name: '设计资料.txt', size: 1024, type: 'file', isDirectory: false })
    compression.updateFileSettings('C:/项目/设计资料.txt', { ...compression.globalSettings, password: 'temporary-secret', filename: '归档资料' })
    ;(await import(taskPath)).useTaskStore().addTask({ id: 'pending-extract', name: '待解压资源.zip', type: 'decompression', sourceFiles: ['C:/项目/待解压资源.zip'], outputPath: 'D:/我的结果', extractToSubfolder: true, configurationMode: 'individual' })
    ;(await import(convPath)).useArchiveConversionStore().addSources(['C:/项目/转换资源.zip'])
    ;(await import(draftsPath)).usePendingDraftStore().flush()
  })
  await page.reload(); await page.waitForSelector('h1')
  const recovery = page.getByTestId('pending-draft-recovery')
  await expect(recovery).toContainText('上次还有 3 项待处理任务')
  await expect(page.getByTestId('compression-draft-row')).toHaveCount(0)
  await recovery.getByRole('button', { name: '恢复任务', exact: true }).click()
  await expect(page.getByTestId('compression-draft-row')).toHaveCount(1)
  await expect(page.getByTestId('global-progress-summary')).toContainText('等待中')
  await page.getByTestId('compression-draft-row').click()
  await expect(page.getByTestId('draft-recovery-notice')).toContainText('此任务使用过密码')
  expect(await page.evaluate(() => (window as any).__conversionCalls)).toEqual([])
  expect(await page.evaluate(() => localStorage.getItem('long-pending-archive-drafts-v1'))).not.toContain('temporary-secret')
  await page.getByTestId('draft-recovery-notice').getByRole('textbox').fill('new-password')
  await page.getByRole('button', { name: '确认密码设置' }).click()
  await expect(page.getByTestId('draft-recovery-notice')).toHaveCount(0)
  await page.screenshot({ path: info.outputPath('draft-recovery.png') })
})

test('conversion recovery asks for the source and new package passwords independently', async ({ page }, info) => {
  await page.setViewportSize({ width: 760, height: 780 })
  await page.goto('/#/compress'); await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const conversionPath = '/src/stores/archiveConversion.ts', draftsPath = '/src/stores/pendingDrafts.ts'
    const conversion = (await import(conversionPath)).useArchiveConversionStore()
    conversion.addSources(['C:/用户资料/加密资料.zip'])
    conversion.sourcePassword = 'source-before-restart'; conversion.password = 'target-before-restart'
    ;(await import(draftsPath)).usePendingDraftStore().flush()
  })
  await page.reload()
  await page.getByRole('button', { name: '恢复任务', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: '压缩包转换', exact: true })
  await expect(dialog).toBeVisible()
  await page.getByTestId('conversion-row').click()
  await expect(page.getByTestId('conversion-no-source-password')).toBeVisible()
  await expect(page.getByTestId('conversion-no-target-password')).toBeVisible()
  await page.getByTestId('conversion-source-password').fill('source-again')
  await expect(page.getByTestId('conversion-no-source-password')).toHaveCount(0)
  await expect(page.getByTestId('conversion-no-target-password')).toBeVisible()
  await page.getByTestId('conversion-start').click()
  await expect(dialog.getByRole('alert').first()).toContainText('新包密码')
  expect(await page.evaluate(() => (window as any).__conversionCalls)).toEqual([])
  await expect(dialog).not.toHaveClass(/fade-enter-active/)
  await expect(dialog.locator('.modal-no-glass')).not.toHaveClass(/pop-enter-active/)
  await page.screenshot({ path: info.outputPath('conversion-password-recovery-760.png') })
  await page.getByTestId('conversion-password').fill('target-again')
  await page.getByTestId('conversion-start').click()
  await expect(page.getByTestId('batch-result-actions')).toContainText('已完成结果 · 1 项')
})

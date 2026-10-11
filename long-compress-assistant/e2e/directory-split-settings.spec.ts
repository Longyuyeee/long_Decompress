import { test, expect } from '@playwright/test'

test('directory ZIP splitting preserves password settings while unsupported formats stay disabled', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 620 })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      const result = message.cmd === 'load_app_settings' ? '{}' : undefined
      ;(window as any)[`_${message.callback}`]?.(result)
    }
  })
  await page.goto('/#/compress')
  await page.waitForSelector('h1')
  await page.evaluate(async () => {
    const path = '/src/stores/compression.ts'
    const store = (await import(path)).useCompressionStore()
    store.addFile({ name: '旅行照片', path: 'C:/旅行照片', size: 123, type: 'directory', isDirectory: true })
  })
  await page.getByRole('button', { name: '展开压缩详情', exact: true }).click()
  const drawer = page.locator('.details-drawer').first()
  const toggle = drawer.getByTestId('compression-split-toggle')
  await expect(toggle).toBeEnabled()
  await toggle.check()
  await drawer.getByTestId('compression-split-size').fill('128')
  await drawer.getByTestId('compression-split-size').blur()
  await drawer.getByTestId('compression-password-input').fill('中文密码 + !测试')
  await expect(toggle).toBeEnabled()
  for (const width of [920, 760]) {
    await page.setViewportSize({ width, height: 620 })
    await toggle.scrollIntoViewIfNeeded()
    await expect(toggle).toBeChecked()
    await expect(drawer.getByTestId('compression-split-size')).toHaveValue('128')
    expect(await drawer.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
    await page.screenshot({ path: info.outputPath(`directory-split-${width}.png`) })
  }
  await expect(drawer.getByTestId('compression-password-input')).toHaveValue('中文密码 + !测试')
  expect(await page.evaluate(async () => {
    const path = '/src/stores/compression.ts'
    const store = (await import(path)).useCompressionStore()
    return store.getEffectiveSettings(store.selectedFiles[0].settings).splitArchive
  })).toBe(true)
  await drawer.getByTestId('compression-password-input').fill('')
  await expect(toggle).toBeEnabled()
  await expect(toggle).toBeChecked()
  await drawer.getByTestId('compression-format-select').click()
  await page.getByRole('option', { name: '7Z', exact: true }).click()
  await expect(toggle).toBeDisabled()
  await expect(toggle).not.toBeChecked()
  await drawer.getByRole('button', { name: '分卷压缩说明', exact: true }).click()
  await expect(page.getByRole('note')).toContainText('暂不支持创建分卷')
})

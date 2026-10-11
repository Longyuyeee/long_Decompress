import { test, expect } from '@playwright/test'

test('ZIP password compatibility is explicit and fits the default window', async ({ page }, info) => {
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
    store.addFile({ name: '资料.txt', path: 'C:/资料.txt', size: 123, type: 'file', isDirectory: false })
  })
  await page.getByRole('button', { name: '展开压缩详情', exact: true }).click()
  const drawer = page.locator('.details-drawer').first()
  await expect(drawer.getByTestId('zip-password-encoding')).toHaveCount(0)
  await drawer.getByTestId('compression-password-input').fill('密码 + !测试')
  const selector = drawer.getByTestId('zip-password-encoding')
  await expect(selector).toContainText('UTF-8')
  await selector.click()
  await page.getByRole('option', { name: '密码编码：Windows 兼容', exact: true }).click()
  await expect(selector).toContainText('Windows 兼容')
  await selector.scrollIntoViewIfNeeded()
  await page.waitForTimeout(350)
  await page.screenshot({ path: info.outputPath('zip-encoding-920.png') })
  await drawer.getByRole('button', { name: 'ZIP 密码编码说明', exact: true }).click()
  await expect(page.getByRole('note')).toContainText('跨语言系统')
  await page.screenshot({ path: info.outputPath('zip-encoding-help.png') })
  await page.keyboard.press('Escape')
  const format = drawer.getByTestId('compression-format-select')
  await format.click()
  await page.getByRole('option', { name: '7Z', exact: true }).click()
  await expect(selector).toHaveCount(0)
  await format.click()
  await page.getByRole('option', { name: 'ZIP', exact: true }).click()
  await expect(selector).toContainText('Windows 兼容')
  await page.setViewportSize({ width: 760, height: 620 })
  await selector.scrollIntoViewIfNeeded()
  await page.waitForTimeout(350)
  await page.screenshot({ path: info.outputPath('zip-encoding-760.png') })
  expect(await drawer.getByTestId('zip-password-encoding-row').evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
})

import { expect, test } from '@playwright/test'

test.describe('Long Decompress desktop shell', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForURL('**/#/decompress')
  })

  test('opens the decompression workspace by default', async ({ page }) => {
    await expect(page.locator('main h1')).toBeVisible()
    await expect(page.locator('[role="button"][tabindex="0"]').first()).toBeVisible()
  })

  test('renders five keyboard-accessible navigation buttons', async ({ page }) => {
    const navigation = page.locator('aside nav > button')
    await expect(navigation).toHaveCount(5)
    await expect(navigation.first()).toHaveAttribute('aria-current', 'page')
  })

  test('navigates to settings from the sidebar', async ({ page }) => {
    await page.locator('aside nav > button').nth(4).click()
    await page.waitForURL('**/#/settings')
    await expect(page.locator('main h1')).toBeVisible()
  })

  test('supports the file-integrity keyboard shortcut', async ({ page }) => {
    await page.keyboard.press('Control+i')
    await page.waitForURL('**/#/integrity')
    await expect(page.locator('main h1')).toBeVisible()
  })

  test('exposes the archive dropzone to assistive technology', async ({ page }) => {
    const dropzone = page.locator('main [role="button"][tabindex="0"]').first()
    await expect(dropzone).toHaveAttribute('aria-label', /.+/)
    await dropzone.focus()
    await expect(dropzone).toBeFocused()
  })
})

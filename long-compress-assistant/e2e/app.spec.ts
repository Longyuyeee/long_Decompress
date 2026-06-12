import { test, expect } from '@playwright/test'

test.describe('LongDecompress App', () => {
  test('has window title', async ({ page }) => {
    await page.goto('/')
    const title = await page.title()
    expect(title).toContain('胧压缩')
  })

  test('navigates to decompress by default', async ({ page }) => {
    await page.goto('/')
    // Hash router should show /decompress
    await page.waitForURL('**/decompress')
    expect(page.url()).toContain('decompress')
  })

  test('sidebar navigation exists', async ({ page }) => {
    await page.goto('/')
    // Should have navigation links
    const nav = page.locator('nav a, [role="navigation"] a')
    expect(await nav.count()).toBeGreaterThan(0)
  })

  test('can navigate to settings', async ({ page }) => {
    await page.goto('/')
    await page.goto('/#/settings')
    await page.waitForLoadState('networkidle')
    expect(page.url()).toContain('settings')
  })
})

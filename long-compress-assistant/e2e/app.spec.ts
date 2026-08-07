import { expect, test } from '@playwright/test'

const responsiveWidths = [1440, 1024, 760, 390]

const expectVerticalOnlyScrolling = async (
  page: import('@playwright/test').Page,
  selectors: string[],
) => {
  const measurements = await page.locator(selectors.join(',')).evaluateAll(elements =>
    elements.map(element => {
      const htmlElement = element as HTMLElement
      const style = getComputedStyle(htmlElement)
      return {
        selector: htmlElement.className || htmlElement.dataset.testid || htmlElement.tagName,
        clientWidth: htmlElement.clientWidth,
        scrollWidth: htmlElement.scrollWidth,
        overflowX: style.overflowX,
      }
    }),
  )

  expect(measurements.length).toBeGreaterThanOrEqual(selectors.length)
  for (const measurement of measurements) {
    // A few pixels of intrinsic text/border rounding are harmless once overflow-x is
    // explicitly hidden; a materially wider child still indicates a broken layout.
    expect(measurement.scrollWidth, `${measurement.selector} overflowed horizontally`).toBeLessThanOrEqual(
      Math.max(measurement.clientWidth + 8, measurement.clientWidth * 1.15),
    )
    expect(measurement.overflowX).toBe('hidden')
  }
}

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

  test('keeps compression and decompression details free of horizontal scrolling', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'responsive overflow matrix runs once in Chromium')

    await page.waitForFunction(() => Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__))
    await page.evaluate(() => window.__LONG_DECOMPRESS_DESKTOP_E2E__!.seedResponsiveWorkspace('compression'))
    await page.locator('aside nav > button').nth(1).click()
    await page.waitForURL('**/#/compress')
    await expect(page.getByTestId('compression-draft-details')).toBeVisible()

    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, [
        '.compression-view',
        '.compression-task-list',
        '.compression-detail-card',
        '.compression-config-panel',
        '[data-testid="compression-draft-execution"]',
        '.pending-log',
      ])
      await expect(page.locator('.compression-config-panel')).toHaveCSS('pointer-events', 'auto')
    }

    await page.evaluate(() => window.__LONG_DECOMPRESS_DESKTOP_E2E__!.seedResponsiveWorkspace('decompression'))
    await page.locator('aside nav > button').first().click()
    await page.waitForURL('**/#/decompress')
    await page.locator('.task-row').click()
    await expect(page.locator('.task-detail-card')).toBeVisible()

    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, [
        '.decompress-view',
        '.aero-table-container',
        '.table-body',
        '.task-detail-card',
        '.task-config-panel',
        '.task-execution-panel',
        '.log-viewport',
      ])
    }
  })
})

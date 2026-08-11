import { expect, test } from '@playwright/test'

const responsiveWidths = [1440, 1024, 760, 390]

const expectVerticalOnlyScrolling = async (
  page: import('@playwright/test').Page,
  selectors: string[],
) => {
  await expect.poll(async () => {
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

    return {
      foundEverySelector: measurements.length >= selectors.length,
      violations: measurements.flatMap(measurement => {
        const maximumWidth = Math.max(measurement.clientWidth + 8, measurement.clientWidth * 1.15)
        const issues: string[] = []
        if (measurement.scrollWidth > maximumWidth) {
          issues.push(
            `${measurement.selector}: scrollWidth=${measurement.scrollWidth}, clientWidth=${measurement.clientWidth}`,
          )
        }
        if (measurement.overflowX !== 'hidden') {
          issues.push(`${measurement.selector}: overflow-x=${measurement.overflowX}`)
        }
        return issues
      }),
    }
  }, {
    message: `responsive layout did not settle without horizontal overflow: ${selectors.join(', ')}`,
    timeout: 3_000,
  }).toEqual({
    foundEverySelector: true,
    violations: [],
  })
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

  test('renders six keyboard-accessible navigation buttons', async ({ page }) => {
    const navigation = page.locator('aside nav > button')
    await expect(navigation).toHaveCount(6)
    await expect(navigation.first()).toHaveAttribute('aria-current', 'page')
  })

  test('navigates to settings from the sidebar', async ({ page }) => {
    await page.getByTestId('nav-Settings').click()
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
    await page.getByTestId('nav-Compress').click()
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
    await page.getByTestId('nav-Decompress').click()
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

  test('keeps archive diagnostics vertically scrollable at narrow widths', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'responsive diagnostics matrix runs once in Chromium')
    await page.keyboard.press('Control+i')
    await page.waitForURL('**/#/integrity')
    await page.getByTestId('archive-diagnostic-mode').click()
    await expect(page.getByTestId('archive-diagnostic-panel')).toBeVisible()

    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, [
        '.integrity-view',
        '.integrity-scroll',
        '[data-testid="archive-diagnostic-panel"]',
      ])
    }
  })
})

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

  test('previews task templates without executing and keeps the audit modal responsive', async ({ page }, testInfo) => {
    test.skip(testInfo.project.name !== 'chromium', 'task-template audit matrix runs once in desktop Chromium')
    await page.addInitScript(() => {
      window.__TAURI_IPC__ = (message: Record<string, any>) => {
        let value: unknown
        if (message.cmd === 'tauri' && message.message?.cmd === 'openDialog') {
          value = 'C:/templates/logs.longtask.json'
        } else if (message.cmd === 'get_compression_profiles') {
          value = []
        } else if (message.cmd === 'get_archive_engine_capabilities') {
          value = {
            available: true,
            fullEngine: true,
            message: 'ready',
            formats: [{ name: '7z', extensions: ['7z'], canCreate: true }],
          }
        } else if (message.cmd === 'preview_task_template') {
          value = {
            template: {
              schema: 'long-decompress-task-template', version: 1, name: '日志归档', icon: '📦', description: '安全归档日志',
              sourceRules: { mode: 'pattern', includePatterns: ['*.log'], sizeRangeMib: null },
              targetRule: { mode: 'choose_at_runtime', filenameTemplate: '{name}-{date}' },
              compression: { format: '7z', level: 7, splitArchive: false, splitSizeMib: null, keepStructure: true, verifyAfter: true, createSolidArchive: true },
              passwordStrategy: { mode: 'prompt_at_runtime' }, exportNotes: [],
            },
            warnings: ['自动应用保持关闭'],
            contentSha256: 'a'.repeat(64),
          }
        } else if (message.cmd === 'load_app_settings') {
          value = '{}'
        }
        ;(window as any)[`_${message.callback}`]?.(value)
      }
    })
    await page.reload()
    await page.waitForFunction(() => Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__))
    await page.evaluate(() => window.__LONG_DECOMPRESS_DESKTOP_E2E__!.seedResponsiveWorkspace('compression'))
    await page.getByRole('button', { name: /压缩中心/ }).click()
    await page.getByRole('button', { name: /全局设置/ }).click()
    await page.getByLabel('全局压缩设置').getByRole('button', { name: /管理配置组/ }).click()
    await page.getByTestId('import-task-template').click()

    const preview = page.getByTestId('task-template-preview')
    await expect(preview).toBeVisible()
    await expect(preview).toContainText('导入不会启动压缩')
    await expect(preview).toContainText('确认导入配置组（不执行）')

    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, ['[data-testid="task-template-preview"]'])
    }
  })

  test('renders bounded archive image preview without horizontal overflow', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'archive preview matrix runs once in Chromium')
    await page.addInitScript(() => {
      const png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg=='
      window.__TAURI_IPC__ = (message: Record<string, any>) => {
        let value: unknown
        if (message.cmd === 'tauri' && message.message?.cmd === 'openDialog') {
          value = 'C:/archives/images.zip'
        } else if (message.cmd === 'get_file_info') {
          value = { name: 'images.zip', size: 128, is_dir: false, modified: 0 }
        } else if (message.cmd === 'browse_archive') {
          value = {
            format: 'ZIP', totalFiles: 1, totalDirectories: 0,
            totalUncompressedSize: 68, totalCompressedSize: 60, encrypted: false,
            entries: [{ path: 'art/preview.png', name: 'preview.png', size: 68, compressedSize: 60, modified: null, crc: '12345678', encrypted: false, isDir: false }],
          }
        } else if (message.cmd === 'preview_archive_image') {
          value = { entryPath: 'art/preview.png', mimeType: 'image/png', dataUrl: png, byteSize: 68, width: 1, height: 1 }
        } else if (message.cmd === 'load_app_settings') {
          value = '{}'
        }
        ;(window as any)[`_${message.callback}`]?.(value)
      }
    })
    await page.reload()
    await page.keyboard.press('Control+b')
    await page.waitForURL('**/#/browser')
    await page.locator('header .browser-primary').click()
    await expect(page.getByText('preview.png').first()).toBeVisible()
    await page.getByRole('button', { name: '预览 preview.png' }).click()
    const preview = page.getByTestId('archive-image-preview')
    await expect(preview.getByRole('img', { name: 'preview.png' })).toBeVisible()
    await expect(preview).toContainText('只读 · 未写入磁盘')

    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, [
        '.browser-page',
        '[data-testid="archive-image-preview"]',
        '.preview-dialog',
      ])
    }
  })
})

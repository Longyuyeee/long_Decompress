import { expect, test } from '@playwright/test'

const responsiveWidths = [1440, 1024, 760, 390]
const desktopDetailViewports = [
  { width: 1440, height: 900 },
  { width: 1024, height: 720 },
  { width: 920, height: 620 },
  { width: 760, height: 520 },
]

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

const expectSideBySide = async (
  page: import('@playwright/test').Page,
  leftSelector: string,
  rightSelector: string,
) => {
  const positions = await page.locator(`${leftSelector}, ${rightSelector}`).evaluateAll(elements =>
    elements.map(element => {
      const rect = element.getBoundingClientRect()
      return { left: rect.left, top: rect.top, width: rect.width }
    }),
  )

  expect(positions).toHaveLength(2)
  expect(positions[0].width).toBeGreaterThan(0)
  expect(positions[1].width).toBeGreaterThan(0)
  expect(Math.abs(positions[0].top - positions[1].top)).toBeLessThanOrEqual(2)
  expect(positions[1].left).toBeGreaterThan(positions[0].left)
}

const expectBoundedDetailPanels = async (
  page: import('@playwright/test').Page,
  selectors: { detail: string; config: string; execution: string; log: string },
) => {
  const measurements = await page.locator([
    selectors.detail,
    selectors.config,
    selectors.execution,
    selectors.log,
  ].join(',')).evaluateAll(elements => elements.map(element => {
    const htmlElement = element as HTMLElement
    const rect = htmlElement.getBoundingClientRect()
    const style = getComputedStyle(htmlElement)
    return {
      width: rect.width,
      height: rect.height,
      clientHeight: htmlElement.clientHeight,
      scrollHeight: htmlElement.scrollHeight,
      overflowY: style.overflowY,
    }
  }))

  expect(measurements).toHaveLength(4)
  const [detail, config, execution, log] = measurements
  expect(detail.height).toBeGreaterThanOrEqual(340)
  expect(config.width).toBeGreaterThanOrEqual(180)
  expect(execution.width).toBeGreaterThanOrEqual(180)
  expect(Math.abs(config.height - execution.height)).toBeLessThanOrEqual(2)
  expect(config.overflowY).toBe('auto')
  expect(execution.overflowY).toBe('hidden')
  expect(log.overflowY).toBe('auto')
  expect(log.scrollHeight).toBeGreaterThan(log.clientHeight)

  const resourceMetrics = page.getByTestId('resource-preflight-metrics')
  await expect(resourceMetrics).toBeVisible()
  await expect(resourceMetrics.locator('.metric')).toHaveCount(4)
  const resourceCard = await page.getByTestId('resource-preflight-card').boundingBox()
  expect(resourceCard?.height).toBeGreaterThanOrEqual(100)
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

  test('renders eight keyboard-accessible navigation buttons including special compression', async ({ page }) => {
    const navigation = page.locator('aside nav > button')
    await expect(navigation).toHaveCount(8)
    await expect(navigation.first()).toHaveAttribute('aria-current', 'page')
    await expect(page.getByTestId('nav-SpecialCompression')).toHaveAccessibleName(/特殊压缩.*Ctrl\+Shift\+S/)
  })

  test('opens the compact special-compression workspace with its tabs in the title row', async ({ page }) => {
    await page.keyboard.press('Control+Shift+s')
    await page.waitForURL('**/#/special-compression')
    const header = page.locator('.special-compression-header')
    const heading = page.locator('.special-compression-heading')
    const tabs = page.getByTestId('special-compression-mode-switch')
    const [headerBox, headingBox, tabsBox] = await Promise.all([header.boundingBox(), heading.boundingBox(), tabs.boundingBox()])
    expect(headerBox).not.toBeNull()
    expect(headingBox).not.toBeNull()
    expect(tabsBox).not.toBeNull()
    expect(tabsBox!.x).toBeGreaterThan(headingBox!.x)
    expect(tabsBox!.y).toBeGreaterThanOrEqual(headerBox!.y - 1)
    expect(tabsBox!.y + tabsBox!.height).toBeLessThanOrEqual(headerBox!.y + headerBox!.height + 1)
    await expect(page.getByTestId('image-toggle-global-settings')).toHaveAttribute('aria-expanded', 'false')
    await expectVerticalOnlyScrolling(page, ['.special-compression-view', '[data-testid="image-compression-workspace"]'])
    await page.getByTestId('compression-mode-video').click()
    await expect(page.getByTestId('video-toggle-global-settings')).toHaveAttribute('aria-expanded', 'false')
    await expectVerticalOnlyScrolling(page, ['[data-testid="video-compression-workspace"]'])
    await page.getByTestId('compression-mode-pdf').click()
    await expectVerticalOnlyScrolling(page, ['[data-testid="pdf-compression-workspace"]'])
  })

  test('keeps the special-compression shell fixed while batch settings use a modal', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'special-compression geometry matrix runs once in Chromium')
    const viewports = [
      { width: 1366, height: 768 },
      { width: 1024, height: 768 },
      { width: 760, height: 520 },
    ]
    await page.keyboard.press('Control+Shift+s')
    await page.waitForURL('**/#/special-compression')

    for (const viewport of viewports) {
      await page.setViewportSize(viewport)
      for (const mode of ['image', 'video'] as const) {
        await page.getByTestId(`compression-mode-${mode}`).click()
        const workspace = page.getByTestId(`${mode}-compression-workspace`)
        await expect(workspace).toBeVisible()
        await page.locator('.special-compression-stage').evaluate(async element => {
          await Promise.all(element.getAnimations().map(animation => animation.finished))
        })
        const shell = page.locator('.special-compression-shell')
        const before = await shell.boundingBox()
        const pageBefore = await page.evaluate(() => ({
          clientHeight: document.documentElement.clientHeight,
          scrollHeight: document.documentElement.scrollHeight,
        }))

        await page.getByTestId(`${mode}-toggle-global-settings`).click()
        await expect(page.getByRole('dialog')).toBeVisible()
        const after = await shell.boundingBox()
        const pageAfter = await page.evaluate(() => ({
          clientHeight: document.documentElement.clientHeight,
          scrollHeight: document.documentElement.scrollHeight,
        }))
        expect(before).not.toBeNull()
        expect(after).not.toBeNull()
        expect(Math.abs(after!.width - before!.width)).toBeLessThanOrEqual(0.1)
        expect(Math.abs(after!.height - before!.height)).toBeLessThanOrEqual(0.1)
        expect(pageAfter.scrollHeight).toBe(pageBefore.scrollHeight)
        expect(pageAfter.scrollHeight).toBeLessThanOrEqual(pageAfter.clientHeight)
        await page.keyboard.press('Escape')
        await expect(page.getByRole('dialog')).toBeHidden()
      }
    }
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

  test('keeps compression and decompression details free of horizontal scrolling', async ({ page, browserName }, testInfo) => {
    test.skip(browserName !== 'chromium', 'responsive overflow matrix runs once in Chromium')
    test.slow()

    await page.waitForFunction(() => Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__))
    await page.evaluate(() => window.__LONG_DECOMPRESS_DESKTOP_E2E__!.seedResponsiveWorkspace('compression'))
    await page.getByTestId('nav-Compress').click()
    await page.waitForURL('**/#/compress')
    await expect(page.getByTestId('compression-draft-details')).toBeVisible()

    for (const viewport of desktopDetailViewports) {
      await page.setViewportSize(viewport)
      await expectVerticalOnlyScrolling(page, [
        '.compression-view',
        '.compression-task-list',
        '.compression-detail-card',
        '.compression-config-panel',
        '[data-testid="compression-draft-execution"]',
        '.pending-log',
      ])
      await expectSideBySide(
        page,
        '.compression-config-panel',
        '[data-testid="compression-draft-execution"]',
      )
      await expectBoundedDetailPanels(page, {
        detail: '[data-testid="compression-draft-details"]',
        config: '[data-testid="compression-draft-config"]',
        execution: '[data-testid="compression-draft-execution"]',
        log: '[data-testid="compression-log-viewport"]',
      })
      await expect(page.locator('.compression-config-panel')).toHaveCSS('pointer-events', 'auto')
      if (viewport.width === 1024 || viewport.width === 920 || viewport.width === 760) {
        await page.screenshot({ path: testInfo.outputPath(`compression-${viewport.width}x${viewport.height}.png`), fullPage: false })
      }
      if (viewport.width === 760) {
        const scrollTop = await page.getByTestId('compression-draft-config').evaluate(element => {
          element.scrollTop = element.scrollHeight
          return element.scrollTop
        })
        expect(scrollTop).toBeGreaterThan(0)
        await page.waitForTimeout(100)
        await page.screenshot({ path: testInfo.outputPath('compression-resource-760x520.png'), fullPage: false })
      }
    }

    await page.evaluate(() => window.__LONG_DECOMPRESS_DESKTOP_E2E__!.seedResponsiveWorkspace('decompression'))
    await page.getByTestId('nav-Decompress').click()
    await page.waitForURL('**/#/decompress')
    await page.locator('.task-row').click()
    await expect(page.locator('.task-detail-card')).toBeVisible()

    for (const viewport of desktopDetailViewports) {
      await page.setViewportSize(viewport)
      await expectVerticalOnlyScrolling(page, [
        '.decompress-view',
        '.aero-table-container',
        '.table-body',
        '.task-detail-card',
        '.task-config-panel',
        '.task-execution-panel',
        '.log-viewport',
      ])
      await expectSideBySide(page, '.task-config-panel', '.task-execution-panel')
      await expectBoundedDetailPanels(page, {
        detail: '[data-testid="decompression-task-details"]',
        config: '[data-testid="decompression-config-panel"]',
        execution: '[data-testid="decompression-execution-panel"]',
        log: '[data-testid="decompression-log-viewport"]',
      })
      if (viewport.width === 1024 || viewport.width === 920 || viewport.width === 760) {
        await page.screenshot({ path: testInfo.outputPath(`decompression-${viewport.width}x${viewport.height}.png`), fullPage: false })
      }
      if (viewport.width === 760) {
        const scrollTop = await page.getByTestId('decompression-config-panel').evaluate(element => {
          element.scrollTop = element.scrollHeight
          return element.scrollTop
        })
        expect(scrollTop).toBeGreaterThan(0)
        await page.waitForTimeout(100)
        await page.screenshot({ path: testInfo.outputPath('decompression-resource-760x520.png'), fullPage: false })
      }
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
    test.slow()
    await page.addInitScript(() => {
      let openDialogCount = 0
      ;(window as any).__TASK_TEMPLATE_COMPRESSION_STARTED__ = false
      window.__TAURI_IPC__ = (message: Record<string, any>) => {
        let value: unknown
        if (message.cmd === 'tauri' && message.message?.cmd === 'openDialog') {
          openDialogCount += 1
          value = openDialogCount === 1
            ? 'C:/templates/logs.longtask.json'
            : openDialogCount === 2
              ? 'C:/logs'
              : ['C:/logs/keep.log', 'C:/logs/skip.tmp']
        } else if (message.cmd === 'get_compression_profiles') {
          value = [{
            id: 'logs', name: '日志归档', icon: '📦', description: '安全归档日志',
            config: { format: '7z', level: 7, password: null, split_archive: false, split_size: null, keep_structure: true, delete_after: false, verify_after: true, create_solid_archive: true, filename_template: '{name}-{date}', extra_params: {} },
            auto_apply: { enabled: false, mode: 'pattern', file_patterns: ['*.log'], exclude_patterns: ['*.tmp'], size_range: null },
            password_strategy: 'none', stats: { use_count: 0, success_count: 0, failure_count: 0, total_files_processed: 0, total_bytes_processed: 0 }, created_at: 0, last_used_at: null,
          }]
        } else if (message.cmd === 'list_task_template_watch_folders') {
          value = []
        } else if (message.cmd === 'list_pending_task_template_watch_batches') {
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
              sourceRules: { mode: 'pattern', includePatterns: ['*.log'], excludePatterns: ['*.tmp'], sizeRangeMib: null },
              targetRule: { mode: 'choose_at_runtime', filenameTemplate: '{name}-{date}' },
              compression: { format: '7z', level: 7, splitArchive: false, splitSizeMib: null, keepStructure: true, verifyAfter: true, createSolidArchive: true },
              passwordStrategy: { mode: 'prompt_at_runtime' }, exportNotes: [],
            },
            warnings: ['自动应用保持关闭'],
            contentSha256: 'a'.repeat(64),
          }
        } else if (message.cmd === 'plan_task_template_draft') {
          value = {
            profileId: 'logs', profileName: '日志归档',
            accepted: [{ path: 'C:/logs/keep.log', name: 'keep.log', size: 12, isDirectory: false }],
            excluded: [{ candidate: { path: 'C:/logs/skip.tmp', name: 'skip.tmp', size: 3, isDirectory: false }, reason: '命中排除规则' }],
            warnings: ['该计划只会创建压缩草稿，不会启动任务'],
          }
        } else if (message.cmd === 'preview_task_template_watch_folder') {
          value = {
            profileId: 'logs', profileName: '日志归档', rootPath: 'C:/logs', scannedFiles: 3,
            accepted: [{ path: 'C:/logs/keep.log', name: 'keep.log', size: 12, isDirectory: false }],
            excluded: [{ candidate: { path: 'C:/logs/changing.log', name: 'changing.log', size: 3, isDirectory: false }, reason: '文件在稳定观察窗口内发生变化、消失或无法读取' }],
            truncated: false, stabilityWindowMs: 750,
            warnings: ['本结果仅为一次性只读预览，不会保存监控、创建草稿或启动压缩'],
          }
        } else if (message.cmd === 'compress_files') {
          ;(window as any).__TASK_TEMPLATE_COMPRESSION_STARTED__ = true
          value = 'unexpected-task'
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

    await preview.getByRole('button', { name: '取消' }).click()
    await page.getByTestId('preview-watch-folder-logs').click()
    const watchPreview = page.getByTestId('watch-folder-preview')
    await expect(watchPreview).toContainText('当前仍是一次性扫描')
    await expect(watchPreview).toContainText('文件在稳定观察窗口内发生变化')
    await expect(watchPreview).not.toContainText('确认创建')
    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, ['[data-testid="watch-folder-preview"]'])
    }
    await page.getByTestId('close-watch-folder-preview').click()

    await page.getByTestId('create-template-draft-logs').click()
    const draftPlan = page.getByTestId('template-draft-plan')
    await expect(draftPlan).toContainText('命中排除规则')
    await expect(draftPlan).toContainText('只创建草稿，不启动任务')
    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, ['[data-testid="template-draft-plan"]'])
    }
    await page.getByTestId('confirm-template-draft').click()
    await expect(page.getByRole('button', { name: /keep-\d{4}-\d{2}-\d{2}\.7z.*等待中/ })).toBeVisible()
    await expect.poll(() => page.evaluate(() => (window as any).__TASK_TEMPLATE_COMPRESSION_STARTED__))
      .toBe(false)
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
        } else if (message.cmd === 'file_manager_locations') {
          value = [
            { name: '主目录', path: 'C:/Users/test', kind: 'home' },
            { name: 'D 盘', path: 'D:/', kind: 'drive' },
          ]
        } else if (message.cmd === 'list_files') {
          value = []
        } else if (message.cmd === 'browse_archive') {
          value = {
            format: 'ZIP', totalFiles: 1, totalDirectories: 0,
            totalUncompressedSize: 68, totalCompressedSize: 60, encrypted: false,
            entries: [{ path: 'art/preview.png', name: 'preview.png', size: 68, compressedSize: 60, modified: null, crc: '12345678', encrypted: false, isDir: false }],
          }
        } else if (message.cmd === 'get_archive_engine_capabilities') {
          value = {
            available: true,
            fullEngine: true,
            formats: [{ name: 'ZIP', extensions: ['zip'], canCreate: true }],
            browseExtensions: ['zip'],
            nestedExtensions: ['zip'],
            boundedPreviewFormats: ['ZIP'],
            imagePreviewExtensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'],
            textPreviewExtensions: ['txt', 'md', 'log'],
            message: 'ready',
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
    const fileManager = page.getByTestId('dual-pane-file-manager')
    await expect(fileManager).toBeVisible()
    await expect(fileManager.locator('.file-pane')).toHaveCount(2)
    await expect(fileManager).not.toContainText('把压缩包拖到这里')
    await expect(fileManager.locator('[data-testid^="file-manager-selection-mode-"]')).toHaveCount(2)
    await expect(fileManager.locator('[data-testid^="file-manager-breadcrumbs-"]')).toHaveCount(2)
    const selectionMode = page.getByTestId('file-manager-selection-mode-left')
    await selectionMode.click()
    await expect(selectionMode).toHaveAttribute('aria-pressed', 'true')
    await selectionMode.click()
    await expect(selectionMode).toHaveAttribute('aria-pressed', 'false')
    await fileManager.locator('.file-list').first().click({ button: 'right', position: { x: 24, y: 24 } })
    await expect(page.getByTestId('file-manager-open-same-other')).toContainText('另一栏打开相同文件夹')
    await page.keyboard.press('Escape')
    await expectVerticalOnlyScrolling(page, ['[data-testid="dual-pane-file-manager"]', '.file-pane', '.file-list'])
    await page.getByTestId('file-manager-open-archive').click()
    await expect(page.locator('.browser-page')).toBeVisible()
    await page.locator('[data-entry-path="art/"]').dblclick()
    await expect(page.getByTestId('archive-breadcrumbs')).toContainText('art')
    await expect(page.getByText('preview.png').first()).toBeVisible()
    await page.locator('[data-entry-path="art/preview.png"]').click({ button: 'right' })
    const contextMenu = page.getByTestId('archive-context-menu')
    await expect(contextMenu).toContainText('内部查看器打开')
    await expect(contextMenu).toContainText('使用默认应用打开')
    await expect(contextMenu).toContainText('解压到当前输出目录')
    await expect(contextMenu).toContainText('显示详细信息')
    await expect(contextMenu).not.toContainText('进入压缩包')
    const menuBox = await contextMenu.boundingBox()
    const menuViewport = page.viewportSize()
    expect(menuBox).not.toBeNull()
    expect(menuViewport).not.toBeNull()
    expect(menuBox!.x).toBeGreaterThanOrEqual(0)
    expect(menuBox!.y).toBeGreaterThanOrEqual(0)
    expect(menuBox!.x + menuBox!.width).toBeLessThanOrEqual(menuViewport!.width)
    expect(menuBox!.y + menuBox!.height).toBeLessThanOrEqual(menuViewport!.height)
    await page.getByTestId('archive-context-details').click()
    const details = page.getByTestId('archive-entry-details')
    await expect(details).toContainText('art/preview.png')
    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, [
        '.browser-page',
        '[data-testid="archive-entry-details"]',
        '.archive-details-dialog',
      ])
    }
    await page.getByRole('button', { name: '关闭条目详情' }).click()
    await page.locator('[data-entry-path="art/preview.png"]').click({ button: 'right' })
    await page.getByTestId('archive-context-preview').click()
    const previewDialog = page.getByTestId('archive-entry-preview')
    const preview = page.getByTestId('archive-image-preview')
    await expect(preview.getByRole('img', { name: 'preview.png' })).toBeVisible()
    await expect(previewDialog).toContainText('只读 · 未写入磁盘')

    for (const width of responsiveWidths) {
      await page.setViewportSize({ width, height: 800 })
      await expectVerticalOnlyScrolling(page, [
        '.browser-page',
        '.preview-stage',
        '.preview-dialog',
      ])
    }
  })
})

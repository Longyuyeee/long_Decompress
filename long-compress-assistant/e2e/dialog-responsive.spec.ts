import { test, expect, type Page, type Locator } from '@playwright/test'

// Headless Chromium hides scrollbars by default; keep the desktop scroll affordance.
test.use({ launchOptions: { ignoreDefaultArgs: ['--hide-scrollbars'] } })

const archive = 'C:\\Users\\39633\\Desktop\\' + '归档资料和项目文件_'.repeat(5) + '\\xc-ict-front-master-84762be.tar.gz'
const scenarios = [
  { name: 'reference', width: 1024, height: 720, dpr: 1, scale: 100, font: 'normal', theme: 'sepia' },
  { name: 'compact', width: 920, height: 620, dpr: 1, scale: 125, font: 'large', theme: 'dark' },
  { name: 'high-dpi', width: 640, height: 480, dpr: 2, scale: 100, font: 'normal', theme: 'sepia' },
  { name: 'short-large-type', width: 640, height: 360, dpr: 2, scale: 200, font: 'x-large', theme: 'dark' },
  { name: 'narrow', width: 360, height: 480, dpr: 1, scale: 150, font: 'large', theme: 'light' },
] as const

async function openApp(page: Page, scenario: typeof scenarios[number]) {
  await page.addInitScript(s => {
    localStorage.setItem('app-theme', s.theme)
    localStorage.setItem('app-settings', JSON.stringify({ theme: s.theme, uiScale: s.scale, autoCheckUpdates: false,
      accessibility: { fontSize: s.font, reduceMotion: true, highContrast: false, colorBlindMode: 'none', focusIndicator: true } }))
    const win = window as any
    win.__TAURI_IPC__ = (message: any) => {
      let result: any
      if (message.cmd === 'is_context_menu_registered') result = true
      else if (message.cmd === 'load_history' || message.cmd === 'take_pending_context_actions') result = []
      else if (message.cmd === 'get_archive_engine_capabilities') result = { available: true, fullEngine: true, formats: [], browseExtensions: [], nestedExtensions: [], boundedPreviewFormats: [], imagePreviewExtensions: [], textPreviewExtensions: [], message: 'ready' }
      else if (message.cmd === 'preview_extraction_output') result = message.outputPath + '\\' + message.filePath.split(/[\\/]/).pop().replace(/\.tar\.gz$/i, '')
      else if (message.cmd === 'archive_file_associations') result = ['zip','7z','rar','zipx','tar','gz','bz2','xz','zst','tgz','tbz2','txz','lz','lzo','lz4','br','cab','arj','alz','egg','zpaq'].map(extension => ({ extension: '.' + extension, registered: true, isDefault: false }))
      else if (message.cmd === 'tauri' && message.__tauriModule === 'App') result = '1.7.41'
      else if (message.cmd === 'tauri' && message.__tauriModule === 'Dialog') result = null
      win[`_${message.callback}`]?.(result)
    }
  }, scenario)
  await page.goto('/#/settings?section=behavior', { waitUntil: 'domcontentloaded', timeout: 90000 })
  await expect(page.locator('.settings-view')).toBeVisible()
  const fontFactor = scenario.font === 'x-large' ? 1.25 : scenario.font === 'large' ? 1.125 : 1
  await expect.poll(() => page.evaluate(() => parseFloat(getComputedStyle(document.documentElement).fontSize)))
    .toBeCloseTo(16 * scenario.scale / 100 * fontFactor, 1)
}

async function expectDialogFits(page: Page, dialog: Locator) {
  const frame = await dialog.locator('.app-dialog').evaluate(el => {
    const rect = el.getBoundingClientRect()
    return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: innerWidth, height: innerHeight }
  })
  expect(frame.left).toBeGreaterThanOrEqual(0)
  expect(frame.top).toBeGreaterThanOrEqual(0)
  expect(frame.right).toBeLessThanOrEqual(frame.width)
  expect(frame.bottom).toBeLessThanOrEqual(frame.height)
  const overflow = await dialog.locator('.app-dialog').evaluate(root => [root, ...root.querySelectorAll('button, .dialog-body, .archive-open-dialog, .open-source, .open-choice>span')]
    .filter(el => el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 1)
    .map(el => ({ element: el.className, width: el.clientWidth, scroll: el.scrollWidth })))
  expect(overflow).toEqual([])
  await expect(dialog.getByRole('button', { name: 'Close dialog', exact: true })).toBeInViewport({ ratio: 1 })
}

for (const scenario of scenarios) {
  test.describe(scenario.name, () => {
    test.use({ viewport: { width: scenario.width, height: scenario.height }, deviceScaleFactor: scenario.dpr })
    test('archive opening and shared footer stay within the window', async ({ page }, info) => {
      test.setTimeout(120000)
      await openApp(page, scenario)
      await page.evaluate(async path => {
        const storePath = '/src/stores/app.ts'
        const app = (await import(/* @vite-ignore */ storePath)).useAppStore()
        app.enqueueArchiveOpen([path])
      }, archive)
      const dialog = page.getByRole('dialog', { name: '打开压缩包', exact: true })
      await expect(dialog).toBeVisible()
      await expect(dialog.locator('.open-choice')).toHaveCount(4)
      await expectDialogFits(page, dialog)
      const remember = dialog.getByTestId('archive-open-remember')
      await expect(remember).toBeInViewport({ ratio: 1 })
      for (const action of ['here', 'subfolder', 'choose-folder', 'browse']) {
        const button = dialog.getByTestId(`archive-open-${action}`)
        await button.scrollIntoViewIfNeeded()
        // Fractional CSS pixels can be rounded at the scroll edge on high-DPI screens.
        await expect(button).toBeInViewport({ ratio: .99 })
        const clip = await dialog.locator('.dialog-body').boundingBox()
        const bounds = await button.boundingBox()
        expect(bounds!.y).toBeGreaterThanOrEqual(clip!.y - 1)
        expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(clip!.y + clip!.height + 1)
        await expect(remember).toBeInViewport({ ratio: 1 })
        const glyph = await button.locator('i').first().evaluate(el => getComputedStyle(el, '::before').content)
        expect(glyph).not.toMatch(/^(none|normal|""|'')$/)
        const arrow = await button.locator('i').last().boundingBox()
        expect(arrow).not.toBeNull()
        expect(arrow!.x + arrow!.width).toBeLessThanOrEqual(bounds!.x + bounds!.width)
      }
      await remember.check(); await remember.uncheck()
      await dialog.getByTestId('archive-open-choose-folder').click()
      await expect(dialog).toBeVisible() // cancelling the native picker keeps the choices
      await expect(dialog.locator('[aria-busy]')).toHaveAttribute('aria-busy', 'false')
      await expectDialogFits(page, dialog)
      await page.screenshot({ path: info.outputPath('archive-open.png') })
      if (scenario.name === 'reference') {
        // Resize an already-open dialog; the same choices and footer must survive.
        await page.setViewportSize({ width: 640, height: 360 })
        await expectDialogFits(page, dialog)
        await expect(remember).toBeInViewport({ ratio: 1 })
        await page.screenshot({ path: info.outputPath('archive-open-resized.png') })
      }
      await dialog.getByRole('button', { name: 'Close dialog', exact: true }).click()
      await expect(dialog).toHaveCount(0)
      // A second consumer checks common modal sizing and a pinned action footer.
      await page.getByRole('button', { name: '设置默认打开方式…', exact: true }).click()
      const formats = page.getByRole('dialog', { name: '设置默认打开方式', exact: true })
      await expect(formats).toBeVisible()
      await expectDialogFits(page, formats)
      await formats.getByTestId('archive-association-formats').locator('input').last().scrollIntoViewIfNeeded()
      await expect(formats.getByTestId('archive-association-save')).toBeInViewport({ ratio: 1 })
      await expect(formats.getByRole('button', { name: 'Close dialog', exact: true })).toBeInViewport({ ratio: 1 })
      await expectDialogFits(page, formats)
      await page.screenshot({ path: info.outputPath('shared-modal.png') })
      await formats.getByRole('button', { name: '取消', exact: true }).click()
      await expect(formats).toHaveCount(0)
    })
  })
}

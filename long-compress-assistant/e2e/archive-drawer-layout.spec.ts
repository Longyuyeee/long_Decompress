import { expect, test } from '@playwright/test'

for (const center of ['decompress', 'compress'] as const) {
  test(`${center}: expanded rows stay readable and animate without stretching`, async ({ page }, info) => {
    await page.addInitScript(() => {
      window.__TAURI_IPC__ = (message: Record<string, any>) => {
        let result: unknown
        if (message.cmd === 'load_app_settings') result = '{}'
        if (message.cmd === 'preview_extraction_output') result = 'C:/用户资料/输出/项目资料'
        if (message.cmd === 'inspect_extraction_destination') result = 'occupied'
        ;(window as any)[`_${message.callback}`]?.(result)
      }
    })
    await page.setViewportSize({ width: 1280, height: 800 })
    await page.goto(`${process.env.DRAWER_BASELINE_URL || ''}/#/${center}`)
    await page.waitForSelector('h1')
    await page.evaluate(async type => {
      const taskPath = '/src/stores/task.ts'
      const compressionPath = '/src/stores/compression.ts'
      const taskStore = (await import(taskPath)).useTaskStore()
      const compressionStore = (await import(compressionPath)).useCompressionStore()
      for (let index = 0; index < 3; index++) {
        const name = ['项目资料与设计文件', '旅行照片备份', '工作文档'][index]
        if (type === 'decompress') taskStore.addTask({ id: `layout-${index}`, name: `${name}.zip`, type: 'decompression', sourceFiles: [`C:/用户资料/${name}.zip`], outputPath: 'C:/用户资料/输出', format: 'zip' })
        else compressionStore.addFile({ name: `${name}.txt`, path: `C:/用户资料/${name}.txt`, size: 4096, type: 'file', isDirectory: false })
      }
    }, center)
    const toggles = center === 'decompress' ? page.locator('[data-testid^="toggle-archive-task-layout-"]') : page.locator('button[aria-label="展开压缩详情"], button[aria-label="收起压缩详情"]')
    await toggles.nth(0).click()
    await page.waitForTimeout(450)
    const drawer = page.locator('.details-drawer').first()
    if (center === 'compress') {
      await page.setViewportSize({ width: 920, height: 620 })
      await page.waitForTimeout(400)
      const config = await drawer.locator('.compression-config-panel').boundingBox()
      const execution = await drawer.locator('.compression-execution-panel').boundingBox()
      expect(Math.abs(config!.y - execution!.y)).toBeLessThanOrEqual(1)
      const name = drawer.getByTestId('compression-filename-input')
      await name.fill('精选700首华语乐坛经典流行热门歌曲与项目资料备份'.repeat(3))
      const extension = drawer.getByTestId('compression-filename-extension')
      const inputBox = await name.boundingBox()
      const extensionBox = await extension.boundingBox()
      expect(inputBox!.x + inputBox!.width).toBeLessThanOrEqual(extensionBox!.x + 1)
      await expect(extension).toHaveText('.zip')
      await name.evaluate(el => el.scrollIntoView({ block: 'center' }))
      await page.waitForTimeout(250)
      await page.screenshot({ path: info.outputPath('compress-default-long-name.png') })
      const format = drawer.getByTestId('compression-format-select')
      await format.click()
      const menu = page.getByRole('listbox', { name: '目标格式' })
      await expect(menu).toBeVisible()
      await page.waitForTimeout(250)
      const menuBox = await menu.boundingBox()
      expect(menuBox!.y).toBeGreaterThanOrEqual(0)
      expect(menuBox!.y + menuBox!.height).toBeLessThanOrEqual(620)
      await page.screenshot({ path: info.outputPath('compress-format-menu.png') })
      await page.keyboard.press('Escape')
      await expect(format).toBeFocused()
      await format.press('ArrowDown')
      await page.getByRole('option', { name: '7Z', exact: true }).click()
      await expect(extension).toHaveText('.7z')
      await expect(name).toHaveValue(/精选700首/)
      await format.click()
      await page.getByRole('option', { name: 'TGZ.AES 🔒', exact: true }).click()
      await expect(extension).toHaveText('.tar.gz.aes')
      await name.evaluate(el => el.scrollIntoView({ block: 'center' }))
      const compoundInput = await name.boundingBox()
      const compoundSuffix = await extension.boundingBox()
      expect(compoundInput!.x + compoundInput!.width).toBeLessThanOrEqual(compoundSuffix!.x + 1)
      await page.waitForTimeout(250)
      await page.screenshot({ path: info.outputPath('compress-compound-extension.png') })
    }
    if (center === 'decompress') {
      await page.setViewportSize({ width: 920, height: 620 })
      await page.waitForTimeout(400)
      const config = await drawer.locator('.task-config-panel').boundingBox()
      const execution = await drawer.locator('.task-execution-panel').boundingBox()
      expect(Math.abs(config!.y - execution!.y), 'default window keeps configuration and logs side by side').toBeLessThanOrEqual(1)
      expect(config!.width).toBeGreaterThan(250)
      expect(execution!.width).toBeGreaterThan(240)
      await expect(page.getByText('新内容会放入已有目录；是否存在同名文件，将在解压时确认。', { exact: true })).toHaveCount(0)
      await page.screenshot({ path: info.outputPath('decompress-default-920x620.png') })
      const help = drawer.getByRole('button', { name: '解压目录说明', exact: true })
      await help.click()
      await expect(page.getByRole('note', { name: '解压目录说明' })).toContainText('不另建同名子目录')
      await page.screenshot({ path: info.outputPath('decompress-default-help.png') })
      await page.keyboard.press('Escape')
      await expect(help).toBeFocused()
      await expect(page.getByRole('note')).toHaveCount(0)
      await help.press('Enter')
      await expect(page.getByRole('note')).toBeVisible()
      await page.locator('h1').click()
      await expect(page.getByRole('note')).toHaveCount(0)
    }
    for (const width of [1280, 920, 760, 600]) {
      await page.setViewportSize({ width, height: 800 })
      await page.waitForTimeout(400)
      await page.screenshot({ path: info.outputPath(`${center}-${width}.png`) })
      const overflow = await drawer.locator(center === 'decompress' ? '.task-detail-layout' : '.compression-detail-grid').evaluate(el => el.scrollWidth - el.clientWidth)
      if (!process.env.DRAWER_CAPTURE_ONLY) expect.soft(overflow, `${center} ${width}: details must not crop columns`).toBeLessThanOrEqual(1)
      if (center === 'compress' && !process.env.DRAWER_CAPTURE_ONLY) {
        const fields = await drawer.locator('.settings-core-grid > div').evaluateAll(elements => elements.map(el => ({ top: el.getBoundingClientRect().top, width: el.getBoundingClientRect().width })))
        expect.soft(Math.abs(fields[0].top - fields[1].top), 'first two parameter cards align').toBeLessThanOrEqual(1)
        for (const field of fields) expect.soft(field.width, 'fields remain usable').toBeGreaterThan(140)
      }
    }
    await page.setViewportSize({ width: 1280, height: 800 })
    await toggles.nth(0).click()
    await page.waitForTimeout(450)
    await expect(page.locator('.details-drawer')).toHaveCount(0)
    // Observe geometry on actual animation frames; a jump from zero to auto is not an animation.
    await page.evaluate(() => {
      ;(window as any).drawerHeights = []
      const started = performance.now()
      const sample = () => {
        ;(window as any).drawerHeights.push(document.querySelector('.details-drawer')?.getBoundingClientRect().height || 0)
        if (performance.now() - started < 600) requestAnimationFrame(sample)
      }
      requestAnimationFrame(sample)
    })
    await toggles.nth(0).click()
    await page.waitForTimeout(650)
    const heights: number[] = await page.evaluate(() => (window as any).drawerHeights)
    if (!process.env.DRAWER_CAPTURE_ONLY) expect(new Set(heights.map(Math.round)).size, 'height must interpolate across frames').toBeGreaterThan(5)
    if (process.env.DRAWER_CAPTURE_ONLY) return
    // Both items can stay expanded; rapidly reversing a drawer must not leave fixed-height clipping.
    await toggles.nth(1).click()
    await expect(page.locator('.details-drawer')).toHaveCount(2)
    await toggles.nth(0).click()
    await toggles.nth(0).click()
    await page.waitForTimeout(450)
    await expect(page.locator('.details-drawer')).toHaveCount(2)
    expect(await drawer.evaluate(el => (el as HTMLElement).style.height)).toBe('')
    await page.evaluate(() => { document.documentElement.style.fontSize = '125%' })
    await page.waitForTimeout(400)
    await page.screenshot({ path: info.outputPath(`${center}-125percent.png`) })
    expect(await drawer.locator(center === 'decompress' ? '.task-detail-layout' : '.compression-detail-grid').evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1)
    await page.emulateMedia({ reducedMotion: 'reduce' })
    await toggles.nth(0).click()
    await expect(page.locator('.details-drawer')).toHaveCount(1)
  })
}

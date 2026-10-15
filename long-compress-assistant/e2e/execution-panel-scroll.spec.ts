import { expect, test, type Locator } from '@playwright/test'

async function visibleSummaryPoint(panel: Locator) {
  return panel.evaluate(el => {
    const r = el.getBoundingClientRect()
    const log = el.querySelector('[data-testid$="-log-viewport"]')!
    for (const x of [r.x + r.width / 2, r.left + 8, r.right - 20]) {
      for (let y = Math.max(0, r.top) + 8; y < Math.min(innerHeight, r.bottom); y += 12) {
        const hit = document.elementFromPoint(x, y)
        if (hit && el.contains(hit) && !log.contains(hit)) return { x, y }
      }
    }
    throw new Error('No visible execution summary to scroll over')
  })
}

async function expectReadableTaskRow(row: Locator) {
  const layout = await row.evaluate(el => {
    const stage = el.querySelector('.task-status-runtime > div > span')!
    const percent = el.querySelector('.task-status-percent')!
    const runtime = el.querySelector('.task-status-runtime')!
    const a = stage.getBoundingClientRect()
    const b = percent.getBoundingClientRect()
    const r = runtime.getBoundingClientRect()
    const outer = el.getBoundingClientRect()
    return {
      textOverlap: Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left)) * Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top)),
      stageOverflow: a.right - r.right,
      rowOverflow: r.bottom - outer.bottom,
    }
  })
  expect(layout.textOverlap, 'stage and percentage must not overlap').toBeLessThanOrEqual(1)
  expect(layout.stageOverflow, 'stage text must stay in its own area').toBeLessThanOrEqual(1)
  expect(layout.rowOverflow, 'runtime information must fit the task row').toBeLessThanOrEqual(1)
}

for (const type of ['compression', 'decompression'] as const) {
  test(`${type}: wheel reaches execution logs below all status cards`, async ({ page }, info) => {
    test.setTimeout(60_000)
    await page.emulateMedia({ colorScheme: 'dark' })
    await page.addInitScript(() => {
      window.__TAURI_IPC__ = (message: Record<string, any>) => {
        ;(window as any)[`_${message.callback}`]?.(message.cmd === 'load_app_settings' ? '{}' : undefined)
      }
    })
    await page.goto('/#/compress')
    await page.waitForFunction(() => Boolean(window.__LONG_DECOMPRESS_DESKTOP_E2E__))

    for (const state of ['completed', 'running'] as const) {
      await page.evaluate(({ type, state }) => window.__LONG_DECOMPRESS_DESKTOP_E2E__!.seedResponsiveWorkspace(type, state), { type, state })
      await page.getByTestId(type === 'compression' ? 'nav-Compress' : 'nav-Decompress').click()
      if (type === 'decompression' && !await page.getByTestId('decompression-task-details').count()) {
        await page.locator(`[data-task-id="responsive-${type}"]`).click()
      }
      const panel = page.getByTestId(type === 'compression' ? 'compression-draft-execution' : 'decompression-execution-panel')
      const log = page.getByTestId(`${type}-log-viewport`)
      await expect(panel).toBeVisible()

      for (const viewport of [
        { width: 1280, height: 800, scale: '100%' },
        { width: 920, height: 620, scale: '100%' },
        { width: 920, height: 620, scale: '125%' },
        { width: 760, height: 520, scale: '100%' },
        { width: 760, height: 520, scale: '125%' },
      ]) {
        await page.setViewportSize(viewport)
        await page.evaluate(scale => { document.documentElement.style.fontSize = scale }, viewport.scale)
        if (type === 'compression') {
          const title = page.locator('[data-testid="compression-header"] h1')
          expect(await title.evaluate(el => el.getBoundingClientRect().height)).toBeLessThan(60)
        }
        await panel.evaluate(el => { el.scrollTop = 0; el.scrollIntoView({ block: 'start' }) })
        await page.waitForTimeout(450)
        if (type === 'decompression' && state === 'running') {
          await expectReadableTaskRow(page.locator('[data-task-id="responsive-decompression"]'))
        }
        const overview = panel.getByTestId('task-execution-overview')
        const visualProgress = await overview.locator('.execution-progress-track > div').evaluate(el => Number.parseFloat((el as HTMLElement).style.width))
        expect(visualProgress, 'the summary bar must match the displayed progress').toBe(state === 'completed' ? 100 : 46)
        await page.screenshot({ path: info.outputPath(`${type}-${state}-${viewport.width}-${viewport.scale}-overview.png`) })
        const point = await visibleSummaryPoint(panel)
        await page.mouse.move(point.x, point.y)
        await page.mouse.wheel(0, 2000)
        await page.waitForTimeout(250)
        if (await panel.evaluate(el => el.scrollHeight > el.clientHeight + 1)) {
          expect(await panel.evaluate(el => el.scrollTop), 'wheel must scroll the execution pane').toBeGreaterThan(0)
        }
        // In stacked layouts the task list also scrolls to bring the log section into view.
        await log.scrollIntoViewIfNeeded()
        const geometry = await panel.evaluate(el => {
          const log = el.querySelector('[data-testid$="-log-viewport"]')!
          const panelRect = el.getBoundingClientRect()
          const logRect = log.getBoundingClientRect()
          const configRect = el.parentElement!.querySelector('.compression-config-panel, .task-config-panel')!.getBoundingClientRect()
          const row = log.firstElementChild!
          return {
            overflowY: getComputedStyle(el).overflowY,
            scrollTop: el.scrollTop,
            overflow: el.scrollHeight - el.clientHeight,
            logHeight: log.clientHeight,
            visibleLog: Math.max(0, Math.min(panelRect.bottom, logRect.bottom, innerHeight) - Math.max(panelRect.top, logRect.top, 0)),
            horizontalOverflow: el.scrollWidth - el.clientWidth,
            columnWidthDifference: panelRect.left >= configRect.right - 1 ? Math.abs(panelRect.width - configRect.width) : null,
            logRowDirection: getComputedStyle(row).flexDirection,
            logMessageWhiteSpace: getComputedStyle(row.lastElementChild!).whiteSpace,
          }
        })
        await info.attach(`${state}-${viewport.width}-${viewport.scale}`, { body: JSON.stringify(geometry), contentType: 'application/json' })
        await page.screenshot({ path: info.outputPath(`${type}-${state}-${viewport.width}-${viewport.scale}.png`) })
        expect(geometry.overflowY).toBe('auto')
        if (geometry.overflow > 1) expect(geometry.scrollTop, 'wheel must scroll the execution pane').toBeGreaterThan(0)
        expect(geometry.logHeight, 'status cards must leave a readable log viewport').toBeGreaterThanOrEqual(120)
        expect(geometry.visibleLog, 'logs must be reachable inside the visible execution pane').toBeGreaterThanOrEqual(100)
        expect(geometry.horizontalOverflow).toBeLessThanOrEqual(1)
        if (geometry.columnWidthDifference !== null) {
          expect(geometry.columnWidthDifference, 'side-by-side configuration and execution panes must share the width equally').toBeLessThanOrEqual(1)
        }
        if (geometry.logRowDirection === 'column') {
          expect(geometry.logMessageWhiteSpace, 'narrow logs must wrap their full message').not.toBe('nowrap')
        }

        await log.evaluate(el => { el.scrollTop = 0 })
        const logBox = (await log.boundingBox())!
        await page.mouse.move(logBox.x + logBox.width / 2, logBox.y + logBox.height / 2)
        await page.mouse.wheel(0, 100000)
        await expect.poll(() => log.evaluate(el => el.scrollTop)).toBeGreaterThan(0)
        await expect(log.locator(':scope > div').last()).toBeInViewport()
        await page.screenshot({ path: info.outputPath(`${type}-${state}-${viewport.width}-${viewport.scale}-last-log.png`) })
        await page.mouse.wheel(0, -100000)
        await expect.poll(() => log.evaluate(el => el.scrollTop)).toBe(0)
        await expect(log.locator(':scope > div').first()).toBeInViewport()
        const summaryPoint = await visibleSummaryPoint(panel)
        await page.mouse.move(summaryPoint.x, summaryPoint.y)
        await page.mouse.wheel(0, -2000)
        await expect.poll(() => panel.evaluate(el => el.scrollTop)).toBe(0)
      }
    }

    await page.evaluate(type => window.__LONG_DECOMPRESS_DESKTOP_E2E__!.seedResponsiveWorkspace(type, 'running', true), type)
    for (const width of [1280, 920]) {
      await page.setViewportSize({ width, height: width === 1280 ? 800 : 620 })
      await page.evaluate(() => {
        document.documentElement.style.fontSize = '100%'
        document.querySelector('.compression-task-list, .table-body')!.scrollTop = 0
      })
      await page.waitForTimeout(450)
      if (type === 'compression' && width === 920) {
        const fields = await page.locator('.settings-core-grid > div').evaluateAll(elements => elements.slice(0, 2).map(el => el.getBoundingClientRect().toJSON()))
        expect(fields[0].width, 'narrow configuration fields use a full readable row').toBeGreaterThan(200)
        expect(fields[1].y).toBeGreaterThanOrEqual(fields[0].bottom + 8)
      }
      await page.screenshot({ path: info.outputPath(`${type}-structured-${width}.png`) })
    }
  })
}

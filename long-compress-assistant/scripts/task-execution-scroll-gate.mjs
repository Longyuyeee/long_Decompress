import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By } from 'selenium-webdriver'

export async function runTaskExecutionScrollGate({ driver, callDesktopBridge, waitForElement, artifactDirectory, application }) {
  const results = []
  // Use the user's dark appearance in this isolated test WebView.
  await driver.executeScript(() => {
    document.documentElement.classList.remove('light')
    document.documentElement.classList.add('dark')
  })

  async function wheelOver(element, deltaY, excludeLogs = false) {
    const point = await driver.executeScript((el, exclude) => {
      const r = el.getBoundingClientRect()
      const log = el.querySelector('[data-testid$="-log-viewport"]')
      for (const x of [r.x + r.width / 2, r.left + 8, r.right - 20]) {
        for (let y = Math.max(0, r.top) + 8; y < Math.min(innerHeight, r.bottom); y += 12) {
          const hit = document.elementFromPoint(x, y)
          if (hit && el.contains(hit) && (!exclude || !log?.contains(hit))) return { x: Math.round(x), y: Math.round(y) }
        }
      }
      return null
    }, element, excludeLogs)
    assert.ok(point, 'The execution content must have a visible mouse target')
    await driver.actions().scroll(point.x, point.y, 0, deltaY, 'viewport', 100).perform()
    await new Promise(resolve => setTimeout(resolve, 250))
  }

  for (const type of ['compression', 'decompression']) {
    for (const state of ['completed', 'running']) {
      await callDesktopBridge('seedResponsiveWorkspace', type, state, true)
      await (await waitForElement(`[data-testid="nav-${type === 'compression' ? 'Compress' : 'Decompress'}"]`)).click()
      if (type === 'decompression' && !(await driver.findElements(By.css('[data-testid="decompression-task-details"]'))).length) {
        await (await waitForElement('[data-task-id="responsive-decompression"]')).click()
      }
      const panel = await waitForElement(`[data-testid="${type === 'compression' ? 'compression-draft-execution' : 'decompression-execution-panel'}"]`)
      const log = await waitForElement(`[data-testid="${type}-log-viewport"]`)

      for (const size of [
        { width: 1280, height: 800, scale: '100%' },
        { width: 920, height: 620, scale: '100%' },
        { width: 920, height: 620, scale: '125%' },
        { width: 760, height: 520, scale: '100%' },
        { width: 760, height: 520, scale: '125%' },
      ]) {
        await driver.manage().window().setRect({ width: size.width, height: size.height })
        await driver.executeScript((el, scale) => {
          document.documentElement.style.fontSize = scale
          el.scrollTop = 0
          el.parentElement.closest('.compression-task-list, .table-body').scrollTop = 0
        }, panel, size.scale)
        await new Promise(resolve => setTimeout(resolve, 450))
        const taskRow = type === 'decompression' && state === 'running' ? await driver.executeScript(() => {
          const row = document.querySelector('[data-task-id="responsive-decompression"]')
          const stage = row.querySelector('.task-runtime-stage').getBoundingClientRect()
          const percent = row.querySelector('.task-status-percent').getBoundingClientRect()
          const runtime = row.querySelector('.task-status-runtime').getBoundingClientRect()
          const outer = row.getBoundingClientRect()
          return {
            textOverlap: Math.max(0, Math.min(stage.right, percent.right) - Math.max(stage.left, percent.left)) * Math.max(0, Math.min(stage.bottom, percent.bottom) - Math.max(stage.top, percent.top)),
            stageOverflow: stage.right - runtime.right,
            rowOverflow: runtime.bottom - outer.bottom,
          }
        }) : null
        if (taskRow) {
          assert.ok(taskRow.textOverlap <= 1, `Task stage and percentage overlap: ${JSON.stringify(taskRow)}`)
          assert.ok(taskRow.stageOverflow <= 1, `Task stage leaves its area: ${JSON.stringify(taskRow)}`)
          assert.ok(taskRow.rowOverflow <= 1, `Task runtime is clipped: ${JSON.stringify(taskRow)}`)
        }
        writeFileSync(path.join(artifactDirectory, `task-overview-${type}-${state}-${size.width}-${size.scale}.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
        await driver.executeScript(el => el.scrollIntoView({ block: 'start' }), panel)
        await new Promise(resolve => setTimeout(resolve, 150))
        await wheelOver(panel, 2000, true)
        const pane = await driver.executeScript(el => {
          const config = el.parentElement.querySelector('.compression-config-panel, .task-config-panel').getBoundingClientRect()
          const r = el.getBoundingClientRect()
          return {
            overflowY: getComputedStyle(el).overflowY,
            scrollTop: el.scrollTop,
            overflow: el.scrollHeight - el.clientHeight,
            horizontalOverflow: el.scrollWidth - el.clientWidth,
            columnWidthDifference: r.left >= config.right - 1 ? Math.abs(r.width - config.width) : null,
            visualProgress: Number.parseFloat(el.querySelector('.execution-progress-track > div').style.width),
          }
        }, panel)
        assert.equal(pane.overflowY, 'auto', `${type} execution must allow wheel scrolling`)
        if (pane.overflow > 1) assert.ok(pane.scrollTop > 0, `${type} execution wheel did not move: ${JSON.stringify(pane)}`)
        assert.ok(pane.horizontalOverflow <= 1)
        if (pane.columnWidthDifference !== null) assert.ok(pane.columnWidthDifference <= 1)
        assert.equal(pane.visualProgress, state === 'completed' ? 100 : 46)

        await driver.executeScript(el => { el.scrollIntoView({ block: 'center' }); el.scrollTop = 0 }, log)
        await new Promise(resolve => setTimeout(resolve, 150))
        await wheelOver(log, 100000)
        const logLayout = await driver.executeScript(el => {
          const last = el.lastElementChild
          let clip = last.getBoundingClientRect()
          let top = Math.max(0, clip.top)
          let bottom = Math.min(innerHeight, clip.bottom)
          for (let parent = last.parentElement; parent; parent = parent.parentElement) {
            if (['auto', 'scroll', 'hidden'].includes(getComputedStyle(parent).overflowY)) {
              const r = parent.getBoundingClientRect()
              top = Math.max(top, r.top)
              bottom = Math.min(bottom, r.bottom)
            }
          }
          const hit = document.elementFromPoint(clip.x + clip.width / 2, (top + bottom) / 2)
          return {
            height: el.clientHeight,
            scrollTop: el.scrollTop,
            scrollHeight: el.scrollHeight,
            lastLogVisible: bottom > top && !!hit && last.contains(hit),
            lastLogText: last.textContent,
          }
        }, log)
        assert.ok(logLayout.height >= 120, `${type} log viewport collapsed`)
        assert.ok(logLayout.scrollTop > 0, `${type} log wheel did not move`)
        assert.equal(logLayout.lastLogVisible, true, `${type} last log is clipped: ${JSON.stringify(logLayout)}`)
        const screenshot = `task-scroll-${type}-${state}-${size.width}-${size.scale}.png`
        writeFileSync(path.join(artifactDirectory, screenshot), Buffer.from(await driver.takeScreenshot(), 'base64'))
        await wheelOver(log, -100000)
        assert.equal(await driver.executeScript(el => el.scrollTop, log), 0, `${type} log must scroll back to its first entry`)
        await wheelOver(panel, -2000, true)
        assert.equal(await driver.executeScript(el => el.scrollTop, panel), 0, `${type} execution must scroll back to its summary`)
        results.push({ type, state, ...size, taskRow, pane, log: logLayout, scrollBackToTop: true, screenshot })
      }
    }
  }
  writeFileSync(path.join(artifactDirectory, 'task-execution-scroll-results.json'), JSON.stringify({
    testKind: 'real-windows-webview2-task-execution-scroll',
    fixtureTasks: true,
    binarySha256: createHash('sha256').update(readFileSync(application)).digest('hex'),
    results,
    passed: true,
  }, null, 2))
}

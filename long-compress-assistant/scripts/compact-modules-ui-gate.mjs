import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

// Focused UI regression: actual WebView layout and the controls moved into
// secondary views. Existing archive-format matrices are deliberately not rerun.
export async function runCompactModulesUiGate({ driver, bridge, application, root, dataDirectory }) {
  const base = path.join(root, 'test-results', 'compact-modules-ui')
  const work = path.join(base, 'fixtures')
  mkdirSync(work, { recursive: true })
  const binarySha256 = createHash('sha256').update(readFileSync(application)).digest('hex')
  const layoutOnly = process.argv.includes('--compact-modules-layout-only')
  const receipt = path.join(base, layoutOnly ? 'layout-result.json' : 'result.json')
  const report = existsSync(receipt) ? JSON.parse(readFileSync(receipt, 'utf8')) : { binarySha256, checks: [], layouts: [], screenshots: [] }
  assert.equal(report.binarySha256, binarySha256, 'Use a separate report for a changed application')
  const passed = name => report.checks.includes(name)
  const save = () => writeFileSync(receipt, JSON.stringify(report, null, 2) + '\n')
  const done = name => { report.checks.push(name); save(); console.log('[compact-ui] ' + name) }
  const find = selector => driver.wait(async () => {
    try {
      for (const element of await driver.findElements(By.css(selector))) if (await element.isDisplayed()) return element
      return false
    } catch (error) { if (error.name === 'StaleElementReferenceError') return false; throw error }
  }, 15000, 'Missing visible ' + selector)
  async function click(selector) {
    const element = await find(selector)
    await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await driver.wait(async () => await element.isEnabled() && await driver.executeScript(e => {
      const r = e.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
      return e === hit || e.contains(hit)
    }, element), 10000, 'Control is obscured or disabled: ' + selector)
    await element.click()
  }
  async function fill(selector, value) {
    const element = await find(selector)
    await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), element)
    await element.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE)
    if (value) await element.sendKeys(value)
  }
  const close = () => click('[role="dialog"] .modal-no-glass > button')
  const text = async selector => (await find(selector)).getAttribute('textContent')
  const sql = query => JSON.parse(execFileSync('python', ['-X', 'utf8', '-c',
    'import sqlite3,json,sys\nc=sqlite3.connect(sys.argv[1]);c.row_factory=sqlite3.Row\nprint(json.dumps([dict(r) for r in c.execute(sys.argv[2])],ensure_ascii=False))',
    path.join(dataDirectory, 'data.db'), query], { encoding: 'utf8', windowsHide: true }))

  async function shot(name, anchor, sizes = [[1100, 700], [900, 600], [760, 520]]) {
    for (const [width, height] of sizes) {
      if (report.layouts.some(r => r.name === name && r.outerWidth === width)) continue
      await driver.manage().window().setRect({ width, height })
      await find(anchor)
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
      await driver.executeScript(selector => {
        const modal = document.querySelector('[role="dialog"]')
        if (!modal) document.querySelector('.module-page').scrollTop = 0
        else document.querySelector(selector).scrollIntoView({ block: 'center', behavior: 'instant' })
      }, anchor)
      const layout = await driver.executeScript(() => {
        const page = document.querySelector('.module-page'), modal = [...document.querySelectorAll('.modal-no-glass')].at(-1)
        const titles = [...document.querySelectorAll('.sidebar-nav-label')].map(e => ({ text: e.textContent.trim(), width: e.getBoundingClientRect().width, height: e.getBoundingClientRect().height, overflow: e.scrollWidth - e.clientWidth }))
        const bounds = modal?.getBoundingClientRect()
        return { width: innerWidth, height: innerHeight, pageOverflow: page.scrollWidth - page.clientWidth, documentOverflow: document.documentElement.scrollWidth - innerWidth, titles,
          modal: bounds ? { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height, overflow: modal.scrollWidth - modal.clientWidth } : null }
      })
      assert.ok(layout.pageOverflow <= 1 && layout.documentOverflow <= 1, JSON.stringify(layout))
      assert.equal(layout.titles.length, 11)
      for (const title of layout.titles) assert.ok(title.text && title.width > 0 && title.height > 0 && title.overflow <= 1, JSON.stringify(title))
      if (layout.modal) assert.ok(layout.modal.x >= 0 && layout.modal.y >= 0 && layout.modal.x + layout.modal.width <= layout.width + 1 && layout.modal.y + layout.modal.height <= layout.height + 1 && layout.modal.overflow <= 1, JSON.stringify(layout))
      const file = `${name}-${width}x${height}.png`
      writeFileSync(path.join(base, file), Buffer.from(await driver.takeScreenshot(), 'base64'))
      report.layouts.push({ name, outerWidth: width, outerHeight: height, ...layout }); report.screenshots.push(file); save()
    }
    await driver.manage().window().setRect({ width: 1100, height: 700 })
  }

  if (!layoutOnly) execFileSync('python', ['-X', 'utf8', '-c', `import pathlib,zipfile,sys
p=pathlib.Path(sys.argv[1]);left=p/'项目资料（本地文件夹）';left.mkdir(exist_ok=True)
files={'文档/项目说明.txt':'shared-content','文档/合同草稿-需要核对.txt':'left-content','素材/这是一个比较长的资料名称-用于检查窄窗口排版.txt':'long-name-content'}
for name,content in files.items():
 f=left/name;f.parent.mkdir(parents=True,exist_ok=True);f.write_text(content,encoding='utf8')
for filename in ['项目资料-交付归档.zip','项目资料-历史备份.zip']:
 with zipfile.ZipFile(p/filename,'w',zipfile.ZIP_DEFLATED) as z:
  for name,content in files.items():z.writestr(name,content if '合同' not in name else 'right-content')
  z.writestr('文档/交付清单.md','delivered-list')
`, work], { windowsHide: true })
  const archives = ['项目资料-交付归档.zip', '项目资料-历史备份.zip'].map(name => path.join(work, name))
  const left = path.join(work, '项目资料（本地文件夹）')

  await driver.manage().setTimeouts({ script: 120000 })
  await driver.manage().window().setRect({ width: 1100, height: 700 })
  if (layoutOnly) {
    // The functional receipt belongs to the first binary; retain it and only
    // recapture the adjusted layouts on the final binary. Use the real settings
    // controls to check the dark/emerald theme from the user's screenshot.
    await click('[data-testid="nav-Settings"]')
    await click('.settings-theme-options button:nth-child(2)')
    await click('.settings-accent-options button[title="emerald"]')
    await driver.wait(() => driver.executeScript(() => document.documentElement.classList.contains('dark')), 10000)
    await click('[data-testid="nav-ArchiveLibrary"]')
    await click('[data-testid="library-search-tab"]')
    await fill('[data-testid="library-query"]', '项目说明')
    await driver.wait(async () => (await text('.results-caption')).includes('找到 2 个文件'), 10000)
    const filters = await find('[data-testid="library-search-filters"]')
    if (await filters.getAttribute('open') !== null) await click('[data-testid="library-search-filters"] > summary')
    await shot('final-library-search', '[data-testid="library-query"]')
    await click('[data-testid="library-sources-tab"]')
    if (await (await find('.source-summary')).getAttribute('aria-expanded') !== 'true') await click('.source-summary')
    await shot('final-library-sources', '.source-details', [[1100, 700], [760, 520]])
    await click('[data-testid="library-search-tab"]')
    await click('[data-testid="library-select-page"]')
    await click('[data-testid="library-batch-retrieval"]')
    await shot('final-library-batch', '[data-testid="library-retrieval"]', [[760, 520]])
    await close()
    await click('[data-testid="nav-Workflows"]')
    await click('[data-testid="workflow-edit-tab"]')
    await shot('final-workflow-editor', '[data-testid="workflow-plan-name"]')
    await click('[data-testid="workflow-new-open"]')
    await shot('final-workflow-templates', '[data-testid="workflow-templates"]', [[760, 520]])
    await close()
    await click('[data-testid="workflow-runs-tab"]')
    await shot('final-workflow-results', '[data-testid="workflow-run-details"]', [[1100, 700], [760, 520]])
    await click('[data-testid="workflow-directory-tab"]')
    await shot('final-workflow-automation', '[data-testid="workflow-queue-tab"]', [[1100, 700], [760, 520]])
    // The revised "add schedule" action switches to its own rule list.
    await click('[data-testid="workflow-schedule-new"]')
    await close()
    assert.equal(await (await find('[data-testid="workflow-schedules-tab"]')).getAttribute('aria-pressed'), 'true')
    await click('[data-testid="nav-SourceComparison"]')
    await bridge('queueDesktopDialogSelections', [left]); await click('[data-comparison-folder="left"]')
    await driver.wait(async () => (await text('[data-comparison-source="left"]')).includes('完整清单已读取'), 15000)
    await bridge('queueDesktopDialogSelections', [archives[0]]); await click('[data-comparison-zip="right"]')
    await find('[data-testid="comparison-results"]')
    await shot('final-comparison-results', '[data-testid="comparison-results"]')
    await click('[data-comparison-expand="文档/合同草稿-需要核对.txt"]')
    await click('[data-comparison-select="left:文档/合同草稿-需要核对.txt"]')
    await click('[data-comparison-select="right:文档/合同草稿-需要核对.txt"]')
    await click('[data-testid="comparison-retrieve-open"]')
    await shot('final-comparison-batch', '[role="dialog"] .modal-no-glass', [[760, 520]])
    await close()
    done('final-dark-layouts-and-affected-secondary-controls')
  }

  if (!layoutOnly && !passed('library-search-management-inline-source-saved-search-and-batch-dialog')) {
  await click('[data-testid="nav-ArchiveLibrary"]')
  await find('[data-testid="library-query"]')
  assert.equal((await driver.findElements(By.css('.source-summary'))).length, 0, 'Management must not precede search on entry')
  if (!sql("SELECT * FROM archive_library_sources WHERE name='项目资料-交付归档.zip'").length) {
    await bridge('queueDesktopDialogSelections', [archives]); await click('[data-testid="library-add-files"]')
    await find('.source-summary'); await click('[data-testid="library-index-pending"]')
    await driver.wait(() => sql('SELECT * FROM archive_library_sources').filter(r => r.status === 'ready').length === 2, 30000)
  } else await click('[data-testid="library-sources-tab"]')
  await click('.source-summary')
  await find('.source-details')
  assert.equal(await (await find('[data-testid="library-maintenance"]')).getAttribute('open'), null)
  await shot('library-sources', '.source-details')
  await click('[data-testid="library-search-tab"]')
  await fill('[data-testid="library-query"]', '项目说明')
  await driver.wait(async () => (await text('.results-caption')).includes('找到 2 个文件'), 10000)
  assert.equal(await (await find('[data-testid="library-search-filters"]')).getAttribute('open'), null)
  await shot('library-search', '[data-testid="library-query"]')
  await click('[data-testid="library-select-page"]')
  await click('[data-testid="library-sources-tab"]'); await click('[data-testid="library-search-tab"]')
  assert.match(await text('.library-selection'), /已选 2 个文件/)
  await click('[data-testid="library-batch-retrieval"]')
  await shot('library-batch-dialog', '[data-testid="library-retrieval"]', [[1100, 700], [760, 520]])
  await close()
  await click('[data-testid="library-search-filters"] > summary')
  await find('[data-testid="library-category"]')
  await click('[data-testid="library-save-search"]')
  await fill('[data-testid="library-search-name"]', '常用项目说明')
  await click('[data-testid="library-confirm-save-search"]')
  await driver.wait(async () => (await text('[data-testid="library-saved-searches"]')).includes('常用项目说明'), 10000)
  done('library-search-management-inline-source-saved-search-and-batch-dialog')
  }

  if (!layoutOnly && !passed('workflow-template-plan-dialog-inline-settings-and-real-two-step-run')) {
  await click('[data-testid="nav-Workflows"]')
  await click('[data-testid="workflow-new-open"]')
  await shot('workflow-templates', '[data-testid="workflow-templates"]', [[1100, 700], [760, 520]])
  await click('[data-testid="workflow-template-repack"]')
  await fill('[data-testid="workflow-plan-name"]', '交付资料整理')
  await click('[data-testid="workflow-save"]')
  await driver.wait(() => sql("SELECT * FROM workflow_plans WHERE name='交付资料整理'").length > 0, 10000)
  await shot('workflow-editor', '[data-testid="workflow-plan-name"]')
  await click('[data-workflow-step="1"]')
  await shot('workflow-inline-settings', '[data-testid="compression-filename-input"]', [[1100, 700], [760, 520]])
  await click('[data-workflow-step="1"]')
  await click('[data-testid="workflow-plans-open"]')
  await shot('workflow-plans', '[data-testid="workflow-plan-list"]', [[1100, 700], [760, 520]])
  await click('[data-workflow-plan="交付资料整理"]')
  const output = path.join(work, '工作流输出'); mkdirSync(output, { recursive: true })
  await click('[data-testid="workflow-run-open"]')
  await bridge('queueDesktopDialogSelections', [[archives[0]]]); await click('[data-testid="workflow-choose-files"]')
  await fill('[data-testid="workflow-output"]', output)
  await click('[data-testid="workflow-run-confirm"]')
  const run = await driver.wait(() => {
    const row = sql('SELECT record FROM workflow_runs ORDER BY started_at DESC LIMIT 1')[0]
    if (!row) return false
    const record = JSON.parse(row.record)
    return !['pending', 'running'].includes(record.status) && record
  }, 60000)
  assert.equal(run.status, 'completed', JSON.stringify(run))
  await find('[data-testid="workflow-run-details"]')
  await shot('workflow-results', '[data-testid="workflow-run-details"]')
  done('workflow-template-plan-dialog-inline-settings-and-real-two-step-run')
  }
  if (!layoutOnly && !passed('workflow-automation-queue-schedule-and-directory-secondary-views')) {
  await click('[data-testid="nav-Workflows"]')
  await click('[data-testid="workflow-directory-tab"]')
  assert.equal((await driver.findElements(By.css('.rule-list'))).length, 0)
  await shot('workflow-automation-queue', '[data-testid="workflow-queue-tab"]')
  await click('[data-testid="workflow-schedules-tab"]')
  await click('[data-testid="workflow-schedule-new"]')
  await find('[role="dialog"]')
  await shot('workflow-schedule-dialog', '[role="dialog"] .modal-no-glass', [[760, 520]])
  await close()
  await click('[data-testid="workflow-directories-tab"]')
  await click('[data-testid="workflow-rule-new"]')
  await find('[role="dialog"]'); await close()
  done('workflow-automation-queue-schedule-and-directory-secondary-views')
  }

  if (!layoutOnly && !passed('comparison-collapsed-sources-real-content-check-inline-details-and-multi-select-dialog')) {
  await click('[data-testid="nav-SourceComparison"]')
  await bridge('queueDesktopDialogSelections', [left]); await click('[data-comparison-folder="left"]')
  await driver.wait(async () => (await text('[data-comparison-source="left"]')).includes('完整清单已读取'), 15000)
  await bridge('queueDesktopDialogSelections', [archives[0]]); await click('[data-comparison-zip="right"]')
  await find('[data-testid="comparison-results"]')
  assert.equal(await (await find('[data-testid="comparison-source-picker"]')).getAttribute('open'), null)
  await shot('comparison-results', '[data-testid="comparison-results"]')
  await click('[data-testid="comparison-source-picker"] > summary')
  await shot('comparison-sources', '[data-comparison-source="right"]', [[1100, 700], [760, 520]])
  await click('[data-testid="comparison-source-picker"] > summary')
  await click('[data-testid="comparison-content-panel"] > summary')
  await click('[data-testid="comparison-verify-filtered"]')
  await driver.wait(async () => (await text('[data-testid="comparison-content-progress"]')).includes('本次核对完成'), 20000)
  await shot('comparison-content-check', '[data-testid="comparison-content-progress"]', [[1100, 700], [760, 520]])
  await click('[data-comparison-expand="文档/合同草稿-需要核对.txt"]')
  await find('[data-comparison-details="文档/合同草稿-需要核对.txt"]')
  await click('[data-comparison-select="left:文档/合同草稿-需要核对.txt"]')
  await click('[data-comparison-select="right:文档/合同草稿-需要核对.txt"]')
  await click('[data-testid="comparison-retrieve-open"]')
  await find('[role="dialog"]')
  await shot('comparison-selection-dialog', '[role="dialog"] .modal-no-glass', [[1100, 700], [760, 520]])
  await close()
  done('comparison-collapsed-sources-real-content-check-inline-details-and-multi-select-dialog')
  }
  report.completedAt = new Date().toISOString(); save()
  console.log(JSON.stringify({ checks: report.checks.length, layouts: report.layouts.length, binarySha256: report.binarySha256 }))
}

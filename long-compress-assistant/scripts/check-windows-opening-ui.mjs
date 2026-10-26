import { chromium } from '@playwright/test'
import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync } from 'node:fs'

// Browser integration evidence; native dialogs/IPC use deterministic fixtures.
// No installed associations, user files or real Explorer preferences are changed.
const base = process.env.WINDOWS_OPEN_UI_URL || 'http://127.0.0.1:1438'
const output = 'test-results/windows-opening-ui'
mkdirSync(output, { recursive: true })
const browser = await chromium.launch({ headless: true })
const checks = [], layouts = [], errors = []
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
  page.on('pageerror', error => errors.push(error.message))
  await page.addInitScript(() => {
    window.__calls = []
    window.__nextFolder = null
    window.__holdExtractions = false
    window.__pendingExtractions = []
    const entries = [
      { path: 'docs/', name: 'docs', isDir: true, size: 0 },
      { path: 'docs/notes.txt', name: 'notes.txt', isDir: false, size: 32 },
      { path: 'readme.txt', name: 'readme.txt', isDir: false, size: 32 },
      { path: 'cover.png', name: 'cover.png', isDir: false, size: 68 },
    ].map(entry => ({ ...entry, encrypted: false, compressedSize: entry.size, crc: '12345678', modified: '2026-10-26T10:00:00Z' }))
    const image = { dataUrl: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4e8AAAAASUVORK5CYII=', width: 1, height: 1, byteSize: 68, mimeType: 'image/png' }
    window.__TAURI_IPC__ = message => {
      window.__calls.push(message)
      if(message.cmd==='extract_file' && window.__holdExtractions){window.__pendingExtractions.push(message);return}
      let value
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'is_context_menu_registered') value = true
      if (message.cmd === 'take_pending_context_actions') value = []
      if (message.cmd === 'list_task_history_page') value = { records: [], nextCursor: null }
      if (message.cmd === 'archive_file_associations') value = ['zip', '7z', 'rar', 'zipx', 'tar', 'gz', 'bz2', 'xz', 'zst', 'alz', 'egg', 'zpaq'].map(extension => ({ extension: '.' + extension, registered: true, isDefault: extension === 'zip' }))
      if (message.cmd === 'get_archive_engine_capabilities') value = { available: true, fullEngine: true, formats: [{ name: 'zip', extensions: ['zip'], canCreate: true }], browseExtensions: ['zip', '7z', 'rar'], nestedExtensions: ['zip'], boundedPreviewFormats: ['ZIP'], boundedTextPreviewFormats: ['ZIP'], imagePreviewExtensions: ['png', 'jpg'], textPreviewExtensions: ['txt', 'md', 'json'], message: 'ready' }
      if (message.cmd === 'file_manager_locations') value = [{ name: '下载', path: 'C:\\Downloads', kind: 'drive' }, { name: '目标目录', path: 'D:\\Target', kind: 'drive' }]
      if (message.cmd === 'list_files') value = message.path.startsWith('D:') ? [{ path: 'D:\\Target\\目标资料', name: '目标资料', size: 0, is_dir: true }] : [{ path: 'C:\\Downloads\\docs', name: 'docs', size: 0, is_dir: true }, { path: 'C:\\Downloads\\bundle.zip', name: 'bundle.zip', size: 1024, is_dir: false, extension: 'zip' }, { path: 'C:\\Downloads\\notes.txt', name: 'notes.txt', size: 32, is_dir: false, extension: 'txt' }, { path: 'C:\\Downloads\\cover.png', name: 'cover.png', size: 68, is_dir: false, extension: 'png' }]
      if (message.cmd === 'browse_archive') value = { filePath: message.filePath, format: 'ZIP', totalFiles: 3, totalUncompressedSize: 132, totalCompressedSize: 1024, encrypted: false, revision: 'fixture', entries }
      if (message.cmd === 'preview_archive_text' || message.cmd === 'file_manager_preview_text') value = { content: '文本预览内容\n第二行', encoding: 'UTF-8', lineCount: 2, byteSize: 32, totalSize: 32, truncated: false }
      if (message.cmd === 'preview_archive_image' || message.cmd === 'file_manager_preview_image') value = image
      if (message.cmd === 'preview_extraction_output') value = `${message.outputPath}\\bundle`
      if (message.cmd === 'detect_split_archive') value = null
      if (message.cmd === 'preflight_operation_resources') value = { canStart: true, status: 'ready', warnings: [], outputPath: message.request?.outputPath || 'C:\\Downloads', estimatedOutputBytes: 132, availableBytes: 1000000000, requiredBytes: 132, reserveBytes: 0, estimateReliable: true }
      if (message.cmd === 'extract_file') value = message.outputPath || 'C:\\Downloads'
      if (message.cmd === 'get_file_info') value = { name: message.path?.split(/[\\/]/).pop(), size: 1024, is_dir: false }
      if (message.__tauriModule === 'Dialog' && message.message?.cmd === 'openDialog') value = window.__nextFolder
      window[`_${message.callback}`]?.(value)
    }
  })
  const app = async code => page.evaluate(async code => { const module = await import('/src/stores/app.ts'); return new Function('app', code)(module.useAppStore()) }, code)
  const shot = async name => {
    await page.evaluate(() => Promise.all(document.getAnimations().filter(animation=>animation.effect?.getTiming().iterations!==Infinity).map(animation=>animation.finished.catch(()=>{}))))
    await page.waitForFunction(()=>[...document.querySelectorAll('[role="dialog"], [role="dialog"] .app-dialog')].filter(el=>el.getClientRects().length).every(el=>Number(getComputedStyle(el).opacity)>=.99))
    await page.screenshot({ path: `${output}/${name}.png` })
  }
  await page.goto(`${base}/#/browser`, { waitUntil: 'domcontentloaded', timeout: 60000 })
  await page.locator('[data-file-pane="left"] .file-row').first().waitFor({ timeout: 60000 })
  const left = page.locator('[data-file-pane="left"]'), right = page.locator('[data-file-pane="right"]')
  await right.locator('.file-row').first().click()
  const otherPath = await right.locator('input[type=text]').inputValue()
  await shot('directory-1440x900')
  await left.locator('.file-row').filter({ hasText: 'bundle.zip' }).dblclick()
  await left.locator('[data-entry-path="readme.txt"]').waitFor()
  assert.equal(await right.locator('input[type=text]').inputValue(), otherPath)
  assert.equal(await right.locator('.file-row.selected').count(), 1)
  assert.equal(await left.locator('.file-head').innerText(), await right.locator('.file-head').innerText())
  assert.equal(await left.locator('.browser-navigation').count(), 0)
  checks.push('archive keeps pane, shared navigation/list and opposite selection')
  for (const [width, height] of [[1440, 900], [1100, 700], [760, 520]]) {
    await page.setViewportSize({ width, height })
    await page.evaluate(() => Promise.all(document.getAnimations().filter(animation=>animation.effect?.getTiming().iterations!==Infinity).map(animation=>animation.finished.catch(()=>{}))))
    const layout = await page.evaluate(() => {
      const box = el => { const r = el.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height, right: r.right, bottom: r.bottom } }
      return { overflow: document.documentElement.scrollWidth - innerWidth, panes: [...document.querySelectorAll('.file-pane')].map(el => ({ box: box(el), nav: box(el.querySelector('.pane-chrome')), head: box(el.querySelector('.file-head')), list: box(el.querySelector('.file-list')), footer: box(el.querySelector('footer')), overflow: el.scrollWidth - el.clientWidth })) }
    })
    assert.ok(layout.overflow <= 1, JSON.stringify(layout))
    assert.ok(layout.panes.every(pane => pane.overflow <= 1 && pane.list.height > 100), JSON.stringify(layout))
    assert.ok(Math.abs(layout.panes[0].head.y - layout.panes[1].head.y) < 2, JSON.stringify(layout))
    layouts.push({ width, height, ...layout }); await shot(`archive-${width}x${height}`)
  }
  await left.locator('[data-entry-path="readme.txt"]').click()
  assert.equal(await left.locator('.file-row.selected').count(), 1)
  await left.getByRole('button', { name: '预览 readme.txt', exact: true }).click()
  await page.getByTestId('archive-text-preview').waitFor(); await shot('archive-text-preview')
  await page.getByRole('button', { name: '关闭预览', exact: true }).click()
  await left.locator('[data-entry-path="cover.png"]').dblclick()
  await page.getByTestId('archive-image-preview').waitFor(); await page.getByRole('button', { name: '关闭预览', exact: true }).click()
  checks.push('single selection plus common text and image preview inside archive')
  await left.locator('[data-entry-path="docs/"]').dblclick()
  await left.locator('[data-entry-path="docs/notes.txt"]').waitFor()
  assert.ok((await left.locator('input[type=text]').inputValue()).endsWith('/docs'))
  await left.getByRole('button', { name: '上一级', exact: true }).click()
  await left.locator('[data-entry-path="readme.txt"]').waitFor()
  await left.getByRole('button', { name: '上一级', exact: true }).click()
  await left.locator('.file-row').filter({hasText:'bundle.zip'}).waitFor()
  await left.getByRole('button', { name: '前进', exact: true }).click()
  await left.locator('[data-entry-path="readme.txt"]').waitFor()
  checks.push('up exits archive to original directory and forward restores archive')
  await left.getByTestId('archive-pane-tools').click(); await shot('archive-options')
  await page.keyboard.press('Escape')
  await app("app.enqueueArchiveOpen(['C:/Downloads/bundle.zip'])")
  await page.getByTestId('archive-open-dialog').waitFor()
  assert.equal(await page.locator('.open-choice').count(), 4)
  await shot('four-open-actions-760x520')
  await page.getByTestId('archive-open-browse').click()
  await page.getByTestId('archive-open-dialog').waitFor({ state: 'hidden' })
  checks.push('four-action ask dialog opens dual pane directly')
  await app("app.updateSettings({archiveOpenBehavior:'ask'});app.enqueueArchiveOpen(['C:/Downloads/bundle.zip'])")
  await page.getByTestId('archive-open-choose-folder').click()
  assert.equal(await page.getByTestId('archive-open-dialog').count(), 1)
  await page.evaluate(() => { window.__nextFolder = 'E:/Chosen' })
  await page.getByTestId('archive-open-choose-folder').click()
  await page.getByTestId('archive-open-dialog').waitFor({ state: 'hidden' })
  await page.getByTestId('decompression-header').waitFor()
  await page.waitForFunction(()=>window.__calls.some(call=>call.cmd==='extract_file' && call.outputPath==='E:/Chosen'))
  checks.push('cancel folder picker keeps ask; chosen folder enters extraction center')
  await page.goto(`${base}/#/settings`, { waitUntil: 'domcontentloaded' })
  await page.locator('[data-settings-section="behavior"]').click()
  await page.getByTestId('archive-opening-settings').scrollIntoViewIfNeeded()
  for(const button of await page.getByRole('button',{name:/^关闭.*提示$/}).all())await button.click()
  await shot('opening-settings-760x520')
  await page.getByTestId('archive-association-configure').click()
  await page.getByTestId('archive-association-formats').waitFor(); await shot('association-formats-760x520')
  checks.push('settings group and format selection fit small window')
  await page.getByRole('dialog').getByRole('button',{name:'取消',exact:true}).click()
  await page.getByTestId('archive-open-behavior').click()
  assert.equal(await page.getByRole('option').count(),5)
  await page.getByRole('option',{name:'在双栏浏览器中查看',exact:true}).click()
  assert.equal(await page.evaluate(()=>JSON.parse(localStorage.getItem('app-settings')).archiveOpenBehavior),'browse')
  await app("app.enqueueArchiveOpen(['C:/Downloads/bundle.zip'])")
  await page.getByTestId('archive-pane-browser').waitFor()
  assert.equal(await page.getByTestId('archive-open-dialog').count(),0)
  checks.push('choosing default browse in settings persists and bypasses the ask dialog')
  await app("app.theme='obsidian';app.accentColor='#d9b76f'")
  await page.waitForFunction(()=>{const el=document.querySelector('.file-manager');return el && el.getBoundingClientRect().height>200 && !el.closest('[style*="display: none"]')})
  await shot('archive-obsidian-760x520')
  await app("app.updateSettings({archiveOpenBehavior:'ask'});app.enqueueArchiveOpen(['C:/Downloads/bundle.zip'])")
  await page.getByTestId('archive-open-dialog').waitFor();await shot('four-open-actions-obsidian-760x520')
  await page.getByRole('dialog').getByRole('button',{name:'Close dialog',exact:true}).click()
  await page.evaluate(()=>{window.__holdExtractions=true})
  await app("app.updateSettings({archiveOpenBehavior:'here'});app.enqueueArchiveOpen(['E:/first.zip'])")
  await page.waitForFunction(()=>window.__pendingExtractions.length===1)
  await page.getByTestId('nav-Settings').click()
  await page.locator('[data-settings-section="behavior"]').waitFor()
  await app("app.enqueueArchiveOpen(['F:/second.zip'])")
  await page.getByTestId('decompression-header').waitFor()
  assert.equal(await page.evaluate(()=>window.__pendingExtractions.length),1)
  await page.evaluate(()=>{const message=window.__pendingExtractions.shift();window[`_${message.callback}`](message.outputPath)})
  await page.waitForFunction(()=>window.__pendingExtractions.length===1 && window.__pendingExtractions[0].filePath==='F:/second.zip')
  await page.evaluate(()=>{const message=window.__pendingExtractions.shift();window[`_${message.callback}`](message.outputPath);window.__holdExtractions=false})
  checks.push('running extraction survives settings navigation; next system opening runs after it completes')
  assert.deepEqual(errors, [], JSON.stringify(errors))
  console.log(JSON.stringify({ checks, layouts, errors }))
} finally {
  writeFileSync(`${output}/result.json`, JSON.stringify({ fixtureNativeIPC: true, checks, layouts, errors }, null, 2))
  await browser.close()
}

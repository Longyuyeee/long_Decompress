import { chromium } from '@playwright/test'
import { readFile, mkdir } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const directory = path.dirname(fileURLToPath(import.meta.url))
await mkdir(directory, { recursive: true })
const browser = await chromium.launch({ headless: true })
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 1000 }, deviceScaleFactor: 1.5, colorScheme: 'light' })
  await page.addInitScript(() => {
    window.__TAURI_IPC__ = message => {
      let result
      if (message.cmd === 'load_app_settings') result = JSON.stringify({ theme: 'light', autoCheckUpdates: false })
      else if (message.cmd === 'get_archive_engine_capabilities') result = { available: true, fullEngine: true, formats: [], browseExtensions: [], nestedExtensions: [], boundedPreviewFormats: [], imagePreviewExtensions: [], textPreviewExtensions: [], message: '' }
      else if (message.cmd === 'check_rar_compression_support') result = { available: false, message: '未检测到本机 RAR 编码器' }
      else if (message.cmd === 'check_auto_start') result = false
      else if (message.cmd === 'is_context_menu_registered') result = true
      else if (message.cmd === 'tauri' && message.__tauriModule === 'App') result = '1.4.1'
      else if (message.cmd === 'load_history' || message.cmd === 'take_pending_context_actions') result = []
      window[`_${message.callback}`]?.(result)
    }
  })
  await page.goto('http://127.0.0.1:1438/#/settings')
  await page.waitForSelector('.settings-view')
  await page.evaluate(async () => {
    const module = await import('/src/stores/app.ts')
    const store = module.useAppStore()
    store.theme = 'light'
    store.language = 'zh-CN'
    store.accentColor = '#0ea5e9'
  })
  await page.evaluate(() => document.fonts.ready)
  await page.waitForTimeout(750)
  if (process.argv.includes('--reference')) {
    await page.screenshot({ path: path.join(directory, 'settings-current-reference.png') })
    await page.goto('http://127.0.0.1:1438/#/integrity')
    await page.waitForTimeout(600)
    await page.screenshot({ path: path.join(directory, 'existing-page-reference.png') })
  } else {
    await page.addStyleTag({ content: await readFile(path.join(directory, 'settings-preview.css'), 'utf8') })
    await page.waitForTimeout(350)
    await page.screenshot({ path: path.join(directory, 'settings-style-preview.png') })
    await page.locator('.settings-view > div').evaluate(element => {
      const lower = element.firstElementChild.children[2]
      element.scrollTop += lower.getBoundingClientRect().top - element.getBoundingClientRect().top
    })
    await page.screenshot({ path: path.join(directory, 'settings-style-details.png') })
    console.log('Static settings previews saved. Application source unchanged.')
  }
} finally {
  await browser.close()
}

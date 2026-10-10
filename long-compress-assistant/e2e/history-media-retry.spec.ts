import { test, expect } from '@playwright/test'

test('retry a PDF in its own directory, reanalyse and ask again before lossy processing', async ({ page }, info) => {
  await page.setViewportSize({ width: 920, height: 680 })
  await page.addInitScript(() => {
    const commands: string[] = []
    ;(window as any).retryCommands = commands
    window.__TAURI_IPC__ = (message: Record<string, any>) => {
      commands.push(message.cmd)
      let value: unknown
      if (message.cmd === 'load_app_settings') value = '{}'
      if (message.cmd === 'get_file_info') value = { name: message.path.split('/').pop(), size: 500, is_dir: !message.path.endsWith('.pdf') }
      if (message.cmd === 'list_task_history_page') value = { records: [{
        id: 'original-pdf', name: '报告.pdf', taskType: 'compression', workloadKind: 'pdf', status: 'failed',
        sourcePaths: ['C:/报告.pdf'], outputPath: 'D:/out/报告.optimized.pdf',
        completedAt: '2026-10-10T00:00:00Z', durationMs: 2, processedBytes: 0, totalBytes: 0, logs: [],
        errorMessage: '输出目录不可用', executionConfig: { schemaVersion: 1, kind: 'pdf-optimization',
          outputDirectory: 'D:/out', mode: 'compatible-image-optimization', allowLargerOutput: true },
      }], nextCursor: null }
      if (message.cmd === 'analyze_pdf_input') value = {
        source: message.request.path, inputBytes: 500, analysisComplete: true, pageCount: 2,
        encrypted: false, passwordState: 'not-required', hasDigitalSignature: false, signatureFieldNames: [],
        hasFormFields: false, formFieldNames: [], hasAttachments: false, attachmentNames: [],
        outlineCount: 0, warnings: [], blockingReasons: [],
      }
      ;(window as any)[`_${message.callback}`]?.(value)
    }
  })
  await page.goto('/#/history')
  await page.getByTestId('history-media-retry').click()
  await page.getByTestId('media-retry-output').fill('E:/新报告')
  await page.getByTestId('media-retry-create').click()
  await page.getByRole('link', { name: '前往对应工作区确认并启动' }).click()
  await expect(page.getByTestId('compression-mode-pdf')).toHaveAttribute('aria-current', 'page')
  await expect(page.getByTestId('pdf-risk-confirmation')).not.toBeChecked()
  await expect(page.getByTestId('pdf-allow-larger-output')).toBeChecked()
  await expect(page.getByTestId('pdf-retry-output')).toHaveValue('E:/新报告')
  await expect(page.getByTestId('pdf-output-preview')).toContainText('E:/新报告/报告.optimized.pdf')
  await expect(page.getByTestId('pdf-start-batch')).toBeDisabled()
  const commands = await page.evaluate(() => (window as any).retryCommands as string[])
  expect(commands.filter(cmd => cmd === 'analyze_pdf_input')).toHaveLength(1)
  expect(commands).not.toContain('compress_pdf_file')
  await page.getByTestId('pdf-risk-confirmation').scrollIntoViewIfNeeded()
  await page.screenshot({ path: info.outputPath('pdf-retry-confirmation.png') })
})

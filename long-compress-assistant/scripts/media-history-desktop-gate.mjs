import assert from 'node:assert/strict'
import { mkdirSync, readFileSync, readdirSync, writeFileSync, statSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
import { By } from 'selenium-webdriver'

export async function runMediaHistoryDesktopGate({ driver, bridge, waitForElement, fixtureDirectory, artifactDirectory, root, binarySha256 }) {
  const clickVisible = async element => {
    await driver.executeScript('arguments[0].scrollIntoView({block:"center",behavior:"instant"})', element)
    await driver.wait(() => driver.executeScript(el => {
      const r = el.getBoundingClientRect()
      return el.contains(document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2))
    }, element), 10_000, 'wait for the control to be reachable after drawer animation')
    await element.click()
  }
  const fixtures = path.join(root, 'test-results/media-fixture-audit/fixtures')
  const cases = [
    { kind: 'image', source: path.join(fixtures, 'images/large-photo.jpg'), config: { schemaVersion: 1, kind: 'image-compression', settings: {
      mode: 'lossy', quality: 70, resizeMode: 'limit', maxWidth: 800, maxHeight: 600, outputFormat: 'webp', preserveMetadata: true, outputDirectory: '', conflictPolicy: 'rename',
    } } },
    { kind: 'video', source: path.join(fixtures, 'videos/h265.mp4'), config: { schemaVersion: 1, kind: 'video-compression', outputDirectory: '', settings: { preset: 'balanced', quality: 70, maxWidth: 640, maxHeight: 480 } } },
    { kind: 'pdf', source: path.join(fixtures, 'pdfs/text-vector.pdf'), config: { schemaVersion: 1, kind: 'pdf-optimization', outputDirectory: '', mode: 'lossless-organization', allowLargerOutput: true } },
  ]
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const results = []
  for (const item of cases) {
    const output = path.join(fixtureDirectory, `retry-${item.kind}`)
    mkdirSync(output)
    const sourceHash = hash(item.source)
    if (item.kind === 'image') item.config.settings.outputDirectory = output
    else item.config.outputDirectory = output
    const original = { id: `prior-${item.kind}`, name: `历史重试-${item.kind}`, taskType: 'compression', workloadKind: item.kind, status: 'failed',
      sourcePaths: [item.source], outputPath: '', completedAt: new Date().toISOString(), durationMs: 1,
      processedBytes: 0, totalBytes: 0, logs: [], errorMessage: '旧输出目录不可用', executionConfig: item.config }
    await bridge('seedHistoryRecords', [original])
    await (await waitForElement('[data-testid="nav-History"]')).click()
    const record = await driver.wait(async () => {
      const rows = await driver.findElements(By.css('[data-testid="history-record-row"]'))
      for (const row of rows) if ((await row.getText()).includes(original.name)) return row
      return false
    }, 15_000)
    await (await record.findElement(By.css('[data-testid="history-media-retry"]'))).click()
    await clickVisible(await waitForElement('[data-testid="media-retry-create"]'))
    const link = await waitForElement('[data-testid="history-media-draft"] a')
    assert.deepEqual(readdirSync(output), [], 'media draft must not encode automatically')
    await clickVisible(link)
    const workspace = await waitForElement(`[data-testid="${item.kind}-compression-workspace"]`)
    if (item.kind === 'pdf') {
      const freeze = await waitForElement('[data-testid="pdf-freeze-configuration"]')
      await driver.wait(() => freeze.isEnabled(), 30_000)
      await clickVisible(freeze)
    }
    const start = await workspace.findElement(By.css('.workspace-toolbar .primary-action'))
    await driver.wait(() => start.isEnabled(), 30_000)
    await clickVisible(start)
    const completed = await driver.wait(async () => {
      const row = (await bridge('taskHistory')).find(row => row.retryOfTaskId === original.id)
      if (row?.status === 'failed') throw new Error(JSON.stringify(row))
      return row?.status === 'completed' ? row : false
    }, 120_000)
    assert.equal(completed.workloadKind, item.kind)
    assert.equal(path.dirname(completed.outputPath), output)
    assert.ok(statSync(completed.outputPath).size > 0)
    assert.equal(hash(item.source), sourceHash, 'retry must preserve the source')
    assert.deepEqual(completed.executionConfig, item.config)
    assert.equal((await bridge('taskHistory')).find(row => row.id === original.id).status, 'failed')
    results.push({ kind: item.kind, taskId: completed.id, sourceHash, outputHash: hash(completed.outputPath), outputBytes: statSync(completed.outputPath).size, metrics: completed.metrics })
  }
  writeFileSync(path.join(artifactDirectory, 'media-history-user-result.json'), JSON.stringify({
    verifiedAt: new Date().toISOString(), binarySha256, testKind: 'real-ui-engine-sqlite-with-seeded-prior-record', results,
    nativePickerVerified: false,
  }, null, 2))
}

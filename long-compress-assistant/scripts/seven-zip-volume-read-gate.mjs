import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync, rmSync } from 'node:fs'
import path from 'node:path'
import { By } from 'selenium-webdriver'

// Independent archives enter the real extract_file command and its resource
// preflight/staging/commit path. This is not a UI-import or creation-UI test.
export async function runSevenZipVolumeReadGate({ driver, waitForElement, callDesktopBridge, callDesktopBridgeFailure, runFixtureCommand, bundledSevenZip, fixtureDirectory, artifactDirectory }) {
  const results = []
  for (const mode of ['plain', 'content', 'header']) {
    const root = path.join(fixtureDirectory, `7z-volume-${mode}`)
    const source = path.join(root, '资料')
    mkdirSync(path.join(source, '空目录'), { recursive: true })
    const payload = '中文密码分卷实际载荷\n'.repeat(100)
    writeFileSync(path.join(source, '内容.txt'), payload)
    writeFileSync(path.join(source, '空文件.txt'), '')
    const archive = path.join(root, '分享.7z')
    const password = mode === 'plain' ? null : '中文密码 + !测试'
    runFixtureCommand(bundledSevenZip, ['a', '-t7z', '-y', '-v37b', ...(password ? [`-p${password}`, `-mhe=${mode === 'header' ? 'on' : 'off'}`] : []), archive, '资料'], `7Z ${mode} volumes`, { cwd: root })
    const parts = readdirSync(root).filter(name => /^分享\.7z\.\d+$/.test(name)).sort().map(name => path.join(root, name))
    assert.ok(parts.length > 2)
    const snapshots = parts.map(file => readFileSync(file))
    const output = path.join(root, '输出')
    await callDesktopBridge('extractArchive', parts[0], output, password)
    assert.equal(readFileSync(path.join(output, '资料', '内容.txt'), 'utf8'), payload)
    assert.equal(readFileSync(path.join(output, '资料', '空文件.txt')).length, 0)
    assert.deepEqual(readdirSync(path.join(output, '资料', '空目录')), [])
    if (password) {
      const wrongOutput = path.join(root, '错误密码')
      const wrong = await callDesktopBridgeFailure('extractArchive', parts[0], wrongOutput, 'wrong')
      assert.ok(wrong.includes('[archive-inspection:ambiguous-data]'), wrong)
      assert.ok(!existsSync(path.join(wrongOutput, '资料', '内容.txt')))
      // Byte 40 lies in packed encrypted data, not the CRC-protected start header.
      const damaged = Buffer.from(snapshots[1]); damaged[3] ^= 0x5a
      writeFileSync(parts[1], damaged)
      try {
        const damagedOutput = path.join(root, 'damaged-payload-password')
        const failure = await callDesktopBridgeFailure('extractArchive', parts[0], damagedOutput, password)
        assert.ok(failure.includes('[archive-inspection:ambiguous-data]'), failure)
        assert.ok(!existsSync(path.join(damagedOutput, '资料', '内容.txt')))
        const history = (await callDesktopBridge('taskHistory')).find(record => record.outputPath === damagedOutput)
        assert.equal(history?.status, 'failed')
        assert.ok(history.errorMessage.includes('[archive-inspection:ambiguous-data]'), JSON.stringify(history))
      } finally {
        writeFileSync(parts[1], snapshots[1])
      }
    }
    rmSync(parts[1]) // Exact synthetic middle volume owned by this test.
    try {
      const missingOutput = path.join(root, '缺卷')
      const missing = await callDesktopBridgeFailure('extractArchive', parts[0], missingOutput, password)
      assert.match(missing, /incomplete|missing|缺|不完整/i)
      assert.ok(!existsSync(path.join(missingOutput, '资料', '内容.txt')))
    } finally {
      writeFileSync(parts[1], snapshots[1])
    }
    const diagnostics = []
    for (const fault of ['missing-tail', 'truncated-tail', 'damaged-header']) {
      const index = fault === 'damaged-header' ? 0 : parts.length - 1
      if (fault === 'missing-tail') rmSync(parts[index])
      else if (fault === 'truncated-tail') writeFileSync(parts[index], snapshots[index].subarray(0, -1))
      else {
        const damaged = Buffer.from(snapshots[index]); damaged[12] ^= 1
        writeFileSync(parts[index], damaged)
      }
      try {
        // Also reject structurally broken encrypted archives before asking for a password.
        for (const supplied of password ? [null, password] : [null]) {
          const target = path.join(root, `${fault}-${supplied ? 'password' : 'none'}`)
          const failure = await callDesktopBridgeFailure('extractArchive', parts[0], target, supplied)
          assert.ok(failure.includes(`[archive-inspection:${fault === 'damaged-header' ? 'damaged' : 'incomplete'}]`), failure)
          assert.ok(!existsSync(path.join(target, '资料', '内容.txt')))
          const history = (await callDesktopBridge('taskHistory')).find(record => record.outputPath === target)
          assert.ok(history, `Missing final history for ${target}`)
          assert.equal(history.status, 'failed')
          assert.ok(history.errorMessage?.includes(fault === 'damaged-header' ? '归档头校验失败' : '可能缺少尾卷'), JSON.stringify(history))
          diagnostics.push({ fault, passwordProvided: Boolean(supplied), error: history.errorMessage })
        }
      } finally {
        writeFileSync(parts[index], snapshots[index])
      }
    }
    assert.equal(readFileSync(path.join(source, '内容.txt'), 'utf8'), payload)
    parts.forEach((file, index) => assert.deepEqual(readFileSync(file), snapshots[index]))
    results.push({ mode, volumes: parts.length, correctOutput: true, wrongPasswordRejected: Boolean(password), missingVolumeRejected: true, diagnostics, sourceAndVolumesPreserved: true })
  }
  for (const [query, expected] of [['missing-tail-password', '可能缺少尾卷'], ['damaged-header-password', '归档头校验失败'], ['damaged-payload-password', '不能确定为密码错误']]) {
    await driver.navigate().refresh()
    await (await waitForElement('[data-testid="nav-History"]')).click()
    await (await waitForElement('[data-testid="history-search"]')).sendKeys(query)
    await driver.wait(async () => (await driver.findElements(By.css('[data-testid="history-record-row"]'))).length === 2, 10_000)
    await (await waitForElement('[data-testid="history-record-row"]')).click()
    const finalReason = await waitForElement('[data-testid="history-final-failure"]')
    await driver.wait(async () => driver.executeScript(() => document.getAnimations().every(animation => animation.playState !== 'running')), 10_000)
    const displayedReason = await finalReason.getText()
    assert.ok(displayedReason.includes(expected), displayedReason)
    if (query !== 'damaged-payload-password') assert.ok(!displayedReason.includes('密码错误'), displayedReason)
    writeFileSync(path.join(artifactDirectory, `seven-zip-${query}-history.png`), Buffer.from(await driver.takeScreenshot(), 'base64'))
  }
  await driver.navigate().refresh()
  await waitForElement('[data-testid="nav-History"]')
  writeFileSync(path.join(artifactDirectory, 'seven-zip-volume-read.json'), JSON.stringify({ via: 'isolated-desktop-production-extract-command', uiImportVerified: false, results }, null, 2))
}

import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync, rmSync } from 'node:fs'
import path from 'node:path'

// Independent archives enter the real extract_file command and its resource
// preflight/staging/commit path. This is not a UI-import or creation-UI test.
export async function runSevenZipVolumeReadGate({ callDesktopBridge, callDesktopBridgeFailure, runFixtureCommand, bundledSevenZip, fixtureDirectory, artifactDirectory }) {
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
      assert.match(wrong, /密码|password/i)
      assert.ok(!existsSync(path.join(wrongOutput, '资料', '内容.txt')))
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
    assert.equal(readFileSync(path.join(source, '内容.txt'), 'utf8'), payload)
    parts.forEach((file, index) => assert.deepEqual(readFileSync(file), snapshots[index]))
    results.push({ mode, volumes: parts.length, correctOutput: true, wrongPasswordRejected: Boolean(password), missingVolumeRejected: true, sourceAndVolumesPreserved: true })
  }
  writeFileSync(path.join(artifactDirectory, 'seven-zip-volume-read.json'), JSON.stringify({ via: 'isolated-desktop-production-extract-command', uiImportVerified: false, results }, null, 2))
}

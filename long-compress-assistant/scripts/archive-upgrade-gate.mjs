import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, existsSync, readdirSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { randomBytes, createHash } from 'node:crypto'
import path from 'node:path'

// Focused real Tauri command coverage for 1.4.3; no unrelated install/media matrix.
export async function runArchiveUpgradeGate({ driver, bridge, failure, fixtureDirectory, artifactDirectory, bundledSevenZip }) {
  const root = path.join(fixtureDirectory, 'archive-upgrade')
  const source = path.join(root, '原始资料')
  mkdirSync(path.join(source, '中文目录', '空目录'), { recursive: true })
  const bytes = randomBytes(128 * 1024 + 31)
  writeFileSync(path.join(source, '中文目录', '原文件.bin'), bytes)
  writeFileSync(path.join(source, '说明.txt'), '分卷、转换、中文密码与空目录应保留。')
  const original = path.join(root, '原包.zip')
  const created = spawnSync(bundledSevenZip, ['a', '-tzip', original, '.\\*'], { cwd: source, encoding: 'utf8', windowsHide: true })
  assert.equal(created.status, 0, created.stderr)
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const originalHash = hash(original)
  const records = []
  for (const format of ['rar', 'tar.lz4', 'tar.br', 'tar.gz']) {
    const output = path.join(root, `升级分卷.${format}`)
    const password = format === 'rar' ? '中文 密码+!🔐' : null
    const result = await bridge('convertArchive', original, null, output, {
      format, level: 3, password, split_size: 32768, encrypt_filenames: format === 'rar', allow_insecure_password_cli: format === 'rar',
    })
    assert.equal(result.split, true); assert.ok(result.files.length > 1)
    assert.ok(existsSync(result.primaryPath)); assert.equal(result.files[0].path, result.primaryPath)
    assert.ok(result.files.every(file => file.bytes > 0 && existsSync(file.path)))
    if (format === 'rar') {
      const wrong = await failure('extractArchive', result.primaryPath, path.join(root, '错误密码'), 'wrong-password')
      assert.match(wrong, /password|密码/i)
    }
    const restored = path.join(root, `还原-${format}`)
    await bridge('extractArchive', result.primaryPath, restored, password)
    assert.deepEqual(readFileSync(path.join(restored, '中文目录', '原文件.bin')), bytes)
    assert.deepEqual(readFileSync(path.join(restored, '说明.txt')), readFileSync(path.join(source, '说明.txt')))
    assert.ok(existsSync(path.join(restored, '中文目录', '空目录')))
    assert.deepEqual(readdirSync(restored).sort(), ['中文目录', '说明.txt'].sort())
    const merged = path.join(root, `合卷-${format}.zip`)
    const converted = await bridge('convertArchive', result.primaryPath, password, merged, { format: 'zip', password: null })
    assert.equal(converted.split, false)
    const mergedRestored = path.join(root, `合卷还原-${format}`)
    await bridge('extractArchive', merged, mergedRestored, null)
    assert.deepEqual(readFileSync(path.join(mergedRestored, '中文目录', '原文件.bin')), bytes)
    records.push({ format, volumes: result.files.length, totalBytes: result.totalBytes, payloadSha256: hash(path.join(restored, '中文目录', '原文件.bin')) })
  }
  for (const format of ['lz4', 'br']) {
    const payload = path.join(root, `单文件-${format}.txt`)
    writeFileSync(payload, bytes)
    const archive = payload + '.' + format
    const destination = path.join(root, '单文件还原-' + format)
    await bridge('runArchiveRoundTrip', payload, archive, destination, format, null)
    assert.deepEqual(readFileSync(path.join(destination, path.basename(payload))), bytes)
    records.push({ format, payloadSha256: hash(path.join(destination, path.basename(payload))) })
  }
  assert.equal(hash(original), originalHash)
  await driver.takeScreenshot().then(base64 => writeFileSync(path.join(artifactDirectory, 'archive-upgrade-desktop.png'), Buffer.from(base64, 'base64')))
  writeFileSync(path.join(artifactDirectory, 'archive-upgrade-results.json'), JSON.stringify({ version: '1.4.3', originalHash, records }, null, 2))
}

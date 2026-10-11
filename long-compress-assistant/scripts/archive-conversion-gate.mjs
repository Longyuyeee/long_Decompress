import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { randomBytes, createHash } from 'node:crypto'
import path from 'node:path'

export async function runArchiveConversionGate({ bridge, failure, fixtureDirectory, artifactDirectory, bundledSevenZip }) {
  const root = path.join(fixtureDirectory, 'conversion')
  const source = path.join(root, '原始文件')
  mkdirSync(path.join(source, '中文目录', '空目录'), { recursive: true })
  const payload = randomBytes(160000)
  writeFileSync(path.join(source, '中文目录', '内容.bin'), payload)
  writeFileSync(path.join(source, '说明.txt'), '转换应保留内容与目录，不是把旧包再次打包。')
  const run = (args, ok = true) => {
    const result = spawnSync(bundledSevenZip, args, { cwd: source, encoding: 'utf8', windowsHide: true })
    if (ok) assert.equal(result.status, 0, result.stdout + result.stderr)
    else assert.notEqual(result.status, 0, 'Old password unexpectedly accepted')
  }
  const original = path.join(root, '原包.zip')
  run(['a', '-tzip', '-pold-password', '-mem=AES256', original, '.\\*'])
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  const originalHash = hash(original)
  const target = path.join(root, '改密分卷.7z')
  const result = await bridge('convertArchive', original, 'old-password', target, {
    format: '7z', password: 'new-password', encrypt_filenames: true, split_size: 65536,
  })
  assert.equal(result.split, true)
  assert.ok(result.files.length > 1)
  run(['t', result.primaryPath, '-pold-password'], false)
  const restored = path.join(root, '还原')
  run(['x', result.primaryPath, '-pnew-password', `-o${restored}`, '-y'])
  assert.deepEqual(readFileSync(path.join(restored, '中文目录', '内容.bin')), payload)
  assert.deepEqual(readFileSync(path.join(restored, '说明.txt')), readFileSync(path.join(source, '说明.txt')))
  assert.ok(existsSync(path.join(restored, '中文目录', '空目录')))
  assert.deepEqual(readdirSync(restored).sort(), ['中文目录', '说明.txt'].sort())
  const plain = path.join(root, '去密合卷.zip')
  const merged = await bridge('convertArchive', result.primaryPath, 'new-password', plain, { format: 'zip', password: null })
  assert.equal(merged.split, false)
  const plainRestored = path.join(root, '去密还原')
  run(['x', plain, `-o${plainRestored}`, '-y'])
  assert.deepEqual(readFileSync(path.join(plainRestored, '中文目录', '内容.bin')), payload)
  assert.ok(existsSync(path.join(plainRestored, '中文目录', '空目录')))
  assert.deepEqual(readFileSync(path.join(plainRestored, '说明.txt')), readFileSync(path.join(source, '说明.txt')))
  const wrongOutput = path.join(root, '错误密码.7z')
  assert.match(await failure('convertArchive', original, 'wrong-password', wrongOutput, { format: '7z' }), /转换读取原包失败/)
  assert.equal(existsSync(wrongOutput), false)
  assert.match(await failure('convertArchive', original, 'old-password', original, { format: 'zip' }), /不能覆盖/)
  const corrupt = path.join(root, '损坏.zip')
  writeFileSync(corrupt, readFileSync(original).subarray(0, 50))
  const corruptOutput = path.join(root, '损坏转换.7z')
  assert.match(await failure('convertArchive', corrupt, null, corruptOutput, { format: '7z' }), /转换读取原包失败/)
  assert.equal(existsSync(corruptOutput), false)
  assert.equal(hash(original), originalHash)
  assert.deepEqual(readdirSync(root).filter(name => name.startsWith('.long-convert-')), [])
  writeFileSync(path.join(artifactDirectory, 'archive-conversion.json'), JSON.stringify({
    sourcePreserved: true, contentVerified: true, emptyDirectoryPreserved: true,
    passwordChanged: true, resplit: true, mergedAndDecrypted: true, wrongPasswordRejected: true,
    stagingCleaned: true, corruptInputRejected: true, scope: 'real backend command, not UI acceptance',
  }, null, 2))
}

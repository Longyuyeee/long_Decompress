// Original RAR4 output, not renamed RAR5 files. The old encoder is fixture-only.
import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, statSync, existsSync } from 'node:fs'
import { randomBytes, createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'

const root = path.resolve(process.argv[2] || 'test-results/rar4-1419-original')
const tool = path.resolve(process.env.LONG_RAR4_FIXTURE_TOOL || 'test-results/rar4-1419-tools/rar624/Rar.exe')
const source = path.join(root, 'source')
mkdirSync(path.join(source, '中文目录', '空目录'), { recursive: true })
const payload = path.join(source, '中文目录', '原文件.bin')
if (!existsSync(payload)) writeFileSync(payload, randomBytes(180_123))
writeFileSync(path.join(source, '说明.txt'), 'RAR4 旧格式恢复测试：保留原件、密码和文件内容。\n'.repeat(12))
const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
const run = args => {
  const result = spawnSync(tool, args, { cwd: source, windowsHide: true, timeout: 60000, encoding: 'utf8' })
  assert.ifError(result.error)
  assert.equal(result.status, 0, JSON.stringify({ args, stdout: result.stdout, stderr: result.stderr }))
  return result.stdout
}
const outputs = []
for (const style of ['single', 'old', 'modern']) {
  for (const mode of ['none', 'content', 'headers']) {
    const name = `${style}-${mode}`
    const dir = path.join(root, name)
    mkdirSync(dir, { recursive: true })
    const output = path.join(dir, '原始归档.rar')
    const passwordArgs = mode === 'none' ? [] : [`-${mode === 'headers' ? 'hp' : 'p'}Rar4-1419-中文!`]
    const args = ['a', '-ma4', '-m0', '-idq', '-cfg-', '-rr5p', '-r',
      ...(style === 'single' ? [] : ['-v60000b']), ...(style === 'old' ? ['-vn'] : []), ...passwordArgs, output, '.\\*']
    if (!readdirSync(dir).length) {
      run(args)
      if (style !== 'single') run(['rv2', '-idq', '-cfg-', ...passwordArgs.map(p => p.replace('-hp', '-p')), style === 'modern' ? path.join(dir, '原始归档.part1.rar') : output])
    }
    const first = style === 'modern' ? path.join(dir, '原始归档.part1.rar') : output
    assert.deepEqual([...readFileSync(first).subarray(0, 7)], [82, 97, 114, 33, 26, 7, 0], 'must be native RAR4 magic')
    const listing = run(['lt', '-cfg-', ...passwordArgs.map(p => p.replace('-hp', '-p')), first])
    run(['t', '-idq', '-cfg-', ...passwordArgs.map(p => p.replace('-hp', '-p')), first])
    outputs.push({ name, style, mode, args, listing, files: readdirSync(dir).map(name => ({ name, bytes: statSync(path.join(dir, name)).size, sha256: hash(path.join(dir, name)) })) })
  }
}
const noRecord = path.join(root, 'no-record'); mkdirSync(noRecord, { recursive: true })
const noRecordPath = path.join(noRecord, '无恢复记录.rar')
if (!existsSync(noRecordPath)) run(['a', '-ma4', '-m0', '-idq', '-cfg-', '-r', noRecordPath, '.\\*'])
outputs.push({ name: 'no-record', files: readdirSync(noRecord).map(name => ({ name, bytes: statSync(path.join(noRecord, name)).size, sha256: hash(path.join(noRecord, name)) })) })
const manifest = { generatedAt: new Date().toISOString(), tool, toolSha256: hash(tool), toolVersion: run(['-iver']).trim(), publicPassword: 'Rar4-1419-中文!', source: [payload, path.join(source, '说明.txt')].map(file => ({ name: path.relative(source, file), bytes: statSync(file).size, sha256: hash(file) })), outputs }
writeFileSync(path.join(root, 'manifest.json'), JSON.stringify(manifest, null, 2))
console.log(JSON.stringify({ toolVersion: manifest.toolVersion, outputs: outputs.map(o => ({ name: o.name, files: o.files.map(f => f.name) })) }, null, 2))

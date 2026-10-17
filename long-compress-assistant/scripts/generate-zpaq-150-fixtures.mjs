import { mkdirSync, writeFileSync, readFileSync, utimesSync, unlinkSync, existsSync } from 'node:fs'
import { resolve, join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
const root = resolve('test-results/zpaq-150-fixtures')
mkdirSync(root, { recursive: true })
const tool = resolve('test-results/zpaq-150-tools/upstream/zpaq64.exe')
const source = join(root, '项目 文档')
mkdirSync(join(source, '空目录'), { recursive: true })
const expected = { '说明.txt': '第二版：当前文件视图，不混入旧版本。\r\n', '空文件.txt': '', '数据.bin': Buffer.from(Array.from({length: 32768}, (_, i) => i * 37 & 255)) }
const password = '中文 密码!-钥匙'
const invoke = (...args) => {
  const r = spawnSync(tool, args, { cwd: root, encoding: 'utf8' })
  if (r.status !== 0) throw new Error(`${r.status}: ${r.stdout}\n${r.stderr}`)
  return r.stdout
}
for (const encrypted of [false, true]) {
  const key = encrypted ? ['-key', password] : [] // public synthetic fixture credential, official generator only
  const archive = encrypted ? 'journal-key.zpaq' : 'journal.zpaq'
  if (existsSync(join(root, archive))) throw new Error('Fixture already exists; use a fresh test directory')
  writeFileSync(join(source, '说明.txt'), '第一版：此内容应被新版替代。')
  writeFileSync(join(source, '已删除.txt'), '第二次更新应删除')
  for (const [name, data] of Object.entries(expected)) if (name !== '说明.txt') writeFileSync(join(source, name), data)
  invoke('a', archive, '项目 文档', '-method', '1', '-threads', '2', '-until', '2026-10-18', '10:00:00', ...key)
  writeFileSync(join(source, '说明.txt'), expected['说明.txt'])
  utimesSync(join(source, '说明.txt'), new Date('2026-10-18T03:00:00Z'), new Date('2026-10-18T03:00:00Z'))
  unlinkSync(join(source, '已删除.txt'))
  invoke('a', archive, '项目 文档', '-method', '1', '-threads', '2', '-until', '2026-10-18', '12:00:00', ...key)
  invoke('a', encrypted ? 'stream-key.zpaq' : 'stream.zpaq', '项目 文档', '-method', 's4.0', '-threads', '2', ...key)
}
const files = ['journal.zpaq', 'journal-key.zpaq', 'stream.zpaq', 'stream-key.zpaq']
const hashes = Object.fromEntries(files.map(name => [name, createHash('sha256').update(readFileSync(join(root, name))).digest('hex')]))
writeFileSync(join(root, 'manifest.json'), JSON.stringify({ generator: tool, password, hashes, payload: Object.fromEntries(Object.entries(expected).map(([name, data]) => [name, createHash('sha256').update(data).digest('hex')])) }, null, 2))
console.log(JSON.stringify({ root, hashes }))

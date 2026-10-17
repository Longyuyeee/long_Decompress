// Independent, unmodified Bandizip output. Never rename numeric ZIPX slices.
import { mkdirSync, writeFileSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { randomBytes, createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'

const root = path.resolve(process.argv[2] || 'test-results/zipx-1418-original')
const tool = process.env.LONG_BANDIZIP || 'E:/Bandzip/Bandizip/bz.exe'
const source = path.join(root, 'source')
mkdirSync(path.join(source, '中文目录', '空目录'), { recursive: true })
const payload = path.join(source, '中文目录', '原文件.bin')
try { readFileSync(payload) } catch { writeFileSync(payload, randomBytes(170_123)) }
writeFileSync(path.join(source, '说明.txt'), '真实 ZIPX 标准分卷测试。中文、空目录和跨卷内容。\n'.repeat(20))
const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex')
const run = (args, cwd = source) => {
  const result = spawnSync(tool, args, { cwd, encoding: 'utf8', timeout: 60000 })
  if (result.status !== 0) throw new Error(JSON.stringify({ args, status: result.status, stdout: result.stdout, stderr: result.stderr }))
  return result.stdout
}
const outputs = []
for (const mode of ['none', 'password']) {
  const dir = path.join(root, mode)
  mkdirSync(dir, { recursive: true })
  const output = path.join(dir, '真实原包.zipx')
  const args = ['c', '-fmt:zipx', '-l:5', '-v:65536', '-r', ...(mode === 'password' ? ['-p:ZipxTest-1418'] : []), output, '.\\*']
  if (!readdirSync(dir).length) console.log(run(args))
  const listing = run(['l', output])
  const test = run(['t', ...(mode === 'password' ? ['-p:ZipxTest-1418'] : []), output])
  outputs.push({ mode, args, listing, test, files: readdirSync(dir).map(name => ({ name, bytes: statSync(path.join(dir, name)).size, sha256: hash(path.join(dir, name)) })) })
}
const manifest = { generatedAt: new Date().toISOString(), tool, toolSha256: hash(tool), publicPassword: 'ZipxTest-1418', source: [payload, path.join(source, '说明.txt')].map(file => ({ name: path.relative(source, file), bytes: statSync(file).size, sha256: hash(file) })), outputs }
writeFileSync(path.join(root, 'manifest.json'), JSON.stringify(manifest, null, 2))
console.log(JSON.stringify(manifest, null, 2))

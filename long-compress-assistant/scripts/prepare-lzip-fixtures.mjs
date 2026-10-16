import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, writeFileSync, readFileSync, copyFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')
export function linuxPath(value) {
  const normalized = path.resolve(value).replaceAll('\\', '/')
  assert.match(normalized, /^[A-Za-z]:\//, 'WSL fixture tools need an absolute Windows drive path')
  return `/mnt/${normalized[0].toLowerCase()}${normalized.slice(2)}`
}
export function lzipTools(projectRoot) {
  const executable = process.env.LONG_GNU_LZIP || linuxPath(path.join(projectRoot, 'test-results/lzip-tools/unpacked/usr/bin/lzip.lzip'))
  const run = (tool, args) => {
    const result = spawnSync('wsl.exe', ['-d', process.env.LONG_LZIP_DISTRO || 'Ubuntu', '--exec', tool, ...args], {
      windowsHide: true, encoding: null, timeout: 120000, maxBuffer: 64 * 1024 * 1024,
    })
    assert.ifError(result.error)
    assert.equal(result.status, 0, `${tool}: ${result.stderr?.toString()}`)
    return result.stdout
  }
  return { executable, run, lzip: args => run(executable, args) }
}
export function makeLzipFixtures(root, projectRoot) {
  const source = path.join(root, '原文件')
  mkdirSync(path.join(source, '中文目录/空目录'), { recursive: true })
  writeFileSync(path.join(source, '说明.txt'), '收到 LZIP 压缩文件，先浏览，再选择解压。\n')
  const data = Buffer.alloc(32791)
  for (let i = 0; i < data.length; i++) data[i] = ((i * 37) ^ (i >>> 8)) & 255
  writeFileSync(path.join(source, '中文目录/数据.bin'), data)
  writeFileSync(path.join(source, '空文件.txt'), '')
  const tools = lzipTools(projectRoot)
  const version = tools.lzip(['--version']).toString().split(/\r?\n/)[0]
  assert.match(version, /^lzip /)
  const single = path.join(root, '说明.txt.lz')
  writeFileSync(single, tools.lzip(['-c', '-1', linuxPath(path.join(source, '说明.txt'))]))
  const tar = path.join(root, '资料.tar')
  tools.run('/usr/bin/tar', ['--format=ustar', '-cf', linuxPath(tar), '-C', linuxPath(source), '说明.txt', '中文目录', '空文件.txt'])
  const tarLzip = path.join(root, '资料.tar.lz')
  writeFileSync(tarLzip, tools.lzip(['-c', '-1', linuxPath(tar)]))
  const alias = path.join(root, '资料别名.tlz')
  copyFileSync(tarLzip, alias)
  const multi = path.join(root, '多成员.txt.lz')
  const part = path.join(root, '第二段.txt')
  writeFileSync(part, '第二成员：两个成员还原成同一个文件。\n')
  writeFileSync(multi, Buffer.concat([readFileSync(single), tools.lzip(['-c', '-1', linuxPath(part)])]))
  const multiContent = Buffer.concat([readFileSync(path.join(source, '说明.txt')), readFileSync(part)])
  for (const file of [single, tarLzip, alias, multi]) tools.lzip(['-t', linuxPath(file)])
  const manifest = { generator: version, generatorPath: tools.executable, generatorSha256: sha256(readFileSync(path.join(projectRoot, 'test-results/lzip-tools/unpacked/usr/bin/lzip.lzip'))),
    files: [single, tarLzip, alias, multi].map(file => ({ name: path.basename(file), sha256: sha256(readFileSync(file)), size: readFileSync(file).length })),
    payloads: ['说明.txt', '中文目录/数据.bin', '空文件.txt'].map(name => ({ name, sha256: sha256(readFileSync(path.join(source, name))), size: readFileSync(path.join(source, name)).length })),
    multiContentSha256: sha256(multiContent), note: '.tlz is a byte-identical conventional alias of the generated .tar.lz. GNU lzip is a test tool only and is not distributed with the application.' }
  writeFileSync(path.join(root, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n')
  return { source, single, tarLzip, alias, multi, multiContent, manifest, tools }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  const root = path.resolve(process.argv[2] || path.join(projectRoot, 'test-results/lzip-fixtures'))
  console.log(JSON.stringify(makeLzipFixtures(root, projectRoot).manifest, null, 2))
}

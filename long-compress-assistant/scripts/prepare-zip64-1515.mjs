import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, statfsSync, statSync, writeFileSync, writeSync } from 'node:fs'
import { createReadStream } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const work = path.join(project, 'test-results', 'zip64-1515')
const source = path.join(work, 'source')
const manifestPath = path.join(work, 'source.json')
mkdirSync(source, { recursive: true })
const bytes = 2 ** 32 + 131071
const file = path.join(source, '01-实际写入4GiB.bin')
const sha = async file => {
  const hash = createHash('sha256')
  for await (const chunk of createReadStream(file)) hash.update(chunk)
  return hash.digest('hex')
}
if (existsSync(manifestPath)) {
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
  assert.equal(statSync(file).size, bytes)
  assert.equal(await sha(file), manifest.files[0].sha256)
  console.log(JSON.stringify({ reused: true, work, bytes, sha256: manifest.files[0].sha256 }))
} else {
  assert.ok(!existsSync(file), 'An incomplete previous source must be kept for diagnosis, not silently reused')
  const disk = statfsSync(work)
  assert.ok(disk.bavail * disk.bsize > 24 * 1024 ** 3, 'Keep room for the large ZIP, staged volumes and one extracted copy')
  // A deterministic nonzero 16 KiB pattern is repeated within each written
  // chunk. Every logical byte is physically passed to write(), never truncate
  // or sparse allocation. Deflate remains compact enough for password cases.
  let seed = 1515
  const pattern = Buffer.alloc(16384)
  for (let i = 0; i < pattern.length; i++) {
    seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5
    pattern[i] = seed & 255
  }
  const block = Buffer.alloc(4 * 1024 * 1024)
  for (let offset = 0; offset < block.length; offset += pattern.length) pattern.copy(block, offset)
  const hash = createHash('sha256')
  const fd = openSync(file, 'wx')
  let written = 0
  try {
    while (written < bytes) {
      const chunk = block.subarray(0, Math.min(block.length, bytes - written))
      let at = 0
      while (at < chunk.length) at += writeSync(fd, chunk, at, chunk.length - at)
      hash.update(chunk)
      written += chunk.length
      if (written % (1024 ** 3) === 0) console.log(`[zip64-1515] actually wrote ${written} bytes`)
    }
  } finally { closeSync(fd) }
  const tail = '大文件后面的末尾说明不能丢失。\nZIP64 / 密码 / 标准卷 / 数字卷。\n'
  writeFileSync(path.join(source, '99-末尾说明.txt'), tail)
  writeFileSync(path.join(source, '中间空文件.txt'), '')
  mkdirSync(path.join(source, '空目录'))
  const manifest = {
    generator: 'prepare-zip64-1515.mjs', seed: 1515, physicalWrite: true,
    sparseOrPreallocationUsed: false, bytesActuallyWritten: written,
    files: [
      { name: path.basename(file), bytes, sha256: hash.digest('hex') },
      { name: '99-末尾说明.txt', bytes: Buffer.byteLength(tail), sha256: createHash('sha256').update(tail).digest('hex') },
      { name: '中间空文件.txt', bytes: 0, sha256: createHash('sha256').digest('hex') },
    ], emptyDirectory: '空目录',
  }
  assert.equal(statSync(file).size, bytes)
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2))
  console.log(JSON.stringify({ work, ...manifest }))
}

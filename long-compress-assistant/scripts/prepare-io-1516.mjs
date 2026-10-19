import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, readdirSync, writeFileSync, writeSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
const root = process.cwd(), work = path.join(root, 'test-results/io-1516')
mkdirSync(work, { recursive: true })
if (existsSync(path.join(work, 'originals.json'))) { console.log('Reusing fixed original inputs'); process.exit(0) }
const source = path.join(work, 'source'); mkdirSync(source)
const input = path.join(source, 'payload.bin'), block = Buffer.alloc(65536), bytes = 128 * 1024 * 1024
let seed = 1516
for (let i = 0; i < block.length; i++) { seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5; block[i] = seed & 255 }
const fd = openSync(input, 'wx'), hash = createHash('sha256')
try { for (let n = 0; n < bytes; n += block.length) { writeSync(fd, block); hash.update(block) } } finally { closeSync(fd) }
const expected = hash.digest('hex'), password = 'IO1516 public test + !'
const seven = path.join(root, 'src-tauri/resources/archive-engine/7z.exe'), archive = path.join(work, 'original.zip')
const result = spawnSync(seven, ['a', '-tzip', '-mm=BZip2', '-mem=AES256', '-p' + password, '-v64k', archive, 'payload.bin'], { cwd: source, windowsHide: true, encoding: 'utf8', timeout: 180000 })
assert.equal(result.status, 0, result.stdout + result.stderr)
writeFileSync(path.join(work, 'original-tool.txt'), result.stdout + result.stderr)
const parts = readdirSync(work).filter(n => /^original\.zip\.\d+$/.test(n)).sort().map(name => ({ name, sha256: createHash('sha256').update(readFileSync(path.join(work, name))).digest('hex') }))
assert.ok(parts.length >= 3)
const small = path.join(work, 'small-source'); mkdirSync(small); writeFileSync(path.join(small, 'payload.bin'), block)
const single = path.join(work, 'small.zip')
const other = spawnSync(seven, ['a', '-tzip', '-mm=Copy', single, 'payload.bin'], { cwd: small, windowsHide: true, encoding: 'utf8' })
assert.equal(other.status, 0, other.stdout + other.stderr)
writeFileSync(path.join(work, 'originals.json'), JSON.stringify({ physicalWrite: true, bytes, sha256: expected, password, first: path.join(work, parts[0].name), parts,
  toolSha256: createHash('sha256').update(readFileSync(seven)).digest('hex'), small: single,
  smallPayloadSha256: createHash('sha256').update(block).digest('hex') }, null, 2))
console.log('Prepared genuine BZip2/AES numeric ZIP volumes', parts.length, 'and fixed payload', bytes)

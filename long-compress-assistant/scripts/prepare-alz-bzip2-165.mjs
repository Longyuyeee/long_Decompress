// Negative derivatives of the byte-preserved CC0 v1.5.2 original. Encryption
// and native framing below are controlled regression inputs, not interoperability
// evidence or newly supported producer combinations. Never rewrite the original.
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'

const root = path.resolve('tests/fixtures/archives/alz-bzip2-165')
const source = path.resolve('tests/fixtures/archives/alz-152/bzip2.alz')
const hash = bytes => createHash('sha256').update(bytes).digest('hex')
const original = readFileSync(source)
const inventory = JSON.parse(readFileSync('tests/fixtures/archives/alz-152/manifest.json', 'utf8'))
const upstream = inventory.cases.find(c => c.name === 'bzip2')
assert.equal(hash(original), upstream.members.find(p => p.name === 'bzip2.alz').sha256)
const expected = upstream.expected
const password = 'Public ALZ Bzip2 165!'
const table = Array.from({ length: 256 }, (_, i) => {
  let crc = i
  for (let bit = 0; bit < 8; bit++) crc = (crc & 1) ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1
  return crc >>> 0
})
function encrypted(data) {
  const keys = [305419896, 591751049, 878082192]
  const update = plain => {
    keys[0] = ((keys[0] >>> 8) ^ table[(keys[0] ^ plain) & 255]) >>> 0
    keys[1] = (Math.imul((keys[1] + (keys[0] & 255)) | 0, 134775813) + 1) >>> 0
    keys[2] = ((keys[2] >>> 8) ^ table[(keys[2] ^ (keys[1] >>> 24)) & 255]) >>> 0
  }
  for (const byte of Buffer.from(password)) update(byte)
  return Buffer.from(data.map(plain => {
    const temp = (keys[2] | 2) & 65535
    const key = ((Math.imul(temp, temp ^ 1) & 65535) >>> 8) & 255
    update(plain)
    return plain ^ key
  }))
}
const offset = 8, width = original[offset + 11] >> 4
const nameLength = original.readUInt16LE(offset + 4)
const sizeOffset = offset + 19
const size = original.readUIntLE(sizeOffset, width)
const dataOffset = sizeOffset + width * 2 + nameLength
const payload = original.subarray(dataOffset, dataOffset + size)
assert.equal(original[offset + 13], 1)
const bits = [...payload].map(byte => byte.toString(2).padStart(8, '0')).join('')
const marker = (0x444c5a02).toString(2).padStart(32, '0')
const end = bits.lastIndexOf(marker)
assert.ok(end > 32 && bits.length - end - 32 < 8)
const missingBits = bits.slice(0, end).padEnd(Math.ceil(end / 8) * 8, '0')
const missing = Buffer.from(missingBits.match(/.{8}/g).map(s => Number.parseInt(s, 2)))
const cases = []
mkdirSync(root, { recursive: true })
const save = (name, bytes) => {
  const file = path.join(root, name)
  mkdirSync(path.dirname(file), { recursive: true })
  if (existsSync(file)) assert.deepEqual(readFileSync(file), bytes, 'Retain the frozen derivative: ' + name)
  else writeFileSync(file, bytes)
  return { name, bytes: bytes.length, sha256: hash(bytes) }
}
for (const fault of ['trailing', 'missing-end']) for (const key of [false, true]) for (const volumes of [false, true]) {
  const name = `${key ? 'key' : 'plain'}-${fault}-${volumes ? 'volumes' : 'single'}`
  const packed = fault === 'trailing' ? Buffer.concat([payload, Buffer.alloc(5, 0xa5)]) : missing
  const header = Buffer.from(original.subarray(0, dataOffset))
  header.writeUIntLE(packed.length, sizeOffset, width)
  let data = packed
  if (key) {
    header[offset + 11] |= 1
    const check = Buffer.alloc(12, 0x35)
    check[11] = (original.readUInt32LE(offset + 15) >>> 24) & 255
    data = encrypted(Buffer.concat([check, packed]))
  }
  const bytes = Buffer.concat([header, data, original.subarray(dataOffset + size)])
  let members
  if (!volumes) members = [save(name + '.alz', bytes)]
  else {
    const body = bytes.subarray(8, -16), chunk = Math.ceil(body.length / 3)
    members = Array.from({ length: 3 }, (_, i) => {
      const head = Buffer.from(bytes.subarray(0, 8)), tail = Buffer.from(bytes.subarray(-16))
      head.writeUInt16LE(i, 6)
      tail.writeUInt32LE(i === 2 ? 0x025a4c43 : 0x035a4c43, 12)
      return save(`${name}/${name}.${i ? 'a0' + (i - 1) : 'alz'}`, Buffer.concat([head, body.subarray(i * chunk, (i + 1) * chunk), tail]))
    })
  }
  cases.push({ name, derived: true, controlledEncryption: key, controlledNativeFraming: volumes, fault, password: key ? password : null, archive: members[0].name, members })
}
// A correct short verifier and complete output cannot prove correctness if the
// declared CRC differs. Keep uncertain-data handling in that case.
const ambiguous = Buffer.from(readFileSync(path.join(root, 'key-missing-end-single.alz')))
ambiguous.writeUInt32LE((ambiguous.readUInt32LE(offset + 15) ^ 1) >>> 0, offset + 15)
const uncertain = save('key-missing-end-crc-mismatch.alz', ambiguous)
const manifest = {
  original: 'tests/fixtures/archives/alz-152/bzip2.alz', originalSha256: hash(original),
  dataLicense: 'CC0-1.0 inherited from the original fixture; controlled derivatives are not original-producer evidence',
  interpretation: 'Only negative regression inputs. No new encryption/method/native-volume compatibility claim.',
  expected, cases, uncertain: { ...uncertain, password },
}
writeFileSync(path.join(root, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n')
assert.equal(hash(readFileSync(source)), manifest.originalSha256)
console.log(`Retained original, prepared ${cases.length} explicit negative combinations and one uncertain-data boundary.`)

import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import path from 'node:path'

export async function runZipCodecGate({ bridge, failure, fixtureDirectory, artifactDirectory, bundledSevenZip }) {
  const root = path.join(fixtureDirectory, 'zip-codecs')
  const source = path.join(root, 'source')
  mkdirSync(source, { recursive: true })
  const payload = Buffer.concat([randomBytes(65536), Buffer.alloc(65536, 65)])
  writeFileSync(path.join(source, '中文 data.bin'), payload)
  const results = []
  // 7-Zip's encoder is independent of the product's native ZIP reader.
  for (const codec of ['Copy', 'Deflate', 'Deflate64', 'BZip2', 'LZMA', 'XZ']) {
    for (const encryption of ['none', 'AES256', 'ZipCrypto']) {
      const name = `${codec}-${encryption}`
      const archive = path.join(root, `${name}.zip`)
      const password = encryption === 'none' ? null : 'compatibility + !123'
      const args = ['a', '-tzip', `-mm=${codec}`, archive, '.\\*']
      if (password) args.push(`-p${password}`, `-mem=${encryption}`)
      const created = spawnSync(bundledSevenZip, args, { cwd: source, encoding: 'utf8', windowsHide: true })
      assert.equal(created.status, 0, `${name}: ${created.stdout}${created.stderr}`)
      const output = path.join(root, `${name}-out`)
      await bridge('extractArchive', archive, output, password)
      assert.deepEqual(readFileSync(path.join(output, '中文 data.bin')), payload, name)
      if (password) {
        const error = await failure('extractArchive', archive, path.join(root, `${name}-wrong`), 'wrong-password')
        assert.ok(error.length > 0, `${name}: wrong password must fail`)
      }
      const bytes = readFileSync(archive)
      const firstPart = `${archive}.001`
      for (let offset = 0, index = 1; offset < bytes.length; offset += 32768, index++) {
        writeFileSync(`${archive}.${String(index).padStart(3, '0')}`, bytes.subarray(offset, offset + 32768))
      }
      const splitOutput = path.join(root, `${name}-split-out`)
      await bridge('extractArchive', firstPart, splitOutput, password)
      assert.deepEqual(readFileSync(path.join(splitOutput, '中文 data.bin')), payload, `${name} numeric volumes`)
      results.push({ codec, encryption, single: true, numericVolumes: true, wrongPasswordRejected: !!password })
      console.log(`[zip-codecs] ${name} single + numeric volumes passed`)
      writeFileSync(path.join(artifactDirectory, 'zip-codecs.json'), JSON.stringify({ results, complete: false }, null, 2))
    }
  }
  // Bundled upstream 7-Zip 26.02 cannot encode ZIP method 93. Build a small
  // independent raw-block Zstandard ZIP, retaining 7-Zip's filename/CRC fields.
  const zstdSource = path.join(root, 'zstd-source')
  mkdirSync(zstdSource)
  const zstdPayload = Buffer.from('Zstandard ZIP 中文内容')
  writeFileSync(path.join(zstdSource, 'zstd.txt'), zstdPayload)
  const zstdArchive = path.join(root, 'Zstd.zip')
  const created = spawnSync(bundledSevenZip, ['a', '-tzip', '-mm=Copy', zstdArchive, 'zstd.txt'], { cwd: zstdSource, encoding: 'utf8', windowsHide: true })
  assert.equal(created.status, 0, created.stdout + created.stderr)
  const stored = readFileSync(zstdArchive)
  const dataStart = 30 + stored.readUInt16LE(26) + stored.readUInt16LE(28)
  const centralStart = dataStart + zstdPayload.length
  assert.equal(stored.readUInt32LE(centralStart), 0x02014b50)
  const frame = Buffer.concat([Buffer.from([0x28, 0xb5, 0x2f, 0xfd, 0x20, zstdPayload.length, (zstdPayload.length << 3) | 1, 0, 0]), zstdPayload])
  const local = Buffer.from(stored.subarray(0, dataStart))
  const centralAndEnd = Buffer.from(stored.subarray(centralStart))
  local.writeUInt16LE(63, 4); local.writeUInt16LE(93, 8); local.writeUInt32LE(frame.length, 18)
  centralAndEnd.writeUInt16LE(63, 6); centralAndEnd.writeUInt16LE(93, 10); centralAndEnd.writeUInt32LE(frame.length, 20)
  const endOffset = centralAndEnd.length - 22
  assert.equal(centralAndEnd.readUInt32LE(endOffset), 0x06054b50)
  centralAndEnd.writeUInt32LE(dataStart + frame.length, endOffset + 16)
  writeFileSync(zstdArchive, Buffer.concat([local, frame, centralAndEnd]))
  const checked = spawnSync(bundledSevenZip, ['t', zstdArchive], { encoding: 'utf8', windowsHide: true })
  assert.equal(checked.status, 0, checked.stdout + checked.stderr)
  const zstdOutput = path.join(root, 'zstd-out')
  await bridge('extractArchive', zstdArchive, zstdOutput, null)
  assert.deepEqual(readFileSync(path.join(zstdOutput, 'zstd.txt')), zstdPayload)
  results.push({ codec: 'Zstd', encryption: 'none', single: true, fixture: 'independent raw-block, externally verified', encryptedVariants: 'not externally generated' })
  writeFileSync(path.join(artifactDirectory, 'zip-codecs.json'), JSON.stringify({ results, complete: true }, null, 2))
}

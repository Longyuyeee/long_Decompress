import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync } from 'node:fs'
import { randomBytes } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'

// External WinRAR produces the standard disks; 7-Zip independently validates
// them. The ZIP64 case changes only the end records of this tiny fixture.
export function makeStandardZipFixtures(root, winrar, sevenZip) {
  const source = path.join(root, 'source')
  mkdirSync(path.join(source, '中文目录', '空目录'), { recursive: true })
  const payload = randomBytes(160 * 1024 + 31)
  const text = '我要先看看这个分卷包的说明，再解压到文件夹。\n'
  writeFileSync(path.join(source, '中文目录', '原文件.bin'), payload)
  writeFileSync(path.join(source, '说明.txt'), text)
  writeFileSync(path.join(source, '像素.png'), Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=', 'base64'))
  const password = 'ZIP sample + !'
  const cases = []
  for (const mode of ['plain', 'zipcrypto', 'aes', 'zip64']) {
    const archive = path.join(root, `下载资料-${mode}.zip`)
    const secret = ['zipcrypto', 'aes'].includes(mode) ? password : null
    const args = ['a', '-afzip', '-m0', '-r', '-v64k', '-ibck', '-inul', '-cfg-']
    if (secret) args.push(`-p${secret}`)
    if (mode === 'zipcrypto') args.push('-mezl')
    args.push(archive, '.\\*')
    const result = spawnSync(winrar, args, { cwd: source, windowsHide: true, encoding: 'utf8' })
    assert.equal(result.status, 0, `WinRAR ${mode}: ${result.stderr}`)
    if (mode === 'zip64') {
      const original = readFileSync(archive)
      const offset = original.length - 22
      assert.equal(original.subarray(offset, offset + 4).toString('hex'), '504b0506')
      const disk = original.readUInt16LE(offset + 4)
      const cdDisk = original.readUInt16LE(offset + 6)
      const end = Buffer.alloc(56 + 20)
      end.writeUInt32LE(0x06064b50, 0); end.writeBigUInt64LE(44n, 4)
      end.writeUInt16LE(45, 12); end.writeUInt16LE(45, 14)
      end.writeUInt32LE(disk, 16); end.writeUInt32LE(cdDisk, 20)
      end.writeBigUInt64LE(BigInt(original.readUInt16LE(offset + 8)), 24)
      end.writeBigUInt64LE(BigInt(original.readUInt16LE(offset + 10)), 32)
      end.writeBigUInt64LE(BigInt(original.readUInt32LE(offset + 12)), 40)
      end.writeBigUInt64LE(BigInt(original.readUInt32LE(offset + 16)), 48)
      end.writeUInt32LE(0x07064b50, 56); end.writeUInt32LE(disk, 60)
      end.writeBigUInt64LE(BigInt(offset), 64); end.writeUInt32LE(disk + 1, 72)
      const footer = Buffer.from(original.subarray(offset))
      footer.fill(255, 8, 20)
      writeFileSync(archive, Buffer.concat([original.subarray(0, offset), end, footer]))
    }
    const verify = spawnSync(sevenZip, ['t', archive, ...(secret ? [`-p${secret}`] : []), '-y'], { windowsHide: true, encoding: 'utf8' })
    assert.equal(verify.status, 0, `Independent 7-Zip ${mode}: ${verify.stdout} ${verify.stderr}`)
    const base = path.basename(archive, '.zip')
    const parts = readdirSync(root).filter(name => name.startsWith(base + '.') && /\.(zip|z\d+)$/i.test(name)).map(name => path.join(root, name))
    cases.push({ mode, archive, password: secret, parts })
  }
  return { cases, source, payload, text }
}

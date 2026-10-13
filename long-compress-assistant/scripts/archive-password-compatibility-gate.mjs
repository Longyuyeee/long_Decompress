import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync } from 'node:fs'
import { createHash, randomBytes } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { runZipCodecGate } from './zip-codec-gate.mjs'

const commit = '19ff56da4f4790064346579a1a7f18a0230b0ac6'
const base = `https://raw.githubusercontent.com/libarchive/libarchive/${commit}/libarchive/test/`
const sha = bytes => createHash('sha256').update(bytes).digest('hex')
function decode(bytes) {
  const lines = bytes.toString('ascii').split(/\r?\n/)
  const begin = lines.findIndex(line => line.startsWith('begin '))
  assert.ok(begin >= 0)
  return Buffer.concat(lines.slice(begin + 1).flatMap(line => {
    if (!line || line === 'end') return []
    const length = (line.charCodeAt(0) - 32) & 63, result = []
    for (let i = 1; i + 3 < line.length && result.length < length; i += 4) {
      const [a,b,c,d] = [...line.slice(i,i+4)].map(char => (char.charCodeAt(0)-32)&63)
      result.push((a<<2)|(b>>4), (b<<4)|(c>>2), (c<<6)|d)
    }
    return [Buffer.from(result.slice(0,length))]
  }))
}
function crc32(bytes) {
  let crc = 0xffffffff
  for (const byte of bytes) { crc ^= byte; for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0) }
  return (crc ^ 0xffffffff) >>> 0
}
function tree(root, prefix = '') {
  return readdirSync(root, { withFileTypes: true }).flatMap(entry => {
    const name = prefix + entry.name, file = path.join(root, entry.name)
    return entry.isDirectory() ? [{ path: name + '/', directory: true }, ...tree(file, name + '/')] : [{ path: name, sha256: sha(readFileSync(file)) }]
  }).sort((a,b) => a.path.localeCompare(b.path))
}
export async function runArchivePasswordCompatibilityGate(context) {
  const { bridge, failure, fixtureDirectory, artifactDirectory, bundledSevenZip, root: projectRoot } = context
  await runZipCodecGate(context)
  const cache = path.join(projectRoot, 'test-results', 'rar-compatibility-fixtures')
  mkdirSync(cache, { recursive: true })
  const sourceEvidence = [], results = []
  const fetchFixture = name => {
    const encodedPath = path.join(cache, name + '.uu')
    if (!existsSync(encodedPath)) {
      const download = spawnSync('curl.exe', ['--fail', '--location', '--retry', '2', '--max-time', '45', '--output', encodedPath, base + name + '.uu'], { encoding: 'utf8', timeout: 60000, windowsHide: true })
      assert.equal(download.status, 0, download.stderr)
    }
    const encoded = readFileSync(encodedPath), bytes = decode(encoded)
    sourceEvidence.push({ name, url: base + name + '.uu', encodedSha256: sha(encoded), decodedSha256: sha(bytes) })
    return bytes
  }
  const verifyRead = async (name, first, password = null, independentFirst = first) => {
    const independent = path.join(fixtureDirectory, name + '-independent'), output = path.join(fixtureDirectory, name + '-app')
    const checked = spawnSync(bundledSevenZip, ['x', '-y', ...(password ? [`-p${password}`] : []), `-o${independent}`, independentFirst], { encoding: 'utf8', windowsHide: true })
    assert.equal(checked.status, 0, checked.stdout + checked.stderr)
    await bridge('extractArchive', first, output, password)
    assert.deepEqual(tree(output), tree(independent), name)
    if (password) assert.ok((await failure('extractArchive', first, output + '-wrong', 'wrong-password')).length > 0)
    results.push({ name, independentPayloadsMatch: true, wrongPasswordRejected: !!password })
    console.log(`[password-compatibility] ${name} passed`)
  }
  // Real upstream RAR4/RAR5 solid and encrypted headers; each solid/header
  // fixture uses one password. Non-solid upstream mixed-password samples are
  // intentionally not misrepresented as one-password archives.
  for (const name of ['test_read_format_rar4_solid_encrypted.rar', 'test_read_format_rar4_encrypted_filenames.rar',
    'test_read_format_rar5_solid_encrypted.rar', 'test_read_format_rar5_encrypted_filenames.rar']) {
    const file = path.join(fixtureDirectory, name)
    writeFileSync(file, fetchFixture(name))
    const before = sha(readFileSync(file))
    await verifyRead(name, file, 'password')
    assert.equal(sha(readFileSync(file)), before)
  }
  for (const [stem, count] of [['test_rar_multivolume_single_file', 3], ['test_read_format_rar5_multiarchive_solid', 4]]) {
    const folder = path.join(fixtureDirectory, stem); mkdirSync(folder)
    const files = []
    for (let index = 1; index <= count; index++) {
      const name = `${stem}.part${stem.includes('rar5') ? String(index).padStart(2,'0') : index}.rar`
      const file = path.join(folder, name); writeFileSync(file, fetchFixture(name)); files.push(file)
    }
    const hashes = files.map(file => sha(readFileSync(file)))
    await verifyRead(stem, files[1], null, files[0]) // User selected a middle volume; external reference uses its canonical entry.
    assert.deepEqual(files.map(file => sha(readFileSync(file))), hashes)
    if (!stem.includes('rar5')) {
      const old = path.join(fixtureDirectory, 'rar4-old-numbering'); mkdirSync(old)
      const oldFiles = files.map((file,index) => {
        const bytes = Buffer.from(readFileSync(file))
        assert.equal(bytes[9], 0x73) // RAR4 main header following its signature.
        const headerSize = bytes.readUInt16LE(12)
        bytes.writeUInt16LE(bytes.readUInt16LE(10) & ~0x10, 10) // Clear NEWNUMBERING.
        bytes.writeUInt16LE(crc32(bytes.subarray(9, 7 + headerSize)) & 0xffff, 7)
        const target = path.join(old, index === 0 ? 'old.rar' : `old.r${String(index-1).padStart(2,'0')}`)
        writeFileSync(target, bytes); return target
      })
      await verifyRead('rar4-old-numbering', oldFiles[1], null, oldFiles[0])
    }
  }
  const sevenSource = path.join(fixtureDirectory, 'seven-source'); mkdirSync(sevenSource)
  const payload = randomBytes(160000); writeFileSync(path.join(sevenSource, '中文.bin'), payload)
  writeFileSync(path.join(sevenSource, '说明.txt'), '固实和文件名加密应保留内容')
  for (const solid of [false, true]) for (const hidden of [false, true]) {
    const name = `7z-solid-${solid}-hidden-${hidden}`, archive = path.join(fixtureDirectory, name + '.7z')
    const created = spawnSync(bundledSevenZip, ['a','-t7z',`-ms=${solid ? 'on' : 'off'}`,`-mhe=${hidden ? 'on' : 'off'}`,'-p中文 密码+!🔐',archive,'.\\*'], { cwd: sevenSource, encoding: 'utf8', windowsHide: true })
    assert.equal(created.status, 0, created.stdout + created.stderr)
    const bytes = readFileSync(archive)
    for (let at=0,index=1; at<bytes.length; at+=65536,index++) writeFileSync(`${archive}.${String(index).padStart(3,'0')}`, bytes.subarray(at,at+65536))
    await verifyRead(name, archive + '.002', '中文 密码+!🔐', archive + '.001')
    assert.equal(sha(readFileSync(archive)), sha(bytes))
  }
  writeFileSync(path.join(artifactDirectory, 'archive-password-compatibility.json'), JSON.stringify({ results, sourceEvidence,
    realDesktopNativeCommands: true, existingArchiveSourcePreserved: true, oldRarFixture: 'real upstream RAR4 payload; old-numbering main-header flag and CRC adapted, independently validated',
    mixedPerEntryRarPasswords: 'not claimed as one-password support' }, null, 2))
}

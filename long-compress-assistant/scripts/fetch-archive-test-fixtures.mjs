import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const outputDirectory =
  process.env.LONG_DECOMPRESS_EXTERNAL_FIXTURE_DIR ||
  path.join(root, 'test-results', 'external-archive-fixtures')
const libarchiveCommit = '19ff56da4f4790064346579a1a7f18a0230b0ac6'
const rawBase =
  `https://raw.githubusercontent.com/libarchive/libarchive/${libarchiveCommit}` +
  '/libarchive/test'

const fixtures = [
  {
    source: 'test_read_format_rar5_stored.rar.uu',
    sourceSha256: 'ec73ba623a8e8eee4909dcdf45f0526ff9adc1d856d054ce742e6c1ba1fb5fa8',
    output: 'libarchive-rar5-stored.rar',
    outputSha256: '35d75e315d164d2e329afc28f7d844f013271b4fcffd4ddd78efcdd114a383a7',
  },
  {
    source: 'test_read_format_lha_lh0.lzh.uu',
    sourceSha256: '1dcf4ffdc72f02985edcbdcd93ac1098cee403dc2a8a53837ba8f6d581d11a62',
    output: 'libarchive-lha-lh0.lzh',
    outputSha256: '35fa46a93d8fb0c1697afcc8c0aea0ad355e37296cfda8e87e36d6f61ea8f4cd',
  },
  {
    source: 'test_read_format_cpio_svr4_gzip_rpm.rpm.uu',
    sourceSha256: 'a43cb0e4957a961afbe6b11675437bcd5b6dfc6a740fa9afdc18bfd4835b57fc',
    output: 'libarchive-cpio-svr4-gzip.rpm',
    outputSha256: 'dcc4f3ab933335bf3822d94079fdf4f0b7ac4c65773ca8e9a973a1cec7c553d6',
  },
]

const sha256 = value => createHash('sha256').update(value).digest('hex')

async function downloadBytes(url, temporaryPath) {
  if (process.platform === 'win32') {
    const result = spawnSync(
      'curl.exe',
      ['--fail', '--location', '--retry', '4', '--output', temporaryPath, url],
      { encoding: 'utf8', timeout: 60_000, windowsHide: true },
    )
    assert.ifError(result.error)
    assert.equal(result.status, 0, `curl failed: ${result.stderr || result.stdout}`)
    return readFileSync(temporaryPath)
  }
  const response = await fetch(url)
  assert.equal(response.ok, true, `download failed: HTTP ${response.status}`)
  return Buffer.from(await response.arrayBuffer())
}

function decodeUuencoded(source) {
  const lines = source.toString('ascii').split(/\r?\n/)
  const beginIndex = lines.findIndex(line => line.startsWith('begin '))
  assert.notEqual(beginIndex, -1, 'UUencoded fixture is missing its begin header')
  const chunks = []
  for (const line of lines.slice(beginIndex + 1)) {
    if (line === 'end') break
    if (!line) continue
    const byteCount = (line.charCodeAt(0) - 32) & 63
    if (byteCount === 0) continue
    const decoded = []
    for (let index = 1; index + 3 < line.length && decoded.length < byteCount; index += 4) {
      const a = (line.charCodeAt(index) - 32) & 63
      const b = (line.charCodeAt(index + 1) - 32) & 63
      const c = (line.charCodeAt(index + 2) - 32) & 63
      const d = (line.charCodeAt(index + 3) - 32) & 63
      decoded.push((a << 2) | (b >> 4), (b << 4) | (c >> 2), (c << 6) | d)
    }
    chunks.push(Buffer.from(decoded.slice(0, byteCount)))
  }
  return Buffer.concat(chunks)
}

mkdirSync(outputDirectory, { recursive: true })
for (const fixture of fixtures) {
  const outputPath = path.join(outputDirectory, fixture.output)
  if (existsSync(outputPath) && sha256(readFileSync(outputPath)) === fixture.outputSha256) {
    console.log(`[fixtures] verified cached ${fixture.output}`)
    continue
  }

  const temporaryPath = `${outputPath}.download`
  const encoded = await downloadBytes(`${rawBase}/${fixture.source}`, temporaryPath)
  rmSync(temporaryPath, { force: true })
  assert.equal(
    sha256(encoded),
    fixture.sourceSha256,
    `${fixture.source} does not match the pinned source hash`,
  )
  const decoded = decodeUuencoded(encoded)
  assert.equal(
    sha256(decoded),
    fixture.outputSha256,
    `${fixture.output} does not match the pinned decoded hash`,
  )
  writeFileSync(outputPath, decoded)
  console.log(`[fixtures] downloaded and verified ${fixture.output}`)
}

console.log(outputDirectory)

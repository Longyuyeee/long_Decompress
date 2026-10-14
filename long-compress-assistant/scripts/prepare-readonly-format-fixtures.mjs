import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const patool = 'https://raw.githubusercontent.com/wummel/patool/eb8185c6ef6e5fc0ed22a495d0c7bf18bb7d8c58/tests/data/'
const goarj = 'https://media.githubusercontent.com/media/urtie/goarj/bd42c0c161628c65b62acd67cf6f51558cd0e81e/testdata/'
export const readonlyFixtures = [
  { name: 't.arj', url: patool + 't.arj', sha256: 'd566017d0b3eb7d920b95943ee4d565a4144aac03ce01659eb270af6bcd87543', entry: 't/t.txt', contents: '42' },
  { name: 't.txt.Z', url: patool + 't.txt.Z', sha256: 'c4abf4fdd80744dd16507cee58520c42330af6cf8764c5ec7475191c90be3c95', entry: 't.txt', contents: '42' },
  { name: 't.udf', url: patool + 't.udf', sha256: 'c50266e61cd281c4e7cb55c5d1eab1b0cfe26c8d361bb8c72f254fc56b679f17', entry: 't/t.txt', contents: '42' },
  ...[
    '08691044f6c7de2c29b0fc62dc4727ce7d2e692cbd18da855771407715718281',
    '3cf02ba1756655e8b1a8c33b078090d43617020952a7db465e54ed161adc6cb0',
    'c096a73497a8690aec3fbe2443a722ef9dd0248c280c705b7fa9309599623616',
    'e6e97d4df29dde39bd6576048f8ccffa65a21342828a1d06bff0fdffbb3e7746',
  ].map((sha256, index) => ({ name: `compat_method${index + 1}.arj`, url: goarj + `compat_method${index + 1}.arj`,
    sha256, entry: 'compat_payload.bin', contents: 'goarj-fixture-compatibility-block-0123456789\n'.repeat(400) })),
]
export const sha256 = file => createHash('sha256').update(readFileSync(file)).digest('hex')

export function prepareReadonlyFixtures() {
  const cache = path.join(root, 'node_modules', '.cache', 'archive-next', 'fixtures')
  mkdirSync(cache, { recursive: true })
  return readonlyFixtures.map(fixture => {
    const file = path.join(cache, fixture.name)
    if (!existsSync(file) || sha256(file) !== fixture.sha256) {
      const result = spawnSync('curl.exe', ['--fail', '--location', '--retry', '2', '--silent', '--show-error', '--output', file, fixture.url],
        { encoding: 'utf8', windowsHide: true, timeout: 60_000 })
      assert.ifError(result.error); assert.equal(result.status, 0, result.stderr)
    }
    assert.equal(sha256(file), fixture.sha256, `Wrong fixture bytes: ${fixture.name}`)
    return { ...fixture, file }
  })
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  console.log(JSON.stringify(prepareReadonlyFixtures().map(({ contents, ...fixture }) => fixture), null, 2))
}

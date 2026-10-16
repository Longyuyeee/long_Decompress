import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { dirname, resolve, join, delimiter } from 'node:path'
import { fileURLToPath } from 'node:url'
import { brotliDecompressSync } from 'node:zlib'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const outputRoot = resolve(root, process.argv[2] || `test-results/archive-performance-1415/independent-${Date.now()}`)
assert.ok(!existsSync(outputRoot), 'Use a new isolated directory')
const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, { cwd: root, maxBuffer: 8 * 1024 * 1024, ...options })
  assert.ifError(result.error)
  assert.equal(result.status, 0, `${command} failed: ${result.stderr?.toString('utf8')}`)
  return result.stdout
}
const git = run('git', ['rev-parse', 'HEAD']).toString('utf8').trim()
const sourcePaths = ['src-tauri/src/services/codec_stream.rs', 'src-tauri/src/services/tar_codec_stream.rs',
  'src-tauri/src/services/native_compression/tar.rs']
const digest = bytes => createHash('sha256').update(bytes).digest('hex')
const sourceSha256 = Object.fromEntries(sourcePaths.map(file => [file, digest(readFileSync(join(root, file)))]))
run('cargo', ['test', '--release', '--manifest-path', 'src-tauri/Cargo.toml', '--lib',
  'services::codec_stream::measurements::export_tar_stream_fixtures', '--', '--ignored', '--exact', '--nocapture'],
{ env: { ...process.env, LONG_TAR1415_EXPORT_ROOT: outputRoot } })
const reference = readFileSync(join(outputRoot, 'reference.tar'))
const python = process.env.LONG_TAR1415_PYTHON || 'python'
const tools = join(root, 'test-results/archive-performance-1415/python-tools')
const pythonEnv = { ...process.env, PYTHONPATH: [tools, process.env.PYTHONPATH].filter(Boolean).join(delimiter) }
const pythonVersion = run(python, ['-c', 'import sys,lz4; print(sys.version); print(lz4.__version__)'], { env: pythonEnv }).toString('utf8').trim()
const results = []
for (const format of ['tar.lz4', 'tar.br']) {
  const input = join(outputRoot, `实际产物.${format}`)
  const decoded = format === 'tar.br' ? brotliDecompressSync(readFileSync(input))
    : run(python, ['-c', 'import sys,lz4.frame; sys.stdout.buffer.write(lz4.frame.decompress(open(sys.argv[1], "rb").read()))', input], { env: pythonEnv })
  assert.deepEqual(decoded, reference, 'Independent decoder must reproduce the complete reference TAR')
  const tarFile = join(outputRoot, `${format}.decoded.tar`)
  writeFileSync(tarFile, decoded)
  const extracted = join(outputRoot, `${format}-independent`)
  mkdirSync(extracted)
  // Windows bsdtar 3.7.2 otherwise interprets legacy GNU TAR names using ACP.
  // State this reader setting explicitly; never relax payload/name assertions.
  run('tar', ['--options', 'hdrcharset=UTF-8', '-xf', tarFile, '-C', extracted])
  const files = ['说明.txt', '空文件.txt', '子目录/内容.bin'].map(name => {
    const original = readFileSync(join(outputRoot, '中文资料', name))
    const restored = readFileSync(join(extracted, '中文资料', name))
    assert.deepEqual(restored, original)
    return { name, bytes: restored.length, sha256: digest(restored) }
  })
  assert.ok(statSync(join(extracted, '中文资料/子目录/空目录')).isDirectory())
  results.push({ format, input_sha256: digest(readFileSync(input)), decoded_tar_sha256: digest(decoded), files, empty_directory_preserved: true })
}
assert.deepEqual(Object.fromEntries(sourcePaths.map(file => [file, digest(readFileSync(join(root, file)))])), sourceSha256)
assert.equal(run('git', ['rev-parse', 'HEAD']).toString('utf8').trim(), git)
const report = {
  verified_at: new Date().toISOString(), git_commit: git, source_sha256: sourceSha256,
  scope: 'Actual native streamed outputs; Python lz4.frame / Node Brotli independently decode full TAR bytes; Windows bsdtar with explicit hdrcharset=UTF-8 extracts and verifies all payloads and the empty directory. Not desktop or installed-release evidence.',
  unicode_reader_boundary: 'Windows bsdtar 3.7.2 default ACP produced garbled names. UTF-8 was explicitly selected; complete decoded TAR bytes equal the pre-existing native TAR reference. This does not claim default Unicode interoperability with every reader.',
  python_lz4: pythonVersion, node: process.version, bsdtar: run('tar', ['--version']).toString('utf8').trim(), results,
}
writeFileSync(join(outputRoot, 'verification.json'), JSON.stringify(report, null, 2) + '\n')
console.log(JSON.stringify(report, null, 2))

import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import os from 'node:os'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const output = resolve(root, process.argv[2] || 'test-results/archive-performance-1415/tar-creation.json')
const run = (command, args) => {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 })
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.error || result.stderr || result.stdout}`)
  return result.stdout.trim()
}
const commit = run('git', ['rev-parse', 'HEAD'])
const dirty = !!run('git', ['status', '--porcelain'])
const paths = ['src-tauri/src/services/codec_stream.rs', 'src-tauri/src/services/codec_stream_measurement.rs',
  'src-tauri/src/services/native_compression/tar.rs', 'src-tauri/src/services/tar_codec_stream.rs',
  'scripts/measure-tar-creation.mjs'].filter(file => existsSync(resolve(root, file)))
const hashes = () => Object.fromEntries(paths.map(file => [file, createHash('sha256').update(readFileSync(resolve(root, file))).digest('hex')]))
const sourceHashes = hashes()
const stdout = run('cargo', ['test', '--release', '--manifest-path', 'src-tauri/Cargo.toml', '--lib',
  'services::codec_stream::measurements::tar_creation_measurement', '--', '--exact', '--ignored', '--nocapture'])
const prefix = 'TAR_CREATION_JSON '
const records = stdout.split(/\r?\n/).filter(line => line.startsWith(prefix)).map(line => JSON.parse(line.slice(prefix.length)))
assert.equal(records.length, 12)
const summary = {}
for (const scenario of ['large', 'small']) for (const format of ['tar.lz4', 'tar.br']) {
  const rows = records.filter(row => row.scenario === scenario && row.format === format)
  assert.equal(rows.length, 3)
  assert.equal(new Set(rows.map(row => row.fixture_sha256)).size, 1)
  assert.ok(rows.every(row => row.payload_hashes_verified))
  const times = rows.map(row => row.compression_ms).sort((a, b) => a - b)
  summary[`${scenario}:${format}`] = { median_ms: times[1], min_ms: times[0], max_ms: times[2],
    temporary_peak_bytes_untimed: rows[0].temporary_peak_bytes_untimed }
}
assert.equal(run('git', ['rev-parse', 'HEAD']), commit)
assert.deepEqual(hashes(), sourceHashes, 'Measured source changed during the run')
const report = {
  generated_at: new Date().toISOString(), git: { commit, dirty_at_start: dirty }, source_sha256: sourceHashes,
  machine: { platform: os.platform(), release: os.release(), cpu: os.cpus()[0]?.model, memory_bytes: os.totalmem() },
  rustc: run('rustc', ['--version']), profile: 'release',
  scope: 'Native creation only, level 6. 32 MiB seeded bytes and 1000 x 4 KiB seeded files. Separate untimed per-control-check temporary-file observation; excludes source and output. Three timed rounds without directory sampling; output SHA256 checked outside timing. OS cache not flushed; excludes desktop, verification/publication, physical I/O and peak memory.',
  summary, records,
}
mkdirSync(dirname(output), { recursive: true })
writeFileSync(output, JSON.stringify(report, null, 2) + '\n')
console.log(JSON.stringify({ output, summary }, null, 2))

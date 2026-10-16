// Measures only the password preflight, not extraction or physical disk I/O.
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import os from 'node:os'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const output = resolve(root, process.argv[2] || 'test-results/archive-performance-1415/preflight.json')
const run = (command, args) => {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 })
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.error || result.stderr || result.stdout}`)
  return result.stdout.trim()
}
const commit = run('git', ['rev-parse', 'HEAD'])
const dirty = !!run('git', ['status', '--porcelain'])
const sourcePaths = ['src-tauri/src/services/native_extraction/zip.rs', 'src-tauri/src/services/zip_selection.rs',
  'src-tauri/src/services/universal_engine.rs', 'scripts/measure-zip-preflight.mjs']
const sourceHashes = () => Object.fromEntries(sourcePaths.map(file => [file,
  createHash('sha256').update(readFileSync(resolve(root, file))).digest('hex')]))
const initialSources = sourceHashes()
const stdout = run('cargo', ['test', '--release', '--manifest-path', 'src-tauri/Cargo.toml', '--lib',
  'services::native_extraction::zip::tests::zip_password_preflight_measurement', '--', '--exact', '--ignored', '--nocapture'])
const prefix = 'ZIP_PREFLIGHT_JSON '
const records = stdout.split(/\r?\n/).filter(line => line.startsWith(prefix)).map(line => JSON.parse(line.slice(prefix.length)))
assert.equal(records.length, 20)
const summary = {}
for (const scenario of ['5000_plain', '5000_plain_plus_one_zipcrypto']) {
  const matching = records.filter(row => row.scenario === scenario)
  assert.equal(matching.length, 10)
  assert.equal(new Set(matching.map(row => row.fixture_sha256)).size, 1, 'A/B must use the same file')
  summary[scenario] = {}
  for (const legacy of [true, false]) {
    const rows = matching.filter(row => row.legacy === legacy)
    assert.equal(rows.length, 5)
    const times = rows.map(row => row.elapsed_ms).sort((a, b) => a - b)
    summary[scenario][legacy ? 'legacy' : 'optimized'] = {
      median_ms: times[2], min_ms: times[0], max_ms: times[4],
      logical_read_bytes: rows[0].logical_read_bytes, read_calls: rows[0].read_calls, seek_calls: rows[0].seek_calls,
    }
  }
  const optimized = summary[scenario].optimized
  assert.equal(optimized.seek_calls, scenario === '5000_plain' ? 0 : 1)
  assert.equal(optimized.logical_read_bytes, summary[scenario].legacy.logical_read_bytes,
    'Optimization must not remove encrypted payload probing')
}
assert.equal(run('git', ['rev-parse', 'HEAD']), commit, 'Source commit changed during measurement')
assert.deepEqual(sourceHashes(), initialSources, 'Measured source changed during execution')
const report = {
  schema_version: 1, generated_at: new Date().toISOString(),
  git: { commit, dirty_at_start: dirty, dirty_at_end: !!run('git', ['status', '--porcelain']) },
  source_sha256: initialSources,
  machine: { platform: os.platform(), release: os.release(), cpu: os.cpus()[0]?.model,
    logical_processors: os.cpus().length, memory_bytes: os.totalmem() },
  rustc: run('rustc', ['--version']), profile: 'release',
  scope: 'Disk-backed password preflight only, after archive parsing and selection. Legacy function frozen from f2a22b2; same binary/file, alternating order, one warm-up and five measured rounds. OS cache not flushed. Logical seek/read calls are not physical I/O. No extraction/UI/memory improvement claim.',
  summary, records,
}
mkdirSync(dirname(output), { recursive: true })
writeFileSync(output, JSON.stringify(report, null, 2) + '\n')
console.log(JSON.stringify({ output, summary }, null, 2))

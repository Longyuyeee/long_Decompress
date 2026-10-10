import { spawnSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import os from 'node:os'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const iterations = Number(process.env.LONG_NATIVE_ZIP_ITERATIONS || 3)
if (!Number.isInteger(iterations) || iterations < 1 || iterations > 20) throw new Error('Iterations must be 1..20')
const output = resolve(root, process.argv[2] || 'test-results/native-zip-baseline/result.json')
const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, ...options })
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.error || result.stderr || result.stdout}`)
  return result.stdout.trim()
}
const initialCommit = run('git', ['rev-parse', 'HEAD'])
const initialStatus = run('git', ['status', '--porcelain'])
const samples = []
for (let iteration = 1; iteration <= iterations; iteration++) {
  console.log(`Native ZIP engine: iteration ${iteration}/${iterations}`)
  const stdout = run('cargo', ['test', '--release', '--manifest-path', 'src-tauri/Cargo.toml', '--lib',
    'services::native_zip_baseline::native_zip_engine_baseline', '--', '--ignored', '--exact', '--nocapture'], {
    env: { ...process.env, LONG_NATIVE_ZIP_ROOT: process.env.LONG_NATIVE_ZIP_ROOT || root },
  })
  const records = stdout.split(/\r?\n/).filter(line => line.startsWith('NATIVE_ZIP_JSON '))
    .map(line => JSON.parse(line.slice('NATIVE_ZIP_JSON '.length)))
  if (records.length !== 3 || new Set(records.map(item => item.scenario)).size !== 3) throw new Error('Expected three verified scenarios')
  for (const record of records) {
    const first = samples.find(sample => sample.scenario === record.scenario)
    if (first && first.fixture_sha256 !== record.fixture_sha256) throw new Error('Fixture changed between iterations')
    samples.push({ ...record, iteration })
    console.log(`${record.scenario}: compress ${record.compression_ms.toFixed(2)} ms, extract ${record.extraction_ms.toFixed(2)} ms; SHA-256 verified`)
  }
}
const summary = Object.fromEntries(['large', 'small', 'mixed_media'].map(scenario => {
  const matching = samples.filter(sample => sample.scenario === scenario)
  const stats = key => {
    const sorted = matching.map(sample => sample[key]).sort((a, b) => a - b)
    const middle = Math.floor(sorted.length / 2)
    return { median: sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2,
      min: sorted[0], max: sorted.at(-1) }
  }
  return [scenario, { compression_ms: stats('compression_ms'), extraction_ms: stats('extraction_ms') }]
}))
const report = {
  schema_version: 1, generated_at: new Date().toISOString(), scope: 'native_zip_engine_without_window_or_transaction',
  git: { commit: initialCommit, dirty_at_start: !!initialStatus,
    commit_at_end: run('git', ['rev-parse', 'HEAD']), dirty_at_end: !!run('git', ['status', '--porcelain']) },
  machine: { platform: os.platform(), release: os.release(), architecture: os.arch(),
    cpu: os.cpus()[0]?.model, logical_processors: os.cpus().length, memory_bytes: os.totalmem() },
  toolchain: { rustc: run('rustc', ['--version']), cargo: run('cargo', ['--version']) },
  conditions: { iterations, root: process.env.LONG_NATIVE_ZIP_ROOT || root, cache: 'not_flushed',
    scenario_order: ['large', 'small', 'mixed_media'], profile: 'release',
    cpu_usage: null, peak_memory: null, io_counters: null, ui_latency: null,
    note: 'Observation only; not an A/B comparison or end-to-end desktop benchmark.' },
  summary, samples,
}
mkdirSync(dirname(output), { recursive: true })
writeFileSync(output, JSON.stringify(report, null, 2) + '\n')
console.log(`Saved ${samples.length} verified samples: ${output}`)

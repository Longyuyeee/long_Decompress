import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { randomBytes, createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'

// Production IPC, including verification/publication, not a codec microbenchmark.
export async function runArchivePerformanceGate({ invoke, fixtureDirectory, artifactDirectory, application, buildKind }) {
  const results = []
  const scan = (directory, temporary = false) => {
    let all = 0, temp = 0
    for (const name of readdirSync(directory)) {
      const file = path.join(directory, name)
      try {
        const info = statSync(file)
        const isTemp = temporary || name.startsWith('.long-') || name.startsWith('.split-source-')
        if (info.isDirectory()) { const child = scan(file, isTemp); all += child.all; temp += child.temp }
        else { all += info.size; if (isTemp) temp += info.size }
      } catch (error) { if (error.code !== 'ENOENT') throw error }
    }
    return { all, temp }
  }
  const digest = file => createHash('sha256').update(readFileSync(file)).digest('hex')
  for (const kind of ['large', 'small', 'mixed']) {
    const root = path.join(fixtureDirectory, `performance-${kind}`)
    const source = path.join(root, 'source')
    mkdirSync(source, { recursive: true })
    if (kind !== 'small') writeFileSync(path.join(source, 'large.bin'), randomBytes((kind === 'large' ? 64 : 16) * 1048576))
    if (kind !== 'large') for (let i = 0; i < 2000; i++) writeFileSync(path.join(source, `文件-${i}.txt`), Buffer.alloc(1024, i % 256))
    const expected = Object.fromEntries(readdirSync(source).map(name => [name, digest(path.join(source, name))]))
    const inputBytes = scan(source).all
    let peakTemporaryBytes = 0, peakWorkspaceBytes = scan(root).all, samples = 0
    const sample = () => { const current = scan(root); peakTemporaryBytes = Math.max(peakTemporaryBytes, current.temp); peakWorkspaceBytes = Math.max(peakWorkspaceBytes, current.all); samples++ }
    const timer = setInterval(sample, 50)
    const archive = path.join(root, 'result.zip'), restored = path.join(root, 'restored')
    let compressionMs, extractionMs
    try {
      const start = performance.now()
      await invoke('compress_files', { taskId: `perf-compress-${kind}-${Date.now()}`, files: [source], outputPath: archive,
        options: { format: 'zip', level: 6, verify_after: true, delete_after: false, preserve_paths: true } })
      compressionMs = performance.now() - start
      const extractStart = performance.now()
      await invoke('extract_file', { taskId: `perf-extract-${kind}-${Date.now()}`, filePath: archive, outputPath: restored, password: null,
        options: { preserve_paths: true, preserve_timestamps: true, skip_corrupted: false, extract_only_newer: false,
          file_filter: null, enable_bruteforce: false, bruteforce_wordlists: [],
          delete_after: false, create_subdirectory: false, overwrite_existing: false, conflict_policy: 'rename' } })
      extractionMs = performance.now() - extractStart
    } finally { sample(); clearInterval(timer) }
    // Preserve-paths compression keeps the selected source directory itself.
    for (const [name, hash] of Object.entries(expected)) assert.equal(digest(path.join(restored, 'source', name)), hash)
    const memory = spawnSync('powershell.exe', ['-NoProfile', '-Command',
      'Get-Process | Where-Object { $_.Path -eq $env:LONG_PERF_APP } | Select-Object Id,PeakWorkingSet64,WorkingSet64 | ConvertTo-Json -Compress'],
      { encoding: 'utf8', windowsHide: true, env: { ...process.env, LONG_PERF_APP: application } })
    assert.equal(memory.status, 0, memory.stderr)
    const processMemory = JSON.parse(memory.stdout)
    assert.ok(processMemory, 'The measured backend process must still exist')
    results.push({ kind, inputBytes, fileCount: Object.keys(expected).length, compressionMs, extractionMs,
      totalMs: compressionMs + extractionMs, peakTemporaryBytes, peakWorkspaceBytes, sampleIntervalMs: 50, samples,
      processMemory, memoryScope: 'backend process lifetime peak; not per-case delta or full process tree', payloadHashesVerified: true })
    console.log(`[archive-performance] ${kind}: ${Math.round(compressionMs)} + ${Math.round(extractionMs)} ms`)
  }
  writeFileSync(path.join(artifactDirectory, 'archive-performance.json'), JSON.stringify({ buildKind, results,
    scope: 'one local-volume run per workload; sampled temporary logical file sizes may miss short peaks; no speedup claim' }, null, 2))
}

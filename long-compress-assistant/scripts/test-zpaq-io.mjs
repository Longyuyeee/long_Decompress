import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

// Build with scripts/build-zpaq-engine.ps1 -TestIO. Fault controls are absent
// from the shipped binary. Never fill a real disk or touch personal archives.
const root = mkdtempSync(resolve('test-results/zpaq-io-'))
const tool = resolve('test-results/zpaq-io-test/long-zpaq.exe')
const production = resolve('src-tauri/resources/zpaq-engine/long-zpaq.exe')
const source = resolve(root, 'source.txt')
writeFileSync(source, 'real payload '.repeat(16384))
const results = []
function run(exe, args, fault) {
  return spawnSync(exe, args, { encoding: 'utf8', timeout: 30000,
    env: { ...process.env, LONG_ZPAQ_TEST_IO: fault ?? '' } })
}
for (const fault of ['write', 'shortwrite', 'read', 'seek', 'close', 'flush']) {
  const archive = resolve(root, `create-${fault}.zpaq`)
  const result = run(tool, ['a', archive, source, '-method', '1', '-threads', '2'], fault)
  assert.notEqual(result.status, 0, `${fault} must fail creation`)
  assert.match(result.stderr, /LONG_ZPAQ_IO:/, `${fault} retains I/O cause`)
  results.push({ operation: 'create', fault, exitCode: result.status, detail: result.stderr.trim() })
}
for (const kind of ['journal', 'stream']) {
  for (const fault of ['write', 'shortwrite', 'read', 'seek', 'close', 'flush']) {
    const output = resolve(root, `extract-${kind}-${fault}`)
    mkdirSync(output)
    const result = run(tool, ['x', resolve(`src-tauri/tests/fixtures/zpaq/${kind}.zpaq`), '-to', output, '-threads', '2'], fault)
    assert.notEqual(result.status, 0, `${kind} ${fault} must fail extraction`)
    assert.match(result.stderr, /LONG_ZPAQ_IO:/)
    results.push({ operation: `extract-${kind}`, fault, exitCode: result.status, detail: result.stderr.trim() })
  }
}
// A production build must not acquire a fault environment backdoor.
const result = run(production, ['l', resolve('src-tauri/tests/fixtures/zpaq/journal.zpaq')], 'read')
assert.equal(result.status, 0, result.stderr)
assert.ok(!readFileSync(production).includes(Buffer.from('LONG_ZPAQ_TEST_IO')))
writeFileSync(resolve(root, 'results.json'), JSON.stringify({ results, productionFaultHookAbsent: true }, null, 2))
console.log(JSON.stringify({ root, passed: results.length, productionFaultHookAbsent: true }))

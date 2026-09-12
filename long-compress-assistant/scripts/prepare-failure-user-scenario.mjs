// Synthetic files only: reproduce a user's intact download and truncated download.
import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const results = path.join(root, 'test-results')
mkdirSync(results, { recursive: true })
const fixture = mkdtempSync(path.join(results, 'failure-user-scenario-'))
const payload = '我下载了一份资料，希望完整解压；失败时请先告诉我原因。\n'.repeat(100)
writeFileSync(path.join(fixture, '资料.txt'), payload)
const result = spawnSync(path.join(root, 'src-tauri/resources/archive-engine/7z.exe'),
  ['a', '-tzip', '-mx=0', '完整资料.zip', '资料.txt'],
  { cwd: fixture, encoding: 'utf8', windowsHide: true })
assert.equal(result.status, 0, result.stderr || result.stdout)
const intact = readFileSync(path.join(fixture, '完整资料.zip'))
writeFileSync(path.join(fixture, '下载中断.zip'), intact.subarray(0, Math.floor(intact.length / 2)))
mkdirSync(path.join(fixture, 'output'))
console.log(fixture)

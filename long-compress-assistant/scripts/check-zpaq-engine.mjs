import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
const root = new URL('../', import.meta.url)
const lock = JSON.parse(readFileSync(new URL('zpaq-engine.lock.json', root), 'utf8'))
for (const [name, expected] of Object.entries(lock.files)) {
  const actual = createHash('sha256').update(readFileSync(new URL(name, root))).digest('hex')
  if (actual !== expected) throw new Error(`ZPAQ source/binary lock mismatch: ${name}`)
}
const config = JSON.parse(readFileSync(new URL('src-tauri/tauri.conf.json', root), 'utf8'))
if (!config.tauri.bundle.resources.includes('resources/zpaq-engine/*')) throw new Error('ZPAQ resources are not bundled')
console.log(`ZPAQ ${lock.upstream} adapter, sources, license and resource inclusion verified`)

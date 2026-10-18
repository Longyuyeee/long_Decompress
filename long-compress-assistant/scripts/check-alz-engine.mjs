import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
const root=new URL('../',import.meta.url)
const lock=JSON.parse(readFileSync(new URL('alz-engine.lock.json',root),'utf8'))
for(const [name,expected] of Object.entries(lock.files)) {
  if(createHash('sha256').update(readFileSync(new URL(name,root))).digest('hex')!==expected)throw new Error(`ALZ source/resource mismatch: ${name}`)
}
const config=JSON.parse(readFileSync(new URL('src-tauri/tauri.conf.json',root),'utf8'))
for(const resource of ['resources/alz-engine/*','resources/alz-engine/licenses/*'])
  if(!config.tauri.bundle.resources.includes(resource))throw new Error('ALZ resource is not bundled: '+resource)
console.log('ALZ adapter, decoder, native framing and dependency notices verified')

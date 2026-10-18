import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
const root=new URL('../',import.meta.url)
const lock=JSON.parse(readFileSync(new URL('egg-engine.lock.json',root),'utf8'))
for(const [name,expected] of Object.entries(lock.files)) {
  if(createHash('sha256').update(readFileSync(new URL(name,root))).digest('hex')!==expected)throw new Error(`EGG source/resource mismatch: ${name}`)
}
const config=JSON.parse(readFileSync(new URL('src-tauri/tauri.conf.json',root),'utf8'))
for(const resource of ['resources/egg-engine/*','resources/egg-engine/licenses/*'])
  if(!config.tauri.bundle.resources.includes(resource))throw new Error('EGG resource is not bundled: '+resource)
console.log('EGG adapter, decoder, native framing and dependency notices verified')

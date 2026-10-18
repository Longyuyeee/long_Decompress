// Generated resource/source inventory, including redistributed dependency notices.
import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
const metadata=JSON.parse(execFileSync('cargo',['metadata','--locked','--format-version','1','--manifest-path','tools/alz-engine/Cargo.toml'],{encoding:'utf8',maxBuffer:8*1024*1024}))
const destination='src-tauri/resources/alz-engine/licenses';mkdirSync(destination,{recursive:true})
const packages=[]
for(const p of metadata.packages) {
  const root=path.dirname(p.manifest_path)
  const names=readdirSync(root).filter(n=>/^(?:LICENSE|COPYING|UNLICENSE|COPYRIGHT)/i.test(n)&&!n.includes('.rs'))
  for(const name of names) copyFileSync(path.join(root,name),path.join(destination,`${p.name}-${p.version}-${name}`))
  packages.push({name:p.name,version:p.version,license:p.license,notices:names})
}
writeFileSync('src-tauri/resources/alz-engine/dependencies.json',JSON.stringify(packages,null,2)+'\n')
const files={}
for(const dir of ['vendor/unalz/src','tools/alz-engine/src','src-tauri/resources/alz-engine']) {
  for(const name of readdirSync(dir,{recursive:true,withFileTypes:true}).filter(x=>x.isFile()).map(x=>path.relative('.',path.join(x.parentPath,x.name)).replaceAll('\\','/')).sort())
    files[name]=createHash('sha256').update(readFileSync(name)).digest('hex')
}
for(const name of ['vendor/unalz/Cargo.toml','vendor/unalz/LICENSE','vendor/unalz/LONG_MODIFICATIONS.md','tools/alz-engine/Cargo.toml','tools/alz-engine/Cargo.lock','src-tauri/src/services/alz_volumes.rs'])
  files[name]=createHash('sha256').update(readFileSync(name)).digest('hex')
writeFileSync('alz-engine.lock.json',JSON.stringify({upstream:'unalz 0.2.1',platform:'windows-x64',files},null,2)+'\n')
console.log(`Locked ALZ helper and ${packages.length} dependency notices`)

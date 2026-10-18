import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
const metadata=JSON.parse(execFileSync('cargo',['metadata','--locked','--format-version','1','--manifest-path','tools/egg-engine/Cargo.toml'],{encoding:'utf8',maxBuffer:8*1024*1024}))
const destination='src-tauri/resources/egg-engine/licenses';mkdirSync(destination,{recursive:true})
const packages=[]
for(const p of metadata.packages) {
  const root=path.dirname(p.manifest_path)
  const names=readdirSync(root,{withFileTypes:true}).flatMap(e=> {
    if(!/^(?:LICENSE|COPYING|UNLICENSE|COPYRIGHT)/i.test(e.name)||e.name.includes('.rs'))return []
    if(e.isFile())return [e.name]
    return readdirSync(path.join(root,e.name),{recursive:true,withFileTypes:true}).filter(f=>f.isFile()).map(f=>path.relative(root,path.join(f.parentPath,f.name)))
  })
  for(const name of names) copyFileSync(path.join(root,name),path.join(destination,`${p.name}-${p.version}-${name.replaceAll(/[\\/]/g,'_')}`))
  packages.push({name:p.name,version:p.version,license:p.license,notices:names})
}
writeFileSync('src-tauri/resources/egg-engine/dependencies.json',JSON.stringify(packages,null,2)+'\n')
const files={}
for(const dir of ['vendor/unegg/src','tools/egg-engine/src','src-tauri/resources/egg-engine']) {
  for(const name of readdirSync(dir,{recursive:true,withFileTypes:true}).filter(x=>x.isFile()).map(x=>path.relative('.',path.join(x.parentPath,x.name)).replaceAll('\\','/')).sort())
    files[name]=createHash('sha256').update(readFileSync(name)).digest('hex')
}
for(const name of ['vendor/unegg/Cargo.toml','vendor/unegg/LICENSE','vendor/unegg/LONG_MODIFICATIONS.md','tools/egg-engine/Cargo.toml','tools/egg-engine/Cargo.lock','src-tauri/src/services/egg_volumes.rs'])
  files[name]=createHash('sha256').update(readFileSync(name)).digest('hex')
writeFileSync('egg-engine.lock.json',JSON.stringify({upstream:'unegg 0.2.1 at 9929009ca4d010e8cd88bb796f7a1868162026d4',platform:'windows-x64',files},null,2)+'\n')
console.log(`Locked EGG helper and ${packages.length} dependency notices`)

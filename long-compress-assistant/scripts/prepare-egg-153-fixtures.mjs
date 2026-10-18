// Independently establish fixed expected contents; never derive expectations from our helper.
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
const tool=process.env.LONG_BANDIZIP||'E:/Bandzip/Bandizip/bz.exe'
const root=path.resolve('tests/fixtures/archives/egg-153'),source=path.join(root,'upstream'),scratch=path.resolve('test-results/egg-153-independent')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
if(existsSync(path.join(root,'manifest.json'))) {
  const m=JSON.parse(readFileSync(path.join(root,'manifest.json'),'utf8'))
  for(const c of m.cases)for(const f of c.members)assert.equal(sha(path.join(source,f.name)),f.sha256)
  console.log('Fixed original EGG inventory verified; expected payloads not regenerated.');process.exit(0)
}
mkdirSync(scratch,{recursive:true});const cases=[]
for(const [name,password]of[['store',null],['low',null],['normal',null],['store5',null],['deflate5',null],['lzma5',null],['bzip2_big',null],['mixed_store_deflate5',null],['zip20','test1234'],['enc_zip20','test1234'],['enc_zip20_bzip2','test1234'],['split',null],['split_store',null]]) {
  const archive=name.startsWith('split')?name+'.vol1.egg':name+'.egg'
  const names=readdirSync(source).filter(n=>n===archive||n.startsWith(name+'.vol')&&n.endsWith('.egg'))
  const out=path.join(scratch,name)
  const r=spawnSync(tool,['x','-y',...(password?['-p:'+password]:[]),'-o:'+out,path.join(source,archive)],{windowsHide:true,timeout:120000})
  assert.ifError(r.error);assert.equal(r.status,0,String(r.stdout)+String(r.stderr));const expected=tree(out);assert.ok(expected.length)
  cases.push({name,archive,password,members:names.map(name=>({name,bytes:readFileSync(path.join(source,name)).length,sha256:sha(path.join(source,name))})),expected})
}
writeFileSync(path.join(root,'manifest.json'),JSON.stringify({upstream:'https://github.com/alkegi/unegg/tree/9929009ca4d010e8cd88bb796f7a1868162026d4/tests/data',dataLicense:'CC0-1.0',independentTool:tool,independentToolSha256:sha(tool),checkedAt:new Date().toISOString(),cases},null,2)+'\n')
console.log(`Independent original EGG groups: ${cases.length}`)

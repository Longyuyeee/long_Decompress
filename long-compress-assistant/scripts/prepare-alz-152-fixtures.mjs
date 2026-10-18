// Preserve upstream originals; establish known payloads with independent Bandizip.
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
const tool=process.env.LONG_BANDIZIP || 'E:/Bandzip/Bandizip/bz.exe'
const source=path.resolve('test-results/unalz-152-upstream/tests/data')
const root=path.resolve('tests/fixtures/archives/alz-152')
const scratch=path.resolve('test-results/alz-152-independent')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
mkdirSync(root,{recursive:true});mkdirSync(scratch,{recursive:true})
const cases=[]
for(const [name,password] of [['store',null],['normal',null],['low',null],['bzip2',null],['dir_tree',null],['enc_store','test1234'],['enc_deflate','test1234'],['zip20','test1234'],['split',null],['split_store',null]]) {
  const names=readdirSync(source).filter(f=>f===name+'.alz'||new RegExp('^'+name+'\\.[a-z]\\d{2}$').test(f))
  for(const f of names)copyFileSync(path.join(source,f),path.join(root,f))
  const out=path.join(scratch,name+'-cp949')
  const r=spawnSync(tool,['x','-y','-cp:949',...(password?['-p:'+password]:[]),'-o:'+out,path.join(root,name+'.alz')],{windowsHide:true,timeout:120000})
  assert.ifError(r.error);assert.equal(r.status,0,String(r.stdout)+String(r.stderr))
  const expected=tree(out);assert.ok(expected.length)
  cases.push({name,archive:name+'.alz',password,members:names.map(n=>({name:n,bytes:readFileSync(path.join(root,n)).length,sha256:sha(path.join(root,n))})),expected})
}
const manifest={upstream:'https://github.com/alkegi/unalz/tree/30b4ad40c5452c686df69b3b45554de3073e57a1/tests/data',dataLicense:'CC0-1.0',independentTool:tool,independentToolSha256:sha(tool),filenameEncoding:'CP949, as identified by the upstream original filename table; automatic local CP936 produced mojibake',checkedAt:new Date().toISOString(),cases}
writeFileSync(path.join(root,'manifest.json'),JSON.stringify(manifest,null,2)+'\n')
console.log(JSON.stringify({cases:cases.length,files:cases.reduce((s,c)=>s+c.expected.length,0),manifest:path.join(root,'manifest.json')},null,2))

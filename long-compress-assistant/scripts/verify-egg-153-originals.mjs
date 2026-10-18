import assert from 'node:assert/strict'
import { readFileSync,writeFileSync,mkdirSync,readdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
const base=path.resolve('tests/fixtures/archives/egg-153/original-alzip'),manifest=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8')),tool='E:/Bandzip/Bandizip/bz.exe',helper=path.resolve('src-tauri/resources/egg-engine/long-egg.exe')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
const cases=[]
for(const c of manifest.cases){
  for(const f of c.members)assert.equal(sha(path.join(base,f.name)),f.sha256)
  const out=path.resolve('test-results/egg-153-independent',c.name);mkdirSync(out,{recursive:true})
  const r=spawnSync(tool,['x','-y','-cp:936',...(c.password?['-p:'+c.password]:[]),'-o:'+out,path.join(base,c.archive)],{windowsHide:true,timeout:60000});assert.ifError(r.error);assert.equal(r.status,0,String(r.stdout)+String(r.stderr));assert.deepEqual(tree(out),c.expected.slice().sort((a,b)=>a.path.localeCompare(b.path)))
  const actual=path.resolve('test-results/egg-153-product',c.name);mkdirSync(actual,{recursive:true})
  const result=spawnSync(helper,['extract',path.join(base,c.archive)],{input:JSON.stringify({password:c.password,encoding:'cp936',output:actual}),windowsHide:true,encoding:'utf8',timeout:60000});assert.equal(result.status,0,result.stdout+result.stderr);assert.deepEqual(tree(actual),c.expected.slice().sort((a,b)=>a.path.localeCompare(b.path)))
  cases.push({archive:c.archive,encoding:'cp936',originalUnicodeNames:true,independentPayloadHashes:true,emptyDirectory:true})
}
writeFileSync(path.join(base,'independent.json'),JSON.stringify({checkedAt:new Date().toISOString(),toolSha256:sha(tool),toolVersion:'Bandizip 7.38 Beta',helperSha256:sha(helper),cases},null,2)+'\n')
console.log(cases)

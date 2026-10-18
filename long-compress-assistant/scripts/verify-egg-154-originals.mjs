import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,readdirSync,mkdirSync,existsSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import {createHash,randomUUID} from 'node:crypto'
import path from 'node:path'
const base=path.resolve('tests/fixtures/archives/egg-154/original-alzip'),m=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8')),tool='E:/Bandzip/Bandizip/bz.exe',helper=path.resolve(process.env.LONG_EGG_HELPER||'tools/egg-engine/target/debug/long-egg.exe')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
const cases=[]
for(const c of m.cases){
 for(const f of c.members)assert.equal(sha(path.join(base,f.name)),f.sha256)
 for(const kind of ['independent','product']){
  const out=path.resolve('test-results/egg-154-'+kind,c.name+'-'+randomUUID());mkdirSync(out,{recursive:true})
  const r=kind==='independent'?spawnSync(tool,['x','-y','-cp:936',...(c.password?['-p:'+c.password]:[]),'-o:'+out,path.join(base,c.archive)],{windowsHide:true,timeout:60000}):spawnSync(helper,['extract',path.join(base,c.archive)],{input:JSON.stringify({password:c.password,encoding:c.encoding,output:out}),windowsHide:true,encoding:'utf8',timeout:60000})
  assert.ifError(r.error);assert.equal(r.status,0,c.name+' '+kind+': '+String(r.stdout)+String(r.stderr));assert.deepEqual(tree(out),c.expected,c.name+' '+kind);assert.ok(existsSync(path.join(out,c.emptyDirectory)))
 }
 cases.push({name:c.name,independentAndProductPayloads:true,originalNames:true,emptyDirectory:true})
}
writeFileSync(path.join(base,'independent.json'),JSON.stringify({checkedAt:new Date().toISOString(),toolSha256:sha(tool),helperSha256:sha(helper),cases},null,2)+'\n');console.log(cases)

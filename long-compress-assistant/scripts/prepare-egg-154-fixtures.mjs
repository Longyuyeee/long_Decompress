// Fix independent expectations before executing the product decoder.
import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {readFileSync,writeFileSync,mkdirSync,readdirSync,existsSync} from 'node:fs'
import {createHash} from 'node:crypto'
import path from 'node:path'
const base=path.resolve('tests/fixtures/archives/egg-153/upstream'),dest='tests/fixtures/archives/egg-154',tool='E:/Bandzip/Bandizip/bz.exe'
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
mkdirSync(dest,{recursive:true})
const inventory=path.join(dest,'manifest.json')
if(existsSync(inventory)){const m=JSON.parse(readFileSync(inventory,'utf8'));for(const c of m.cases)assert.equal(sha(path.resolve(dest,c.archive)),c.archiveSha256);console.log('Fixed complex originals retained.');process.exit(0)}
const cases=[]
for(const name of ['azo','azo_pe','solid','aes128','aes256','lea128','lea256','enc_aes128','enc_aes256','enc_aes256_bzip2','enc_lea_bzip2','optimal5']){
 const encrypted=/aes|lea/.test(name),password=encrypted?'test1234':null,archive=path.join(base,name+'.egg'),out=path.resolve('test-results/egg-154-independent',name)
 const r=spawnSync(tool,['x','-y',...(password?['-p:'+password]:[]),'-o:'+out,archive],{windowsHide:true,timeout:120000});assert.ifError(r.error);assert.equal(r.status,0,String(r.stdout)+String(r.stderr))
 const expected=tree(out);assert.ok(expected.length)
 cases.push({name,archive:path.relative(dest,archive).replaceAll('\\','/'),archiveSha256:sha(archive),password,expected})
}
writeFileSync(inventory,JSON.stringify({source:'unegg 0.2.1 original CC0 fixtures; original names retained, not inferred algorithms from names',tool,toolSha256:sha(tool),checkedAt:new Date().toISOString(),cases},null,2)+'\n')
console.log('Independent complex EGG groups: '+cases.length)

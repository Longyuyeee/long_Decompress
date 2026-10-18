// Original RAR 7.23 archives, never execute the generated SFX input.
import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,mkdirSync,readdirSync,existsSync} from 'node:fs'
import path from 'node:path'
const base=path.resolve('test-results/rar-156-originals'),manifest=path.join(base,'manifest.json')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
if(existsSync(manifest)){
  const m=JSON.parse(readFileSync(manifest,'utf8'))
  for(const c of m.cases)for(const f of c.members)assert.equal(sha(path.join(base,c.name,f.name)),f.sha256)
  console.log('Existing original RAR progress fixtures retained');process.exit(0)
}
const producer='E:/Winrar/Rar.exe',independent=path.resolve('src-tauri/resources/archive-engine/7z.exe'),source=path.join(base,'source')
mkdirSync(source,{recursive:true})
const payload=readFileSync('test-results/sfx-155-control/source/payload.bin').subarray(0,16*1024*1024)
writeFileSync(path.join(source,'payload.bin'),payload)
const expected={path:'payload.bin',bytes:payload.length,sha256:sha(path.join(source,'payload.bin'))},cases=[]
for(const layout of ['single','volumes','sfx'])for(const protection of ['plain','contents','headers']){
  const name=layout+'-'+protection,dir=path.join(base,name);mkdirSync(dir,{recursive:true})
  const password=protection==='plain'?null:'Rar156+中文123!',args=['a','-m0','-idq','-cfg-']
  if(layout==='sfx')args.push('-sfx')
  if(layout!=='single')args.push('-v4m')
  if(password)args.push((protection==='headers'?'-hp':'-p')+password)
  args.push(path.join(dir,'收到的资料.'+(layout==='sfx'?'exe':'rar')),'payload.bin')
  const r=spawnSync(producer,args,{cwd:source,encoding:'utf8',windowsHide:true,timeout:120000})
  assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr)
  const members=readdirSync(dir).filter(n=>/\.(rar|exe)$/.test(n)).sort().map(name=>({name,bytes:readFileSync(path.join(dir,name)).length,sha256:sha(path.join(dir,name))}))
  const archive=members.find(m=>layout==='sfx'?m.name.endsWith('.exe'):layout==='volumes'?/part1\.rar$/.test(m.name):true).name
  const independentOut=path.join(dir,'independent'),check=spawnSync(independent,['x','-tRAR5','-y','-sccUTF-8','-o'+independentOut,...(password?['-p'+password]:[]),path.join(dir,archive)],{encoding:'utf8',windowsHide:true,timeout:120000})
  assert.equal(check.status,0,check.stdout+check.stderr);assert.equal(sha(path.join(independentOut,'payload.bin')),expected.sha256)
  cases.push({name,layout,protection,password,archive,members,expected,independentReadback:true})
}
writeFileSync(manifest,JSON.stringify({createdAt:new Date().toISOString(),producer,producerSha256:sha(producer),independent,independentSha256:sha(independent),cases},null,2)+'\n')
console.log('Nine independent original RAR5 layouts/password combinations verified')

// Read-only extraction of the packaged helper. Does not execute the installer.
import assert from 'node:assert/strict'
import {execFileSync,spawnSync} from 'node:child_process'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {createHash} from 'node:crypto'
import path from 'node:path'
export function inspectEggInstaller(installer,destination){
  mkdirSync(destination,{recursive:true});const seven=path.resolve('src-tauri/resources/archive-engine/7z.exe'),lock=JSON.parse(readFileSync('egg-engine.lock.json','utf8')),sha=b=>createHash('sha256').update(b).digest('hex')
  const list=execFileSync(seven,['l','-slt',installer],{encoding:'utf8',maxBuffer:8*1024*1024}),paths=[...list.matchAll(/^Path = (.+)\r?$/gm)].map(m=>m[1].trim()),wanted=Object.entries(lock.files).filter(([n])=>n.startsWith('src-tauri/resources/egg-engine/')),bundled=new Map()
  for(const[n]of wanted){const suffix=n.slice('src-tauri/resources/'.length),match=paths.filter(p=>p.replaceAll('\\','/').endsWith(suffix));assert.equal(match.length,1,n);bundled.set(n,match[0])}
  execFileSync(seven,['x','-y','-o'+destination,installer,...bundled.values()],{maxBuffer:4*1024*1024})
  let engine,notices=0
  for(const[n,hash]of wanted){const bytes=readFileSync(path.join(destination,bundled.get(n)));assert.equal(sha(bytes),hash,n);if(n.endsWith('/long-egg.exe'))engine=bytes;if(n.includes('/licenses/'))notices++}
  assert.ok(engine);const tool=path.join(destination,'long-egg.exe');writeFileSync(tool,engine)
  const base=path.resolve('tests/fixtures/archives/egg-153'),m=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8')),originals=JSON.parse(readFileSync(path.join(base,'original-alzip/manifest.json'),'utf8')),cases=[]
  for(const[group,source,encoding]of[[m.cases,path.join(base,'upstream'),null],[originals.cases,path.join(base,'original-alzip'),'cp936']])for(const c of group){const r=spawnSync(tool,['verify',path.join(source,c.archive)],{input:JSON.stringify({password:c.password,encoding}),encoding:'utf8',windowsHide:true,timeout:30000});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr);assert.equal(JSON.parse(r.stdout).result.verified,true);cases.push({name:c.name,bundledEngineContentVerification:true})}
  const out=path.join(destination,'中文所选');mkdirSync(out,{recursive:true})
  const selected=spawnSync(tool,['extract',path.join(base,'original-alzip/native-key-volumes.vol3.egg')],{input:JSON.stringify({password:'PublicEGG+123!',output:out,selected:['说明 文件.txt']}),encoding:'utf8',windowsHide:true,timeout:30000});assert.equal(selected.status,0,selected.stdout+selected.stderr)
  assert.equal(sha(readFileSync(path.join(out,'说明 文件.txt'))),originals.cases[0].expected.find(e=>e.path==='说明 文件.txt').sha256)
  return{engineSha256:sha(engine),engineBytes:engine.length,noticesVerified:notices,cases,nativeMiddleMemberSelectedExtract:true,installationExecuted:false,engineExecutedFromExtractedInstaller:true}
}

import {inspectEggInstaller as inspectExisting} from './inspect-egg-154-installer.mjs'
import assert from 'node:assert/strict'
import {readFileSync,mkdirSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import path from 'node:path'
export function inspectEggInstaller(installer,destination){
 const existing=inspectExisting(installer,destination),base=path.resolve('tests/fixtures/archives/egg-1511'),manifest=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8')),helper=path.join(destination,'long-egg.exe'),cases=[],sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 for(const c of manifest.cases){for(const f of c.members)assert.equal(sha(path.join(base,f.name)),f.sha256);const r=spawnSync(helper,['verify',path.join(base,c.archive)],{input:JSON.stringify({password:c.password,encoding:'cp936'}),encoding:'utf8',windowsHide:true,timeout:30000});assert.equal(r.status,0,r.stdout+r.stderr);assert.equal(JSON.parse(r.stdout).result.verified,true);cases.push({name:c.name,bundledEngineCompleteVerification:true})}
 assert.equal(cases.length,20);const c=manifest.cases.find(c=>c.name==='ordinary-lea256-m3-volumes'),out=path.join(destination,'LEA中文中卷所选');mkdirSync(out,{recursive:true});const r=spawnSync(helper,['extract',path.join(base,c.members.find(f=>f.name.includes('.vol3.egg')).name)],{input:JSON.stringify({password:c.password,encoding:'cp936',output:out,selected:['文档/99最后.txt']}),encoding:'utf8',windowsHide:true,timeout:30000});assert.equal(r.status,0,r.stdout+r.stderr);assert.equal(sha(path.join(out,'文档/99最后.txt')),c.expected.find(e=>e.path==='文档/99最后.txt').sha256)
 return{...existing,chinesePasswordCases:cases,chinesePasswordGroupCount:20,totalOriginalGroups:55,chineseLeaLzmaMiddleSelected:true}
}

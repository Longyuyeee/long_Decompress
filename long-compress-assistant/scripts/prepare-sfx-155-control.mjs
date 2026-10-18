// Ignored real original RAR5 SFX volume control, never execute the output EXE.
import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {mkdirSync,copyFileSync,readFileSync,writeFileSync,existsSync,readdirSync} from 'node:fs'
import {createHash} from 'node:crypto'
import path from 'node:path'
const base=path.resolve('test-results/sfx-155-control'),source=path.join(base,'source'),tool='E:/Winrar/Rar.exe',sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex');mkdirSync(source,{recursive:true})
const manifest=path.join(base,'manifest.json');if(existsSync(manifest)){const m=JSON.parse(readFileSync(manifest,'utf8'));for(const f of m.members)assert.equal(sha(path.join(base,f.name)),f.sha256);console.log('Existing original SFX control retained');process.exit(0)}
copyFileSync('test-results/egg-153-control/control.bin',path.join(source,'payload.bin'))
const r=spawnSync(tool,['a','-sfx','-m0','-r','-idq','-cfg-','-v32m',path.join(base,'控制原包.exe'),'.\\*'],{cwd:source,encoding:'utf8',windowsHide:true,timeout:120000});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr)
const members=readdirSync(base).filter(n=>/\.(exe|rar)$/.test(n)).sort().map(name=>({name,bytes:readFileSync(path.join(base,name)).length,sha256:sha(path.join(base,name))}));writeFileSync(manifest,JSON.stringify({createdAt:new Date().toISOString(),producer:tool,producerSha256:sha(tool),archive:members.find(m=>m.name.endsWith('.exe')).name,members,expected:[{path:'payload.bin',bytes:readFileSync(path.join(source,'payload.bin')).length,sha256:sha(path.join(source,'payload.bin'))}]},null,2)+'\n');console.log('Original 256 MiB RAR5 SFX control ready')

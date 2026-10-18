// Real 512 MiB solid Deflate producer input, ignored large files for task control.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,mkdirSync,existsSync,copyFileSync,createReadStream} from 'node:fs'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
const sha=async p=>{const h=createHash('sha256');for await(const b of createReadStream(p))h.update(b);return h.digest('hex')}
const base=path.resolve('test-results/egg-154-control'),source=path.join(base,'source'),archive=path.join(base,'control.egg'),tool=path.resolve('test-results/alzip-152-portable/app/ALZipCon.exe')
assert.equal(await sha(tool),'13c664d8b809348104999efb734587e2847aab04e5511af2e5345e13d2528433');mkdirSync(source,{recursive:true})
const oldPath=path.join(base,'manifest.json')
if(existsSync(oldPath)){const m=JSON.parse(readFileSync(oldPath,'utf8'));assert.equal(await sha(archive),m.archiveSha256);for(const e of m.expected)assert.equal(await sha(path.join(source,e.path)),e.sha256);console.log('Existing original solid control retained.');process.exit(0)}
for(const name of ['first.bin','last.bin'])if(!existsSync(path.join(source,name)))copyFileSync('test-results/egg-153-control/control.bin',path.join(source,name))
if(!existsSync(archive)){const r=spawnSync(tool,['-a','-s','-m1',path.join(source,'*'),archive],{cwd:path.dirname(tool),windowsHide:true,timeout:180000});assert.ifError(r.error);assert.equal(r.status,0)}
const expected=[];for(const name of ['first.bin','last.bin'])expected.push({path:name,bytes:268435456,sha256:await sha(path.join(source,name))})
writeFileSync(oldPath,JSON.stringify({createdAt:new Date().toISOString(),generatorSha256:await sha(tool),archiveSha256:await sha(archive),bytes:536870912,expected},null,2)+'\n');console.log('Original 512 MiB solid control ready.')

import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {mkdirSync,writeFileSync,readFileSync,readdirSync,statSync} from 'node:fs'
import {createHash} from 'node:crypto'
import path from 'node:path'
const linux=p=>'/mnt/'+p[0].toLowerCase()+p.slice(2).replaceAll('\\','/')
const oracle=path.resolve('test-results/tar-1513/linux-oracle/extracted'),tool=path.join(oracle,'usr/bin/bsdtar'),run=args=>spawnSync('wsl.exe',['-d','Ubuntu','--','env','LC_ALL=C.UTF-8','LD_LIBRARY_PATH='+linux(path.join(oracle,'usr/lib/x86_64-linux-gnu')),linux(tool),...args],{encoding:'utf8',windowsHide:true,timeout:30000})
const sha=b=>createHash('sha256').update(b).digest('hex'),tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:statSync(path.join(p,e.name)).size,sha256:sha(readFileSync(path.join(p,e.name)))}]).sort((a,b)=>a.path.localeCompare(b.path))
const root=path.resolve('tests/fixtures/archives/tar-1513'),m=JSON.parse(readFileSync(path.join(root,'manifest.json'),'utf8')),cases=[]
for(const c of m.cases){const file=path.join(root,c.archive),out=path.resolve('test-results/tar-1513/independent-libarchive',c.name);assert.equal(sha(readFileSync(file)),c.original.sha256);mkdirSync(out,{recursive:true});const r=run(['-xf',linux(file),'-C',linux(out)]);assert.equal(r.status,0,r.stdout+r.stderr);assert.deepEqual(tree(out),[...c.expected].sort((a,b)=>a.path.localeCompare(b.path)),c.archive);cases.push({archive:c.archive,originalSha256:c.original.sha256,allLogicalContentHashes:true})}
const result={checkedAt:new Date().toISOString(),producer:m.producer,independent:run(['--version']).stdout.trim(),independentSha256:sha(readFileSync(tool)),source:'Ubuntu noble libarchive-tools/libarchive13t64 3.7.2-2, extracted without installing; validation-only, not shipped',cases}
assert.equal(result.cases.length,20);assert.ok(result.cases.every(c=>c.allLogicalContentHashes))
mkdirSync('docs/evidence/tar-sparse-1.5.13',{recursive:true});writeFileSync('docs/evidence/tar-sparse-1.5.13/originals.json',JSON.stringify(result,null,2)+'\n')
console.log('20 real GNU originals independently decoded by Linux libarchive; all logical sizes, names and hashes matched.')

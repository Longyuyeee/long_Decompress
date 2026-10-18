// Real original producer, ignored large data, for pause/resume/cancel.
import assert from 'node:assert/strict'
import { readFileSync,writeFileSync,mkdirSync,existsSync,copyFileSync } from 'node:fs'
import { createHash,randomBytes } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
const tool=path.resolve('test-results/alzip-152-portable/app/ALZipCon.exe'),base=path.resolve('test-results/egg-153-control'),password='PublicEGG+control!';mkdirSync(base,{recursive:true})
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex');assert.equal(sha(tool),'13c664d8b809348104999efb734587e2847aab04e5511af2e5345e13d2528433')
const source=path.join(base,'control.bin'),archive=path.join(base,'control.egg')
if(existsSync(path.join(base,'manifest.json'))){const old=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8'));assert.equal(sha(archive),old.archiveSha256);assert.equal(sha(source),old.sourceSha256);console.log('Existing original control archive retained and verified.');process.exit(0)}
if(!existsSync(source)){if(existsSync('test-results/alz-152-control/control.bin'))copyFileSync('test-results/alz-152-control/control.bin',source);else writeFileSync(source,randomBytes(256*1024*1024));}
if(!existsSync(archive)){const r=spawnSync(tool,['-a','-m0','-p'+password,source,archive],{cwd:path.dirname(tool),windowsHide:true,timeout:180000});assert.ifError(r.error);assert.equal(r.status,0)}
const manifest={createdAt:new Date().toISOString(),generatorSha256:sha(tool),archiveSha256:sha(archive),sourceSha256:sha(source),bytes:readFileSync(source).length,password}
writeFileSync(path.join(base,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');console.log(manifest)

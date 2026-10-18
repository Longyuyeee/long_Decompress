// Genuine original-producer archive for interrupting a running ALZ decoder.
// Large random data stays ignored; the report carries its exact provenance.
import assert from 'node:assert/strict'
import { randomBytes, createHash } from 'node:crypto'
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
const root=path.resolve('test-results/alz-152-control');mkdirSync(root,{recursive:true})
const tool=path.resolve('test-results/alzip-152-portable/app/ALZipCon.exe')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
assert.equal(sha(tool),'13c664d8b809348104999efb734587e2847aab04e5511af2e5345e13d2528433')
const archive=path.join(root,'control.alz'),source=path.join(root,'control.bin'),password='PublicALZ+control!'
if(!existsSync(source))writeFileSync(source,randomBytes(256*1024*1024))
if(!existsSync(archive)) {
  const r=spawnSync(tool,['-a','-m0','-p'+password,source,archive],{cwd:path.dirname(tool),windowsHide:true,timeout:180000})
  assert.ifError(r.error);assert.equal(r.status,0)
}
const report={createdAt:new Date().toISOString(),generatorSha256:sha(tool),archiveSha256:sha(archive),sourceSha256:sha(source),bytes:readFileSync(source).length,password}
writeFileSync(path.join(root,'manifest.json'),JSON.stringify(report,null,2)+'\n')
console.log(report)

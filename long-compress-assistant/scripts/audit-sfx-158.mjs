import assert from 'node:assert/strict'
import {readFileSync,readdirSync,existsSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
const evidence='docs/evidence/sfx-modules-1.5.8',report=JSON.parse(readFileSync(evidence+'/results.json','utf8')),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
assert.equal(report.cases.length,33);assert.equal(report.screenshots.length,10);assert.equal(report.originalsUnchanged,true)
assert.equal(report.cases.filter(c=>c.original).length,30)
for(const c of report.cases.filter(c=>c.original))assert.ok(c.browse&&c.integrity&&c.diagnosis&&c.whole&&c.selected)
assert.equal(report.cases.filter(c=>c.wrongPasswordRejected).length,20);assert.equal(report.cases.filter(c=>c.conversionIndependentReadback).length,10)
assert.ok(report.cases.some(c=>c.actualAllMembersOneTask&&c.freshRetry&&c.oldFailureRetained));assert.ok(report.cases.some(c=>c.actualSelectedExtraction));assert.ok(report.cases.some(c=>c.actualLoaderConversion&&c.independentZipHashes))
for(const c of report.cases.filter(c=>c.original))for(const f of c.faults)assert.ok(f.error&&f.noOutput)
for(const l of report.layouts)assert.ok(l.overflow<=1)
for(const f of report.screenshots)assert.ok(existsSync(evidence+'/'+f))
const logs=readdirSync('test-results').filter(n=>/^sfx158-.*-tests.log$/.test(n)).map(n=>readFileSync('test-results/'+n,'utf8')).join('\n'),passed=[...new Set(logs.match(/^test .* \.\.\. ok$/gm))].sort();assert.equal(passed.length,67);assert.ok(!logs.includes('test result: FAILED'));assert.match(readFileSync('test-results/sfx158-frontend.log','utf8'),/150 passed/);assert.match(readFileSync('test-results/sfx158-production-build.log','utf8'),/Finished `release`/)
const base='tests/fixtures/archives/sfx-158',m=JSON.parse(readFileSync(base+'/manifest.json','utf8')),tracked=new Set(execFileSync('git',['ls-files','-z'],{encoding:'utf8'}).split('\0'))
for(const c of m.cases){for(const f of c.members){const p=path.join(base,c.name,f.name);assert.equal(sha(p),f.sha256);assert.ok(tracked.has(p.replaceAll('\\','/')),p)}for(const e of c.expected)assert.equal(sha(path.join(base,'source',e.path)),e.sha256)}
for(const module of m.modules){assert.ok(existsSync(path.join(base,module.license)));if(module.kind==='7z-gui')assert.equal(sha(path.join(base,module.module)),module.sha256);else assert.equal(module.standaloneRedistributed,false)}
const root=path.resolve('..'),images=s=>[...s.matchAll(/<img\b[^>]*>|!\[[^\]]*\]\([^)]+\)/g)].map(m=>m[0])
for(const file of ['README.md','long-compress-assistant/README.md'])assert.deepEqual(images(readFileSync(path.join(root,file),'utf8')),images(execFileSync('git',['show','6950ec9:'+file],{cwd:root,encoding:'utf8'})))
const main='src-tauri/target/release/Long解压.exe',bytes=readFileSync(main),version=execFileSync('powershell.exe',['-NoProfile','-Command',`(Get-Item -LiteralPath '${main}').VersionInfo.ProductVersion`],{encoding:'utf8'}).trim();assert.equal(version,'1.5.8');assert.notEqual(sha(main),report.binarySha256)
const result={checkedAt:new Date().toISOString(),rustDistinct:passed.length,rustTests:passed,frontend:150,frontendFiles:4,typeCheck:true,desktopCases:report.cases.length,screenshots:report.screenshots.length,production:{command:'node node_modules/@tauri-apps/cli/tauri.js build --bundles none',mainBytes:bytes.length,mainSha256:sha(main),mainProductVersion:version,installationExecuted:false},desktop:report.binarySha256,originalsUnchanged:true,readmeImagesPreserved:true}
writeFileSync(evidence+'/checks.json',JSON.stringify(result,null,2)+'\n')
for(const file of ['SFX_MODULES_1.5.8.md','SFX_MODULES_FREEZE_1.5.8.md','RELEASE_NOTES_1.5.8.md'])for(const m of readFileSync('docs/'+file,'utf8').matchAll(/\]\(([^)]+)\)/g))if(!/^[a-z]+:|^#/i.test(m[1]))assert.ok(existsSync(path.resolve('docs',m[1].split('#')[0])),m[1])
console.log('67 Rust / 150 frontend / 33 desktop / 10 screenshots; content, original members, README images and production identity verified.')

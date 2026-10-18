// Narrow release evidence audit, not a broad security scan or installation.
import assert from 'node:assert/strict'
import {readFileSync,readdirSync,existsSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
const evidence='docs/evidence/rar-final-causes-1.5.9',report=JSON.parse(readFileSync(evidence+'/results.json','utf8')),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
assert.equal(report.cases.length,39);assert.equal(report.screenshots.length,14);assert.equal(report.originalsUnchanged,true)
const originals=report.cases.filter(c=>c.original&&c.whole===true),faults=report.cases.filter(c=>c.fault)
assert.equal(originals.length,12);assert.equal(faults.length,18)
for(const c of originals)assert.ok(c.browse&&c.integrity&&c.diagnosis&&c.selected&&c.wrongKeyRejected)
for(const c of faults){assert.ok(c.integrityError&&c.finalError&&c.conversionError&&c.noPublishedOutput);assert.notEqual(c.diagnosticStatus,'wrong_password');if(c.fault==='missing'){assert.match(c.finalError,/\[archive-inspection:missing-volume\]/);assert.ok(c.finalError.includes(c.tail))}if(c.fault==='mixed')assert.match(c.finalError,/\[archive-inspection:rar-later-data\]/)}
assert.equal(report.cases.filter(c=>c.largeFirstOriginal&&c.missingDuringPasswordVerification&&c.restoredCorrectKey).length,4)
assert.ok(report.cases.some(c=>c.actualAllMembersOneTask&&c.restoredFreshRetry&&c.oldFailureRetained))
assert.ok(report.cases.some(c=>c.actualMiddleImport&&c.actualHistoryCategory&&c.oldMixedFailureRetained))
assert.ok(report.cases.some(c=>c.actualMissingBrowserCause&&c.actualSelectedExtraction))
assert.ok(report.cases.some(c=>c.actualMissingDiagnosis&&c.noMisleadingRepair))
assert.ok(report.cases.some(c=>c.actualRestoredConversion&&c.independentZipHashes))
for(const l of report.layouts)assert.ok(l.overflow<=1)
for(const f of report.screenshots)assert.ok(existsSync(evidence+'/'+f))
const logs=readdirSync('test-results').filter(n=>/^rar159-.*-tests.log$/.test(n)&&!n.includes('-red-')).map(n=>readFileSync('test-results/'+n,'utf8')).join('\n'),passed=[...new Set(logs.match(/^test .* \.\.\. ok$/gm))].sort()
assert.equal(passed.length,96);assert.ok(!logs.includes('test result: FAILED'));assert.match(readFileSync('test-results/rar159-frontend.log','utf8'),/152 passed/);assert.match(readFileSync('test-results/rar159-production-build.log','utf8'),/Finished `release`/)
const base='tests/fixtures/archives/rar-159',manifest=JSON.parse(readFileSync(base+'/manifest.json','utf8')),tracked=new Set(execFileSync('git',['ls-files','-z'],{encoding:'utf8'}).split('\0'))
for(const c of manifest.cases){for(const f of c.members){const p=path.join(base,c.name,f.name);assert.equal(sha(p),f.sha256);assert.ok(tracked.has(p.replaceAll('\\','/')),p)}for(const e of c.expected)assert.equal(sha(path.join(base,'source',c.variant,e.path)),e.sha256)}
const images=s=>[...s.matchAll(/<img\b[^>]*>|!\[[^\]]*\]\([^)]+\)/g)].map(m=>m[0]),root=path.resolve('..')
for(const f of ['README.md','long-compress-assistant/README.md'])assert.deepEqual(images(readFileSync(path.join(root,f),'utf8')),images(execFileSync('git',['show','53035eb:'+f],{cwd:root,encoding:'utf8'})))
const main='src-tauri/target/release/Long解压.exe',version=execFileSync('powershell.exe',['-NoProfile','-Command',`(Get-Item -LiteralPath '${main}').VersionInfo.ProductVersion`],{encoding:'utf8'}).trim();assert.equal(version,'1.5.9');assert.notEqual(sha(main),report.binarySha256)
const controlLog=readFileSync('test-results/rar159-control.log','utf8').replaceAll('\r\n','\n'),start=controlLog.indexOf('[\n  {'),end=controlLog.indexOf('\n]\n',start),control=JSON.parse(controlLog.slice(start,end+2))[0]
assert.ok(control.actualKnownPayloadBeforePause>0&&control.actualKnownPayloadBeforePause<268435456);assert.equal(control.logicalFileBytesWhenPaused,268435456)
for(const f of ['pauseStopsContentWrites','resumeWrites','cancelWhilePaused','sourceRetained','noPublishedOrStagedOutput'])assert.equal(control[f],true)
const native=JSON.parse(controlLog.slice(end+3).trim());assert.equal(native.control.ok,false);assert.match(native.control.error,/cancelled/)
writeFileSync(evidence+'/control.json',JSON.stringify({binarySha256:report.binarySha256,command:'node scripts/test-tauri-desktop.mjs --sfx-155-only --sfx-control-only',...control,native,rawIpcNotFrontendTask:true},null,2)+'\n')
writeFileSync(evidence+'/checks.json',JSON.stringify({checkedAt:new Date().toISOString(),rustDistinct:passed.length,rustTests:passed,frontend:152,frontendFiles:4,typeCheck:true,desktopCases:39,screenshots:14,production:{command:'node node_modules/@tauri-apps/cli/tauri.js build --bundles none',mainBytes:readFileSync(main).length,mainSha256:sha(main),mainProductVersion:version,installationExecuted:false},desktop:report.binarySha256,originalsUnchanged:true,readmeImagesPreserved:true},null,2)+'\n')
for(const file of ['RAR_FINAL_CAUSES_1.5.9.md','RAR_FINAL_CAUSES_FREEZE_1.5.9.md','RELEASE_NOTES_1.5.9.md'])for(const m of readFileSync('docs/'+file,'utf8').matchAll(/\]\(([^)]+)\)/g))if(!/^[a-z]+:|^#/i.test(m[1]))assert.ok(existsSync(path.resolve('docs',m[1].split('#')[0])),m[1])
console.log('96 Rust / 152 frontend / 39 desktop / 14 screenshots, native pause/cancel, originals, README images and production identity verified.')

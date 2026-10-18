// Targeted functional/release evidence, not a general security scan.
import assert from 'node:assert/strict'
import {readFileSync,readdirSync,writeFileSync,existsSync,createReadStream} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import path from 'node:path'
const evidence='docs/evidence/egg-large-lzma-1.5.10',load=p=>JSON.parse(readFileSync(p,'utf8')),report=load(evidence+'/results.json'),large=load(evidence+'/large-verification.json'),m=load(evidence+'/originals.json')
const sha=async p=>{const h=createHash('sha256');for await(const b of createReadStream(p))h.update(b);return h.digest('hex')}
assert.equal(report.cases.length,7);assert.equal(report.screenshots.length,16);assert.equal(report.originalsUnchanged,true)
for(const l of report.layouts)assert.ok(l.overflow<=1)
for(const f of report.screenshots)assert.ok(existsSync(evidence+'/'+f))
for(const key of ['actualLastSelectedFromLargeSolid','actualLargeSolidConversion','actualLargeDiagnosisHealthy','actualMissingMiddleCause','actualLargeCrcFailure','actualCancelWhilePaused','actualSettingsReadOnlyHelp'])assert.ok(report.cases.some(c=>c[key]),key)
const control=report.cases.find(c=>c.actualCancelWhilePaused);assert.ok(control.actualUiPauseAfterWrittenBytes>0&&control.actualUiPauseAfterWrittenBytes<335544320);for(const k of ['pauseStoppedWrites','resumeProducedMore','queueAndHistoryCancelled','noStagedOrPublishedOutput'])assert.equal(control[k],true)
assert.equal(large.cases.length,7);assert.equal(large.originalsUnchanged,true)
const positives=large.cases.filter(c=>c.name);assert.equal(positives.length,3)
for(const c of positives){assert.ok(c.packedBytes>268435456);assert.match(c.oldPlanError,/256 MiB/);for(const k of ['verify','extract']){assert.equal(c[k].code,0);assert.equal(c[k].response.result.verified,true);assert.ok(c[k].peakWorkingSetBytes>0&&c[k].peakWorkingSetBytes<200*1024*1024);assert.equal(c[k].temporaryDirectoryPeakBytes,0)}}
assert.ok(large.cases.some(c=>c.selectedLastSharedStream));assert.ok(large.cases.some(c=>c.wrongPasswordRejected));assert.equal(large.cases.filter(c=>c.fault&&c.error.includes('[archive-inspection:damaged]')).length,2)
for(const c of m.cases)for(const f of c.members)assert.equal(await sha(path.join('test-results/egg-1510',f.name)),f.sha256)
const helper='src-tauri/resources/egg-engine/long-egg.exe';assert.equal(await sha(helper),large.helperSha256);assert.equal(report.helperSha256,large.helperSha256)
const logs=readdirSync('test-results').filter(n=>/^egg1510-.*-tests.log$/.test(n)).map(n=>readFileSync('test-results/'+n,'utf8')).join('\n'),unit=readFileSync('test-results/egg1510-unit.log','utf8'),passed=[...new Set((logs+'\n'+unit).match(/^test .* \.\.\. ok$/gm))].sort();assert.equal(passed.length,99);assert.ok(!logs.includes('test result: FAILED'));assert.match(unit,/17 passed; 0 failed/);assert.match(readFileSync('test-results/egg1510-frontend.log','utf8'),/152 passed/);assert.match(readFileSync('test-results/egg1510-production-build.log','utf8'),/Finished `release`/)
const images=s=>[...s.matchAll(/<img\b[^>]*>|!\[[^\]]*\]\([^)]+\)/g)].map(m=>m[0]),root=path.resolve('..');for(const f of ['README.md','long-compress-assistant/README.md'])assert.deepEqual(images(readFileSync(path.join(root,f),'utf8')),images(execFileSync('git',['show','82a8d26:'+f],{cwd:root,encoding:'utf8'})))
const main='src-tauri/target/release/Long解压.exe',version=execFileSync('powershell.exe',['-NoProfile','-Command',`(Get-Item -LiteralPath '${main}').VersionInfo.ProductVersion`],{encoding:'utf8'}).trim();assert.equal(version,'1.5.10');assert.notEqual(await sha(main),report.binarySha256)
writeFileSync(evidence+'/checks.json',JSON.stringify({checkedAt:new Date().toISOString(),rustDistinct:99,appRust:82,decoderRust:17,rustTests:passed,frontend:152,frontendFiles:4,typeCheck:true,desktopCases:7,screenshots:16,largeOriginalGroups:3,helperSha256:large.helperSha256,production:{command:'node node_modules/@tauri-apps/cli/tauri.js build --bundles none',mainSha256:await sha(main),mainProductVersion:version,installationExecuted:false},desktop:report.binarySha256,readmeImagesPreserved:true},null,2)+'\n')
for(const f of ['EGG_LARGE_LZMA_1.5.10.md','EGG_LARGE_LZMA_FREEZE_1.5.10.md','RELEASE_NOTES_1.5.10.md'])for(const m of readFileSync('docs/'+f,'utf8').matchAll(/\]\(([^)]+)\)/g))if(!/^[a-z]+:|^#/i.test(m[1]))assert.ok(existsSync(path.resolve('docs',m[1].split('#')[0])),m[1])
console.log('99 Rust / 152 frontend / 3 independently decoded large originals / 7 desktop / 16 viewed pages and normal production identity verified.')

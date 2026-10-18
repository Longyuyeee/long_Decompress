import assert from 'node:assert/strict'
import {readFileSync,readdirSync,existsSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
const report=JSON.parse(readFileSync('docs/evidence/rar-progress-1.5.6/results.json','utf8'))
assert.equal(report.cases.length,12);assert.equal(report.screenshots.length,8)
for(const c of report.cases.filter(c=>c.original))assert.ok(c.noOutputLengthEstimate&&c.outputSha256===c.expected.sha256)
for(const c of report.cases.filter(c=>c.actualQueuePauseResumeComplete||c.actualQueueCancel)){
  assert.ok(c.actualKnownBytesBeforePause>1024*1024&&c.actualKnownBytesBeforePause<c.logicalBytes)
  assert.ok(c.reportedBytes>=c.actualKnownBytesBeforePause&&c.reportedBytes<c.logicalBytes)
  if(c.actualQueuePauseResumeComplete){assert.ok(Math.abs(c.reportedProgress-c.reportedBytes/c.logicalBytes)<1e-6);assert.ok(Math.abs(c.excludedMs-c.pausedMs)<250)}
}
assert.ok(report.cases.some(c=>c.lateCrcFailure&&c.noFalseCompletion))
for(const l of report.layouts)assert.ok(l.overflow<=1)
for(const f of report.screenshots)assert.ok(existsSync('docs/evidence/rar-progress-1.5.6/'+f))
const logs=readdirSync('test-results').filter(n=>/^rar156-final-.*-tests.log$/.test(n)).map(n=>readFileSync('test-results/'+n,'utf8')).join('\n')
const passed=new Set(logs.match(/^test .* \.\.\. ok$/gm));assert.equal(passed.size,49);assert.ok(!logs.includes('test result: FAILED'))
assert.match(readFileSync('test-results/rar156-frontend.log','utf8'),/133 passed/)
const root=path.resolve('..'),images=s=>[...s.matchAll(/<img\b[^>]*>|!\[[^\]]*\]\([^)]+\)/g)].map(m=>m[0])
for(const file of ['README.md','long-compress-assistant/README.md']){
  const before=execFileSync('git',['show','5dc0211:'+file],{cwd:root,encoding:'utf8'}),after=readFileSync(path.join(root,file),'utf8');assert.deepEqual(images(after),images(before),file)
}
for(const file of ['RAR_PROGRESS_1.5.6.md','RAR_PROGRESS_FREEZE_1.5.6.md','RELEASE_NOTES_1.5.6.md']){
  for(const m of readFileSync('docs/'+file,'utf8').matchAll(/\]\(([^)]+)\)/g)){if(!/^[a-z]+:|^#/i.test(m[1]))assert.ok(existsSync(path.resolve('docs',m[1].split('#')[0])),m[1])}
}
console.log('49 distinct Rust / 133 frontend / 12 real desktop cases / 8 pages; real byte totals, pause clocks, late CRC rejection, source/output identities, links and original README images verified.')

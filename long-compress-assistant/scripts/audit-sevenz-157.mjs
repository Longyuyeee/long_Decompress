import assert from 'node:assert/strict'
import {readFileSync,readdirSync,existsSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
const report=JSON.parse(readFileSync('docs/evidence/sevenz-methods-1.5.7/results.json','utf8'))
assert.equal(report.cases.length,25);assert.equal(report.screenshots.length,8);assert.equal(report.originalsUnchanged,true)
assert.equal(report.cases.filter(c=>c.original).length,18);assert.equal(report.cases.filter(c=>c.corruptMethod&&c.noPublishedOutput).length,4)
for(const c of report.cases.filter(c=>c.original))assert.ok(c.browse&&c.integrity&&c.whole&&c.solidSelected)
assert.ok(report.cases.some(c=>c.actualPasswordCorrection&&c.oldFailureRetained));assert.ok(report.cases.some(c=>c.actualSelectedExtraction));assert.ok(report.cases.some(c=>c.actualEncryptedPpmdConversion&&c.independentZipHashes))
for(const l of report.layouts)assert.ok(l.overflow<=1)
for(const f of report.screenshots)assert.ok(existsSync('docs/evidence/sevenz-methods-1.5.7/'+f))
const logs=readdirSync('test-results').filter(n=>/^sevenz157-.*-tests.log$/.test(n)).map(n=>readFileSync('test-results/'+n,'utf8')).join('\n'),passed=new Set(logs.match(/^test .* \.\.\. ok$/gm))
assert.equal(passed.size,61);assert.ok(!logs.includes('test result: FAILED'));assert.match(readFileSync('test-results/sevenz157-frontend.log','utf8'),/150 passed/)
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),base='tests/fixtures/archives/sevenz-157',m=JSON.parse(readFileSync(base+'/manifest.json','utf8'))
for(const c of m.cases){for(const f of c.members)assert.equal(sha(path.join(base,c.name,f.name)),f.sha256);for(const e of c.expected)assert.equal(sha(path.join(base,'source',e.path)),e.sha256)}
const root=path.resolve('..'),images=s=>[...s.matchAll(/<img\b[^>]*>|!\[[^\]]*\]\([^)]+\)/g)].map(m=>m[0])
for(const file of ['README.md','long-compress-assistant/README.md'])assert.deepEqual(images(readFileSync(path.join(root,file),'utf8')),images(execFileSync('git',['show','b9b5292:'+file],{cwd:root,encoding:'utf8'})))
for(const file of ['SEVENZ_METHODS_1.5.7.md','SEVENZ_METHODS_FREEZE_1.5.7.md','RELEASE_NOTES_1.5.7.md'])for(const m of readFileSync('docs/'+file,'utf8').matchAll(/\]\(([^)]+)\)/g))if(!/^[a-z]+:|^#/i.test(m[1]))assert.ok(existsSync(path.resolve('docs',m[1].split('#')[0])),m[1])
console.log('61 distinct Rust / 150 frontend / 25 desktop cases / 8 pages; original source and volume hashes, final causes, selected output, conversion, documentation links and README images verified.')

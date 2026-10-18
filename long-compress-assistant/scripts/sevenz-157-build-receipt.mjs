import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,readdirSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {execFileSync} from 'node:child_process'
const logs=readdirSync('test-results').filter(n=>/^sevenz157-.*-tests.log$/.test(n)),text=logs.map(n=>readFileSync('test-results/'+n,'utf8')).join('\n'),passed=[...new Set(text.match(/^test .* \.\.\. ok$/gm))].sort()
assert.equal(passed.length,61);assert.ok(!text.includes('test result: FAILED'));assert.match(readFileSync('test-results/sevenz157-frontend.log','utf8'),/150 passed/)
assert.match(readFileSync('test-results/sevenz157-production-build.log','utf8'),/Finished `release`/)
const main='src-tauri/target/release/Long解压.exe',bytes=readFileSync(main),version=execFileSync('powershell.exe',['-NoProfile','-Command',`(Get-Item -LiteralPath '${main}').VersionInfo.ProductVersion`],{encoding:'utf8'}).trim();assert.equal(version,'1.5.7')
const receipt={checkedAt:new Date().toISOString(),rustDistinct:passed.length,rustTests:passed,frontend:150,frontendFiles:4,typeCheck:true,production:{command:'node node_modules/@tauri-apps/cli/tauri.js build --bundles none',mainBytes:bytes.length,mainSha256:createHash('sha256').update(bytes).digest('hex'),mainProductVersion:version,installationExecuted:false},desktop:JSON.parse(readFileSync('docs/evidence/sevenz-methods-1.5.7/results.json','utf8')).binarySha256}
assert.notEqual(receipt.production.mainSha256,receipt.desktop);writeFileSync('docs/evidence/sevenz-methods-1.5.7/checks.json',JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify(receipt.production))

// Aggregate completed local checks. Screenshots are viewed by the reviewer,
// not inferred as visually correct merely because a file exists.
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,copyFileSync,mkdirSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
const read=p=>readFileSync(p,'utf8'),sha=b=>createHash('sha256').update(b).digest('hex')
const dest='docs/evidence/tar-routing-1.5.14',desktop=JSON.parse(read('test-results/desktop-e2e/tar-1514-results.json'))
assert.equal(desktop.cases.length,36);assert.equal(desktop.screenshots.length,18);assert.equal(desktop.originalsAndFaultsUnchanged,true)
assert.equal(process.argv[2],'--eighteen-screenshots-reviewed','Reviewer must actually inspect all final pages first')
const isolated=process.argv[3];assert.match(isolated,/^desktop-e2e-data-[0-9a-f]+$/)
const database='/mnt/e/ProjectSave/ClaudeProject/Long-Decompress/long-compress-assistant/test-results/'+isolated+'/data.db'
const sql='import sqlite3,json,sys; c=sqlite3.connect("file:"+sys.argv[1]+"?mode=ro",uri=True); c.row_factory=sqlite3.Row; print(json.dumps([dict(r) for r in c.execute("SELECT id,status,processed_bytes,total_bytes,error_message FROM task_operation_history WHERE source_paths LIKE ?",("%control.tar%",))]))'
const sqlHistory=JSON.parse(execFileSync('wsl.exe',['-d','Ubuntu','--exec','python3','-c',sql,database],{encoding:'utf8'}));assert.deepEqual(sqlHistory.map(r=>r.status).sort(),['cancelled','completed']);assert.ok(sqlHistory.every(r=>r.total_bytes===2147483665&&r.error_message===null))
const filters=['tar_compatibility','extraction_transaction-tests','cancellation','archive_browser-tests','archive_preview-tests','compression_verification-tests','native_compression-tar-tests','task_control-tests','tar_aes_engine-tests','codec_stream-tests','archive_diagnostics-tests','preflight']
const tests=new Set(),ignored=[]
for(const name of filters){const log=read('test-results/tar1514-'+name+'.log');assert.match(log,/test result: ok\./);assert.doesNotMatch(log,/test result: FAILED\./);for(const m of log.matchAll(/^test ([^\r\n]+) \.\.\. ok\r?$/gm))tests.add(m[1]);for(const m of log.matchAll(/^test ([^\r\n]+) \.\.\. ignored[^\r\n]*$/gm))ignored.push(m[1])}
const frontend=read('test-results/tar1514-frontend.log');assert.match(frontend,/Test Files\s+6 passed/);assert.match(frontend,/Tests\s+170 passed/)
assert.match(read('test-results/tar1514-identity.log'),/Release identity verified: v1\.5\.13/)
assert.match(read('test-results/tar1514-production-build.log'),/Finished `release` profile/)
const main=path.resolve('src-tauri/target/release/Long解压.exe'),version=execFileSync('powershell.exe',['-NoProfile','-Command',`(Get-Item -LiteralPath '${main.replaceAll("'","''")}').VersionInfo.ProductVersion`],{encoding:'utf8'}).trim();assert.equal(version,'1.5.14')
for(const file of['../README.md','README.md']){
  const repoPath=file.startsWith('../')?'README.md':'long-compress-assistant/README.md',before=execFileSync('git',['show','794fcc9:'+repoPath],{encoding:'utf8'}),images=s=>s.match(/<img\b[^>]*>|!\[[^\]]*\]\([^)]*\)/g)||[]
  assert.deepEqual(images(read(file)),images(before))
}
mkdirSync(dest,{recursive:true});copyFileSync('test-results/desktop-e2e/tar-1514-results.json',dest+'/results.json')
const screenshots=desktop.screenshots.map(file=>{const bytes=readFileSync('test-results/desktop-e2e/'+file);copyFileSync('test-results/desktop-e2e/'+file,dest+'/'+file);return{file,bytes:bytes.length,sha256:sha(bytes),reviewed:true}})
const result={checkedAt:new Date().toISOString(),runtimeSourceCommit:process.env.TAR_RUNTIME_SOURCE_COMMIT,rustDistinct:tests.size,rustTests:[...tests].sort(),ignoredNotCounted:ignored,frontend:170,frontendFiles:6,typeCheck:true,desktopCases:36,viewedScreenshots:18,screenshots,originals:20,derivedFaults:4,production:{mainProductVersion:version,mainSha256:sha(readFileSync(main)),installationExecuted:false},desktopSha256:desktop.binarySha256,actualSparseControl:desktop.cases.at(-1),isolatedDatabase:isolated,sqlHistory,readmeImagesPreserved:true,notes:'TAR logical sizes/payload correctness and real controls verified. Fine-grained TAR runtime byte percentage is not delivered by this version; Windows output is a regular zero-filled file, not an NTFS sparse allocation promise.'}
assert.notEqual(result.production.mainSha256,result.desktopSha256)
writeFileSync(dest+'/checks.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({...result,rustTests:undefined,screenshots:undefined},null,2))

// Candidate / official receipt. Never installs or runs the personal application.
import assert from 'node:assert/strict'
import {inspectEggInstaller} from './inspect-egg-1511-installer.mjs'
import {inspectSfxInstaller} from './inspect-sfx-155-installer.mjs'
import {readFileSync,writeFileSync,mkdirSync,readdirSync,existsSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import path from 'node:path'
const version='1.5.11',tag='v'+version,mode=process.argv[2]
const gh=args=>execFileSync('gh',args,{encoding:'utf8',maxBuffer:8*1024*1024}),sha=b=>createHash('sha256').update(b).digest('hex')
const productVersion=p=>execFileSync('powershell.exe',['-NoProfile','-Command',`(Get-Item -LiteralPath '${p.replaceAll("'","''")}').VersionInfo.ProductVersion`],{encoding:'utf8'}).trim()
const api=()=>JSON.parse(gh(['api','repos/Longyuyeee/long_Decompress/releases/tags/'+tag]))
if(mode==='candidate'){
  const [candidateRun,candidateCommit,functionalFreeze]=process.argv.slice(3),run=JSON.parse(gh(['run','view',candidateRun,'--json','status,conclusion,headSha']))
  assert.equal(run.status,'completed');assert.equal(run.conclusion,'success');assert.equal(run.headSha,candidateCommit)
  const dest=path.resolve('test-results/v1.5.11-candidate');mkdirSync(dest,{recursive:true});gh(['run','download',candidateRun,'-n','windows-nsis-installer','-D',dest])
  const files=readdirSync(dest).filter(f=>f.endsWith('.exe'));assert.equal(files.length,1);const file=path.join(dest,files[0]),bytes=readFileSync(file);assert.equal(productVersion(file),version)
  const result={checkedAt:new Date().toISOString(),functionalFreeze,candidateCommit,candidateRun:Number(candidateRun),candidateTree:execFileSync('git',['rev-parse',candidateCommit+'^{tree}'],{encoding:'utf8'}).trim(),installer:files[0],installerBytes:bytes.length,installerSha256:sha(bytes),installerProductVersion:version,packagedMain:inspectSfxInstaller(file,path.join(dest,'main-and-notice'),version,['archive-engine/licenses/sevenz-rust-Apache-2.0.txt','archive-engine/licenses/ppmd-rust-MIT-0.md'])}
  result.packagedEgg=inspectEggInstaller(file,path.join(dest,'egg-helper'))
  writeFileSync(path.join(dest,'verified.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result,null,2));process.exit(0)
}
const tagCommit=execFileSync('git',['rev-parse',tag+'^{commit}'],{encoding:'utf8'}).trim()
const releaseRun=JSON.parse(gh(['run','list','--workflow','release.yml','--branch',tag,'--limit','5','--json','databaseId,headSha,conclusion,status'])).find(r=>r.headSha===tagCommit&&r.status==='completed'&&r.conclusion==='success')
assert.ok(releaseRun,'Formal workflow must finish successfully before notes edits')
const body=readFileSync('docs/RELEASE_NOTES_1.5.11.md','utf8').replace(/\]\((EGG_PASSWORD_COMBINATIONS(?:_FREEZE)?_1\.5\.11\.md)\)/g,'](https://github.com/Longyuyeee/long_Decompress/blob/master/long-compress-assistant/docs/$1)').trim()
const original=path.resolve('test-results/v1.5.11-release-original'),chinese=path.resolve('test-results/v1.5.11-release-chinese'),final=path.resolve('test-results/v1.5.11-release-final')
if(mode==='notes'){
  assert.equal(api().draft,false);mkdirSync(original,{recursive:true});mkdirSync(chinese,{recursive:true});writeFileSync(path.join(chinese,'body.md'),body+'\n')
  gh(['release','edit',tag,'--notes-file',path.join(chinese,'body.md')]);if(!existsSync(path.join(original,'latest.json')))gh(['release','download',tag,'-p','latest.json','-D',original])
  const before=JSON.parse(readFileSync(path.join(original,'latest.json'),'utf8'));assert.equal(before.version,version)
  const next={...before,notes:body},a={...before},b={...next};delete a.notes;delete b.notes;assert.deepEqual(a,b)
  writeFileSync(path.join(chinese,'latest.json'),JSON.stringify(next,null,2)+'\n');gh(['release','upload',tag,path.join(chinese,'latest.json'),'--clobber']);console.log('Chinese notes published; signature, URL, date and all non-notes fields preserved.')
}else if(mode==='verify'){
  const release=api();assert.equal(release.draft,false);assert.equal(release.prerelease,false)
  const exe=`Long-Decompress_${version}_x64-setup.exe`,zip=`Long-Decompress_${version}_x64-setup.nsis.zip`;mkdirSync(final,{recursive:true});const assets=[]
  for(const name of [exe,zip,zip+'.sig','latest.json']){
    gh(['release','download',tag,'-p',name,'-D',final,'--clobber']);const bytes=readFileSync(path.join(final,name)),asset=release.assets.find(a=>a.name===name)
    assert.equal(bytes.length,asset.size);assert.equal(asset.digest,'sha256:'+sha(bytes));assets.push({name,bytes:bytes.length,sha256:sha(bytes),url:asset.browser_download_url})
  }
  const latest=JSON.parse(readFileSync(path.join(final,'latest.json'),'utf8'));assert.equal(latest.version,version);assert.equal(latest.pub_date,release.published_at)
  assert.equal(latest.platforms['windows-x86_64'].signature,readFileSync(path.join(final,zip+'.sig'),'utf8').trim())
  assert.equal(latest.platforms['windows-x86_64'].url,`https://github.com/Longyuyeee/long_Decompress/releases/download/${tag}/${zip}`)
  assert.equal(latest.notes.trim(),body);assert.equal(release.body.trim(),body);assert.match(body,/EGG/)
  const a=JSON.parse(readFileSync(path.join(original,'latest.json'),'utf8')),b={...latest};delete a.notes;delete b.notes;assert.deepEqual(a,b)
  const contained=execFileSync(path.resolve('src-tauri/resources/archive-engine/7z.exe'),['e','-so',path.join(final,zip),'*.exe'],{maxBuffer:64*1024*1024})
  assert.equal(sha(contained),assets.find(a=>a.name===exe).sha256);assert.equal(productVersion(path.join(final,exe)),version)
  const candidate=JSON.parse(readFileSync('test-results/v1.5.11-candidate/verified.json','utf8'))
  const result={verifiedAt:new Date().toISOString(),tag,tagCommit,functionalFreeze:candidate.functionalFreeze,candidateCommit:candidate.candidateCommit,candidateRun:candidate.candidateRun,releaseRun:releaseRun.databaseId,url:release.html_url,publishedAt:release.published_at,installerProductVersion:version,assets,updaterExeMatchesInstaller:true,signatureMatchesManifest:true,chineseNotesMatch:true,onlyManifestNotesChanged:true,packagedMain:inspectSfxInstaller(path.join(final,exe),path.join(final,'main-and-notice'),version,['archive-engine/licenses/sevenz-rust-Apache-2.0.txt','archive-engine/licenses/ppmd-rust-MIT-0.md'])}
  result.packagedEgg=inspectEggInstaller(path.join(final,exe),path.join(final,'egg-helper'))
  writeFileSync(path.join(final,'verified.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result,null,2))
}else throw new Error('candidate RUN COMMIT FREEZE | notes | verify')

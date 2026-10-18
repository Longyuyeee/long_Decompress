// Original ALZip CLI only, extracted without installing. No simulated volumes.
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomBytes, createHash } from 'node:crypto'
import { mkdirSync, writeFileSync, readdirSync, readFileSync, existsSync, copyFileSync } from 'node:fs'
import path from 'node:path'
const tool=path.resolve('test-results/alzip-152-portable/app/ALZipCon.exe')
const existing=path.resolve('tests/fixtures/archives/alz-152/original-alzip/cp936-provenance.json')
if(existsSync(existing)) {
  const p=JSON.parse(readFileSync(existing,'utf8'))
  for(const member of p.members)assert.equal(createHash('sha256').update(readFileSync(path.join(path.dirname(existing),member.name))).digest('hex'),member.sha256)
  console.log('Existing fixed CP936 originals retained and verified; no provenance regenerated.')
  process.exit(0)
}
const source=path.resolve('test-results/alz-152-created/source');mkdirSync(path.join(source,'空目录'),{recursive:true})
writeFileSync(path.join(source,'说明 文件.txt'),'这是来自原始 ALZip 的公开测试资料。\r\n')
writeFileSync(path.join(source,'한글 파일.txt'),'ALZip filename encoding sample\r\n')
writeFileSync(path.join(source,'empty.txt'),'')
if(!existsSync(path.join(source,'payload.bin')))writeFileSync(path.join(source,'payload.bin'),randomBytes(256*1024))
const dest=path.resolve('tests/fixtures/archives/alz-152/original-alzip');mkdirSync(dest,{recursive:true})
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
for(const [name,password,split] of [['native-key-volumes','LongALZ+password!',true],['native-korean-key','한글암호',false]]) {
  const out=path.join(dest,name+'.alz')
  if(!existsSync(out)) {
    const r=spawnSync(tool,['-a','-m0',...(split?['-v64KB']:[]),'-p'+password,path.join(source,'*'),out],{cwd:path.dirname(tool),windowsHide:true,timeout:60000})
    assert.ifError(r.error);assert.equal(r.status,0,'Original ALZip creation failed: '+r.status)
  }
  assert.ok(existsSync(out))
}
const expected=readdirSync(source,{withFileTypes:true}).filter(e=>e.isFile()).map(e=>({path:e.name,bytes:readFileSync(path.join(source,e.name)).length,sha256:sha(path.join(source,e.name))}))
const report={createdAt:new Date().toISOString(),officialDownload:'https://advert.estsoft.com/?event=200803276378987',installerSha256:sha('test-results/ALZip-official-152.exe'),generatorSha256:sha(tool),generatorVersion:'11.0.1.16 (inside ALZip 12.37.0.4 official installer)',data:'Self-authored public fixtures',sourceFiles:expected,emptyDirectory:'空目录',cases:[{archive:'native-key-volumes.alz',password:'LongALZ+password!',split:true},{archive:'native-korean-key.alz',password:'한글암호',split:false}],members:readdirSync(dest).filter(n=>/\.(alz|[a-z]\d{2})$/i.test(n)).map(n=>({name:n,bytes:readFileSync(path.join(dest,n)).length,sha256:sha(path.join(dest,n))}))}
writeFileSync(path.join(dest,'provenance.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report,null,2))

// The original Windows producer cannot encode Korean filenames on this CP936
// system. Preserve the first originals as negative evidence; don't rename '?'s.
// A distinct source set below uses characters the original producer can retain.
const cnSource=path.resolve('test-results/alz-152-created/source-cp936');mkdirSync(path.join(cnSource,'空目录'),{recursive:true})
for(const name of ['说明 文件.txt','payload.bin','empty.txt'])copyFileSync(path.join(source,name),path.join(cnSource,name))
const cnCases=[{archive:'native-cn-key-volumes.alz',password:'中文密码123!',split:true},{archive:'native-cn-key.alz',password:'中文密码123!',split:false}]
for(const c of cnCases) {
  const out=path.join(dest,c.archive)
  if(!existsSync(out)) {
    const r=spawnSync(tool,['-a','-m0',...(c.split?['-v64KB']:[]),'-p'+c.password,path.join(cnSource,'*'),out],{cwd:path.dirname(tool),windowsHide:true,timeout:60000})
    assert.ifError(r.error);assert.equal(r.status,0,'Original CP936 ALZip creation failed')
  }
}
const cnExpected=readdirSync(cnSource,{withFileTypes:true}).filter(e=>e.isFile()).map(e=>({path:e.name,bytes:readFileSync(path.join(cnSource,e.name)).length,sha256:sha(path.join(cnSource,e.name))}))
const cnReport={...report,data:'Separate self-authored CP936-representable source set',sourceFiles:cnExpected,cases:cnCases,encoding:'cp936',negativeOriginals:'Earlier originals retain literal question marks introduced by ALZip on this CP936 system; no filename repair is claimed',members:readdirSync(dest).filter(n=>/^native-cn-.*\.(alz|[a-z]\d{2})$/i.test(n)).map(n=>({name:n,bytes:readFileSync(path.join(dest,n)).length,sha256:sha(path.join(dest,n))}))}
writeFileSync(path.join(dest,'cp936-provenance.json'),JSON.stringify(cnReport,null,2)+'\n')
console.log(JSON.stringify({cp936Cases:cnCases,expected:cnExpected},null,2))

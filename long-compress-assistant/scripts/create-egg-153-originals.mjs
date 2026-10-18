// Use the original ALZip producer; never synthesize a successful archive.
import assert from 'node:assert/strict'
import { randomBytes,createHash } from 'node:crypto'
import { readFileSync,writeFileSync,existsSync,mkdirSync,readdirSync,copyFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
const tool=path.resolve('test-results/alzip-152-portable/app/ALZipCon.exe'),base=path.resolve('tests/fixtures/archives/egg-153/original-alzip')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
assert.equal(sha(tool),'13c664d8b809348104999efb734587e2847aab04e5511af2e5345e13d2528433')
if(existsSync(path.join(base,'manifest.json'))) {const m=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8'));for(const c of m.cases)for(const f of c.members)assert.equal(sha(path.join(base,f.name)),f.sha256);console.log('Original EGG producer inventory retained.');process.exit(0)}
const source=path.resolve('test-results/egg-153-created/source');mkdirSync(path.join(source,'空目录'),{recursive:true});mkdirSync(base,{recursive:true})
writeFileSync(path.join(source,'说明 文件.txt'),'EGG UTF-8 中文名称与原生密码测试。\r\n')
writeFileSync(path.join(source,'empty.txt'),'')
if(!existsSync(path.join(source,'payload.bin')))writeFileSync(path.join(source,'payload.bin'),randomBytes(256*1024))
const expected=readdirSync(source,{withFileTypes:true}).filter(e=>e.isFile()).map(e=>({path:e.name,bytes:readFileSync(path.join(source,e.name)).length,sha256:sha(path.join(source,e.name))})),cases=[]
for(const [name,password,split]of[['native-plain',null,false],['native-key','PublicEGG+123!',false],['native-key-volumes','PublicEGG+123!',true],['native-cn-key','中文密码123!',false]]) {
  const archive=name+'.egg',out=path.join(base,archive)
  if(!existsSync(out)&&!existsSync(path.join(base,name+'.vol1.egg'))){const r=spawnSync(tool,['-a','-m0',...(split?['-v64KB']:[]),...(password?['-p'+password]:[]),path.join(source,'*'),out],{cwd:path.dirname(tool),windowsHide:true,timeout:90000});assert.ifError(r.error);assert.equal(r.status,0,String(r.stdout)+String(r.stderr))}
  const actual=split?name+'.vol1.egg':archive
  assert.ok(existsSync(path.join(base,actual)));const members=readdirSync(base).filter(n=>n===archive||n.startsWith(name+'.vol')&&n.endsWith('.egg')).map(name=>({name,bytes:readFileSync(path.join(base,name)).length,sha256:sha(path.join(base,name))}))
  cases.push({archive:actual,name,password,split,members,expected,emptyDirectory:'空目录'})
}
writeFileSync(path.join(base,'manifest.json'),JSON.stringify({createdAt:new Date().toISOString(),generator:'Original ALZipCon 11.0.1.16 inside official ALZip 12.37.0.4',generatorSha256:sha(tool),dataLicense:'CC0-1.0 self-authored public payloads',cases},null,2)+'\n')
console.log(cases.map(c=>c.archive))

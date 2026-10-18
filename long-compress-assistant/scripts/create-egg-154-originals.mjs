// Original ALZipCon -tp 1/2 and -s. Positive archives are never patched.
import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {readFileSync,writeFileSync,mkdirSync,readdirSync,existsSync,copyFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import path from 'node:path'
const tool=path.resolve('test-results/alzip-152-portable/app/ALZipCon.exe'),base=path.resolve('tests/fixtures/archives/egg-154/original-alzip'),source=path.resolve('test-results/egg-154-source')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
assert.equal(sha(tool),'13c664d8b809348104999efb734587e2847aab04e5511af2e5345e13d2528433')
if(existsSync(path.join(base,'manifest.json'))){const m=JSON.parse(readFileSync(path.join(base,'manifest.json'),'utf8'));for(const c of m.cases)for(const f of c.members)assert.equal(sha(path.join(base,f.name)),f.sha256);console.log('Fixed original complex EGGs retained.');process.exit(0)}
mkdirSync(base,{recursive:true});mkdirSync(path.join(source,'空目录'),{recursive:true});mkdirSync(path.join(source,'文档'),{recursive:true})
copyFileSync('test-results/egg-153-created/source/payload.bin',path.join(source,'payload.bin'))
writeFileSync(path.join(source,'文档/说明 文件.txt'),'用户收到的中文加密分卷；选择一个文件，或者补回原卷继续。\r\n'.repeat(250))
writeFileSync(path.join(source,'empty.txt'),'')
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
const expected=tree(source),cases=[]
for(const [name,opts,password]of[
 ['aes128-cn',['-tp1','-m1'],'中文密码123!'],['aes256-cn',['-tp2','-m4'],'中文密码123!'],
 ['aes128-volumes',['-tp1','-m1','-v64KB'],'中文密码123!'],['aes256-volumes',['-tp2','-m4','-v64KB'],'中文密码123!'],
 ['solid-deflate',['-s','-m1'],null],['solid-high',['-s','-m3'],null],
]){
 const out=path.join(base,name+'.egg')
 if(!existsSync(out)&&!existsSync(path.join(base,name+'.vol1.egg'))){const r=spawnSync(tool,['-a',...opts,...(password?['-p'+password]:[]),path.join(source,'*'),out],{cwd:path.dirname(tool),windowsHide:true,timeout:120000});assert.ifError(r.error);assert.equal(r.status,0,name+': '+String(r.stdout)+String(r.stderr))}
 const members=readdirSync(base).filter(n=>n===name+'.egg'||n.startsWith(name+'.vol')&&n.endsWith('.egg')).map(name=>({name,bytes:readFileSync(path.join(base,name)).length,sha256:sha(path.join(base,name))}))
 assert.ok(members.length);cases.push({name,archive:members.some(m=>m.name===name+'.vol1.egg')?name+'.vol1.egg':name+'.egg',producerArgs:opts,password,encoding:'cp936',members,expected,emptyDirectory:'空目录'})
}
writeFileSync(path.join(base,'manifest.json'),JSON.stringify({createdAt:new Date().toISOString(),generator:'Original ALZipCon 11.0.1.16 in official ALZip 12.37.0.4',generatorSha256:sha(tool),payloadLicense:'CC0-1.0 self authored',cases},null,2)+'\n')
console.log('Original complex groups generated: '+cases.length)

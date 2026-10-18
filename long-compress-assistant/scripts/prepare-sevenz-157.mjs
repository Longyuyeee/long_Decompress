// Original 7-Zip output; the application under test never produces these inputs.
import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,existsSync} from 'node:fs'
import path from 'node:path'
const base=path.resolve('tests/fixtures/archives/sevenz-157'),manifest=path.join(base,'manifest.json')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
if(existsSync(manifest)){const m=JSON.parse(readFileSync(manifest,'utf8'));for(const c of m.cases)for(const f of c.members)assert.equal(sha(path.join(base,c.name,f.name)),f.sha256);console.log('Existing 7Z method originals retained');process.exit(0)}
const producer='E:/7z/7-Zip/7z.exe',independent=path.resolve('src-tauri/resources/archive-engine/7z.exe'),source=path.join(base,'source')
mkdirSync(path.join(source,'文档'),{recursive:true});mkdirSync(path.join(source,'空目录'),{recursive:true})
const payload=Buffer.alloc(262144);for(let i=0;i<payload.length;i+=32)createHash('sha256').update('sevenz157-original-'+i).digest().copy(payload,i)
writeFileSync(path.join(source,'payload.bin'),payload);writeFileSync(path.join(source,'文档/说明 文件.txt'),'收到的资料包含中文路径、空文件和固实流。\r\n'.repeat(500));writeFileSync(path.join(source,'empty.txt'),'')
const expected=['payload.bin','文档/说明 文件.txt','empty.txt'].map(name=>({path:name,bytes:readFileSync(path.join(source,name)).length,sha256:sha(path.join(source,name))}))
const cases=[],password='Seven157+中文123!'
for(const method of ['PPMd','Deflate','Deflate64','BZip2','LZMA2','BCJ2'])for(const mode of (['LZMA2','BCJ2'].includes(method)?['plain-solid']:['plain','plain-solid','contents-solid','headers-volumes'])){
 const name=method.toLowerCase()+'-'+mode,dir=path.join(base,name);mkdirSync(dir,{recursive:true})
 const archive=path.join(dir,'收到的资料.7z'),encrypted=/contents|headers/.test(mode),volume=mode==='headers-volumes'
 const methods=method==='BCJ2'?['-m0=BCJ2','-m1=LZMA2','-m2=LZMA2','-m3=LZMA2','-mb0:1','-mb0s1:2','-mb0s2:3']:['-m0='+method]
 const args=['a','-t7z','-y','-sccUTF-8','-mtc=off','-mta=off',...methods,'-ms='+(/solid|volumes/.test(mode)?'on':'off'),...(encrypted?['-p'+password,'-mhe='+(/headers/.test(mode)?'on':'off')]:[]),...(volume?['-v128k']:[]),archive,'.']
 const r=spawnSync(producer,args,{cwd:source,encoding:'utf8',windowsHide:true,timeout:120000});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr)
 const members=readdirSync(dir).sort().map(name=>({name,bytes:readFileSync(path.join(dir,name)).length,sha256:sha(path.join(dir,name))})),first=members[0].name
 if(volume)assert.ok(members.length>1)
 const out=path.resolve('test-results/sevenz-157-independent',name);mkdirSync(out,{recursive:true})
 const check=spawnSync(independent,['x','-y','-sccUTF-8','-o'+out,...(encrypted?['-p'+password]:[]),path.join(dir,first)],{encoding:'utf8',windowsHide:true,timeout:120000});assert.equal(check.status,0,check.stdout+check.stderr)
 for(const e of expected)assert.equal(sha(path.join(out,e.path)),e.sha256)
 const listing=spawnSync(producer,['l','-slt','-sccUTF-8',...(encrypted?['-p'+password]:[]),path.join(dir,first)],{encoding:'utf8',windowsHide:true});assert.equal(listing.status,0)
 cases.push({name,method,mode,password:encrypted?password:null,archive:first,members,expected,producerArgs:args,independentReadback:true,methodListing:listing.stdout.split(/\r?\n/).filter(l=>/^(Method|Solid|Blocks) =/.test(l))})
}
writeFileSync(manifest,JSON.stringify({createdAt:new Date().toISOString(),producer,producerSha256:sha(producer),independent,independentSha256:sha(independent),cases},null,2)+'\n');console.log(cases.length+' original method fixtures independently decoded')

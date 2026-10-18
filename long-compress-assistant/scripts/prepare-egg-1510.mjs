// Original ALZip output, not synthetic positive EGG headers. Large bytes stay ignored.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {spawnSync} from 'node:child_process'
import {openSync,writeSync,closeSync,createReadStream,readFileSync,writeFileSync,mkdirSync,existsSync,readdirSync,statSync,utimesSync} from 'node:fs'
import path from 'node:path'
const root=path.resolve('test-results/egg-1510'),tool=path.resolve('test-results/alzip-152-portable/app/ALZipCon.exe'),bz='E:/Bandzip/Bandizip/bz.exe'
const sha=async p=>{const h=createHash('sha256');for await(const b of createReadStream(p))h.update(b);return h.digest('hex')}
assert.equal(await sha(tool),'13c664d8b809348104999efb734587e2847aab04e5511af2e5345e13d2528433');mkdirSync(root,{recursive:true})
if(existsSync(path.join(root,'manifest.json'))){const m=JSON.parse(readFileSync(path.join(root,'manifest.json'),'utf8'));for(const c of m.cases)for(const f of c.members)assert.equal(await sha(path.join(root,f.name)),f.sha256);console.log('Original large EGG groups retained.');process.exit(0)}
const payload=(name,start,count)=>{const p=path.join(root,'source',name);mkdirSync(path.dirname(p),{recursive:true});if(!existsSync(p)){const fd=openSync(p,'wx');try{for(let i=start;i<start+count;i++){const b=createHash('shake256',{outputLength:1024*1024}).update('Long public EGG 1.5.10 payload / '+i).digest();b.fill(0,0,65536);writeSync(fd,b)}}finally{closeSync(fd)}utimesSync(p,new Date('2020-01-02T03:04:05Z'),new Date('2020-01-02T03:04:05Z'))}assert.equal(statSync(p).size,count*1024*1024);return p}
const large=payload('普通/大文件.bin',0,320),first=payload('固实/01前段.bin',0,192),last=payload('固实/99最后.bin',192,128)
writeFileSync(path.join(root,'source/固实/说明 文件.txt'),'只取最后一个文件，也要核对前面的共享流。\r\n');writeFileSync(path.join(root,'source/固实/empty.txt'),'');mkdirSync(path.join(root,'source/固实/空目录'),{recursive:true})
const tree=async dir=>{const out=[];for(const e of readdirSync(dir,{withFileTypes:true})){if(e.isDirectory()){for(const x of await tree(path.join(dir,e.name)))out.push({...x,path:e.name+'/'+x.path})}else out.push({path:e.name,bytes:statSync(path.join(dir,e.name)).size,sha256:await sha(path.join(dir,e.name))})}return out.sort((a,b)=>a.path.localeCompare(b.path))}
const cases=[]
for(const[name,opts,source,password]of[['single',['-m3'],large,null],['solid',['-m3','-s'],path.join(root,'source/固实/*'),null],['password-volumes',['-m3','-v96MB','-pPublicEGG1510+123!'],large,'PublicEGG1510+123!']]){
 console.log('Original ALZip creating',name);const out=path.join(root,name+'.egg');if(!existsSync(out)&&!existsSync(path.join(root,name+'.vol1.egg'))){const r=spawnSync(tool,['-a',...opts,source,out],{cwd:path.dirname(tool),windowsHide:true,timeout:900000});assert.ifError(r.error);assert.equal(r.status,0,String(r.stdout)+String(r.stderr))}
 const members=[];for(const n of readdirSync(root).filter(n=>n===name+'.egg'||n.startsWith(name+'.vol')&&n.endsWith('.egg')).sort())members.push({name:n,bytes:statSync(path.join(root,n)).size,sha256:await sha(path.join(root,n))});assert.ok(members.length)
 const archive=members.some(m=>m.name===name+'.vol1.egg')?name+'.vol1.egg':name+'.egg',expected=await tree(path.join(root,'source',name==='solid'?'固实':'普通')),independent=path.join(root,'independent',name)
 console.log('Independent Bandizip reading',name);const r=spawnSync(bz,['x','-y',...(password?['-p:'+password]:[]),'-o:'+independent,path.join(root,archive)],{windowsHide:true,timeout:900000});assert.ifError(r.error);assert.equal(r.status,0,String(r.stdout)+String(r.stderr));assert.deepEqual(await tree(independent),expected)
 cases.push({name,archive,producerArgs:opts,password,expected,emptyDirectory:name==='solid'?'空目录':null,members})
}
const manifest={createdAt:new Date().toISOString(),producer:'ALZipCon 11.0.1.16 in official ALZip 12.37.0.4',producerSha256:await sha(tool),independentTool:bz,independentToolSha256:await sha(bz),payloadLicense:'CC0-1.0 self-authored; SHAKE256(seed + global MiB index), first 64KiB of each MiB zeroed; fixed source mtime',seed:'Long public EGG 1.5.10 payload / ',largeOriginalsLocation:'test-results/egg-1510 (ignored); reproduce with this script and original producer, verify payloads; regenerated encryption metadata may have a new archive identity',cases}
writeFileSync(path.join(root,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');console.log('Three independently decoded genuine large EGG groups ready.')

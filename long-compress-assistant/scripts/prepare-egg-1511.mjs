// Probe genuine producer combinations before changing the decoder. Never patch positive headers.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {spawnSync} from 'node:child_process'
import {readFileSync,writeFileSync,mkdirSync,readdirSync,existsSync,utimesSync} from 'node:fs'
import path from 'node:path'
const base=path.resolve('test-results/egg-1511'),source=path.join(base,'source'),producer=path.resolve('test-results/alzip-152-portable/app/ALZipCon.exe'),independent='E:/Bandzip/Bandizip/bz.exe',helper=path.resolve('src-tauri/resources/egg-engine/long-egg.exe')
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
assert.equal(sha(producer),'13c664d8b809348104999efb734587e2847aab04e5511af2e5345e13d2528433')
mkdirSync(path.join(source,'文档'),{recursive:true});mkdirSync(path.join(source,'空目录'),{recursive:true})
for(const [name,bytes]of [['01前段.bin',createHash('shake256',{outputLength:262144}).update('Public EGG 1.5.11 fixed payload').digest()],['文档/99最后.txt',Buffer.from('这份中文密码归档由原 ALZip 生成；只取最后文件仍要核对共享流。\r\n'.repeat(250))],['empty.txt',Buffer.alloc(0)]]){
 const f=path.join(source,name);if(!existsSync(f))writeFileSync(f,bytes);assert.deepEqual(readFileSync(f),bytes);utimesSync(f,new Date('2020-01-02T03:04:05Z'),new Date('2020-01-02T03:04:05Z'))
}
const expected=tree(source),password='中文密码1511!',cases=[],attempts=[]
for(const solid of [false,true])for(const [cipher,tp]of [['zipcrypto',0],['aes128',1],['aes256',2],['lea128',3],['lea256',4]])for(const method of [1,3])for(const volumes of [false,true]){
 const name=`${solid?'solid':'ordinary'}-${cipher}-m${method}${volumes?'-volumes':''}`,dir=path.join(base,'originals',name),archive=path.join(dir,name+'.egg'),args=['-a',...(solid?['-s']:[]),'-m'+method,'-tp'+tp,...(volumes?['-v64KB']:[]),'-p'+password,path.join(source,'*'),archive];mkdirSync(dir,{recursive:true})
 let code=null
 if(!readdirSync(dir).some(n=>n.endsWith('.egg'))){const r=spawnSync(producer,args,{cwd:path.dirname(producer),windowsHide:true,timeout:60000});assert.ifError(r.error);code=r.status}
 const files=readdirSync(dir).filter(n=>n.endsWith('.egg'));if(!files.length){attempts.push({name,args:args.slice(0,-2),producerExitCode:code,produced:false});continue}
 const entry=files.find(n=>n.includes('.vol1.egg'))||files[0],out=path.join(base,'independent',name);const r=spawnSync(independent,['x','-y','-p:'+password,'-o:'+out,path.join(dir,entry)],{windowsHide:true,timeout:60000});assert.ifError(r.error)
 const result={name,producerExitCode:code,args:args.slice(0,-2),independentExitCode:r.status,produced:true};attempts.push(result)
 let verification='Bandizip independent extraction'
 if(r.status!==0){result.independentError=String(r.stdout)+String(r.stderr);const roundtrip=path.join(base,'producer-roundtrip-fixed',name),original=spawnSync(producer,['-x','-p'+password,path.join(dir,entry),roundtrip],{cwd:path.dirname(producer),windowsHide:true,timeout:60000});assert.ifError(original.error);assert.equal(original.status,0);assert.deepEqual(tree(path.join(roundtrip,entry.replace(/\.egg$/, ''))),expected);result.originalProducerRoundtrip=true;verification='Original ALZip extraction and known payload hashes; Bandizip 7.38 Beta CRC failure retained'}else assert.deepEqual(tree(out),expected)
 const p=spawnSync(helper,['list',path.join(dir,entry)],{input:'{}',encoding:'utf8',windowsHide:true,timeout:10000});assert.equal(p.status,0,p.stdout+p.stderr);const metadata=JSON.parse(p.stdout).result
 // Confirm actual header flags/methods, not a requested flag or archive filename.
 const c={name,archive:path.relative(base,path.join(dir,entry)).replaceAll('\\','/'),requested:{solid,cipher,method,volumes},actual:{solid:metadata.solid,encryption:[...new Set(metadata.entries.map(e=>e.encryption).filter(Boolean))],methods:[...new Set(metadata.entries.flatMap(e=>e.methods))]},verification,password,encoding:'cp936',members:files.sort().map(n=>({name:path.relative(base,path.join(dir,n)).replaceAll('\\','/'),bytes:readFileSync(path.join(dir,n)).length,sha256:sha(path.join(dir,n))})),expected,emptyDirectory:'空目录'};cases.push(c);console.log(name,JSON.stringify(c.actual))
}
writeFileSync(path.join(base,'manifest.json'),JSON.stringify({createdAt:new Date().toISOString(),producer:'Original ALZipCon 11.0.1.16 in official ALZip 12.37.0.4',producerSha256:sha(producer),independentTool:independent,independentSha256:sha(independent),payloadLicense:'CC0-1.0 self-authored fixed SHAKE256 and text; producer archives unmodified',expected,cases,attempts},null,2)+'\n');console.log('Independent original groups:',cases.length,'producer attempts:',attempts.length)

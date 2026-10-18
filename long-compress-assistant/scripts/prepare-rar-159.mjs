// Generate original RAR4/5 volumes, never launch an input SFX.
import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,existsSync} from 'node:fs'
import path from 'node:path'
const base=path.resolve('tests/fixtures/archives/rar-159'),manifest=path.join(base,'manifest.json'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
if(existsSync(manifest)){const m=JSON.parse(readFileSync(manifest,'utf8'));for(const c of m.cases)for(const f of c.members)assert.equal(sha(path.join(base,c.name,f.name)),f.sha256);console.log('Fixed RAR originals retained');process.exit(0)}
const password='Rar159+中文123!',independent=path.resolve('src-tauri/resources/archive-engine/7z.exe'),cases=[]
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
for(const family of ['rar4-old-native','rar4-modern-native','rar4-old-sfx','rar4-modern-sfx','rar5-native','rar5-sfx'])for(const variant of ['original','other']){
 const name=family+'-'+variant,dir=path.join(base,name),source=path.join(base,'source',variant);mkdirSync(dir,{recursive:true});mkdirSync(path.join(source,'文档/空目录'),{recursive:true})
 if(!existsSync(path.join(source,'payload.bin'))){const b=Buffer.alloc(196608);for(let i=0;i<b.length;i+=32)createHash('sha256').update('rar159-'+variant+'-'+i).digest().copy(b,i);writeFileSync(path.join(source,'payload.bin'),b);writeFileSync(path.join(source,'01已验证.txt'),'这一条完整的加密内容先通过 CRC，再读取跨卷内容。\r\n'.repeat(8));writeFileSync(path.join(source,'文档/说明 文件.txt'),'完整内容必须匹配原始卷组。\r\n'.repeat(100));writeFileSync(path.join(source,'empty.txt'),'')}
 const rar4=family.startsWith('rar4'),sfx=family.endsWith('sfx'),producer=rar4?path.resolve('test-results/rar4-1419-tools/rar624/Rar.exe'):'E:/Winrar/Rar.exe',args=['a','-idq','-cfg-','-m0','-r','-ds',...(rar4?['-ma4']:[]),...(family.includes('-old-')?['-vn']:[]),...(sfx?['-sfx']:[]),'-v64k','-hp'+password,path.join(dir,'收到的资料.'+(sfx?'exe':'rar')),'01已验证.txt','payload.bin','文档','empty.txt']
 const r=spawnSync(producer,args,{cwd:source,encoding:'utf8',windowsHide:true,timeout:120000});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr)
 const members=readdirSync(dir).sort().map(name=>({name,bytes:readFileSync(path.join(dir,name)).length,sha256:sha(path.join(dir,name))})),archive=members.find(m=>sfx?m.name.endsWith('.exe'):family.includes('-old-')?m.name.endsWith('.rar'):/part1\.rar$/.test(m.name)).name
 const listing=spawnSync(independent,['l',rar4?'-tRAR':'-tRAR5','-slt','-sccUTF-8','-p'+password,path.join(dir,archive)],{encoding:'utf8',windowsHide:true});assert.equal(listing.status,0,listing.stdout+listing.stderr)
 const out=path.resolve('test-results/rar-159-independent',name),check=spawnSync(independent,['x',rar4?'-tRAR':'-tRAR5','-y','-sccUTF-8','-p'+password,'-o'+out,path.join(dir,archive)],{encoding:'utf8',windowsHide:true,timeout:120000});assert.equal(check.status,0,check.stdout+check.stderr);assert.deepEqual(tree(out),tree(source))
 cases.push({name,family,variant,password,archive,members,expected:tree(source),source:'source/'+variant,emptyDirectory:'文档/空目录',producer,producerSha256:sha(producer),producerArgs:args,independentReadback:true,entryOrder:listing.stdout.split(/\r?\n/).filter(l=>/^Path =/.test(l))})
}
writeFileSync(manifest,JSON.stringify({createdAt:new Date().toISOString(),dataLicense:'CC0-1.0 self-authored; original SFX modules retain sfx-155 notices',independentSha256:sha(independent),cases},null,2)+'\n');console.log('12 original RAR4/5 encrypted-header groups independently decoded')

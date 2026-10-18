// Original producer archives only. Never execute an input SFX.
import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,existsSync} from 'node:fs'
import {createHash,randomBytes} from 'node:crypto'
import path from 'node:path'
const base=path.resolve('tests/fixtures/archives/sfx-155'),source=path.join(base,'source'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
mkdirSync(path.join(source,'文档','空目录'),{recursive:true});if(!existsSync(path.join(source,'payload.bin')))writeFileSync(path.join(source,'payload.bin'),randomBytes(262144));writeFileSync(path.join(source,'文档','说明 文件.txt'),'自解压首卷和外置卷作为数据读取，保留源文件。\n'.repeat(200));writeFileSync(path.join(source,'empty.txt'),'')
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
const old=path.resolve('test-results/rar4-1419-tools/rar624/Rar.exe'),current='E:/Winrar/Rar.exe',seven='E:/7z/7-Zip/7z.exe',module='E:/7z/7-Zip/7z.sfx',independent=path.resolve('src-tauri/resources/archive-engine/7z.exe'),password='Sfx155+123!',cases=[],rejected=[]
const run=(tool,args,cwd=source)=>{const r=spawnSync(tool,args,{cwd,encoding:'utf8',windowsHide:true,timeout:120000});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr);return r.stdout}
for(const kind of ['rar4-single','rar4-old','rar4-modern','rar5-modern','7z-volumes'])for(const mode of ['plain','contents','headers']){
 const dir=path.join(base,kind+'-'+mode);mkdirSync(dir,{recursive:true});const key=mode==='plain'?null:password,tool=kind.startsWith('rar4')?old:kind.startsWith('rar5')?current:seven
 const args=kind.startsWith('rar')?['a','-sfx',...(kind.startsWith('rar4')?['-ma4']:[]),'-m0','-r','-idq','-cfg-',...(kind.endsWith('single')?[]:['-v64k']),...(kind.endsWith('old')?['-vn']:[]),...(key?[`-${mode==='headers'?'hp':'p'}${key}`]:[]),path.join(dir,'用户资料.exe'),'.\\*']:['a','-t7z','-mx=0',`-sfx${module}`,'-v64k',...(key?[`-p${key}`,`-mhe=${mode==='headers'?'on':'off'}`]:[]),path.join(dir,'用户资料.exe'),'.\\*']
 if(!readdirSync(dir).length){const r=spawnSync(tool,args,{cwd:source,encoding:'utf8',windowsHide:true,timeout:120000});assert.ifError(r.error);if(r.status!==0){rejected.push({kind,mode,status:r.status,stdout:r.stdout,stderr:r.stderr});continue}}
 const members=readdirSync(dir).filter(n=>!n.endsWith('.json')).sort().map(name=>({name,bytes:readFileSync(path.join(dir,name)).length,sha256:sha(path.join(dir,name))})),first=members.find(m=>m.name.endsWith('.exe'))?.name||members[0].name
 const out=path.resolve('test-results/sfx-155-independent-payload',kind+'-'+mode);run(independent,['x',...(kind.startsWith('rar')?[kind.startsWith('rar4')?'-tRAR':'-tRAR5']:[]),'-y',...(key?[`-p${key}`]:[]),'-o'+out,path.join(dir,kind.startsWith('rar')?first:'用户资料.7z.001')],base);assert.deepEqual(tree(out),tree(source))
 cases.push({name:kind+'-'+mode,kind,mode,password:key,archive:kind+'-'+mode+'/'+first,members,producer:tool,producerSha256:sha(tool),args,expected:tree(source),emptyDirectory:'文档/空目录'})
}
const result={createdAt:new Date().toISOString(),dataLicense:'CC0-1.0 self authored',independentTool:independent,independentSha256:sha(independent),cases,rejected};const file=path.join(base,'manifest.json');if(existsSync(file)){const prior=JSON.parse(readFileSync(file,'utf8'));assert.deepEqual(cases,prior.cases,'Do not replace fixed originals or expected payloads')}else writeFileSync(file,JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({cases:cases.map(c=>({name:c.name,members:c.members.map(m=>m.name)})),rejected},null,2))

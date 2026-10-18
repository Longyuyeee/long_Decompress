// Trusted original producers only; generated input EXEs are never launched.
import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,existsSync,copyFileSync} from 'node:fs'
import path from 'node:path'
const base=path.resolve('tests/fixtures/archives/sfx-158'),manifest=path.join(base,'manifest.json'),sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
if(existsSync(manifest)){const m=JSON.parse(readFileSync(manifest,'utf8'));for(const c of m.cases)for(const f of c.members)assert.equal(sha(path.join(base,c.name,f.name)),f.sha256);console.log('Fixed original SFX retained');process.exit(0)}
const source=path.join(base,'source'),previous=path.resolve('tests/fixtures/archives/sfx-155/source');mkdirSync(source,{recursive:true})
const copyTree=(a,b)=>{mkdirSync(b,{recursive:true});for(const e of readdirSync(a,{withFileTypes:true}))e.isDirectory()?copyTree(path.join(a,e.name),path.join(b,e.name)):copyFileSync(path.join(a,e.name),path.join(b,e.name))};copyTree(previous,source)
const tree=p=>readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?tree(path.join(p,e.name)).map(x=>({...x,path:e.name+'/'+x.path})):[{path:e.name,bytes:readFileSync(path.join(p,e.name)).length,sha256:sha(path.join(p,e.name))}]).sort((a,b)=>a.path.localeCompare(b.path))
const run=(tool,args,cwd=source)=>{const r=spawnSync(tool,args,{cwd,encoding:'utf8',windowsHide:true,timeout:120000});assert.ifError(r.error);assert.equal(r.status,0,r.stdout+r.stderr);return r.stdout}
const independent=path.resolve('src-tauri/resources/archive-engine/7z.exe'),password='Sfx158+中文123!',cases=[],modules=[]
for(const version of ['19.00','23.01','26.04']){
 const tools=path.resolve('test-results/sfx-158-tools',version),tool=path.join(tools,'7z.exe'),module=path.join(tools,'7z.sfx'),moduleDir=path.join(base,'modules',version);mkdirSync(moduleDir,{recursive:true});copyFileSync(module,path.join(moduleDir,'7z.sfx'));copyFileSync(path.join(tools,'License.txt'),path.join(moduleDir,'License.txt'))
 const installer=path.resolve('test-results/sfx-158-tools',`7z${version.replace('.','')}-x64.exe`),url=version==='19.00'?'https://www.7-zip.org/a/7z1900-x64.exe':`https://github.com/ip7z/7zip/releases/download/${version}/7z${version.replace('.','')}-x64.exe`
 modules.push({version,kind:'7z-gui',module:`modules/${version}/7z.sfx`,bytes:readFileSync(module).length,sha256:sha(module),license:`modules/${version}/License.txt`,installerUrl:url,installerSha256:sha(installer),producerSha256:sha(tool)})
 for(const layout of ['external','embedded'])for(const mode of ['plain','contents','headers']){
  const name=`7z-${version}-${layout}-${mode}`,dir=path.join(base,name);mkdirSync(dir,{recursive:true});const encrypted=mode!=='plain',archive=path.join(dir,'收到的资料.exe'),args=['a','-t7z','-mx=0','-sccUTF-8','-sfx'+module,...(layout==='external'?['-v64k']:[]),...(encrypted?['-p'+password,'-mhe='+(mode==='headers'?'on':'off')]:[]),archive,'.\\*'];run(tool,args)
  const data=layout==='external'?path.join(dir,'收到的资料.7z.001'):archive,out=path.resolve('test-results/sfx-158-independent',name);run(independent,['x','-y','-sccUTF-8','-o'+out,...(encrypted?['-p'+password]:[]),data],base);assert.deepEqual(tree(out),tree(source))
  cases.push({name,kind:'7z',version,layout,mode,password:encrypted?password:null,archive:path.basename(archive),members:readdirSync(dir).sort().map(name=>({name,bytes:readFileSync(path.join(dir,name)).length,sha256:sha(path.join(dir,name))})),expected:tree(source),emptyDirectory:'文档/空目录',producerArgs:args,independentReadback:true})
 }
}
const rarDir=path.resolve('test-results/rar4-1419-tools/rar624'),rar=path.join(rarDir,'Rar.exe')
for(const moduleName of ['Default.SFX','Default64.SFX','WinCon.SFX','WinCon64.SFX']){
 const module=path.join(rarDir,moduleName),dest=path.join(base,'modules/rar6.24');mkdirSync(dest,{recursive:true});copyFileSync(path.join(rarDir,'License.txt'),path.join(dest,'License.txt'))
 // RAR license forbids redistributing separate module files. Keep their
 // identities and original SFX archives, not standalone RAR modules.
 modules.push({version:'6.24',kind:'rar',module:moduleName,standaloneRedistributed:false,bytes:readFileSync(module).length,sha256:sha(module),license:'modules/rar6.24/License.txt',installerUrl:'https://www.rarlab.com/rar/winrar-x64-624.exe',installerSha256:'794481dbbc9009a2565726fb5b4a4ab2fe216ff9edbb08951548ee765de9b4a6',producerSha256:sha(rar)})
 for(const mode of ['plain','contents','headers']){
  const old=moduleName==='Default.SFX'||moduleName==='WinCon.SFX',name=`rar624-${moduleName.replace('.SFX','')}-${mode}`,dir=path.join(base,name);mkdirSync(dir,{recursive:true});const encrypted=mode!=='plain',args=['a','-sfx'+module,'-ma'+(old?'4':'5'),'-m0','-r','-idq','-cfg-','-v64k',...(old?['-vn']:[]),...(encrypted?['-'+(mode==='headers'?'hp':'p')+password]:[]),path.join(dir,'收到的资料.exe'),'.\\*'];run(rar,args)
  const members=readdirSync(dir).sort().map(name=>({name,bytes:readFileSync(path.join(dir,name)).length,sha256:sha(path.join(dir,name))})),archive=members.find(m=>m.name.endsWith('.exe')).name,out=path.resolve('test-results/sfx-158-independent',name);run(independent,['x','-y','-sccUTF-8','-o'+out,...(encrypted?['-p'+password]:[]),path.join(dir,archive)],base);assert.deepEqual(tree(out),tree(source))
  cases.push({name,kind:'rar',version:'6.24',module:moduleName,layout:'external',mode,password:encrypted?password:null,archive,members,expected:tree(source),emptyDirectory:'文档/空目录',producerArgs:args,independentReadback:true})
 }
}
writeFileSync(manifest,JSON.stringify({createdAt:new Date().toISOString(),dataLicense:'CC0-1.0 self-authored payload; modules retain their original licenses',independentSha256:sha(independent),modules,cases},null,2)+'\n');console.log(`${cases.length} original SFX groups independently decoded`)

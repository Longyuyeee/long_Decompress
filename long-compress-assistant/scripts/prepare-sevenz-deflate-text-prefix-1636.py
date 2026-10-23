"""Original Deflate text prefixes, two-tool readback and labelled tail mutations."""
import hashlib,json,subprocess
from pathlib import Path
base=Path('tests/fixtures/archives/sevenz-deflate-text-prefix-1636');work=Path('test-results/sevenz-deflate-text-prefix-1636');base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve();sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();cap=1024*1024;key='Deflate日志公开密码1636!';manifest=base/'manifest.json'
def run(exe,args,cwd=None,success=True):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=180);assert (r.returncode==0)==success,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace');return r.stdout.decode('utf-8','replace')
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'))
 for c in m['cases']:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 for n,v in m['sideOriginals'].items():assert sha(base/(n+'.7z'))==v['sha256']
 for n,v in m['mutations'].items():assert sha(base/n)==v
 print('Immutable Deflate prefix originals retained');raise SystemExit
old=Path('tests/fixtures/archives/sevenz-ppmd-text-prefix-1631');om=json.loads((old/'manifest.json').read_text(encoding='utf-8'));c=next(c for c in om['cases'] if c['name']=='contents');selected=om['selected'];expected=c['expected']
source=(work/'source').resolve();source.mkdir(exist_ok=True);run(independent,['x','-y','-p'+om['password'],'-o'+str(source),(old/c['name']/c['members'][0]['name']).resolve()])
for e in expected:assert sha(source/e['path'])==e['sha256']
common=['a','-t7z','-y','-sccUTF-8','-mtc=off','-mta=off','-mtm=off','-mhc=off','-ms=off','-m0=Deflate'];cases=[]
for name,headers,volume in [('plain',False,False),('contents',False,False),('headers',True,False),('volumes',True,True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True);args=[*common,*(['-p'+key] if name!='plain' else []),'-mx='+('9' if name=='headers' else '5'),'-mhe='+('on' if headers else 'off'),*(['-v256k'] if volume else []),dest/'Deflate日志.7z','.']
 if not list(dest.iterdir()):run(producer,args,source)
 members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())];out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-p'+key,'-sccUTF-8',dest/members[0]['name']]);assert 'Solid = -' in listing and 'DEFLATE' in listing.upper()
 cases.append({'name':name,'method':'Deflate','encrypted':name!='plain','filenameEncrypted':headers,'members':members,'originalArgs':list(map(str,args)),'expected':expected,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
side={}
for name,flags,entry in [('solid',['-ms=128m','-p'+key],'.'),('store',['-m0=Copy','-p'+key],selected),('late-original',['-p'+key],selected),('late-plain',[],selected)]:
 target=(base/(name+'.7z')).resolve();args=[*[a for a in common if name!='solid' or a!='-ms=off'],*flags,target,entry]
 if not target.exists():run(producer,args,source)
 out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),target])
 for e in expected if entry=='.' else [e for e in expected if e['path']==entry]:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-p'+key,target])
 if name=='solid':assert 'Solid = +' in listing and 'Blocks = 1' in listing
 side[name]={'sha256':sha(target),'originalArgs':list(map(str,args)),'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Solid =','Blocks ='))]}
for original,name,early in [('late-original','tail-damaged',False),('late-original','bad-start',True),('late-plain','plain-tail-damaged',False)]:
 b=bytearray((base/(original+'.7z')).read_bytes());end=32+int.from_bytes(b[12:20],'little')
 if early:b[32]^=255 # Corrupt the start of the AES payload.
 else:b[end-17 if original=='late-original' else end-32]^=128
 target=(base/(name+'.7z')).resolve();target.write_bytes(b);run(independent,['t','-p'+key,target],success=False)
large=(work/'large-source').resolve();large.mkdir(exist_ok=True)
with (large/'huge.txt').open('wb') as f:
 for _ in range(65):f.write(b'L'*cap)
target=(base/'large-total.7z').resolve();args=[*common,'-p'+key,target,'huge.txt'];run(producer,args,large);out=(work/'readback/large-total').resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),target]);assert sha(out/'huge.txt')==sha(large/'huge.txt');side['large-total']={'sha256':sha(target),'originalArgs':list(map(str,args)),'independentReadback':True,'bytes':65*cap}
m={k:om[k] for k in ['selected','renderedPrefixBytes','renderedPrefixSha256','totalSize','boundaries']};m.update(password=key,producer=str(producer),producerSha256=sha(producer),independent=str(independent),independentSha256=sha(independent),cases=cases,sideOriginals=side,mutations={n:sha(base/n) for n in ['tail-damaged.7z','plain-tail-damaged.7z','bad-start.7z']})
manifest.write_text(json.dumps(m,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('Four original plain/AES Deflate groups, five encoding boundaries and 65 MiB source independently read back')

"""Original shared Deflate prefixes and exact dependency limits, two-tool readback."""
import hashlib,json,subprocess
from pathlib import Path
base=Path('tests/fixtures/archives/sevenz-solid-deflate-prefix-1637');work=Path('test-results/sevenz-solid-deflate-prefix-1637');base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve();sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();cap=1024*1024;key='固实Deflate公开密码1637!';manifest=base/'manifest.json'
def run(exe,args,cwd=None,success=True):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=180);assert (r.returncode==0)==success,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace');return r.stdout.decode('utf-8','replace')
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'))
 for c in m['cases']:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 for n,v in m['sideOriginals'].items():assert sha(base/(n+'.7z'))==v['sha256']
 for n,v in m['mutations'].items():assert sha(base/n)==v
 print('Immutable shared Deflate prefix originals retained');raise SystemExit
old=Path('tests/fixtures/archives/sevenz-solid-ppmd-text-prefix-1632');om=json.loads((old/'manifest.json').read_text(encoding='utf-8'));c=next(c for c in om['cases'] if c['name']=='contents');selected=om['selected'];expected=c['expected']
source=(work/'source').resolve();source.mkdir(exist_ok=True);run(independent,['x','-y','-p'+om['password'],'-o'+str(source),(old/c['name']/c['members'][0]['name']).resolve()])
for e in expected:assert sha(source/e['path'])==e['sha256']
common=['a','-t7z','-y','-sccUTF-8','-mtc=off','-mta=off','-mtm=off','-mhc=off','-mqs=off','-ms=128m','-m0=Deflate'];cases=[]
for name,headers,volume in [('plain',False,False),('contents',False,False),('headers',True,False),('volumes',True,True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True);args=[*common,*(['-p'+key] if name!='plain' else []),'-mx='+('9' if name=='headers' else '5'),'-mhe='+('on' if headers else 'off'),*(['-v256k'] if volume else []),dest/'固实Deflate日志.7z','.']
 if not list(dest.iterdir()):run(producer,args,source)
 members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())];out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-p'+key,'-sccUTF-8',dest/members[0]['name']]);assert 'Solid = +' in listing and 'Blocks = 1' in listing and 'DEFLATE' in listing.upper()
 cases.append({'name':name,'method':'Deflate','encrypted':name!='plain','filenameEncrypted':headers,'members':members,'originalArgs':list(map(str,args)),'expected':expected,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
side={}
for name,preceding in [('at-prefix-limit',63*cap),('over-prefix-limit',63*cap+1)]:
 d=(work/name).resolve();d.mkdir(exist_ok=True);(d/'00-huge.txt').write_bytes(b'L'*preceding);(d/'99-selected.txt').write_bytes((source/selected).read_bytes());target=(base/(name+'.7z')).resolve();args=[*common,'-p'+key,target,'.'];run(producer,args,d)
 out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),target])
 for p in d.iterdir():assert sha(out/p.name)==sha(p)
 listing=run(producer,['l','-slt','-p'+key,target]);assert 'Solid = +' in listing and 'Blocks = 1' in listing
 side[name]={'sha256':sha(target),'originalArgs':list(map(str,args)),'precedingBytes':preceding,'selectedPrefixBytes':cap,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Solid =','Blocks ='))]}
for original,name,early in [('contents','tail-damaged',False),('contents','bad-start',True),('plain','plain-tail-damaged',False)]:
 c=next(c for c in cases if c['name']==original);b=bytearray((base/original/c['members'][0]['name']).read_bytes());end=32+int.from_bytes(b[12:20],'little')
 if early:b[32]^=255
 else:b[end-64]^=128
 target=(base/(name+'.7z')).resolve();target.write_bytes(b);run(independent,['t','-p'+key,target],success=False)
m={k:om[k] for k in ['selected','first','renderedPrefixBytes','renderedPrefixSha256','totalSize','firstPrefixSha256','boundaries']};m.update(password=key,producer=str(producer),producerSha256=sha(producer),independent=str(independent),independentSha256=sha(independent),cases=cases,sideOriginals=side,mutations={n:sha(base/n) for n in ['tail-damaged.7z','plain-tail-damaged.7z','bad-start.7z']})
manifest.write_text(json.dumps(m,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('Four original solid Deflate groups and exact 64 MiB/one-byte-over dependency limits independently read back')

"""Two-tool original solid Deflate64 fixtures, including AES and numeric volumes."""
import hashlib,json,subprocess
from pathlib import Path
base=Path('tests/fixtures/archives/sevenz-password-solid-deflate64-preview-1641');work=Path('test-results/sevenz-password-solid-deflate64-preview-1641');base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve();sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();cap=1024*1024;selected='notes/说明 [1].txt';key='Deflate64固实公开密码1641!'
manifest=base/'manifest.json'
def run(exe,args,cwd=None,success=True):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=180);assert (r.returncode==0)==success,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace');return r.stdout.decode('utf-8','replace')
def ensure_large(m):
 other=(work/'large-unrelated.7z').resolve()
 if other.exists():return str(other)
 source=(work/'large-selected-source').resolve();source.mkdir(exist_ok=True)
 c=m['cases'][0];run(independent,['x','-y','-p'+m['password'],'-o'+str(source),(base/c['name']/c['members'][0]['name']).resolve()])
 for e in c['expected']:assert sha(source/e['path'])==e['sha256']
 big=(work/'unrelated-source').resolve();big.mkdir(exist_ok=True)
 with (big/'unrelated.bin').open('wb') as f:f.truncate(65*cap)
 run(producer,['a','-t7z','-y','-mhc=off','-m0=Copy','-ms=off',other,'unrelated.bin'],big)
 run(producer,['a','-t7z','-y','-sccUTF-8','-mhc=off','-m0=Deflate64','-ms=128m','-mqs=off','-p'+m['password'],other,'.'],source);run(independent,['t','-p'+m['password'],other])
 with other.open('r+b') as f:f.seek(64);v=f.read(1);f.seek(64);f.write(bytes([v[0]^1]))
 return str(other)
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'))
 for c in m['cases']:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 for n,v in m['sideOriginals'].items():assert sha(base/n)==v['sha256']
 ensure_large(m);print('Immutable AES solid Deflate64 originals retained');raise SystemExit
source=(work/'source').resolve();source.mkdir(exist_ok=True)
old=Path('tests/fixtures/archives/sevenz-password-deflate64-preview-1639');om=json.loads((old/'manifest.json').read_text(encoding='utf-8'));c=om['cases'][0]
run(independent,['x','-y','-p'+om['password'],'-o'+str(source),(old/c['name']/c['members'][0]['name']).resolve()])
for e in c['expected']:assert sha(source/e['path'])==e['sha256']
(source/'00-first.txt').write_bytes(b'First shared text\n'*200);(source/'zz-after.txt').write_bytes(b'Later shared text\n'*2000)
expected=[{'path':p.relative_to(source).as_posix(),'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(source.rglob('*')) if p.is_file()]
common=['a','-t7z','-y','-sccUTF-8','-mhc=off','-ms=128m','-mqs=off','-mtc=off','-mta=off','-mtm=off','-m0=Deflate64']
cases=[]
for name,level,encrypted,headers,volume in [('contents',5,True,False,False),('headers',9,True,True,False),('headers-volumes',5,True,True,True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True);args=[*common,'-mx='+str(level),*(['-p'+key,'-mhe='+('on' if headers else 'off')] if encrypted else []),*(['-v24k'] if volume else []),dest/'固实Deflate64资料.7z','.'];run(producer,args,source)
 members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())];out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True)
 run(independent,['x','-y',*(['-p'+key] if encrypted else []),'-o'+str(out),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-sccUTF-8',*(['-p'+key] if encrypted else []),dest/members[0]['name']]);assert 'Solid = +' in listing and 'Blocks = 1' in listing and 'DEFLATE64' in listing.upper()
 cases.append({'name':name,'password':key if encrypted else None,'level':level,'filenameEncrypted':headers,'members':members,'originalArgs':list(map(str,args)),'expected':expected,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
side={}
for name,total in [('at-shared-limit',64*cap),('oversize-shared',64*cap+1)]:
 d=(work/name).resolve();d.mkdir(exist_ok=True);(d/'readme.txt').write_bytes(b'Valid selected shared text')
 with (d/'filler.bin').open('wb') as f:f.truncate(total-(d/'readme.txt').stat().st_size)
 target=(base/(name+'.7z')).resolve();args=[*common,'-p'+key,'-mhe=on',target,'.'];run(producer,args,d);out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),target])
 listing=run(producer,['l','-slt','-p'+key,target]);assert 'Solid = +' in listing and 'Blocks = 1' in listing
 for p in d.iterdir():assert sha(p)==sha(out/p.name)
 side[target.name]={'sha256':sha(target),'sharedBytes':total,'originalArgs':list(map(str,args)),'independentReadback':True}
d=(work/'image-limit').resolve();d.mkdir(exist_ok=True);bmp=(source/'gallery/02-grid.bmp').read_bytes();large=bmp+b'\0'*(8*cap-len(bmp));large=large[:2]+len(large).to_bytes(4,'little')+large[6:];(d/'companion.txt').write_bytes(b'Shared companion')
for name,data in [('at-limit.bmp',large),('oversize.bmp',large+b'\0')]:
 (d/name).write_bytes(data);target=(base/(name.split('.')[0]+'.7z')).resolve();args=[*common,'-p'+key,target,name,'companion.txt'];run(producer,args,d);out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),target]);assert sha(out/name)==sha(d/name);assert sha(out/'companion.txt')==sha(d/'companion.txt');side[target.name]={'sha256':sha(target),'bytes':len(data),'originalArgs':list(map(str,args)),'independentReadback':True}
original=base/'contents'/cases[0]['members'][0]['name'];b=bytearray(original.read_bytes());b[48]^=1;(base/'damaged-data.7z').write_bytes(b);run(independent,['t','-p'+key,(base/'damaged-data.7z').resolve()],success=False);side['damaged-data.7z']={'sha256':sha(base/'damaged-data.7z'),'controlledMutation':'ciphertext byte 48 xor 1','independentIntegrityFailed':True}
m={'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'selected':selected,'textBytes':om['textBytes'],'textSha256':om['textSha256'],'images':om['images'],'cases':cases,'sideOriginals':side,'password':key};m['largeUnrelatedIgnored']=ensure_large(m);manifest.write_text(json.dumps(m,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Three original AES solid Deflate64 groups and exact shared/image limits independently read back')

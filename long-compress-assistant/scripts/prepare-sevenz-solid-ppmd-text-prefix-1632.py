"""Original solid 7Z selected prefixes, independently read back before mutation."""
import hashlib,json,subprocess
from pathlib import Path
base=Path('tests/fixtures/archives/sevenz-solid-ppmd-text-prefix-1632');work=Path('test-results/sevenz-solid-ppmd-text-prefix-1632')
base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=base/'manifest.json';cap=1024*1024;selected='logs/20-session [1].txt';first='logs/00-first.txt';key='固实PPMd公开密码1632!'
def run(exe,args,cwd=None,success=True):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=120)
 assert (r.returncode==0)==success,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace')
 return r.stdout.decode('utf-8','replace')
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'))
 for c in m['cases']:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 print('Existing original solid samples retained');raise SystemExit
def rendered(b,encoding):
 b=b[:cap]
 for trim in range(5):
  try:
   p=b[:len(b)-trim] if trim else b
   return p.decode(encoding),len(p)
  except UnicodeError:pass
 raise AssertionError(encoding)
source=(work/'source').resolve();(source/'logs').mkdir(parents=True,exist_ok=True);(source/'empty').mkdir(exist_ok=True)
random=hashlib.shake_256(b'7Z-solid-PPMd-prefix-1632').digest(720000)
text=''.join('INFO 2026-10-24 固实日志开头 · '+random[n:n+24].hex()+'\r\n' for n in range(0,len(random),24)).encode()
files={first:b'FIRST entry\n'*(cap//5), 'logs/10-before.txt':b'PRECEDING dependency only\n'*5000,selected:text,'logs/20-session 1.txt':b'NOT the selected bracket path','logs/empty.txt':b'','logs/readme.txt':'完整小文本保持完整校验。'.encode(),'logs/99-tail.txt':b'UNREAD TAIL\n'*180000}
boundaries={
 '31-utf8.txt':(b'A'*(cap-1)+'中文结尾'.encode()+b'Z'*30,'utf-8','UTF-8'),
 '32-utf8-bom.txt':(b'\xef\xbb\xbf'+b'A'*(cap-4)+'中文结尾'.encode()+b'Z'*30,'utf-8-sig','UTF-8 BOM'),
 '33-utf16le.txt':(b'\xff\xfe'+b'A\0'*((cap-4)//2)+'😀结束'.encode('utf-16le')+b'Z\0'*30,'utf-16','UTF-16LE'),
 '34-utf16be.txt':(b'\xfe\xff'+b'\0A'*((cap-4)//2)+'😀结束'.encode('utf-16be')+b'\0Z'*30,'utf-16','UTF-16BE'),
 '35-gbk.txt':('中文'.encode('gbk')+b'A'*(cap-5)+'中文结束'.encode('gbk')+b'Z'*30,'gbk','GBK')}
for n,(b,_,_) in boundaries.items():files['logs/'+n]=b
files['logs/exact.txt']=b'E'*cap;files['logs/just-over.txt']=b'J'*(cap+1)
files['logs/30-binary.txt']=b'\0'*(cap+4)
for n,b in files.items():(source/n).write_bytes(b)
expected=[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()]
common=['a','-p'+key,'-t7z','-y','-sccUTF-8','-mtc=off','-mta=off','-mtm=off','-mhc=off','-mqs=off','-ms=128m']
cases=[]
codec=['-m0=PPMd','-mo=6','-mmem=8m']
for name,headers,volume in [('plain',False,False),('contents',False,False),('headers',True,False),('volumes',True,True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True);args=[*[a for a in common if name!='plain' or not a.startswith('-p')],*codec,*(['-mo=12','-mmem=16m'] if name=='headers' else []),'-mhe='+('on' if headers else 'off'),*(['-v256k'] if volume else []),dest/'固实PPMd日志.7z','.']
 if not list(dest.iterdir()):run(producer,args,source)
 members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())]
 out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-p'+key,'-sccUTF-8',dest/members[0]['name']]);assert 'Solid = +' in listing and 'Blocks = 1' in listing and 'PPMD' in listing.upper()
 cases.append({'name':name,'method':'PPMd','encrypted':name!='plain','filenameEncrypted':headers,'members':members,'originalArgs':list(map(str,args)),'expected':expected,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
# Controlled damage preserves metadata and the readable selected prefix. These
# are deliberately mutated originals, not natural producer damage samples.
original=(base/'contents'/cases[1]['members'][0]['name']);b=bytearray(original.read_bytes());end=32+int.from_bytes(b[12:20],'little');assert end>32
b[end-1]^=128;(base/'tail-damaged.7z').write_bytes(b);run(independent,['t','-p'+key,(base/'tail-damaged.7z').resolve()],success=False)
b=bytearray(original.read_bytes());b[32]=3;(base/'bad-start.7z').write_bytes(b);run(independent,['t','-p'+key,(base/'bad-start.7z').resolve()],success=False)
plain=base/'plain'/cases[0]['members'][0]['name'];b=bytearray(plain.read_bytes());end=32+int.from_bytes(b[12:20],'little');b[end-1]^=1
(base/'plain-tail-damaged.7z').write_bytes(b);run(independent,['t',(base/'plain-tail-damaged.7z').resolve()],success=False)
# An original >64 MiB shared total with a reachable early prefix, and an
# unreachable selected entry after 64 MiB. Full readback independently checked.
large=(work/'large-source').resolve();large.mkdir(exist_ok=True)
(large/'00-huge.txt').write_bytes(b'L'*(65*cap));(large/'99-selected.txt').write_bytes(text)
large_archive=(base/'large-shared.7z').resolve();args=[*common,*codec,large_archive,'.'];run(producer,args,large)
out=(work/'readback/large-shared').resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),large_archive])
for n in ['00-huge.txt','99-selected.txt']:assert sha(out/n)==sha(large/n)
prefix,n=rendered(text,'utf-8')
manifest.write_text(json.dumps({'password':key,'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'selected':selected,'first':first,'cases':cases,'renderedPrefixBytes':n,'renderedPrefixSha256':hashlib.sha256(prefix.encode()).hexdigest(),'totalSize':len(text),'firstPrefixSha256':hashlib.sha256(files[first][:cap]).hexdigest(),'boundaries':[{'path':'logs/'+name,'encoding':label,'bytes':rendered(b,enc)[1],'sha256':hashlib.sha256(rendered(b,enc)[0].encode()).hexdigest()} for name,(b,enc,label) in boundaries.items()],'sideOriginals':{'large-shared':{'sha256':sha(large_archive),'originalArgs':list(map(str,args)),'independentReadback':True,'listing':run(producer,['l','-slt','-p'+key,large_archive])}},'mutations':{n:sha(base/n) for n in ['tail-damaged.7z','plain-tail-damaged.7z','bad-start.7z']}},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Four original plain/AES solid PPMd groups independently read back; first/middle prefixes, encoding boundaries, damaged tail and shared range prepared')

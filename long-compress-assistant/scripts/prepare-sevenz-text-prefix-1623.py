"""Original non-solid 7Z text prefixes with independent official readback."""
import hashlib,json,subprocess
from pathlib import Path

base=Path('tests/fixtures/archives/sevenz-text-prefix-1623');work=Path('test-results/sevenz-text-prefix-1623')
base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=base/'manifest.json';cap=1024*1024;selected='logs/session [1].txt';key='日志密码1623!'
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'))
 for c in m['cases']:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 print('Existing prefix originals retained');raise SystemExit

def run(exe,args,cwd=None,success=True):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=120)
 assert (r.returncode==0)==success,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace')
 return r.stdout.decode('utf-8','replace')

def rendered(b,encoding):
 b=b[:cap]
 for trim in range(5):
  try:
   p=b[:len(b)-trim] if trim else b
   return p.decode(encoding),len(p)
  except UnicodeError:pass
 raise AssertionError(encoding)

source=(work/'source').resolve();(source/'logs').mkdir(parents=True,exist_ok=True)
random=hashlib.shake_256(b'7Z-prefix-1623').digest(720000)
lines=['INFO 2026-10-23 读取大日志 · '+random[n:n+24].hex()+'\r\n' for n in range(0,len(random),24)]
text=''.join(lines).encode();assert len(text)>2*cap
files={selected:text,'logs/session 1.txt':b'NOT the selected bracket path','readme.txt':'完整小文本仍需完整读取。'.encode(),'exact.txt':b'E'*cap}
boundaries={
 'utf8.txt':(b'A'*(cap-1)+'中文结尾'.encode()+b'Z'*30,'utf-8','UTF-8'),
 'utf8-bom.txt':(b'\xef\xbb\xbf'+b'A'*(cap-4)+'中文结尾'.encode()+b'Z'*30,'utf-8-sig','UTF-8 BOM'),
 'utf16le.txt':(b'\xff\xfe'+b'A\0'*((cap-4)//2)+'😀结束'.encode('utf-16le')+b'Z\0'*30,'utf-16','UTF-16LE'),
 'utf16be.txt':(b'\xfe\xff'+b'\0A'*((cap-4)//2)+'😀结束'.encode('utf-16be')+b'\0Z'*30,'utf-16','UTF-16BE'),
 'gbk.txt':('中文'.encode('gbk')+b'A'*(cap-5)+'中文结束'.encode('gbk')+b'Z'*30,'gbk','GBK'),
}
for n,(b,_,_) in boundaries.items():files['logs/'+n]=b
files['logs/binary.txt']=b'\0'*(cap+4)
for n,b in files.items():(source/n).write_bytes(b)
expected=[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()]
common=['a','-t7z','-y','-sccUTF-8','-mtc=off','-mta=off','-mtm=off','-mhc=off','-ms=off']
cases=[]
for name,method,volume in [('lzma','LZMA',False),('lzma2','LZMA2',False),('volumes','LZMA2',True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True);args=[*common,'-m0='+method,*(['-v256k'] if volume else []),dest/'日志.7z','.']
 run(producer,args,source);members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())]
 out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-o'+str(out),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-sccUTF-8',dest/members[0]['name']]);assert 'Solid = -' in listing
 cases.append({'name':name,'method':method,'members':members,'originalArgs':list(map(str,args)),'expected':expected,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
# Original side cases, plus clearly labelled controlled compressed-data mutations.
side={}
for name,extra in [('solid',['-ms=128m']),('encrypted',['-p'+key]),('store',['-m0=Copy'])]:
 target=(base/(name+'.7z')).resolve();args=[*[arg for arg in common if name!='solid' or arg!='-ms=off'],'-m0=LZMA2',*extra,target,'.' if name=='solid' else selected]
 run(producer,args,source);run(independent,['t',*(['-p'+key] if name=='encrypted' else []),target]);side[name]={'sha256':sha(target),'originalArgs':list(map(str,args)),'independentReadback':True}
late=(base/'late-original.7z').resolve();args=[*common,'-m0=LZMA2',late,selected];run(producer,args,source);run(independent,['t',late]);side['late-original']={'sha256':sha(late),'originalArgs':list(map(str,args)),'independentReadback':True}
b=bytearray(late.read_bytes());end=32+int.from_bytes(b[12:20],'little');assert b[end-1]==0
b[end-1]=3;(base/'tail-damaged.7z').write_bytes(b);run(independent,['t',(base/'tail-damaged.7z').resolve()],success=False)
b=bytearray(late.read_bytes());b[32]=3;(base/'bad-start.7z').write_bytes(b);run(independent,['t',(base/'bad-start.7z').resolve()],success=False)
large=(work/'large-source').resolve();large.mkdir(exist_ok=True)
with (large/'huge.txt').open('wb') as f:
 for _ in range(65):f.write(b'L'*cap)
target=(base/'large-total.7z').resolve();args=[*common,'-m0=LZMA2',target,'huge.txt'];run(producer,args,large)
out=(work/'readback/large-total').resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-o'+str(out),target]);assert sha(out/'huge.txt')==sha(large/'huge.txt');side['large-total']={'sha256':sha(target),'originalArgs':list(map(str,args)),'independentReadback':True,'bytes':65*cap}
other=(work/'unrelated.7z').resolve();run(producer,[*common,'-m0=Copy',other,'huge.txt'],large);run(producer,[*common,'-m0=LZMA2',other,'.'],source);run(independent,['t',other])
with other.open('r+b') as f:f.seek(64);b=f.read(1);f.seek(64);f.write(bytes([b[0]^1]))
prefix,n=rendered(text,'utf-8')
manifest.write_text(json.dumps({'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'selected':selected,'cases':cases,'sideOriginals':side,'mutations':{n:sha(base/n) for n in ['tail-damaged.7z','bad-start.7z']},'renderedPrefixBytes':n,'renderedPrefixSha256':hashlib.sha256(prefix.encode()).hexdigest(),'totalSize':len(text),'boundaries':[{'path':'logs/'+n,'encoding':label,'bytes':rendered(b,enc)[1],'sha256':hashlib.sha256(rendered(b,enc)[0].encode()).hexdigest()} for n,(b,enc,label) in boundaries.items()],'largeUnrelatedIgnored':str(other)},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('3 original non-solid groups independently read back; five encoding boundaries, large total and explicit partial-tail cases prepared')

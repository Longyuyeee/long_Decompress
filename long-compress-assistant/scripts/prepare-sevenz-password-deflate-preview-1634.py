"""Original AES non-solid Deflate images/text; independently read back."""
import hashlib,io,json,subprocess
from pathlib import Path
from PIL import Image,ImageDraw
base=Path('tests/fixtures/archives/sevenz-password-deflate-preview-1634');work=Path('test-results/sevenz-password-deflate-preview-1634');base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve();sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();cap=1024*1024;selected='notes/说明 [1].txt'
manifest=base/'manifest.json'
def run(exe,args,cwd=None,success=True):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=180);assert (r.returncode==0)==success,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace');return r.stdout.decode('utf-8','replace')

source=(work/'source').resolve();(source/'gallery').mkdir(parents=True,exist_ok=True);(source/'notes').mkdir(exist_ok=True)
def ensure_unrelated():
 other=(work/'large-unrelated.7z').resolve()
 if other.exists():return other
 if not (source/selected).exists():
  c=cases[0];run(independent,['x','-y','-p'+key,'-o'+str(source),base.resolve()/c['name']/c['members'][0]['name']])
  for e in c['expected']:assert sha(source/e['path'])==e['sha256']
 big=(work/'unrelated-source').resolve();big.mkdir(exist_ok=True)
 with (big/'unrelated.bin').open('wb') as f:f.truncate(65*cap)
 run(producer,['a','-t7z','-y','-m0=Copy','-mhc=off','-ms=off',other,'unrelated.bin'],big);run(producer,[*common,other,'.'],source);run(independent,['t','-p'+key,other])
 with other.open('r+b') as f:f.seek(64);v=f.read(1);f.seek(64);f.write(bytes([v[0]^1]))
 return other
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'));cases=m['cases'];key=m['password']
 for c in cases:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 common=['a','-t7z','-y','-mhc=off','-ms=off','-m0=Deflate','-p'+key]
 ensure_unrelated();print('Encrypted Deflate originals retained; ignored supplement restored if missing');raise SystemExit
random=hashlib.shake_256(b'Deflate-1634-selected').digest(48000)
text=''.join('INFO 2026-10-24 Deflate 完整文本 · '+random[n:n+16].hex()+'\n' for n in range(0,len(random),16)).encode()
files={selected:text,'notes/说明 1.txt':b'NOT the selected bracket path','notes/empty.txt':b'','notes/utf16.txt':b'\xff\xfe'+'完整 UTF16 文本'.encode('utf-16le'),'notes/gbk.txt':'中文编码说明'.encode('gbk'),'notes/binary.txt':b'\0'*500,'notes/exact.txt':b'T'*cap,'notes/large.txt':b'T'*(cap+1)}
image=Image.new('RGB',(320,240),(17,31,48));draw=ImageDraw.Draw(image)
for y in range(0,240,20):
 for x in range(0,320,20):draw.rectangle((x,y,x+17,y+17),fill=((x*7+y)%256,(y*5+x)%256,((x+y)*3)%256))
images=[]
for name,fmt,size in [('01-cover.png','PNG',(320,240)),('02-grid.bmp','BMP',(96,64)),('04-cover.jpg','JPEG',(320,240)),('05-cover.gif','GIF',(160,120)),('06-cover.webp','WEBP',(160,120))]:
 b=io.BytesIO();image.resize(size).save(b,fmt);data=b.getvalue();p='gallery/'+name;files[p]=data;images.append({'path':p,'width':size[0],'height':size[1],'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
files['gallery/03-bad.png']=b'Not a raster image; archive CRC is valid.'
for p,data in files.items():(source/p).write_bytes(data)

key='Deflate公开测试密码1634!'
common=['a','-t7z','-y','-sccUTF-8','-mhc=off','-ms=off','-mtc=off','-mta=off','-mtm=off','-m0=Deflate','-p'+key]
expected=[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()];cases=[]
for name,level,headers,volume in [('contents',5,False,False),('headers',9,True,False),('headers-volumes',5,True,True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True);args=[*common,'-mx='+str(level),'-mhe='+('on' if headers else 'off'),*(['-v24k'] if volume else []),dest/'密码Deflate资料.7z','.']
 run(producer,args,source)
 members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())]
 out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-p'+key,'-o'+str(out),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-sccUTF-8','-p'+key,dest/members[0]['name']]);assert 'Solid = -' in listing and 'DEFLATE' in listing.upper() and '7zAES' in listing
 cases.append({'name':name,'level':level,'password':key,'filenameEncrypted':headers,'members':members,'originalArgs':list(map(str,args)),'expected':expected,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
side={}
for name,extra,content in [('solid',['-ms=on'],'.'),('single',[],selected)]:
 target=(base/(name+'.7z')).resolve();args=[*[x for x in common if name!='solid' or x!='-ms=off'],*extra,target,content];run(producer,args,source);run(independent,['t','-p'+key,target]);side[name]={'sha256':sha(target),'originalArgs':list(map(str,args)),'independentReadback':True}
b=bytearray((base/'single.7z').read_bytes());b[48]^=1;(base/'damaged-data.7z').write_bytes(b);run(independent,['t','-p'+key,(base/'damaged-data.7z').resolve()],success=False)
limits=(work/'limits').resolve();limits.mkdir(exist_ok=True);bmp=files['gallery/02-grid.bmp'];large=bmp+b'\0'*(8*cap-len(bmp));large=large[:2]+len(large).to_bytes(4,'little')+large[6:];b=io.BytesIO();Image.new('RGB',(9000,1)).save(b,'PNG')
for name,data in [('at-limit.bmp',large),('oversize.bmp',large+b'\0'),('dimensions.png',b.getvalue())]:
 (limits/name).write_bytes(data);target=(base/(name.split('.')[0]+'.7z')).resolve();args=[*common,target,name];run(producer,args,limits);run(independent,['t','-p'+key,target]);side[name]={'sha256':sha(target),'bytes':len(data),'originalArgs':list(map(str,args)),'independentReadback':True}
other=ensure_unrelated()
manifest.write_text(json.dumps({'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'selected':selected,'textBytes':len(text),'textSha256':hashlib.sha256(text).hexdigest(),'images':images,'cases':cases,'sideOriginals':side,'password':key,'mutations':{'damaged-data.7z':{'sha256':sha(base/'damaged-data.7z'),'independentIntegrityFailed':True}},'largeUnrelatedIgnored':str(other)},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Three original AES+Deflate groups independently read back; encrypted boundaries, damaged data and unrelated large block prepared')

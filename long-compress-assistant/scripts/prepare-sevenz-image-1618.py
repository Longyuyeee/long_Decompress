"""Original 7-Zip image inputs, independently decoded; existing receipts retained."""
import hashlib,json,subprocess,io
from pathlib import Path
from PIL import Image,ImageDraw
base=Path('tests/fixtures/archives/sevenz-image-1618');work=Path('test-results/sevenz-image-1618');work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=base/'manifest.json'
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'))
 for c in m['cases']:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 print('Existing 7Z image originals retained');raise SystemExit
base.mkdir(parents=True,exist_ok=True);source=(work/'source').resolve();(source/'gallery').mkdir(parents=True,exist_ok=True)
image=Image.new('RGB',(320,240),(17,31,48));draw=ImageDraw.Draw(image)
for y in range(0,240,20):
 for x in range(0,320,20):draw.rectangle((x,y,x+17,y+17),fill=((x*7+y)%256,(y*5+x)%256,((x+y)*3)%256))
files={};images=[]
for name,fmt,size in [('01-cover.png','PNG',(320,240)),('02-grid.bmp','BMP',(96,64)),('04-cover.jpg','JPEG',(320,240)),('05-cover.gif','GIF',(160,120)),('06-cover.webp','WEBP',(160,120))]:
 buf=io.BytesIO();image.resize(size).save(buf,format=fmt);data=buf.getvalue();p='gallery/'+name;files[p]=data
 images.append({'path':p,'width':size[0],'height':size[1],'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
files['gallery/03-bad.png']=b'This is not an image; original archive CRC is valid.'
files['readme.txt']='7Z 图片读取，文本仍保持 1 MiB 完整预览。'.encode()
files['unrelated.bin']=hashlib.shake_256(b'7z-image-1618').digest(24000)
for name,data in files.items():(source/name).write_bytes(data)
expected=[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()]
def run(exe,args,cwd=None):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=90);assert r.returncode==0,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace');return r.stdout.decode('utf-8','replace')
key='7Z图片密码1618!';cases=[]
for name,method,password,headers,volume,solid in [('lzma','LZMA',None,False,False,False),('lzma2','LZMA2',None,False,False,False),('contents','LZMA2',key,False,False,False),('headers-volumes','LZMA2',key,True,True,False),('solid','LZMA2',None,False,False,True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True);args=['a','-t7z','-y','-sccUTF-8','-mx=1','-mtc=off','-mta=off','-mtm=off','-m0='+method,'-ms='+('on' if solid else 'off'),*(['-p'+password,'-mhe='+('on' if headers else 'off')] if password else []),*(['-v12k'] if volume else []),dest/'相册.7z','.']
 run(producer,args,source);members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())]
 out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-o'+str(out),*(['-p'+password] if password else []),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 cases.append({'name':name,'method':method,'password':password,'solid':solid,'members':members,'expected':expected,'originalArgs':list(map(str,args)),'independentReadback':True})
limits=(work/'limits').resolve();limits.mkdir(exist_ok=True)
bmp=files['gallery/02-grid.bmp'];large=bmp+b'\0'*(8*1024*1024-len(bmp));large=large[:2]+len(large).to_bytes(4,'little')+large[6:]
buf=io.BytesIO();Image.new('RGB',(9000,1)).save(buf,'PNG')
limits_files={'at-limit.bmp':large,'oversize.bmp':large+b'\0','dimensions.png':buf.getvalue()}
for name,data in limits_files.items():
 (limits/name).write_bytes(data);run(producer,['a','-t7z','-y','-mx=1','-m0=LZMA2','-mhc=off','-ms=off',(base/(name.split('.')[0]+'.7z')).resolve(),name],limits)
single=(work/'single').resolve();single.mkdir(exist_ok=True);(single/'preview.png').write_bytes(files['gallery/01-cover.png'])
terminal=base/'terminal.7z';run(producer,['a','-t7z','-y','-m0=LZMA2','-mhc=off','-ms=off',terminal.resolve(),'preview.png'],single)
b=bytearray(terminal.read_bytes());end=32+int.from_bytes(b[12:20],'little');assert b[end-1]==0;b[end-1]=1;(base/'bad-end.7z').write_bytes(b)
bigsource=(work/'large-source').resolve();bigsource.mkdir(exist_ok=True)
with (bigsource/'unrelated.bin').open('wb') as f:f.truncate(65*1024*1024)
(bigsource/'preview.png').write_bytes(files['gallery/01-cover.png']);big=(work/'large-unrelated.7z').resolve()
run(producer,['a','-t7z','-y','-m0=Copy','-ms=off',big,'unrelated.bin'],bigsource);run(producer,['a','-t7z','-y','-mx=1','-m0=LZMA2','-ms=off',big,'preview.png'],bigsource)
manifest.write_text(json.dumps({'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'images':images,'cases':cases,'limits':{n:{'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in limits_files.items()},'mutations':{n:sha(base/n) for n in ['terminal.7z','bad-end.7z']},'largeUnrelatedIgnored':str(big)},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('5 original groups independently decoded; 5 raster types, byte/dimension/end limits prepared')

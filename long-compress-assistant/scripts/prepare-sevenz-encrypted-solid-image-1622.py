"""Original encrypted solid images with independent official readback."""
import hashlib, io, json, subprocess
from pathlib import Path
from PIL import Image, ImageDraw

base=Path('tests/fixtures/archives/sevenz-encrypted-solid-image-1622')
work=Path('test-results/sevenz-encrypted-solid-image-1622')
base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=base/'manifest.json'
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'))
 for c in m['cases']:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 print('Existing encrypted image originals retained');raise SystemExit

def run(exe,args,cwd=None):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=120)
 assert r.returncode==0,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace')
 return r.stdout.decode('utf-8','replace')

key='7Z加密相册1622!'
source=(work/'source').resolve();(source/'gallery').mkdir(parents=True,exist_ok=True)
image=Image.new('RGB',(320,240),(17,31,48));draw=ImageDraw.Draw(image)
for y in range(0,240,20):
 for x in range(0,320,20):draw.rectangle((x,y,x+17,y+17),fill=((x*7+y)%256,(y*5+x)%256,((x+y)*3)%256))
files={};images=[]
for name,fmt,size in [('01-cover.png','PNG',(320,240)),('02-grid.bmp','BMP',(96,64)),('04-cover.jpg','JPEG',(320,240)),('05-cover.gif','GIF',(160,120)),('06-cover.webp','WEBP',(160,120))]:
 buf=io.BytesIO();image.resize(size).save(buf,format=fmt);data=buf.getvalue();p='gallery/'+name;files[p]=data
 images.append({'path':p,'width':size[0],'height':size[1],'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
files['gallery/03-bad.png']=b'Not a raster; the original encrypted archive CRC is valid.'
files['readme.txt']='加密固实相册，说明与图片沿原双栏阅读。'.encode()
files['payload.bin']=hashlib.shake_256(b'encrypted-image-1622').digest(48000)
for n,b in files.items():(source/n).write_bytes(b)
expected=[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()]
common=['a','-t7z','-y','-sccUTF-8','-mx=1','-mtc=off','-mta=off','-mtm=off','-mhc=on','-ms=128m','-p'+key]
cases=[]
for name,method,headers,volume in [('content-lzma','LZMA',False,False),('content-lzma2','LZMA2',False,False),('headers','LZMA2',True,False),('headers-volumes','LZMA',True,True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True)
 args=[*common,'-m0='+method,'-mhe='+('on' if headers else 'off'),*(['-v12k'] if volume else []),dest/'相册.7z','.']
 run(producer,args,source)
 members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())]
 out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True)
 run(independent,['x','-y','-p'+key,'-o'+str(out),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-sccUTF-8','-p'+key,dest/members[0]['name']])
 assert 'Solid = +' in listing and 'Blocks = 1' in listing
 cases.append({'name':name,'method':method,'password':key,'filenameEncrypted':headers,'members':members,'originalArgs':list(map(str,args)),'expected':expected,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})

# Original encrypted boundary cases retain a companion in the same shared block.
limits=(work/'limits').resolve();limits.mkdir(exist_ok=True)
bmp=files['gallery/02-grid.bmp'];at=bmp+b'\0'*(8*1024*1024-len(bmp));at=at[:2]+len(at).to_bytes(4,'little')+at[6:]
buf=io.BytesIO();Image.new('RGB',(9000,1)).save(buf,'PNG')
limitfiles={'at-limit.bmp':at,'oversize.bmp':at+b'\0','dimensions.png':buf.getvalue()}
(limits/'companion.txt').write_bytes(b'Encrypted shared companion\n'*80)
for name,b in limitfiles.items():
 (limits/name).write_bytes(b);target=(base/(name.split('.')[0]+'.7z')).resolve()
 run(producer,[*common,'-m0=LZMA2',target,name,'companion.txt'],limits);run(independent,['t','-p'+key,target])
damaged=bytearray((base/'content-lzma2/相册.7z').read_bytes());damaged[48]^=1;(base/'damaged-data.7z').write_bytes(damaged)
large=(work/'large-source').resolve();large.mkdir(exist_ok=True)
with (large/'huge-outside.bin').open('wb') as f:f.truncate(65*1024*1024)
(large/'preview.png').write_bytes(files['gallery/01-cover.png']);over=(base/'oversize-shared.7z').resolve()
run(producer,[*common,'-m0=LZMA2',over,'.'],large);run(independent,['t','-p'+key,over]);assert 'Blocks = 1' in run(producer,['l','-slt','-p'+key,over])
other=(work/'unrelated.7z').resolve()
run(producer,['a','-t7z','-y','-m0=Copy','-ms=off',other,'huge-outside.bin'],large)
run(producer,[*common,'-m0=LZMA2',other,'.'],source);run(independent,['t','-p'+key,other])
with other.open('r+b') as f:f.seek(64);b=f.read(1);f.seek(64);f.write(bytes([b[0]^1]))
manifest.write_text(json.dumps({'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'images':images,'cases':cases,'limits':{n:{'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in limitfiles.items()},'mutations':{n:sha(base/n) for n in ['damaged-data.7z','oversize-shared.7z']},'largeUnrelatedIgnored':str(other)},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('4 encrypted solid groups independently decoded; 5 raster types, encrypted limits and corrupt unrelated block prepared')

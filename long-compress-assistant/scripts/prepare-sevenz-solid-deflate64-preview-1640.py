"""Original plain solid Deflate64 graph/text, two-tool readback and explicit negatives."""
import hashlib,io,json,subprocess
from pathlib import Path
from PIL import Image,ImageDraw
base=Path('tests/fixtures/archives/sevenz-solid-deflate64-preview-1640');work=Path('test-results/sevenz-solid-deflate64-preview-1640');base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve();sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();cap=1024*1024;selected='notes/说明 [1].txt'
common=['a','-t7z','-y','-sccUTF-8','-mhc=off','-ms=128m','-mqs=off','-mtc=off','-mta=off','-mtm=off','-m0=Deflate64']
manifest=base/'manifest.json'
def run(exe,args,cwd=None,success=True):
 r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=180);assert (r.returncode==0)==success,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace');return r.stdout.decode('utf-8','replace')
def ensure_large(m):
 # Reconstruct only this ignored supplement on a fresh checkout, never recreate
 # committed originals merely because their generated working directory is absent.
 other=(work/'large-unrelated.7z').resolve()
 if other.exists():return str(other)
 source=(work/'large-selected-source').resolve();source.mkdir(exist_ok=True)
 c=m['cases'][0];run(independent,['x','-y','-o'+str(source),(base/c['name']/c['members'][0]['name']).resolve()])
 for e in c['expected']:assert sha(source/e['path'])==e['sha256']
 big=(work/'unrelated-source').resolve();big.mkdir(exist_ok=True)
 with (big/'unrelated.bin').open('wb') as f:f.truncate(65*cap)
 run(producer,['a','-t7z','-y','-mhc=off','-m0=Copy','-ms=off',other,'unrelated.bin'],big);run(producer,[*common,other,'.'],source);run(independent,['t',other])
 with other.open('r+b') as f:f.seek(64);v=f.read(1);f.seek(64);f.write(bytes([v[0]^1]))
 return str(other)
if manifest.exists():
 m=json.loads(manifest.read_text(encoding='utf-8'))
 for c in m['cases']:
  for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
 ensure_large(m);print('Original solid Deflate64 samples retained; missing ignored supplement restored if needed');raise SystemExit
source=(work/'source').resolve();(source/'gallery').mkdir(parents=True,exist_ok=True);(source/'notes').mkdir(exist_ok=True);(source/'empty-folder').mkdir(exist_ok=True)
random=hashlib.shake_256(b'Deflate64-1640-selected').digest(48000)
text=''.join('INFO 2026-10-24 Deflate64 固实完整文本 · '+random[n:n+16].hex()+'\n' for n in range(0,len(random),16)).encode()
files={'00-first.txt':b'First shared text\n'*200,selected:text,'notes/说明 1.txt':b'NOT the selected bracket path','notes/empty.txt':b'','notes/utf16.txt':b'\xff\xfe'+'完整 UTF16 文本'.encode('utf-16le'),'notes/gbk.txt':'中文编码说明'.encode('gbk'),'notes/binary.txt':b'\0'*500,'notes/exact.txt':b'T'*cap,'notes/large.txt':b'T'*(cap+1),'zz-after.txt':b'Later shared data\n'*2000}
image=Image.new('RGB',(320,240),(17,31,48));draw=ImageDraw.Draw(image)
for y in range(0,240,20):
 for x in range(0,320,20):draw.rectangle((x,y,x+17,y+17),fill=((x*7+y)%256,(y*5+x)%256,((x+y)*3)%256))
images=[]
for name,fmt,size in [('01-cover.png','PNG',(320,240)),('02-grid.bmp','BMP',(96,64)),('04-cover.jpg','JPEG',(320,240)),('05-cover.gif','GIF',(160,120)),('06-cover.webp','WEBP',(160,120))]:
 b=io.BytesIO();image.resize(size).save(b,fmt);data=b.getvalue();p='gallery/'+name;files[p]=data;images.append({'path':p,'width':size[0],'height':size[1],'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
files['gallery/03-bad.png']=b'Not a raster image; shared CRC is valid.'
for p,data in files.items():(source/p).write_bytes(data)
expected=[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()];cases=[]
for name,level,volume in [('ordinary',5,False),('maximum',9,False),('volumes',5,True)]:
 dest=(base/name).resolve();dest.mkdir(exist_ok=True);args=[*common,'-mx='+str(level),*(['-v24k'] if volume else []),dest/'固实Deflate64资料.7z','.'];run(producer,args,source)
 members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())];out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-o'+str(out),dest/members[0]['name']])
 for e in expected:assert sha(out/e['path'])==e['sha256']
 listing=run(producer,['l','-slt','-sccUTF-8',dest/members[0]['name']]);assert 'Solid = +' in listing and 'Blocks = 1' in listing and 'DEFLATE64' in listing.upper()
 cases.append({'name':name,'level':level,'members':members,'originalArgs':list(map(str,args)),'expected':expected,'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
side={};key='Deflate64固实公开密码1640!'
target=(base/'encrypted.7z').resolve();args=[*common,'-p'+key,'-mhe=on',target,'.'];run(producer,args,source);run(independent,['t','-p'+key,target]);side['encrypted.7z']={'sha256':sha(target),'originalArgs':list(map(str,args)),'independentReadback':True}
limits=(work/'limits').resolve();limits.mkdir(exist_ok=True);bmp=files['gallery/02-grid.bmp'];large=bmp+b'\0'*(8*cap-len(bmp));large=large[:2]+len(large).to_bytes(4,'little')+large[6:];b=io.BytesIO();Image.new('RGB',(9000,1)).save(b,'PNG');(limits/'companion.txt').write_bytes(b'Complete shared companion\n'*20)
for name,data in [('at-limit.bmp',large),('oversize.bmp',large+b'\0'),('dimensions.png',b.getvalue())]:
 (limits/name).write_bytes(data);target=(base/(name.split('.')[0]+'.7z')).resolve();args=[*common,target,name,'companion.txt'];run(producer,args,limits);run(independent,['t',target]);side[target.name]={'sha256':sha(target),'bytes':len(data),'originalArgs':list(map(str,args)),'independentReadback':True}
for name,total in [('at-shared-limit',64*cap),('oversize-shared',64*cap+1)]:
 d=(work/name).resolve();d.mkdir(exist_ok=True);readme=b'Valid selected shared text';(d/'readme.txt').write_bytes(readme)
 with (d/'filler.bin').open('wb') as f:f.truncate(total-len(readme))
 target=(base/(name+'.7z')).resolve();args=[*common,target,'.'];run(producer,args,d);out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True);run(independent,['x','-y','-o'+str(out),target]);assert sha(out/'readme.txt')==sha(d/'readme.txt') and sha(out/'filler.bin')==sha(d/'filler.bin');side[target.name]={'sha256':sha(target),'sharedBytes':total,'originalArgs':list(map(str,args)),'independentReadback':True}
original=base/'ordinary'/cases[0]['members'][0]['name'];b=bytearray(original.read_bytes());b[32]=(b[32]&~6)|6;(base/'bad-data.7z').write_bytes(b);run(independent,['t',(base/'bad-data.7z').resolve()],success=False)
m={'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'selected':selected,'textBytes':len(text),'textSha256':hashlib.sha256(text).hexdigest(),'images':images,'cases':cases,'sideOriginals':side,'password':key,'badData':{'sha256':sha(base/'bad-data.7z'),'independentIntegrityFailed':True,'controlledMutation':'reserved block type at byte 32'}}
m['largeUnrelatedIgnored']=ensure_large(m);manifest.write_text(json.dumps(m,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('Three original solid Deflate64 groups independently read back; exact shared/image limits, damaged block and unrelated block prepared')

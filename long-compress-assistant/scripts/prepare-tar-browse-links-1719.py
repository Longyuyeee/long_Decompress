"""Real GNU/PAX text and image hardlinks, independently extracted and hashed."""
from pathlib import Path
import hashlib,json,subprocess,tarfile,uuid,zlib,struct
root=Path(__file__).resolve().parents[1];out=root/'tests/fixtures/archives/tar-browse-links-1719';base=root/'test-results/tar-browse-links-1719';out.mkdir(parents=True,exist_ok=True);base.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
def linux(*args):return subprocess.run(['wsl','-d','Ubuntu','--exec',*args],capture_output=True,check=True).stdout
lzop=root/'test-results/lzop-multi-create-172/research/lzop-linux';assert sha(lzop.read_bytes())=='d9b46cf6fa44f4d657e5a3943d361dac24040d0acda37019b19ba285e52e2c53'
def chunk(name,data):return struct.pack('>I',len(data))+name+data+struct.pack('>I',zlib.crc32(name+data))
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',32,24,8,2,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+bytes([40,120,210])*32)*24))+chunk(b'IEND',b'')
long='docs/'+('长文档'*20)+'.txt';own='docs/'+('链接副本'*18)+'.txt';names=['docs','gallery','empty.txt','docs/readme.txt','gallery/cover.png',long,'阅读副本.txt','gallery/封面副本.png','empty-copy.txt',own,'after.txt'];links=[('阅读副本.txt','docs/readme.txt'),('gallery/封面副本.png','gallery/cover.png'),('empty-copy.txt','empty.txt'),(own,long)]
text=(b'INFO bounded TAR.LZO linked document 1719\n'*32000);small='GNU 长目标文档，与包内原件一致。\n'.encode()
source='/tmp/long-browse-1719-'+uuid.uuid4().hex;linux('mkdir','-p',source+'/docs',source+'/gallery')
for name,data in [('empty.txt',b''),('docs/readme.txt',text),('gallery/cover.png',png),(long,small),('after.txt',b'GNU metadata applied once\n')]:
 p=out/'source-data.bin';p.write_bytes(data);linux('cp',unix(p),source+'/'+name)
(out/'source-data.bin').unlink();linux('python3','-c','import os,json,sys;[(os.link(sys.argv[1]+"/"+b,sys.argv[1]+"/"+a))for a,b in json.loads(sys.argv[2])]',source,json.dumps(links))
variants=[]
for fmt in ['gnu','pax']:
 rawpath=out/(fmt+'.tar');archive=out/(fmt+'.tar.lzo');extra=['--pax-option=delete=atime,delete=ctime']if fmt=='pax'else[]
 linux('tar','--format='+fmt,'--no-recursion','--mtime=@1700001719',*extra,'-cf',unix(rawpath),'-C',source,*names);raw=rawpath.read_bytes();archive.write_bytes(linux(unix(lzop),'-c','-N',unix(rawpath)));assert linux(unix(lzop),'-dc',unix(archive))==raw
 extracted=source+'-'+fmt;linux('mkdir',extracted);linux('tar','-xf',unix(rawpath),'-C',extracted);entries=[]
 with tarfile.open(rawpath)as reader:
  members=list(reader);assert [e.name.rstrip('/')for e in members]==names
  for i,e in enumerate(members):
   data=b''if e.isdir()else reader.extractfile(e).read();target=e.linkname if e.islnk()else None
   if not e.isdir():
    assert linux('sha256sum',source+'/'+names[i]).decode().split()[0]==sha(data)==linux('sha256sum',extracted+'/'+names[i]).decode().split()[0]
    if target:assert linux('stat','-c','%i',extracted+'/'+names[i])==linux('stat','-c','%i',extracted+'/'+target)
   entries.append({'name':names[i],'bytes':len(data),'isDirectory':e.isdir(),'sha256':sha(data),'prefixSha256':sha(data[:1048576]),'linkTarget':target,'headerOffset':e.offset,'dataOffset':e.offset_data})
 variants.append({'format':fmt,'tar':rawpath.name,'tarSha256':sha(raw),'archive':archive.name,'sha256':sha(archive.read_bytes()),'entries':entries})
old=[]
for folder in ['tar-hardlink-1717','tar-gnu-longlink-1718']:
 p=root/'tests/fixtures/archives'/folder;m=json.loads((p/'manifest.json').read_text(encoding='utf-8'));assert sha((p/m['tar']).read_bytes())==m['tarSha256']
 with tarfile.open(p/m['tar'])as reader:
  members=list(reader)
  for i,e in enumerate(m['entries']):
   if 'linkTarget'in e:
    data=reader.extractfile(members[e['targetId']]).read();assert sha(data)==e['sha256'];old.append({'folder':folder,'archive':m['archive'],'name':e['name'],'size':e['bytes'],'linkTarget':e['linkTarget'],'prefixSha256':sha(data[:1048576])})
manifest={'producer':linux('tar','--version').decode().splitlines()[0],'recipeVersion':1,'variants':variants,'priorSparseLinkOracles':old,'sources':'Project-authored text/PNG with real Linux hardlinks; GNU full extraction inode and SHA checks, independent Python TAR reads, fixed-SHA official lzop round trip.'}
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');(base/'producer.json').write_text(json.dumps({'passed':True,**manifest},ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('GNU/PAX producer originals and independent sparse link prefix oracles verified.')

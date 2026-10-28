"""GNU producer, real linked source/extracted trees and Python target-content oracle."""
from pathlib import Path
import hashlib,json,subprocess,tarfile,uuid
root=Path(__file__).resolve().parents[1];base=root/'test-results/tar-hardlink-1717';fixture=root/'tests/fixtures/archives/tar-hardlink-1717';base.mkdir(parents=True,exist_ok=True);fixture.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
def linux(*args):return subprocess.run(['wsl','-d','Ubuntu','--exec',*args],capture_output=True,check=True).stdout
if(fixture/'manifest.json').exists():
 cached=json.loads((fixture/'manifest.json').read_text(encoding='utf-8'))
 if all(sha((fixture/cached[k]).read_bytes())==cached[h]for k,h in[('tar','tarSha256'),('archive','sha256')]):print('Fixed-SHA hardlink original reused.');raise SystemExit
old=root/'tests/fixtures/archives/tar-pax10-1714';m=json.loads((old/'manifest.json').read_text(encoding='utf-8'));assert sha((old/'pax10.tar').read_bytes())==m['tarSha256'];lzop=root/'test-results/lzop-multi-create-172/research/lzop-linux';assert sha(lzop.read_bytes())==m['lzopProducerSha256']
source='/tmp/long-hardlink-1717-'+uuid.uuid4().hex;oracle=source+'-oracle';linux('mkdir',source,oracle);linux('tar','-xf',unix(old/'pax10.tar'),'-C',source)
long=m['entries'][4]['name'];sparse='资料/跨块映射.bin';links=[('alias-a.bin','source.bin'),('资料/alias-b.bin','source.bin'),('长目标副本.bin',long),('empty-copy.bin','empty.bin'),('稀疏副本.bin',sparse)]
linux('python3','-c',"import pathlib,sys,json,os,random;p=pathlib.Path(sys.argv[1]);(p/'source.bin').write_bytes(random.Random(1717).randbytes(200003));(p/'empty.bin').touch();[(os.link(p/b,p/a))for a,b in json.loads(sys.argv[2])]",source,json.dumps(links))
names=['prefix.txt','资料','资料/empty','source.bin','alias-a.bin','资料/alias-b.bin',long,'长目标副本.bin','empty.bin','empty-copy.bin',sparse,'稀疏副本.bin','after.txt','late.bin'];path=fixture/'hardlinks.tar';archive=fixture/'hardlinks.tar.lzo'
linux('tar','--format=pax','--sparse','--sparse-version=1.0','--hole-detection=raw','--no-recursion','--mtime=@1700001717','--pax-option=delete=atime,delete=ctime','-cf',unix(path),'-C',source,*names);raw=path.read_bytes();archive.write_bytes(linux(unix(lzop),'-c','-N',unix(path)));assert linux(unix(lzop),'-d','-c',unix(archive))==raw;linux('tar','-xf',unix(path),'-C',oracle)
def digest(stream):
 h=hashlib.sha256();n=0
 while b:=stream.read(65536):h.update(b);n+=len(b)
 return n,h.hexdigest()
entries=[]
with tarfile.open(path)as reader:
 members=list(reader);assert [e.pax_headers.get('GNU.sparse.name',e.name).rstrip('/')for e in members]==names
 for index,e in enumerate(members):
  name=names[index];assert e.mtime==1700001717;target=names.index(e.linkname)if e.islnk()else None
  if target is not None:assert target<index and members[target].isfile()
  size,checksum=(0,sha(b''))if e.isdir()else digest(reader.extractfile(members[target]if target is not None else e))
  if not e.isdir():
   assert linux('sha256sum',source+'/'+name).decode().split()[0]==checksum;assert linux('sha256sum',oracle+'/'+name).decode().split()[0]==checksum
   if target is not None:
    assert linux('stat','-c','%i',source+'/'+name)==linux('stat','-c','%i',source+'/'+names[target]);assert linux('stat','-c','%i',oracle+'/'+name)==linux('stat','-c','%i',oracle+'/'+names[target])
  entries.append({'name':name,'isDirectory':e.isdir(),'bytes':size,'sha256':checksum,'modifiedTime':{'seconds':1700001717,'nanoseconds':0},**({'linkTarget':e.linkname,'targetId':target}if target is not None else {})})
def octal(b):return int(b.rstrip(b'\0 ').lstrip(b' ')or b'0',8)
def fields(b):
 out={};at=0
 while at<len(b):
  space=b.index(b' ',at);length=int(b[at:space]);key,value=b[space+1:at+length-1].decode().split('=',1);out[key]=value;at+=length
 return out
records=[];at=index=0;local={};control=None
while any(raw[at:at+512]):
 h=raw[at:at+512];size=octal(h[124:136]);start=at;at+=512;kind=chr(h[156])
 if kind=='x':local=fields(raw[at:at+size]);control={'offset':start,'bytes':size,'fields':local};at+=(size+511)//512*512;continue
 body=at;consumed=0;ranges=[]
 if local.get('GNU.sparse.major')=='1':
  def line():
   global consumed
   end=raw.index(b'\n',body+consumed);value=int(raw[body+consumed:end]);consumed=end-body+1;return value
  count=line()
  for _ in range(count):ranges.append([line(),line()])
  consumed=(consumed+511)//512*512
 records.append({'name':names[index],'kind':kind,'logicalIndex':index,'headerOffset':start,'bodyOffset':body,'dataOffset':body+consumed,'physicalBytes':size,'storedBytes':size-consumed,'ranges':ranges,'pax':local,'control':control});index+=1;at=body+(size+511)//512*512;local={};control=None
assert index==14 and sum(r['kind']=='1'for r in records)==5
manifest={'recipeVersion':1,'sources':'GNU tar 1.35 producer from project-authored files and real source hardlinks. GNU full extraction verifies link relationships, names and SHA; Python independently reads target contents (not sparse-placeholder link lookup). Official fixed-SHA lzop round trip.','sourceTarSha256':m['tarSha256'],'lzopProducerSha256':m['lzopProducerSha256'],'tar':path.name,'tarSha256':sha(raw),'archive':archive.name,'sha256':sha(archive.read_bytes()),'entries':entries,'records':records}
(fixture/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');(base/'official.json').write_text(json.dumps({'passed':True,**manifest},ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('GNU hardlink originals and extracted source relationships verified.')

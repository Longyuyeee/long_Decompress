"""Independent GNU PAX sparse 0.0 producer and GNU/Python/source oracles."""
from pathlib import Path
import hashlib,json,subprocess,tarfile,uuid
root=Path(__file__).resolve().parents[1];base=root/'test-results/tar-pax00-1716';fixture=root/'tests/fixtures/archives/tar-pax00-1716'
base.mkdir(parents=True,exist_ok=True);fixture.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
def linux(*args):return subprocess.run(['wsl','-d','Ubuntu','--exec',*args],capture_output=True,check=True).stdout
if(fixture/'manifest.json').exists():
 cached=json.loads((fixture/'manifest.json').read_text(encoding='utf-8'))
 if cached.get('recipeVersion')==1 and all(sha((fixture/cached[k]).read_bytes())==cached[h]for k,h in[('tar','tarSha256'),('archive','sha256')]):print('Fixed-SHA PAX 0.0 original and oracle reused.');raise SystemExit
old=root/'tests/fixtures/archives/tar-pax10-1714';original=json.loads((old/'manifest.json').read_text(encoding='utf-8'));assert sha((old/'pax10.tar').read_bytes())==original['tarSha256']
lzop=root/'test-results/lzop-multi-create-172/research/lzop-linux';assert sha(lzop.read_bytes())==original['lzopProducerSha256']
source='/tmp/long-pax00-1716-'+uuid.uuid4().hex;linux('mkdir',source);linux('tar','-xf',unix(old/'pax10.tar'),'-C',source)
names=[e['name']for e in original['entries']];path=fixture/'pax00.tar';archive=fixture/'pax00.tar.lzo'
linux('tar','--format=pax','--sparse','--sparse-version=0.0','--hole-detection=raw','--no-recursion','--mtime=@1700001716','--pax-option=delete=atime,delete=ctime','-cf',unix(path),'-C',source,*names)
raw=path.read_bytes();archive.write_bytes(linux(unix(lzop),'-c','-N',unix(path)));assert linux(unix(lzop),'-d','-c',unix(archive))==raw
def digest(stream):
 h=hashlib.sha256();n=0
 while b:=stream.read(65536):h.update(b);n+=len(b)
 return n,h.hexdigest()
entries=[]
with tarfile.open(path)as reader:
 members=list(reader);assert [m.pax_headers.get('GNU.sparse.name',m.name).rstrip('/')for m in members]==names
 for m in members:
  name=m.pax_headers.get('GNU.sparse.name',m.name).rstrip('/')
  size,checksum=(0,sha(b''))if m.isdir()else digest(reader.extractfile(m))
  if m.isfile():
   p=subprocess.Popen(['wsl','-d','Ubuntu','--exec','tar','-xOf',unix(path),name],stdout=subprocess.PIPE,stderr=subprocess.PIPE);gs,gd=digest(p.stdout);error=p.stderr.read();assert p.wait()==0,error;assert(gs,gd)==(size,checksum);assert linux('sha256sum',source+'/'+name).decode().split()[0]==checksum
  entries.append({'name':name,'isDirectory':m.isdir(),'bytes':size,'sha256':checksum,'modifiedTime':{'seconds':1700001716,'nanoseconds':0}})
def octal(b):return int(b.rstrip(b'\0 ').lstrip(b' ')or b'0',8)
def fields(b):
 out=[];at=0
 while at<len(b):
  space=b.index(b' ',at);length=int(b[at:space]);key,value=b[space+1:at+length-1].decode().split('=',1);out.append([key,value]);at+=length
 return out
records=[];controls=[];at=index=0;local=[]
while any(raw[at:at+512]):
 header=raw[at:at+512];physical=octal(header[124:136]);kind=chr(header[156]);start=at;at+=512
 if kind=='x':
  local=fields(raw[at:at+physical]);controls.append({'offset':start,'bytes':physical,'fields':local});at+=(physical+511)//512*512;continue
 body=at;ranges=[]
 metadata={k:v for k,v in local if k not in ['GNU.sparse.offset','GNU.sparse.numbytes']}
 if 'GNU.sparse.size'in metadata:
  pairs=[(k,int(v))for k,v in local if k in ['GNU.sparse.offset','GNU.sparse.numbytes']]
  assert len(pairs)==int(metadata['GNU.sparse.numblocks'])*2
  for i in range(0,len(pairs),2):
   assert pairs[i][0]=='GNU.sparse.offset'and pairs[i+1][0]=='GNU.sparse.numbytes';ranges.append([pairs[i][1],pairs[i+1][1]])
 records.append({'name':entries[index]['name'],'logicalIndex':index,'headerOffset':start,'bodyOffset':body,'dataOffset':body,'physicalBytes':physical,'storedBytes':physical,'ranges':ranges,'pax':metadata,'paxFields':local,'control':controls[-1]if local else None})
 index+=1;at=body+(physical+511)//512*512;local=[]
assert index==len(entries)and any(r['pax'].get('GNU.sparse.size')and r['control']['bytes']>512 for r in records)
manifest={'recipeVersion':1,'sources':'Self-authored GNU source tree extracted from fixed-SHA v1.7.14 original, including a 96-range file with a multi-block metadata map; real GNU tar PAX sparse 0.0 producer, GNU/Python/source logical hashes agree; fixed-SHA official lzop round trip. No application archive writer.','sourceTarSha256':original['tarSha256'],'gnuVersion':linux('tar','--version').decode().splitlines()[0],'lzopProducerSha256':original['lzopProducerSha256'],'tar':path.name,'tarSha256':sha(raw),'archive':archive.name,'sha256':sha(archive.read_bytes()),'entries':entries,'records':records}
(fixture/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');(base/'official.json').write_text(json.dumps({'passed':True,**manifest},ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('GNU PAX 0.0 original verified, including a ordered pairs spanning several metadata blocks.')

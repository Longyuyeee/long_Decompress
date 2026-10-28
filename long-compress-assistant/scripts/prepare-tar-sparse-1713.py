"""GNU tar producer original, Python/GNU logical content oracles, official lzop."""
from pathlib import Path
import hashlib,json,subprocess,tarfile,uuid
root=Path(__file__).resolve().parents[1];fixture=root/'tests/fixtures/archives/tar-sparse-1713';base=root/'test-results/tar-sparse-1713'
fixture.mkdir(parents=True,exist_ok=True);base.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
def linux(*args):return subprocess.run(['wsl','-d','Ubuntu','--exec',*args],capture_output=True,check=True).stdout
if(fixture/'manifest.json').exists():
    cached=json.loads((fixture/'manifest.json').read_text(encoding='utf-8'))
    if cached.get('recipeVersion')==1 and all(sha((fixture/cached[k]).read_bytes())==cached[h]for k,h in[('tar','tarSha256'),('archive','sha256')]):
        print('Existing fixed-SHA GNU original and independent oracle reused.');raise SystemExit
lzop=root/'test-results/lzop-multi-create-172/research/lzop-linux';provenance=json.loads((root/'tests/fixtures/archives/lzop-prefix-174/manifest.json').read_text(encoding='utf-8'));assert sha(lzop.read_bytes())==provenance['producerSha256']
source='/tmp/long-gnu-sparse-1713-'+uuid.uuid4().hex
many='资料/'+'多段'*40+'.bin'
recipe=[{'name':'prefix.txt','data':'independent sparse recovery prefix\n'}, {'name':'资料','directory':True},{'name':'资料/empty','directory':True},
 {'name':'资料/少段.bin','size':524288,'chunks':[[0,4096,1],[196608,4096,2]]},
 {'name':many,'size':8388608,'chunks':[[i*131072,4096,i+3]for i in range(30)]},
 {'name':'全零.bin','size':1048576,'chunks':[]}, {'name':'大空洞.bin','size':67108864,'chunks':[[17825792,31,50]]},
 {'name':'after.txt','data':'ordinary member after all sparse files\n'}, {'name':'late.bin','size':1100178,'chunks':[[0,1100178,99]]}]
recipe_file=base/'recipe.json';recipe_file.write_text(json.dumps(recipe,ensure_ascii=False),encoding='utf-8')
producer="""import pathlib,json,sys,random,os
base=pathlib.Path(sys.argv[1]);base.mkdir();entries=json.loads(pathlib.Path(sys.argv[2]).read_text())
for e in entries:
 p=base/e['name'];p.parent.mkdir(parents=True,exist_ok=True)
 if e.get('directory'):p.mkdir(exist_ok=True);continue
 if 'data'in e:p.write_bytes(e['data'].encode())
 else:
  with p.open('wb')as f:
   f.truncate(e['size'])
   for offset,length,seed in e['chunks']:f.seek(offset);f.write(random.Random(seed).randbytes(length))
for p in sorted(base.rglob('*'),key=lambda p:len(p.parts),reverse=True):os.utime(p,(1700001713,1700001713))
"""
linux('python3','-c',producer,source,unix(recipe_file))
path=fixture/'gnu.tar';archive=fixture/'gnu.tar.lzo'
linux('tar','--format=gnu','--sparse','--hole-detection=raw','--no-recursion','--mtime=@1700001713','-cf',unix(path),'-C',source,*[e['name']for e in recipe])
archive.write_bytes(linux(unix(lzop),'-c','-N',unix(path)));raw=path.read_bytes();assert linux(unix(lzop),'-d','-c',unix(archive))==raw
def digest_stream(stream):
 h=hashlib.sha256();n=0
 while chunk:=stream.read(65536):h.update(chunk);n+=len(chunk)
 return n,h.hexdigest()
entries=[]
with tarfile.open(path)as reader:
    members=list(reader);assert [m.name.rstrip('/')for m in members]==[e['name']for e in recipe]
    for m,e in zip(members,recipe):
        if m.isdir():size=0;digest=sha(b'')
        else:
            size,digest=digest_stream(reader.extractfile(m))
            command=['wsl','-d','Ubuntu','--exec','tar','-xOf',unix(path),m.name];p=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE);gs,gd=digest_stream(p.stdout);error=p.stderr.read();assert p.wait()==0,error;assert(gs,gd)==(size,digest)
            expected=linux('sha256sum',source+'/'+m.name).decode().split()[0];assert expected==digest
        entries.append({'name':e['name'],'isDirectory':m.isdir(),'bytes':size,'sha256':digest,'modifiedTime':{'seconds':1700001713,'nanoseconds':0}})
def number(b):return int.from_bytes(b,'big')&((1<<(len(b)*8-1))-1)if b[0]&128 else int(b.rstrip(b'\0 ').lstrip(b' ')or b'0',8)
records=[];logical=0;at=0
while any(raw[at:at+512]):
    head=raw[at:at+512];stored=number(head[124:136]);kind=chr(head[156]);start=at;at+=512;extensions=[];ranges=[]
    if kind=='S':
        block=head[386:482];extended=head[482]
        while True:
            for i in range(len(block)//24):
                slot=block[i*24:(i+1)*24]
                if any(slot):ranges.append([number(slot[:12]),number(slot[12:])])
            if not extended:break
            extensions.append(at);extension=raw[at:at+512];at+=512;block=extension[:504];extended=extension[504]
        assert sum(r[1]for r in ranges)==stored
    record={'headerOffset':start,'dataOffset':at,'type':kind,'storedBytes':stored,'ranges':ranges,'extensionOffsets':extensions}
    if kind not in('L','K'):record['logicalIndex']=logical;record['name']=entries[logical]['name'];logical+=1
    at+=(stored+511)//512*512;records.append(record)
assert logical==len(entries)
manifest={'recipeVersion':1,'sources':'Self-authored Linux sparse source files; real GNU tar --format=gnu --sparse --hole-detection=raw producer. GNU/Python/source hashes agree on logical bytes. Official fixed-SHA lzop full round trip. No application archive writer.','formatReference':'https://www.gnu.org/software/tar/manual/html_node/Old-GNU-Format.html','gnuVersion':linux('tar','--version').decode().splitlines()[0],'lzopProducerSha256':provenance['producerSha256'],'tar':path.name,'tarSha256':sha(raw),'archive':archive.name,'sha256':sha(archive.read_bytes()),'entries':entries,'records':records}
(fixture/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');(base/'official.json').write_text(json.dumps({'passed':True,**manifest},ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(f'GNU original verified: {len(entries)} entries; sparse maps and every logical SHA match independent readers.')

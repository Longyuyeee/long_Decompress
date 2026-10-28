"""GNU/PAX real hardlinks to project-authored ZIP/7Z nested content."""
from pathlib import Path
import hashlib,json,subprocess,tarfile,uuid,zipfile,io
root=Path(__file__).resolve().parents[1];out=root/'tests/fixtures/archives/tar-nested-links-1723';base=root/'test-results/tar-nested-links-1723';out.mkdir(parents=True,exist_ok=True);base.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
def linux(*args):return subprocess.run(['wsl','-d','Ubuntu','--exec',*args],capture_output=True,check=True).stdout
lzop=root/'test-results/lzop-multi-create-172/research/lzop-linux';assert sha(lzop.read_bytes())=='d9b46cf6fa44f4d657e5a3943d361dac24040d0acda37019b19ba285e52e2c53'
def zip_bytes(files):
 b=io.BytesIO()
 with zipfile.ZipFile(b,'w',zipfile.ZIP_DEFLATED)as z:
  for name,data in files:z.writestr(name,data)
 return b.getvalue()
deep=zip_bytes([('third.txt',b'Nested third layer 1723\n')]);plain=zip_bytes([('readme.txt','包内链接压缩包内容，保持原双栏浏览。\n'.encode()),('docs/next.zip',deep)])
(out/'secret.txt').write_text('Inner password belongs only to this archive.\n',encoding='utf-8')
encrypted=out/'locked.7z';encrypted.unlink(missing_ok=True)
subprocess.run([str(root/'src-tauri/resources/archive-engine/7z.exe'),'a','-t7z','-m0=Copy','-mhe=on','-pNested1723!',str(encrypted),'secret.txt'],cwd=out,capture_output=True,check=True)
(out/'secret.txt').unlink()
long='payload/'+('长目标内容'*12)+'.zip';alias='links/'+('长名称副本'*12)+'.zip'
contents=[('payload/plain.data',plain),('payload/locked.data',encrypted.read_bytes()),('payload/fake.data',b'Not an archive, despite the ZIP alias.\n'),(long,plain),('tail.txt',b'Outer archive stays unchanged.\n')]
links=[('links/copy.zip','payload/plain.data'),('links/加密副本.7z','payload/locked.data'),('links/fake.zip','payload/fake.data'),(alias,long)]
names=['payload','links',*[n for n,_ in contents],*[n for n,_ in links]]
source='/tmp/long-nested-1723-'+uuid.uuid4().hex;linux('mkdir','-p',source+'/payload',source+'/links')
for name,data in contents:
 p=out/'transfer.bin';p.write_bytes(data);linux('cp',unix(p),source+'/'+name)
(out/'transfer.bin').unlink();linux('python3','-c','import os,json,sys;[(os.link(sys.argv[1]+"/"+b,sys.argv[1]+"/"+a))for a,b in json.loads(sys.argv[2])]',source,json.dumps(links))
variants=[]
for fmt in ['gnu','pax']:
 rawpath=out/(fmt+'.tar');archive=out/(fmt+'.tar.lzo');extra=['--pax-option=delete=atime,delete=ctime']if fmt=='pax'else[]
 linux('tar','--format='+fmt,'--no-recursion','--mtime=@1700001723',*extra,'-cf',unix(rawpath),'-C',source,*names);raw=rawpath.read_bytes();archive.write_bytes(linux(unix(lzop),'-c','-N',unix(rawpath)));assert linux(unix(lzop),'-dc',unix(archive))==raw
 entries=[]
 with tarfile.open(rawpath)as t:
  for e in t:
   data=b''if e.isdir()else t.extractfile(e).read();entries.append({'name':e.name.rstrip('/'),'bytes':len(data),'sha256':sha(data),'linkTarget':e.linkname if e.islnk()else None,'headerOffset':e.offset,'dataOffset':e.offset_data})
 variants.append({'format':fmt,'archive':archive.name,'sha256':sha(archive.read_bytes()),'tar':rawpath.name,'tarSha256':sha(raw),'entries':entries})
manifest={'producer':linux('tar','--version').decode().splitlines()[0],'recipeVersion':1,'sources':'Project-authored ZIP and encrypted 7-Zip content, real Linux hardlinks, GNU/PAX tar and fixed-SHA lzop; independent Python TAR logical bytes. Public fixture credential Nested1723!.','variants':variants,'aliases':[a for a,b in links],'childSha256':sha(plain),'thirdSha256':sha(deep),'encryptedSha256':sha(encrypted.read_bytes())}
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');(base/'producer.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('GNU/PAX nested archive hardlink originals prepared and independently read.')

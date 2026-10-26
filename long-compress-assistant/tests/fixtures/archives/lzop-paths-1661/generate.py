"""Genuine official -P originals; no rewritten or concatenated headers."""
import pathlib,subprocess,hashlib,json,sys
root=pathlib.Path(__file__).resolve().parent;tool=pathlib.Path(sys.argv[1]).resolve()
work=root.parents[3]/'test-results/lzop-paths-names-1661/fixture-work'
source=work/'source';source.mkdir(parents=True,exist_ok=True)
payloads={'docs/notes.txt':'按目录浏览资料，再取回需要的文件。\n'.encode(),'docs/more/notes.txt':b'different file with same basename\n','pictures/cover.png':(root.parent/'aes-codecs-1521/原文件/图标.png').read_bytes(),'empty.txt':b''}
for name,data in payloads.items():
 p=source/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data)
target=root/'paths.lzo'
subprocess.run([str(tool),'-f','-1','-P','--crc32','-CC','-o',str(target),*payloads],cwd=source,check=True,capture_output=True)
checked=subprocess.run([str(tool),'-t','-P',str(target)],check=True,capture_output=True)
out=work/'restored';out.mkdir(exist_ok=True)
restored=subprocess.run([str(tool),'-d','-P','-f','-p'+str(out),str(target)],check=True,capture_output=True)
expected=[]
for n,data in payloads.items():
 assert (out/n).read_bytes()==data,n
 expected.append({'path':n,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
data=target.read_bytes();headers=[];pos=0
while pos<len(data):
 assert data[pos:pos+9]==b'\x89LZO\x00\x0d\x0a\x1a\x0a'
 flags=int.from_bytes(data[pos+17:pos+21],'big');size=data[pos+33];raw=data[pos+34:pos+34+size]
 headers.append({'flags':hex(flags),'rawNameHex':raw.hex(),'pathFlag':bool(flags&0x2000),'multipartFlag':bool(flags&0x400)})
 assert flags&0x400
 pos+=38+size
 while True:
  u=int.from_bytes(data[pos:pos+4],'big');pos+=4
  if not u:break
  c=int.from_bytes(data[pos:pos+4],'big');pos+=4
  pos+=4*bool(flags&1)+4*bool(flags&0x100)
  if c<u:pos+=4*bool(flags&2)+4*bool(flags&0x200)
  pos+=c
assert any(h['pathFlag'] for h in headers)
manifest={'producer':'official Windows lzop 1.03 / LZO 2.04','toolDownloadSha256':'0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a','options':['-1','-P','--crc32','-CC','-o'],'archive':'paths.lzo','archiveSha256':hashlib.sha256(data).hexdigest(),'officialTestExit':checked.returncode,'officialRestoreExit':restored.returncode,'members':headers,'expected':expected,'scope':'Path original only. Legacy codepage/collision/unsafe paths/numeric/damage cases are explicitly derived tests, not external original compatibility claims. LZOP cannot represent independent empty directories.'}
(root/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('official -P original: member path flags, independent restore and every file SHA256 confirmed')

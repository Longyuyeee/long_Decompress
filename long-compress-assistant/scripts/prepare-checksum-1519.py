"""Independent ZIP/CRC32/MD5/SHA256 inputs for the actual checksum workspace."""
from pathlib import Path
import hashlib,json,random,sys,zipfile,zlib,platform

work=Path(sys.argv[1]).resolve();work.mkdir(parents=True,exist_ok=False)
files={'开头.txt':b'first real file\n','数据.bin':random.Random(1519).randbytes(2*1024*1024+37),'末尾.txt':'完整的末尾内容\n'.encode()}
archive=work/'论坛资料.zip'
with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_STORED) as out:
    for name,data in files.items():out.writestr(name,data)
    out.writestr('空目录/',b'')
raw=archive.read_bytes();parts=[]
for offset in range(0,len(raw),800000):
    part=work/(archive.name+'.'+str(len(parts)+1).zfill(3));part.write_bytes(raw[offset:offset+800000]);parts.append(part)
blocked=work/'待解除占用的说明.txt';blocked.mkdir();replacement=b'Hello, World!'
entries=[(p.name,hashlib.sha256(p.read_bytes()).hexdigest()) for p in parts]+[(blocked.name,hashlib.sha256(replacement).hexdigest())]
manifest=work/'校验清单.sha256';manifest.write_text('\ufeff'+''.join(f'{digest}  {name}\n' for name,digest in entries),encoding='utf-8')
plain=work/'独立 已知文本.txt';plain.write_bytes(replacement)
for extension,digest in [('sfv',f'{zlib.crc32(replacement):08X}'),('md5',hashlib.md5(replacement).hexdigest()),('sha256',hashlib.sha256(replacement).hexdigest())]:
    (work/('独立小清单.'+extension)).write_text(('\ufeff; original\r\n'+f'{plain.name} {digest}' if extension=='sfv' else '\ufeff# original\r\n'+f'{digest} *{plain.name}')+'\r\n',encoding='utf-8')
receipt=dict(tool='CPython '+platform.python_version()+' zipfile/hashlib/zlib',work=str(work),manifest=str(manifest),parts=[str(p)for p in parts],blocked=str(blocked),expectedFiles={n:hashlib.sha256(b).hexdigest()for n,b in files.items()},archiveSha256=hashlib.sha256(raw).hexdigest(),partSha256={p.name:hashlib.sha256(p.read_bytes()).hexdigest()for p in parts},emptyDirectory='空目录')
(work/'originals.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(parts=len(parts),files=len(files),tool=receipt['tool'])))

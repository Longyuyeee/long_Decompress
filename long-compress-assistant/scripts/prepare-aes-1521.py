"""Independent six-codec AES V2 originals; no application code is imported."""
from pathlib import Path
import hashlib,io,json,os,platform,random,struct,subprocess,tarfile
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'test-results/aes-1521/python'))
import cryptography,lz4.frame,brotli
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.argon2 import Argon2id
root=Path(__file__).resolve().parents[1]
out=root/'tests/fixtures/archives/aes-codecs-1521';out.mkdir(parents=True,exist_ok=True)
source=out/'原文件';(source/'中文目录/空目录').mkdir(parents=True,exist_ok=True)
rng=random.Random(1521)
payloads={'大文本.txt':bytes(rng.randrange(32,127) for _ in range(96*1024+37)), '说明.txt':'先填写密码，再查看自己需要的文件。\n'.encode(), '中文目录/数据.bin':rng.randbytes(3337), '空文件.txt':b'', '图标.png':(root/'src-tauri/icons/32x32.png').read_bytes()}
for name,data in payloads.items():(source/name).write_bytes(data)
tarbuf=io.BytesIO()
with tarfile.open(fileobj=tarbuf,mode='w',format=tarfile.USTAR_FORMAT) as tar:
    for name,data in payloads.items():
        entry=tarfile.TarInfo(name);entry.size=len(data);entry.mtime=0;tar.addfile(entry,io.BytesIO(data))
    entry=tarfile.TarInfo('中文目录/空目录');entry.type=tarfile.DIRTYPE;tar.addfile(entry)
sha=lambda b:hashlib.sha256(b).hexdigest()
linux=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
lzip=root/'test-results/lzip-tools/unpacked/usr/bin/lzip.lzip'
password='Long 新内层-1521';cases=[]
for istar in [False,True]:
    for codec in ['lz4','br','lz']:
        raw=tarbuf.getvalue() if istar else payloads['大文本.txt']
        if codec=='lz4':compressed=lz4.frame.compress(raw,content_checksum=True)
        elif codec=='br':compressed=brotli.compress(raw,quality=4)
        else:
            temp=out/'gnu-input.tmp';temp.write_bytes(raw)
            compressed=subprocess.run(['wsl.exe','-d','Ubuntu','--exec',linux(lzip),'-c','-1',linux(temp)],check=True,capture_output=True).stdout
            temp.unlink()
        salt,nonce=os.urandom(32),os.urandom(4);chunk=65536
        header=b'AESENC02'+struct.pack('<HHIQBBHIII',76,0,chunk,len(compressed),1,1,0,65536,3,1)+salt+nonce
        key=Argon2id(salt=salt,length=32,iterations=3,lanes=1,memory_cost=65536).derive(password.encode())
        aes=AESGCM(key)
        encrypted=header+b''.join(aes.encrypt(nonce+i.to_bytes(8,'big'),compressed[i*chunk:(i+1)*chunk],header+i.to_bytes(8,'big')) for i in range(max(1,(len(compressed)+chunk-1)//chunk)))
        fmt=('tar.' if istar else '')+codec+'.aes';name=('资料.' if istar else '大文本.txt.')+fmt
        (out/name).write_bytes(encrypted)
        cases.append(dict(format=fmt,archive=name,sha256=sha(encrypted),bytes=len(encrypted),innerSha256=sha(compressed),decodedSha256=sha(raw),tar=istar,kind='tar' if istar else 'plain',codec=codec))
manifest=dict(tool={'python':platform.python_version(),'cryptography':cryptography.__version__,'lz4':lz4.__version__,'brotli':brotli.__version__,'gnuLzipSha256':sha(lzip.read_bytes())},password=password,layout='Existing AESENC02; Argon2id 65536KiB/3/1; AES-256-GCM; 64KiB original chunks; header/index AAD; no application encoder',expectedFiles={k:sha(v) for k,v in payloads.items()},emptyDirectory='中文目录/空目录',plainName='大文本.txt',cases=cases)
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(out/'.gitattributes').write_text('*.txt -text\n*.aes -text\n*.bin -text\n',encoding='utf-8')
(out/'README.md').write_text('# 独立六内层 Long AES V2 原件\n\n由 CPython cryptography、lz4.frame、Brotli 和 GNU lzip 生成；应用与 Rust 编码器未参与。载荷为自有中文文本、确定性随机文本/二进制、空文件/空目录及项目图标。工具版本、内容与原件 SHA256 见 manifest.json。密码为测试用途的公开值。GNU 工具仅供测试，不随应用分发。\n',encoding='utf-8')
print(json.dumps(dict(originals=len(cases),tool=manifest['tool']),ensure_ascii=False))

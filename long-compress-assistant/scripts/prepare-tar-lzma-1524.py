"""Independent TAR/LZMA and existing AESENC02 originals; no application encoder."""
from pathlib import Path
import hashlib, io, json, lzma, os, platform, random, struct, sys, tarfile
root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(root / 'test-results/aes-1521/python'))
import cryptography
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.argon2 import Argon2id

out = root / 'tests/fixtures/archives/tar-lzma-1524'
source = out / '原文件'
(source / '中文目录/空目录').mkdir(parents=True, exist_ok=True)
rng = random.Random(1524)
payloads = {'大文本.txt': bytes(rng.randrange(32, 127) for _ in range(96 * 1024 + 73)),
            '说明.txt': '目录、多文件和密码分卷，请在原来的入口处理。\n'.encode(),
            '中文目录/数据.bin': rng.randbytes(8117), '空文件.txt': b'',
            '图标.png': (root / 'src-tauri/icons/32x32.png').read_bytes()}
for name, data in payloads.items():
    (source / name).write_bytes(data)
tarbuf = io.BytesIO()
with tarfile.open(fileobj=tarbuf, mode='w', format=tarfile.PAX_FORMAT) as tar:
    for name, data in payloads.items():
        entry = tarfile.TarInfo(name); entry.size = len(data); entry.mtime = 0
        tar.addfile(entry, io.BytesIO(data))
    entry = tarfile.TarInfo('中文目录/空目录'); entry.type = tarfile.DIRTYPE
    tar.addfile(entry)
sha = lambda b: hashlib.sha256(b).hexdigest()
password = 'Long 目录密码-1524'
cases = []
for fmt in ['tar.lzma', 'lzma.aes', 'tar.lzma.aes']:
    istar, encrypted = fmt.startswith('tar.'), fmt.endswith('.aes')
    raw = tarbuf.getvalue() if istar else payloads['大文本.txt']
    compressed = lzma.compress(raw, format=lzma.FORMAT_ALONE,
        filters=[{'id': lzma.FILTER_LZMA1, 'preset': 1, 'lc': 2, 'pb': 1}])
    encoded = compressed
    if encrypted:
        salt, nonce, chunk = os.urandom(32), os.urandom(4), 65536
        header = b'AESENC02' + struct.pack('<HHIQBBHIII',76,0,chunk,len(compressed),1,1,0,65536,3,1) + salt + nonce
        key = Argon2id(salt=salt,length=32,iterations=3,lanes=1,memory_cost=65536).derive(password.encode())
        aes = AESGCM(key)
        encoded = header + b''.join(aes.encrypt(nonce+i.to_bytes(8,'big'), compressed[i*chunk:(i+1)*chunk], header+i.to_bytes(8,'big')) for i in range(max(1,(len(compressed)+chunk-1)//chunk)))
    name = ('资料.' if istar else '大文本.txt.') + fmt
    (out / name).write_bytes(encoded)
    assert lzma.decompress(compressed, format=lzma.FORMAT_ALONE) == raw
    cases.append(dict(format=fmt,archive=name,bytes=len(encoded),sha256=sha(encoded),innerSha256=sha(compressed),decodedSha256=sha(raw),tar=istar,encrypted=encrypted))
manifest = dict(tool={'python':platform.python_version(),'cryptography':cryptography.__version__},password=password,
    layout='Python PAX TAR + FORMAT_ALONE LZMA1 lc2/pb1; optional existing AESENC02, Argon2id 65536KiB/3/1 and AES-256-GCM, 64KiB chunks',
    expectedFiles={name:sha(data) for name,data in payloads.items()},emptyDirectory='中文目录/空目录',plainName='大文本.txt',cases=cases)
(out / 'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(out / '.gitattributes').write_text('*.txt -text\n*.bin -text\n*.aes -text\n*.lzma -text\n',encoding='utf-8')
(out / 'README.md').write_text('# 独立 TAR.LZMA / Long AES 原件\n\n使用 Python 标准库 TAR/LZMA 和 cryptography 独立生成，没有调用应用编码器。自有载荷含中文、随机文本/二进制、空文件/目录及项目图标；原件与内容摘要、工具版本、公开测试密码见 manifest.json。加密布局沿用既有 AESENC02，不表示通用 LZMA 原生密码。\n',encoding='utf-8')
print(json.dumps(dict(originals=len(cases),tool=manifest['tool']),ensure_ascii=False))

"""Self-authored controlled ALZ inputs for preview boundaries; not producer originals."""
import hashlib
import json
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT/'tests/fixtures/archives/alz-text-169'
OUT.mkdir(parents=True, exist_ok=True)
footer = b'CLZ\x01'+b'\0'*8+b'CLZ\x02'
head = b'ALZ\x01\x0a\0\0\0'
text = 'ALZ 包内说明，无需先解压。\n第二行：配置与日志只读查看。\n'


def entry(name, data, method=0, bad_crc=False):
    name = name.encode()
    packed = zlib.compress(data)[2:-4] if method == 2 else data
    return (b'BLZ\x01'+struct.pack('<HBIBB',len(name),32,0x5cfb18c3,0x40,0)
            +struct.pack('<BBIII',method,0,zlib.crc32(data)^bad_crc,len(packed),len(data))+name+packed)


cases = []
for name, data, method, bad in [
    ('store',text.encode(),0,False), ('deflate',text.encode()*120,2,False),
    ('legacy','旧编码日志\n第二行'.encode('gbk'),0,False),
    ('utf16','UTF16 配置\nvalue=中文'.encode('utf-16'),0,False),
    ('empty',b'',0,False), ('binary',b'\0\0\0binary\xff',0,False),
    ('crc-damage',text.encode(),2,True),
    ('at-limit',b'x'*(1024*1024),2,False), ('too-large',b'x'*(1024*1024+1),2,False),
]:
    # Unsupported unrelated content must not prevent viewing a good exact entry.
    unrelated=entry('unrelated.bin',b'not method three content',3)
    file=OUT/(name+'.alz')
    file.write_bytes(head+unrelated+entry('docs/readme.txt',data,method,bad)+footer)
    cases.append(dict(name=name,controlled=True,path=file.name,bytes=file.stat().st_size,
                      sha256=hashlib.sha256(file.read_bytes()).hexdigest(),plainBytes=len(data),
                      plainSha256=hashlib.sha256(data).hexdigest(),method=method))
# A separate display fixture leaves the checked decoder inputs byte-for-byte unchanged.
def chunk(kind, data):
    return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',1,1,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0\x22\xaa\xee\xff'))+chunk(b'IEND',b'')
ui=OUT/'ui.alz'; ui.write_bytes((OUT/'store.alz').read_bytes()[:-16]+entry('picture.png',png)+footer)
(OUT/'manifest.json').write_text(json.dumps(dict(controlled=True,historicalProducer=False,
    text=text,interpretation=__doc__,ui=dict(path='ui.alz',sha256=hashlib.sha256(ui.read_bytes()).hexdigest()),cases=cases),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'Prepared {len(cases)} controlled small preview fixtures')

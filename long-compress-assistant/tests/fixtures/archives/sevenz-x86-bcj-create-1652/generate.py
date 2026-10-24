"""Independent BCJ byte oracle from Python's liblzma, no application encoder."""
from pathlib import Path
import hashlib, json, lzma, random, sys
root=Path(__file__).resolve().parent
rng=random.Random(1652)
data=bytearray(rng.randbytes(32791))
for i in list(range(7,len(data)-5,31))+[4092,8189,8190,8191,16380,16383,32780]:
 data[i:i+5]=bytes([0xe8 if i%2 else 0xe9])+((0x7ffffff0-i)&0xffffffff).to_bytes(4,'little') if i%3==0 else bytes([0xe8])+((123456-i)&0xffffffff).to_bytes(4,'little')
(root/'input.bin').write_bytes(data)
for start in [0,0x7ffffffd,0xfffffffd]:
 chain=[{'id':lzma.FILTER_X86,'start_offset':start},{'id':lzma.FILTER_LZMA2,'preset':0}]
 encoded=lzma.compress(data,format=lzma.FORMAT_RAW,filters=chain)
 filtered=lzma.decompress(encoded,format=lzma.FORMAT_RAW,filters=[chain[1]])
 assert lzma.decompress(encoded,format=lzma.FORMAT_RAW,filters=chain)==data
 (root/f'filtered-{start}.bin').write_bytes(filtered)
manifest={'generator':'Python stdlib lzma/liblzma; BCJ + LZMA2 encoded then LZMA2 alone decoded','python':sys.version,'seed':1652,'files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in root.glob('*.bin')}}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
print(json.dumps(manifest))

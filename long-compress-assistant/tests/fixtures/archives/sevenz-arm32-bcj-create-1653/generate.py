"""Deterministic ARM32 instructions and independent Python/liblzma byte oracle.
The ELF is a hand-built minimal Linux ARM EABI exit program plus synthetic code,
not an external application's performance sample.
"""
from pathlib import Path
import hashlib,json,lzma,random,struct,sys
root=Path(__file__).resolve().parent
rng=random.Random(1653);data=bytearray(rng.randbytes(32791))
for offset in range(0,len(data)-3,8):
 word=0xeb000000 | ((0xffffff-offset//4) & 0xffffff)
 struct.pack_into('<I',data,offset,word)
(root/'input.bin').write_bytes(data)
for start in [0,0x7ffffffc,0xfffffffc]:
 chain=[{'id':lzma.FILTER_ARM,'start_offset':start},{'id':lzma.FILTER_LZMA2,'preset':0}]
 encoded=lzma.compress(data,format=lzma.FORMAT_RAW,filters=chain)
 filtered=lzma.decompress(encoded,format=lzma.FORMAT_RAW,filters=[chain[1]])
 assert lzma.decompress(encoded,format=lzma.FORMAT_RAW,filters=chain)==data
 (root/f'filtered-{start}.bin').write_bytes(filtered)
payload=struct.pack('<III',0xe3a00000,0xe3a07001,0xef000000)+data
ident=b'\x7fELF'+bytes([1,1,1])+bytes(9);base=0x10000;size=84+len(payload)
header=struct.pack('<16sHHIIIIIHHHHHH',ident,2,40,1,base+84,52,0,0x05000000,52,32,1,0,0,0)
program=struct.pack('<IIIIIIII',1,0,base,base,size,size,5,4096)
elf=header+program+payload;assert len(elf)==size
(root/'sample-arm32.elf').write_bytes(elf)
manifest={'generator':'Python stdlib liblzma ARM + LZMA2, decoded using LZMA2 alone for independent BCJ bytes','python':sys.version,'seed':1653,'elf':'Hand-built minimal ELF32 little-endian ARM EABI exit program and synthetic instructions; no external real application benchmark','references':['https://github.com/tukaani-project/xz/blob/master/src/liblzma/simple/arm.c','https://github.com/tukaani-project/xz/blob/master/src/liblzma/simple/simple_coder.c'],'files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.iterdir()) if p.suffix in ('.bin','.elf')}}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
print('Independent ARM32 fixture generated:',len(manifest['files']),'files')

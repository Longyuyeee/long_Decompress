"""Independent liblzma Thumb vectors and a hand-built ELF32 Thumb exit program.
Synthetic instruction data, not an external application or compression benchmark.
"""
from pathlib import Path
import hashlib,json,lzma,random,struct,sys
root=Path(__file__).resolve().parent
rng=random.Random(1654);data=bytearray(rng.randbytes(32791))
for offset in range(0,len(data)-3,6):
 immediate=(0x3fffff-offset//2)&0x3fffff
 data[offset:offset+4]=bytes([(immediate>>11)&255,0xf0|((immediate>>19)&7),immediate&255,0xf8|((immediate>>8)&7)])
(root/'input.bin').write_bytes(data)
for start in [0,0x7ffffffe,0xfffffffe]:
 chain=[{'id':lzma.FILTER_ARMTHUMB,'start_offset':start},{'id':lzma.FILTER_LZMA2,'preset':0}]
 encoded=lzma.compress(data,format=lzma.FORMAT_RAW,filters=chain)
 filtered=lzma.decompress(encoded,format=lzma.FORMAT_RAW,filters=[chain[1]])
 assert lzma.decompress(encoded,format=lzma.FORMAT_RAW,filters=chain)==data
 (root/f'filtered-{start}.bin').write_bytes(filtered)
# movs r0,#0; movs r7,#1; svc #0. ELF e_entry low bit declares Thumb.
payload=struct.pack('<HHH',0x2000,0x2701,0xdf00)+data
ident=b'\x7fELF'+bytes([1,1,1])+bytes(9);base=0x10000;size=84+len(payload)
header=struct.pack('<16sHHIIIIIHHHHHH',ident,2,40,1,(base+84)|1,52,0,0x05000000,52,32,1,0,0,0)
program=struct.pack('<IIIIIIII',1,0,base,base,size,size,5,4096)
(root/'sample-arm-thumb.elf').write_bytes(header+program+payload)
manifest={'generator':'Python stdlib liblzma ARMTHUMB + LZMA2; decoded using LZMA2 alone for independent filter bytes','python':sys.version,'seed':1654,'elf':'Hand-built ELF32 ARM EABI Thumb entry and minimal exit program with synthetic instructions; not an external application benchmark','references':['https://github.com/tukaani-project/xz/blob/master/src/liblzma/simple/armthumb.c','https://github.com/tukaani-project/xz/blob/master/src/liblzma/simple/simple_coder.c'],'files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.iterdir()) if p.suffix in ('.bin','.elf')}}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
print('Independent Thumb vectors generated:',len(manifest['files']),'files')

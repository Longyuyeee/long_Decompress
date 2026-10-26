"""Independent 7-Zip RISC-V+Copy vectors and a synthetic minimal RISC-V ELF.
No runtime dependency or external application's compression benchmark.
"""
from pathlib import Path
import hashlib,json,random,struct,subprocess,tempfile,shutil
root=Path(__file__).resolve().parent;app=root.parents[3]
engine=app/'src-tauri/resources/archive-engine/7z.exe'
rng=random.Random(1656);data=bytearray(rng.randbytes(32791))
def jal(rd,imm):
 return 0x6f|(rd<<7)|((imm&0x100000)<<11)|((imm&0x7fe)<<20)|((imm&0x800)<<9)|(imm&0xff000)
for index,offset in enumerate(range(0,len(data)-17,18)):
 rd=[1,5,0,2,3,31][index%6];imm=[0,2,0x7fe,0x800,0xffffe,0x100000,0x1ffffe][index%7]
 struct.pack_into('<I',data,offset,jal(rd,imm))
 pair_rd=[1,5,0,2,10,31][index%6];rs=[pair_rd,(pair_rd+1)&31][index%2]
 inst2=(((index*131)&0xfff)<<20)|(rs<<15)|(10<<7)|[0x67,0x03,0x13,0x23][index%4]
 struct.pack_into('<II',data,offset+4,0x17|(pair_rd<<7)|((index*0x12d000)&0xfffff000),inst2)
 # x2 packed-pair collisions must fake-decode to remain bijective on arbitrary input.
 if index%5==0:struct.pack_into('<II',data,offset+4,0x117|(0x000280e7<<12)&0xffffffff,rng.getrandbits(32))
 data[offset+12:offset+14]=b'\x01\x00'  # C.NOP, hence the next 32-bit instruction is at phase 2.
 struct.pack_into('<I',data,offset+14,jal(1,imm))
(root/'input.bin').write_bytes(data)
def run(args):
 p=subprocess.run([str(engine),*args],capture_output=True);assert p.returncode==0,(args,p.stderr,p.stdout);return p.stdout
with tempfile.TemporaryDirectory(dir=root,prefix='vectors-') as temporary:
 temp=Path(temporary);assert temp.resolve().is_relative_to(root.resolve())
 source=temp/'input.bin';source.write_bytes(data)
 for start in [0,0x7ffffffe,0xfffffffe]:
  archive=temp/f'original-{start}.7z'
  run(['a','-t7z','-mhc=off','-ms=off','-mtm=off','-mtc=off','-mta=off',f'-m0=RISCV:offset={start}','-m1=Copy',str(archive),str(source)])
  packed=archive.read_bytes();assert struct.unpack_from('<Q',packed,12)[0]==len(data)
  assert run(['x','-so',str(archive),'input.bin'])==data
  filtered=packed[32:32+len(data)];assert filtered!=data
  (root/f'filtered-{start}.bin').write_bytes(filtered);shutil.copyfile(archive,root/archive.name)
payload=struct.pack('<III',0x00000513,0x05d00893,0x00000073)+data
ident=b'\x7fELF'+bytes([2,1,1])+bytes(9);base=0x400000;size=120+len(payload)
header=struct.pack('<16sHHIQQQIHHHHHH',ident,2,243,1,base+120,64,0,1,64,56,1,0,0,0)
program=struct.pack('<IIQQQQQQ',1,5,0,base,base,size,size,4096)
(root/'sample-riscv.elf').write_bytes(header+program+payload)
manifest={'generator':'Bundled independent 7-Zip RISC-V + Copy; exact unencrypted packed stream is filter byte oracle; complete independent readback checked','engineSha256':hashlib.sha256(engine.read_bytes()).hexdigest(),'seed':1656,'coverage':['JAL x1/x5 and non-target registers','AUIPC matched/mismatched pairs, sign extension, special x2 fake pairs, x0 landing pads','compressed 16-bit instructions, 32-bit instructions at two-byte phases','signed/unsigned 32-bit PC wrapping'],'elf':'Hand-built ELF64 little-endian RISC-V Linux exit program with synthetic instructions, not an external real application or compression benchmark','references':['https://github.com/tukaani-project/xz/blob/master/src/liblzma/simple/riscv.c','https://github.com/ip7z/7zip/blob/main/CPP/7zip/Compress/BranchRegister.cpp'],'files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.iterdir()) if p.suffix in ('.bin','.elf','.7z')}}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
print('Independent RISC-V vectors generated:',len(manifest['files']),'files')

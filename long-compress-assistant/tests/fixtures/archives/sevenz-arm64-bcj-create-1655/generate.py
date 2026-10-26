"""Independent 7-Zip ARM64+Copy byte vectors and synthetic minimal AArch64 ELF.
No new runtime dependency; no external application's compression benchmark.
"""
from pathlib import Path
import hashlib,json,random,struct,subprocess,tempfile,shutil
root=Path(__file__).resolve().parent;app=root.parents[3]
engine=app/'src-tauri/resources/archive-engine/7z.exe'
rng=random.Random(1655);data=bytearray(rng.randbytes(32791))
immediates=[0,1,0x1ffff,0x20000,0x1dffff,0x1e0000,0x1fffff]
for offset in range(0,len(data)-7,8):
 struct.pack_into('<I',data,offset,0x94000000|((0x3ffffff-offset//4)&0x3ffffff))
 imm=immediates[(offset//8)%len(immediates)]
 struct.pack_into('<I',data,offset+4,0x90000000|((imm&3)<<29)|((imm&0x1ffffc)<<3)|((offset//8)&31))
(root/'input.bin').write_bytes(data)
def run(args):
 p=subprocess.run([str(engine),*args],capture_output=True);assert p.returncode==0,(args,p.stderr,p.stdout);return p.stdout
with tempfile.TemporaryDirectory(dir=root,prefix='vectors-') as temporary:
 temp=Path(temporary);assert temp.resolve().is_relative_to(root.resolve())
 source=temp/'input.bin';source.write_bytes(data)
 for start in [0,0x7ffffffc,0xfffffffc]:
  archive=temp/f'original-{start}.7z'
  run(['a','-t7z','-mhc=off','-ms=off','-mtm=off','-mtc=off','-mta=off',f'-m0=ARM64:offset={start}','-m1=Copy',str(archive),str(source)])
  packed=archive.read_bytes();assert struct.unpack_from('<Q',packed,12)[0]==len(data)
  assert run(['x','-so',str(archive),'input.bin'])==data
  filtered=packed[32:32+len(data)];assert filtered!=data
  (root/f'filtered-{start}.bin').write_bytes(filtered);shutil.copyfile(archive,root/archive.name)
payload=struct.pack('<III',0x52800000,0xd2800ba8,0xd4000001)+data
ident=b'\x7fELF'+bytes([2,1,1])+bytes(9);base=0x400000;size=120+len(payload)
header=struct.pack('<16sHHIQQQIHHHHHH',ident,2,183,1,base+120,64,0,0,64,56,1,0,0,0)
program=struct.pack('<IIQQQQQQ',1,5,0,base,base,size,size,4096)
(root/'sample-arm64.elf').write_bytes(header+program+payload)
manifest={'generator':'Bundled independently implemented 7-Zip ARM64 + Copy; exact unencrypted packed stream is the filter byte oracle; independent complete readback checked','engineSha256':hashlib.sha256(engine.read_bytes()).hexdigest(),'seed':1655,'elf':'Hand-built ELF64 little-endian AArch64 Linux exit program with synthetic BL/ADRP instructions, not an external real application or compression benchmark','references':['https://github.com/tukaani-project/xz/blob/master/src/liblzma/simple/arm64.c','https://github.com/ip7z/7zip/blob/main/CPP/7zip/Compress/BranchRegister.cpp'],'files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.iterdir()) if p.suffix in ('.bin','.elf','.7z')}}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
print('Independent ARM64 vectors generated:',len(manifest['files']),'files')

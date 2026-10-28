"""Independent logical SHA, exact tree, timestamps and real sparse allocation."""
from pathlib import Path
import hashlib,json,os,ctypes
root=Path(__file__).resolve().parents[1];base=root/'test-results/tar-sparse-1713';fixture=root/'tests/fixtures/archives/tar-sparse-1713';m=json.loads((fixture/'manifest.json').read_text(encoding='utf-8'));checks=[]
assert hashlib.sha256((fixture/m['archive']).read_bytes()).hexdigest()==m['sha256']
oracle={e['name']:e for e in m['entries']}
def digest(path):
 h=hashlib.sha256()
 with path.open('rb')as f:
  while b:=f.read(65536):h.update(b)
 return h.hexdigest()
for r in json.loads((base/'native.json').read_text(encoding='utf-8')):
 out=Path(r['result']['outputDirectory']);files={e['name']:oracle[e['name']]for e in r['entries']if not e['isDirectory']};dirs=set()
 for e in r['entries']:
  if e['isDirectory']:dirs.add(e['name'])
  dirs.update(p.as_posix()for p in Path(e['name']).parents if p!=Path('.'))
 assert {p.relative_to(out).as_posix():digest(p)for p in out.rglob('*')if p.is_file()}=={n:e['sha256']for n,e in files.items()}
 assert {p.relative_to(out).as_posix()for p in out.rglob('*')if p.is_dir()}==dirs
 allocations=[]
 for e in r['result']['entries']:
  p=out/e['name'];assert p.stat().st_mtime_ns==1700001713000000000
  if e.get('storedBytes')is not None and os.name=='nt':
   assert p.stat().st_file_attributes&0x200
   high=ctypes.c_ulong();fn=ctypes.windll.kernel32.GetCompressedFileSizeW;fn.argtypes=[ctypes.c_wchar_p,ctypes.POINTER(ctypes.c_ulong)];fn.restype=ctypes.c_ulong;low=fn(str(p),ctypes.byref(high));assert low!=0xffffffff;allocated=(high.value<<32)|low
   if e['bytes']>=1048576:assert allocated<e['bytes']//2,(e,allocated)
   allocations.append({'name':e['name'],'logicalBytes':e['bytes'],'storedBytes':e['storedBytes'],'allocatedBytes':allocated})
 checks.append({'mode':r['mode'],'files':len(files),'directories':len(dirs),'exactLogicalHashesAndTimes':True,'allocations':allocations})
(base/'verified.json').write_text(json.dumps({'passed':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(f'{len(checks)} actual trees: exact logical SHA, paths, times and Windows sparse allocation verified independently.')

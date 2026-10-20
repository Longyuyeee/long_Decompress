import ctypes,hashlib,json,subprocess,time
from pathlib import Path
from ctypes import wintypes
base=Path('test-results/wim-lzms-1533/probe').resolve();base.mkdir(parents=True,exist_ok=True)
source=base/'source';source.mkdir(exist_ok=True);(source/'empty').mkdir(exist_ok=True)
(source/'说明.txt').write_text('LZMS 普通内容归档\n'*1000,encoding='utf-8');(source/'empty.txt').write_bytes(b'')
chunks=[]
for c in range(256):
 chunks.append(b''.join(hashlib.sha256(f'{c}:{n}'.encode()).digest() for n in range(512)));chunks.append(bytes([65+c%26])*16384)
(source/'mixed.bin').write_bytes(b''.join(chunks));(base/'capture.ini').write_bytes(b'[ExclusionList]\n')
class Memory(ctypes.Structure):
 _fields_=[('cb',wintypes.DWORD),('PageFaultCount',wintypes.DWORD)]+[(n,ctypes.c_size_t) for n in ['PeakWorkingSetSize','WorkingSetSize','QuotaPeakPagedPoolUsage','QuotaPagedPoolUsage','QuotaPeakNonPagedPoolUsage','QuotaNonPagedPoolUsage','PagefileUsage','PeakPagefileUsage']]
k=ctypes.WinDLL('kernel32',use_last_error=True);p=ctypes.WinDLL('psapi',use_last_error=True)
k.OpenProcess.argtypes=[wintypes.DWORD,wintypes.BOOL,wintypes.DWORD];k.OpenProcess.restype=wintypes.HANDLE;k.CloseHandle.argtypes=[wintypes.HANDLE]
p.GetProcessMemoryInfo.argtypes=[wintypes.HANDLE,ctypes.POINTER(Memory),wintypes.DWORD]
command=[str(Path('src-tauri/resources/wim-engine/wimlib-imagex.exe').resolve()),'capture',str(source),str(base/'actual.wim'),'Probe','--compress=LZMS','--chunk-size=32768','--threads=1','--no-acls','--check','--config='+str(base/'capture.ini')]
peak=private=0;start=time.perf_counter()
with (base/'capture.log').open('wb') as log:
 child=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,creationflags=0x08000000);handle=k.OpenProcess(0x410,False,child.pid)
 assert handle,ctypes.get_last_error()
 try:
  while True:
   m=Memory();m.cb=ctypes.sizeof(m)
   if p.GetProcessMemoryInfo(handle,ctypes.byref(m),m.cb): peak=max(peak,m.PeakWorkingSetSize);private=max(private,m.PeakPagefileUsage)
   if child.poll() is not None: break
   time.sleep(.01)
 finally: k.CloseHandle(handle)
 assert child.returncode==0,(base/'capture.log').read_text(encoding='utf-8',errors='replace')
elapsed=time.perf_counter()-start;data=(base/'actual.wim').read_bytes();flags=int.from_bytes(data[16:20],'little');assert flags&0x80000 and not flags&0x40000
assert int.from_bytes(data[20:24],'little')==32768
seven=str(Path('src-tauri/resources/archive-engine/7z.exe').resolve())
r=subprocess.run([seven,'x','-y',str(base/'actual.wim'),'-o'+str(base/'independent')],stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=0x08000000);assert r.returncode==0,r.stderr
sha=lambda d:{str(f.relative_to(d)).replace('\\','/'):hashlib.sha256(f.read_bytes()).hexdigest() for f in d.rglob('*') if f.is_file()}
assert sha(source)==sha(base/'independent');assert (base/'independent/empty').is_dir()
result=dict(method='LZMS',nonSolid=True,chunkSize=32768,threads=1,inputBytes=sum(f.stat().st_size for f in source.rglob('*') if f.is_file()),outputBytes=len(data),seconds=round(elapsed,3),sampledPeakWorkingSetBytes=peak,sampledPeakCommitBytes=private,headerFlags=hex(flags),independentHashes=True,emptyDirectory=True,scope='local synthetic probe, not product UI or general benchmark')
(base/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');print(json.dumps(result))

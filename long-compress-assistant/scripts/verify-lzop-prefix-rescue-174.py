"""Compare rescue outputs against a checked, independently decoded official LZOP original."""
from pathlib import Path
import subprocess,hashlib,json,tarfile,tempfile
root=Path(__file__).resolve().parents[1];base=root/'test-results/lzop-prefix-rescue-174';base.mkdir(parents=True,exist_ok=True)
fixture=root/'tests/fixtures/archives/lzop-prefix-174';manifest=json.loads((fixture/'manifest.json').read_text(encoding='utf-8'));archive=fixture/manifest['archive']
exe=root/'test-results/lzop-multi-create-172/research/lzop-linux';package=exe.parent/'lzop-1.03-i386_linux.tar.gz'
sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha(package.read_bytes())==manifest['packageSha256']
with tarfile.open(package) as tar:
 member=next(e for e in tar.getmembers() if e.isfile() and Path(e.name).name=='lzop');assert tar.extractfile(member).read()==exe.read_bytes()
assert sha(exe.read_bytes())==manifest['producerSha256'];assert sha(archive.read_bytes())==manifest['sha256']
def unix(p):
 s=str(p.resolve()).replace('\\','/');return '/mnt/'+s[0].lower()+s[2:]
previous=json.loads((base/'official.json').read_text(encoding='utf-8')) if (base/'official.json').exists() else {}
old=previous.get('officialSource',{})
reused=old.get('sha256')==manifest['sha256'] and old.get('producerSha256')==manifest['producerSha256'] and old.get('officialReadback') and old.get('entries')==manifest['entries']
if not reused:
 out=Path(tempfile.mkdtemp(prefix='official-readback-',dir=base))
 subprocess.run(['wsl','-d','Ubuntu','--exec',unix(exe),'-d','-N','-P','-k','-p'+unix(out),unix(archive)],capture_output=True,check=True)
 actual={p.relative_to(out).as_posix():sha(p.read_bytes()) for p in out.rglob('*') if p.is_file()}
 assert actual=={e['name']:e['sha256'] for e in manifest['entries']}
manifest['officialReadback']=True;manifest['archive']=str(archive.relative_to(root)).replace('\\','/')
checks=[]
for filename in ['native-outputs.json','desktop-outputs.json']:
 p=base/filename
 if not p.exists():continue
 for result in json.loads(p.read_text(encoding='utf-8')):
  expected={e['name']:e for e in manifest['entries'] if e['name'] in result['entries']};assert len(expected)==len(result['entries'])
  out=Path(result['outputDirectory']);actual={f.relative_to(out).as_posix():sha(f.read_bytes()) for f in out.rglob('*') if f.is_file()}
  assert actual=={name:e['sha256'] for name,e in expected.items()},(actual,expected)
  checks.append({'outputDirectory':str(out),'entries':list(expected),'exactNamesAndHashes':True})
(base/'official.json').write_text(json.dumps({'officialSource':manifest,'reusedOfficialReadback':bool(reused),'checks':checks,'passed':True},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'Official five-member original independently decoded (reused={bool(reused)}); {len(checks)} recovery results match its exact paths and contents.')

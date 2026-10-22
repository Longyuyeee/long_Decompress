"""New native-part update only; official frozen originals remain unchanged."""
from pathlib import Path
import hashlib, json, shutil, struct, subprocess, tempfile
root = Path(__file__).resolve().parents[1]
tool = root/'test-results/zpaq-volumes-1526/upstream/zpaq64.exe'
engine = root/'src-tauri/resources/zpaq-engine/long-zpaq.exe'
fixtures = root/'tests/fixtures/archives/zpaq-native-1564'
m = json.loads((fixtures/'manifest.json').read_text(encoding='utf-8'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(tool) == m['toolSha256']
def run(args, data=None, cwd=None):
    r = subprocess.run(list(map(str,args)),input=data,cwd=cwd,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    assert r.returncode == 0, (r.stdout.decode('utf-8','replace'),r.stderr.decode('utf-8','replace'))
    return r.stdout.decode('utf-8','replace').replace('\r\n','\n')
def string(s):
    b = str(s).encode('utf-8'); return struct.pack('<I',len(b))+b
def tree(p): return {f.relative_to(p).as_posix():sha(f) for f in p.rglob('*') if f.is_file()}
cases=[]
with tempfile.TemporaryDirectory(prefix='long-zpaq-161-probe-') as temp:
    work=Path(temp)
    added=work/'本次新增.txt'; added.write_bytes('这次只添加一个文件，旧条目和快照应保留。'.encode())
    for original in m['cases']:
        password=m['password'] if original['encrypted'] else None
        key=['-key',password] if password else []
        for adapter in [False,True]:
            folder=work/f"case-{original['encrypted']}-{adapter}"; folder.mkdir()
            before=[]
            for i,p in enumerate(original['parts'],1):
                src=fixtures/p['file']; assert sha(src)==p['sha256']
                out=folder/f'backup.part{i}.zpaq';shutil.copyfile(src,out);before.append(sha(out))
            pattern=folder/'backup.part*.zpaq'
            if adapter:
                data=(string(password) if password else b'\xff'*4)+struct.pack('<II',0,1)+string(added.as_posix())+string('资料/本次新增.txt')
                text=run([engine,'a',pattern,'-long-stdin','-long-update','-method','14','-threads','1','-noattributes'],data)
                summary=json.loads(text.split('\nLONG_ZPAQ_UPDATE\n')[1]);assert summary['removedEntries']==0
            else:
                # Official CLI adds a specifically selected file without synchronizing old directories.
                target=work/'资料';target.mkdir(exist_ok=True);shutil.copyfile(added,target/added.name)
                run([tool,'a',pattern,'资料/本次新增.txt','-method','14','-threads','1','-force',*key],cwd=work)
            parts=[folder/f'backup.part{i}.zpaq' for i in range(1,5)];assert all(p.is_file() for p in parts)
            assert [sha(p) for p in parts[:3]]==before
            for v in range(1,5):
                dest=folder/f'restore-{v}'
                run([tool,'x',pattern,'-until',v,'-to',dest,'-threads','1','-noattributes',*key])
                expected=dict(original['snapshots'][min(v,3)-1]['expected'])
                if v==4:expected['资料/本次新增.txt']=sha(added)
                assert tree(dest)==expected
            cases.append(dict(encrypted=bool(password),writer='adapter' if adapter else 'official',parts=[dict(bytes=p.stat().st_size,sha256=sha(p)) for p in parts],oldPartsUnchanged=True,independentSnapshots=[1,2,3,4]))
base=root/'test-results/zpaq-native-161';base.mkdir(parents=True,exist_ok=True)
(base/'engine-probe.json').write_bytes((json.dumps(dict(engineSha256=sha(engine),officialSha256=sha(tool),cases=cases),ensure_ascii=False,indent=2)+'\n').encode())
print(json.dumps(cases,ensure_ascii=False))

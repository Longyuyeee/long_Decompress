"""New update semantics only; frozen official originals are inputs, not re-run tests."""
from pathlib import Path
from datetime import datetime, timezone
import hashlib, json, os, shutil, struct, subprocess, tempfile

root = Path(__file__).resolve().parents[1]
base = root / 'test-results/zpaq-incremental-1536'
base.mkdir(parents=True, exist_ok=True)
tool = root / 'test-results/zpaq-volumes-1526/upstream/zpaq64.exe'
engine = root / 'src-tauri/resources/zpaq-engine/long-zpaq.exe'
sha = lambda b: hashlib.sha256(b).hexdigest()
assert sha(tool.read_bytes()) == '7a94b4f1d6323a758c7b0b6344036f166bff0fd44f1c3c86f05b3688023496cb'
fixtures = root / 'tests/fixtures/archives/zpaq-snapshots-1530'
manifest = json.loads((fixtures / 'manifest.json').read_text(encoding='utf-8'))
def string(value):
    data = str(value).encode('utf-8')
    return struct.pack('<I', len(data)) + data
def stdin(password, entries=()):
    return (string(password) if password else b'\xff'*4) + struct.pack('<I', 0) + (struct.pack('<I', len(entries)) + b''.join(string(str(p).replace('\\','/')) + string(n) for p,n in entries) if entries else b'')
def run(args, data=None):
    result = subprocess.run(list(map(str,args)),input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    assert result.returncode == 0, (result.stdout.decode('utf-8','replace'), result.stderr.decode('utf-8','replace'))
    return result.stdout.decode('utf-8','replace').replace('\r\n','\n')
def directory(file, password):
    text = run([engine,'l',file,'-long-stdin','-long-json'],stdin(password))
    return json.loads(text.split('\nLONG_ZPAQ_JSON\n')[1])
def tree(folder):
    return {p.relative_to(folder).as_posix():sha(p.read_bytes()) for p in folder.rglob('*') if p.is_file()}
cases = []
with tempfile.TemporaryDirectory(prefix='long-zpaq-1536-probe-') as temp:
    work = Path(temp)
    for c in manifest['cases']:
        original = fixtures / c['file']; before = original.read_bytes(); assert sha(before) == c['sha256']
        output = work / c['file']; shutil.copyfile(original,output)
        password = manifest['password'] if c['encrypted'] else None
        old = directory(output,password); expected = dict(c['snapshots'][-1]['expected'])
        entry = next(e for e in old['entries'] if e['path'] == '资料/说明.txt')
        same = work / ('same-key.txt' if password else 'same.txt'); same.write_bytes(b'X'*entry['size'])
        when = datetime.strptime(str(entry['date']),'%Y%m%d%H%M%S').replace(tzinfo=timezone.utc).timestamp(); os.utime(same,(when,when))
        added = work / ('added-key.txt' if password else 'added.txt'); added.write_text('本次增量新增。',encoding='utf-8')
        empty = work / ('empty-key' if password else 'empty'); empty.mkdir()
        mappings = [(same,'资料/说明.txt'),(added,'资料/本次新增.txt'),(str(empty)+'/', '资料/本次空目录/')]
        text = run([engine,'a',output,'-long-stdin','-long-update','-method','14','-threads','1','-noattributes'],stdin(password,mappings))
        result = json.loads(text.split('\nLONG_ZPAQ_UPDATE\n')[1]); assert result['removedEntries'] == 0
        assert output.read_bytes().startswith(before), 'Old transaction bytes must remain intact'
        new = directory(output,password); assert len(new['history']['snapshots']) == 4
        expected['资料/说明.txt'] = sha(same.read_bytes()); expected['资料/本次新增.txt'] = sha(added.read_bytes())
        key = ['-key',password] if password else []
        restored = work / ('independent-key' if password else 'independent')
        run([tool,'x',output,'-to',restored,'-threads','1','-noattributes',*key]); assert tree(restored) == expected
        assert (restored/'资料/本次空目录').is_dir()
        for snap in c['snapshots']:
            destination = work / f"old-{bool(password)}-{snap['version']}"
            run([tool,'x',output,'-until',snap['version'],'-to',destination,'-threads','1','-noattributes',*key])
            assert tree(destination) == snap['expected']
        changed = output.read_bytes()
        text = run([engine,'a',output,'-long-stdin','-long-update','-method','14','-threads','1','-noattributes'],stdin(password,mappings))
        unchanged = json.loads(text.split('\nLONG_ZPAQ_UPDATE\n')[1]); assert unchanged['updatedEntries'] == 0 and unchanged['newFragments'] == 0
        assert output.read_bytes() == changed, 'No-change update must not add an empty snapshot'
        assert original.read_bytes() == before
        cases.append(dict(encrypted=c['encrypted'],sameSizeAndDateChangeDetected=True,oldSnapshotCount=3,newSnapshotCount=4,unselectedPreserved=True,independentAllSnapshots=True,originalUnchanged=True,noChangeByteIdentical=True,summary=result))
base.joinpath('engine-probe.json').write_text(json.dumps(dict(engineSha256=sha(engine.read_bytes()),officialSha256=sha(tool.read_bytes()),cases=cases),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(cases,ensure_ascii=False))

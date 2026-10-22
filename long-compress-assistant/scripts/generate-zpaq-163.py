"""Controlled byte damage, not naturally damaged originals; official per-file oracle."""
from pathlib import Path
import hashlib,json,shutil,struct,subprocess,tempfile

root=Path(__file__).resolve().parents[1]
engine=root/'src-tauri/resources/zpaq-engine/long-zpaq.exe'
tool=root/'test-results/zpaq-volumes-1526/upstream/zpaq64.exe'
base=root/'tests/fixtures/archives/zpaq-native-1564'
output=root/'tests/fixtures/archives/zpaq-damaged-163'
m=json.loads((base/'manifest.json').read_text(encoding='utf-8'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(tool)==m['toolSha256']
output.mkdir(parents=True,exist_ok=True)
assert not (output/'manifest.json').exists(),'Use frozen damaged copies; do not regenerate silently'
def string(value):
    b=str(value).encode('utf-8');return struct.pack('<I',len(b))+b
def diagnose(parts,password,snapshot=None):
    data=(string(password) if password else b'\xff'*4)+struct.pack('<I',0)
    args=[str(engine),'l',str(parts[0]),'-long-stdin','-long-diagnose']
    if len(parts)>1:args+=['-long-volumes'];data+=struct.pack('<I',len(parts))+b''.join(string(p.as_posix()) for p in parts)
    if snapshot:args+=['-long-snapshot',str(snapshot['version']),str(snapshot['date'])]
    r=subprocess.run(args,input=data,capture_output=True);assert r.returncode==0,r.stderr.decode('utf-8','replace')
    return json.loads(r.stdout.decode('utf-8').replace('\r\n','\n').split('\nLONG_ZPAQ_BLOCKS\n')[1])
cases=[]
with tempfile.TemporaryDirectory(prefix='long-zpaq-163-official-') as temp:
    work=Path(temp)
    for source in m['cases']:
        password=m['password'] if source['encrypted'] else None
        originals=[base/p['file'] for p in source['parts']]
        for native in [False,True]:
            name=('key-' if password else 'plain-')+('native' if native else 'single')
            folder=output/name;folder.mkdir()
            if native:
                parts=[folder/f'backup.part{i}.zpaq' for i in range(1,4)]
                for src,dst in zip(originals,parts):shutil.copyfile(src,dst)
            else:
                parts=[folder/'backup.zpaq'];parts[0].write_bytes(b''.join(p.read_bytes() for p in originals))
            healthy=diagnose(parts,password);assert all(b['state']=='verified' for b in healthy['blocks'])
            target=max(healthy['blocks'],key=lambda b:b['offset']);offset=target['offset']+target['compressedSize']//2
            position=offset
            for part in parts:
                if position<part.stat().st_size:
                    with part.open('r+b') as f:
                        f.seek(position);old=f.read(1);f.seek(position);f.write(bytes([old[0]^0x40]))
                    mutation=dict(member=part.name,memberOffset=position,logicalOffset=offset,xor=64);break
                position-=part.stat().st_size
            damaged=diagnose(parts,password)
            affected=[e['path'] for e in damaged['entries'] if e['state']=='affected']
            assert len(affected)>=2 and any(e['state']=='verified' for e in damaged['entries'])
            assert [b['id'] for b in damaged['blocks'] if b['state']=='failed']==[target['id']]
            archive=folder/'backup.part*.zpaq' if native else parts[0]
            key=['-key',password] if password else []
            independent=[]
            for n,(entry,expected) in enumerate(source['snapshots'][-1]['expected'].items()):
                dest=work/f'{name}-{n}'
                r=subprocess.run([str(tool),'x',str(archive),'-only',entry,'-until','3','-to',str(dest),'-threads','1','-noattributes',*key],capture_output=True)
                file=dest/entry;passed=r.returncode==0 and file.is_file() and sha(file)==expected
                state=next(e['state'] for e in damaged['entries'] if e['path']==entry)
                assert state!='verified' or passed,(entry,state,r.stderr)
                independent.append(dict(path=entry,exitCode=r.returncode,completeHashMatched=passed,state=state))
                (folder/f'official-{n}.log').write_bytes((r.stdout+r.stderr).replace(b'\r\r\n',b'\n').replace(b'\r\n',b'\n'))
            assert {row['path'] for row in independent if not row['completeHashMatched']}==set(affected)
            old=diagnose(parts,password,source['snapshots'][0]);assert all(e['state']=='verified' for e in old['entries'])
            cases.append(dict(name=name,native=native,encrypted=bool(password),damage='Controlled XOR in compressed data payload; directory and transaction bytes preserved',mutation=mutation,targetBlock=target,parts=[dict(file=p.relative_to(output).as_posix(),size=p.stat().st_size,sha256=sha(p))for p in parts],snapshot=healthy['snapshot'],snapshots=source['snapshots'],affected=affected,independent=independent,olderSnapshotVerified=True))
(output/'manifest.json').write_text(json.dumps(dict(upstream=m['upstream'],packageUrl=m['packageUrl'],toolSha256=sha(tool),engineSha256=sha(engine),password=m['password'],cases=cases),ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps([dict(name=c['name'],affected=len(c['affected']),officialFiles=len(c['independent']))for c in cases]))

"""Check new recovered paths and contents against the independently decoded complete TAR."""
from pathlib import Path
import json,hashlib,tarfile
root=Path(__file__).resolve().parents[1];base=root/'test-results/tar-lzo-prefix-rescue-175'
oracle=json.loads((base/'official.json').read_text(encoding='utf-8'));assert oracle['passed'] and oracle['officialDecoded'] and oracle['independentTarListing']
sha=lambda data:hashlib.sha256(data).hexdigest()
archive=root/'tests/fixtures/archives/tar-lzo-prefix-175/complete.tar.lzo';assert sha(archive.read_bytes())==oracle['original']['sha256']
with tarfile.open(base/'official.tar') as tar:
    entries={h.name.rstrip('/'):{'isDirectory':h.isdir(),'sha256':sha(tar.extractfile(h).read() if h.isfile() else b'')} for h in tar}
assert entries=={e['name']:{'isDirectory':e['isDirectory'],'sha256':e['sha256']} for e in oracle['original']['entries']}
checks=[]
for filename in ['native-outputs.json','desktop-outputs.json']:
    file=base/filename
    if not file.exists():continue
    for result in json.loads(file.read_text(encoding='utf-8')):
        wanted={name:entries[name] for name in result['entries']};out=Path(result['outputDirectory'])
        actual={f.relative_to(out).as_posix():sha(f.read_bytes()) for f in out.rglob('*') if f.is_file()}
        assert actual=={n:e['sha256'] for n,e in wanted.items() if not e['isDirectory']}
        dirs={n for n,e in wanted.items() if e['isDirectory']}
        for n in wanted:
            dirs.update(p.as_posix() for p in Path(n).parents if p!=Path('.'))
        assert {p.relative_to(out).as_posix() for p in out.rglob('*') if p.is_dir()}==dirs
        checks.append({'outputDirectory':str(out),'entries':list(wanted),'exactNamesDirectoriesAndHashes':True})
oracle['recoveryChecks']=checks;(base/'official.json').write_text(json.dumps(oracle,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'{len(checks)} new results match the independent complete TAR: exact paths, folders, empty files and SHA-256.')

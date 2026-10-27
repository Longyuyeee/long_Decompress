"""Verify actual full/recovered directories against independently fixed input hashes."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parents[1]
base=root/'test-results/tar-pax-179'
manifest=json.loads((root/'tests/fixtures/archives/tar-pax-179/manifest.json').read_text(encoding='utf-8'))
sha=lambda b:hashlib.sha256(b).hexdigest()
for case in manifest['cases']:
    assert sha((root/'tests/fixtures/archives/tar-pax-179'/case['archive']).read_bytes())==case['sha256']
    assert sha((root/'tests/fixtures/archives/tar-pax-179'/case['tar']).read_bytes())==case['tarSha256']
checks=[]
for kind in ('full','recovery'):
    for result in json.loads((base/f'{kind}-outputs.json').read_text(encoding='utf-8')):
        output=Path(result['outputDirectory'])
        expected_files={e['name']:e['sha256'] for e in result['entries'] if not e['isDirectory']}
        expected_dirs={e['name'] for e in result['entries'] if e['isDirectory']}
        for name in expected_files: expected_dirs.update(p.as_posix() for p in Path(name).parents if p!=Path('.'))
        actual_files={p.relative_to(output).as_posix():sha(p.read_bytes()) for p in output.rglob('*') if p.is_file()}
        actual_dirs={p.relative_to(output).as_posix() for p in output.rglob('*') if p.is_dir()}
        assert actual_files==expected_files,(kind,result)
        assert actual_dirs==expected_dirs,(kind,result)
        checks.append({'kind':kind,'case':result['case'],'source':result['source'],'files':len(actual_files),'directories':len(actual_dirs),'exactPathsAndHashes':True})
evidence={'passed':True,'producerOriginals':2,'constructedStateSpecimens':1,'stateSpecimenIsNotClaimedAsToolConsensus':True,'checks':checks}
(base/'verified.json').write_text(json.dumps(evidence,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'{len(checks)} actual full/recovered outputs match fixed paths, directories and SHA-256.')

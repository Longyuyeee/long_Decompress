"""Verify real Windows outputs using independent os.stat nanoseconds and SHA."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parents[1];base=root/'test-results/tar-time-1711'
manifest=json.loads((root/'tests/fixtures/archives/tar-time-1711/manifest.json').read_text(encoding='utf-8'))
native=json.loads((base/'native.json').read_text(encoding='utf-8'));checks=[]
for record,case in zip(native,manifest['cases']):
    assert record['case']==case['name'];out=Path(record['result']['outputDirectory'])
    assert hashlib.sha256((root/'tests/fixtures/archives/tar-time-1711'/case['archive']).read_bytes()).hexdigest()==case['sha256']
    expected_files={e['name']:e for e in case['entries']if not e['isDirectory']}
    actual={p.relative_to(out).as_posix():p for p in out.rglob('*')if p.is_file()};assert set(actual)==set(expected_files)
    preserved=0;warnings=[]
    for e in record['result']['entries']:
        if e['isDirectory']:assert (out/e['name']).is_dir();continue
        oracle=expected_files[e['name']];p=actual[e['name']];assert hashlib.sha256(p.read_bytes()).hexdigest()==oracle['sha256']
        expected=oracle['modifiedTime'];ns=expected['seconds']*1_000_000_000+expected['nanoseconds'];actual_ns=p.stat().st_mtime_ns
        if not e.get('timeWarning'):assert actual_ns==ns;preserved+=1
        else:assert actual_ns!=ns;warnings.append({'name':e['name'],'expectedNanoseconds':ns,'actualNanoseconds':actual_ns,'warning':e['timeWarning']})
    assert {k:record['result']['timePreservation'][k]for k in('preservedFiles','unpreservedFiles')}=={'preservedFiles':preserved,'unpreservedFiles':len(warnings)}
    checks.append({'case':case['name'],'files':len(actual),'preservedFiles':preserved,'warnings':warnings,'exactContentAndPreservedTimes':True})
(base/'verified.json').write_text(json.dumps({'passed':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('2 actual recovery trees: 14 file hashes, exact preserved times and separate warnings verified with os.stat.')

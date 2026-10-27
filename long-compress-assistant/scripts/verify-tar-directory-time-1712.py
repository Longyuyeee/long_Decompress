"""Independent os.stat nanoseconds and content hashes on published outputs."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parents[1];base=root/'test-results/tar-directory-time-1712'
manifest=json.loads((root/'tests/fixtures/archives/tar-directory-time-1712/manifest.json').read_text(encoding='utf-8'));native=json.loads((base/'native.json').read_text(encoding='utf-8'));checks=[]
for record in native:
    case=next(c for c in manifest['cases']if c['name']==record['case']);oracle={e['name']:e for e in case['entries']};output=Path(record['result']['outputDirectory']);files=dirs=fw=dw=0
    assert hashlib.sha256((root/'tests/fixtures/archives/tar-directory-time-1712'/case['archive']).read_bytes()).hexdigest()==case['sha256']
    expected_files={e['name']:oracle[e['name']]['sha256']for e in record['entries']if not e['isDirectory']};expected_dirs=set()
    for e in record['entries']:
        if e['isDirectory']:expected_dirs.add(e['name'])
        expected_dirs.update(p.as_posix()for p in Path(e['name']).parents if p!=Path('.'))
    assert {p.relative_to(output).as_posix():hashlib.sha256(p.read_bytes()).hexdigest()for p in output.rglob('*')if p.is_file()}==expected_files
    assert {p.relative_to(output).as_posix()for p in output.rglob('*')if p.is_dir()}==expected_dirs
    for e in record['result']['entries']:
        original=oracle[e['name']];t=original['modifiedTime'];expected=t['seconds']*1_000_000_000+t['nanoseconds'];actual=(output/e['name']).stat().st_mtime_ns
        if not e.get('timeWarning'):
            assert actual==expected,(e,actual,expected)
            if e['isDirectory']:dirs+=1
            else:files+=1
        else:
            assert actual!=expected
            if e['isDirectory']:dw+=1
            else:fw+=1
    assert record['result']['timePreservation']=={'preservedFiles':files,'unpreservedFiles':fw,'preservedDirectories':dirs,'unpreservedDirectories':dw}
    if record['mode']!='full':assert not any(e['name']=='资料'for e in record['result']['entries'])and (output/'资料').stat().st_mtime_ns!=1700001712125000000
    checks.append({'case':case['name'],'mode':record['mode'],'files':files,'preservedDirectories':dirs,'directoryWarnings':dw,'exactPathsContentsAndRecordedTimes':True,'implicitParentsNotGivenOriginalTimes':True})
(base/'verified.json').write_text(json.dumps({'passed':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(f'{len(checks)} actual trees: contents, explicit directory times, independent warnings and implicit parent scope verified.')

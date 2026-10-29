"""Independent original/content hashes and external readback of rescued ZIPs."""
from pathlib import Path
import datetime as dt
import struct
import hashlib
import json
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/zip-zstd-multiframe-1737'
FIXTURES = ROOT / 'tests/fixtures/archives/zip-zstd-multiframe-1737'
SEVEN = ROOT / 'src-tauri/resources/archive-engine/7z.exe'
load = lambda p: json.loads(p.read_text(encoding='utf-8'))
sha = lambda b: hashlib.sha256(b).hexdigest()
producer = load(FIXTURES / 'manifest.json')


def external(archive, expected, label):
    destination = BASE / ('external-' + label)
    destination.mkdir(exist_ok=True)
    result = subprocess.run([str(SEVEN), 'x', '-y', '-bd', '-o' + str(destination), str(archive)], capture_output=True)
    assert result.returncode == 0, (archive, result.stdout.decode(errors='replace'), result.stderr.decode(errors='replace'))
    actual = {p.relative_to(destination).as_posix(): p for p in destination.rglob('*')}
    assert set(actual) == {e['name'].rstrip('/') for e in expected}, actual
    for entry in expected:
        path = actual[entry['name'].rstrip('/')]
        assert path.is_dir() == entry['directory'], path
        if not entry['directory']:
            content = path.read_bytes()
            assert len(content) == entry['size'] and sha(content) == entry['sha256'], path
        assert path.stat().st_mtime_ns == seconds(entry) * 10**9, path


def seconds(entry):
    return entry['utc'] if entry['utc'] is not None else int(dt.datetime(*entry['dos']).timestamp())


def utc_fields(raw):
    fields=[]
    while raw:
        kind,length=struct.unpack_from('<HH',raw)
        assert len(raw)>=4+length
        if kind==0x5455: fields.append(raw[4:4+length])
        raw=raw[4+length:]
    return fields


source_frames = set()
for source in producer['sources']:
    path=FIXTURES/source['path'];raw=path.read_bytes()
    assert sha(raw)==source['sha256']
    with zipfile.ZipFile(path) as archive:
        for info in archive.infolist():
            if info.compress_type!=93:continue
            n,x=struct.unpack_from('<HH',raw,info.header_offset+26)
            at=info.header_offset+30+n+x
            source_frames.add(sha(raw[at:at+info.compress_size]))
for variant in producer['cases']:
    path=FIXTURES/variant['name'];raw=path.read_bytes()
    assert sha(raw)==variant['sha256'] and sha(raw[:variant['central']])==variant['lostSha256']
    with zipfile.ZipFile(path) as archive:
        assert len(archive.infolist())==len(variant['entries'])
        for entry in variant['entries']:
            member=archive.getinfo(entry['name'])
            assert member.compress_type==entry['method'] and member.CRC==entry['crc']
            assert member.compress_size==entry['compressed'] and member.file_size==entry['size']
            if member.compress_type==93:
                assert sum(f['length'] for f in entry['frames'])==entry['compressed']
                for frame in entry['frames']:
                    assert sha(raw[frame['data']:frame['data']+frame['length']]) in source_frames
    external(path,variant['entries'],'control-'+variant['name'])
assert producer['cases'][-1]['entries'][0]['crc']==0xfd2fb528
cases = load(BASE / 'native.json') + load(BASE / 'numeric.json') + load(BASE / 'stops.json') + [load(BASE / 'retry.json')]
checked = []
for index, case in enumerate(cases):
    result = case['result']
    assert result['verified'] and result['partial'] and result['directoryRebuilt'] and not result['outputEncrypted']
    assert any(e['name'] is None and e['status'] == 'unknown' for e in result['entryResults'])
    if 'diagnosis' in case:
        report = case['diagnosis']
        assert report['status'] == 'directory_lost' and report['canRepair'] and not report['integrityTested']
        assert not report['totalUncompressedSizeKnown']
    for source in case['sources']:
        assert sha(Path(source['path']).read_bytes()) == source['sha256']
    archive_path = Path(case['output'])
    expected = case['expected']
    assert result['recoveredFiles'] == sum(not e['directory'] for e in expected)
    assert result['recoveredDirectories'] == sum(e['directory'] for e in expected)
    assert set(result['recoveredEntries']) == {e['name'] for e in expected if not e['directory']}
    raw=archive_path.read_bytes()
    with zipfile.ZipFile(archive_path) as archive:
        assert archive.testzip() is None
        assert set(archive.namelist()) == {e['name'] for e in expected}
        for entry in expected:
            member = archive.getinfo(entry['name'])
            assert member.is_dir() == entry['directory']
            assert list(member.date_time)==entry['dos']
            clock,date=struct.unpack_from('<HH',raw,member.header_offset+10)
            assert [1980+(date>>9),(date>>5)&15,date&31,clock>>11,(clock>>5)&63,(clock&31)*2]==entry['dos']
            n,x=struct.unpack_from('<HH',raw,member.header_offset+26)
            local=raw[member.header_offset+30+n:member.header_offset+30+n+x]
            expected_time=[] if entry['utc'] is None else [b'\x01'+struct.pack('<I',entry['utc'])]
            assert utc_fields(local)==utc_fields(member.extra)==expected_time
            content = archive.read(member)
            assert len(content) == entry['size'] and sha(content) == entry['sha256']
    destination=Path(case['nativeDestination'])
    actual={p.relative_to(destination).as_posix():p for p in destination.rglob('*')}
    assert set(actual)=={e['name'].rstrip('/') for e in expected}
    for e in expected:
        path=actual[e['name'].rstrip('/')]
        assert path.is_dir()==e['directory']
        if not e['directory']: assert len(path.read_bytes())==e['size'] and sha(path.read_bytes())==e['sha256']
        assert path.stat().st_mtime_ns==seconds(e)*10**9,path
    external(archive_path, expected, str(index))
    checked.append({'variant': case['variant'], 'entries': len(expected), 'files': result['recoveredFiles'],
                    'partial': True, 'unknownTotalRetained': True, 'originalUnchanged': True, 'externalReadback': True, 'nativeDatesAndContent': True})
failures = load(BASE / 'failures.json')
assert all(c['noNewOutput'] and c['originalUnchanged'] and c['stagingCleaned'] for c in failures)
assert all(c['missingMiddleRejected'] for c in load(BASE / 'numeric.json'))
receipt = {'version': '1.7.37', 'passed': True, 'frozenSources': len(producer['sources']), 'structuralControls': len(producer['cases']),
           'recoveredArchives': len(cases), 'recoveredEntries': sum(c['entries'] for c in checked),
           'externalReadbacks': len(cases) + len(producer['cases']), 'controlledStops': len(load(BASE / 'stops.json')),
           'noPublicationCases': len(failures), 'checks': checked}
(BASE / 'verified.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: v for k, v in receipt.items() if k != 'checks'}))

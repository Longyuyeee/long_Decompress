"""Independent source epochs, Python ZIP headers and 7-Zip final-file checks."""
from pathlib import Path
import datetime as dt
import gzip
import hashlib
import json
import struct
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/zip-utc-read-1731'
FIXTURES = ROOT / 'tests/fixtures/archives/zip-utc-read-1731'
SEVEN = ROOT / 'src-tauri/resources/archive-engine/7z.exe'
sha = lambda b: hashlib.sha256(b).hexdigest()
load = lambda p: json.loads(p.read_text(encoding='utf-8'))
manifest = load(FIXTURES / 'manifest.json')
assert load(BASE / 'producer.json')['passed']


def fields(raw):
    result = []
    while raw:
        kind, size = struct.unpack_from('<HH', raw)
        assert len(raw) >= 4 + size
        result.append((kind, raw[4:4+size]))
        raw = raw[4+size:]
    return result


def dos_seconds(entry):
    return int(dt.datetime(*entry['dos']).timestamp())


def exact(folder, entries, fallback=False):
    actual = {p.relative_to(folder).as_posix(): p for p in folder.rglob('*')}
    assert set(actual) == {e['name'] for e in entries}, ('exact tree', folder)
    for entry in entries:
        path = actual[entry['name']]
        assert path.is_dir() == entry.get('directory', False), path
        expected = dos_seconds(entry) if fallback or entry['seconds'] is None else entry['seconds']
        assert path.stat().st_mtime_ns == expected * 1_000_000_000, ('mtime', path, expected, path.stat().st_mtime_ns)
        if not path.is_dir():
            assert path.stat().st_size == entry['bytes'] and sha(path.read_bytes()) == entry['sha256'], path


def external(input_path, target, encrypted=False, metadata_control=False):
    result = subprocess.run([str(SEVEN), 'x', '-y', '-sccUTF-8', '-pUtcRead1731!' if encrypted else '-p',
                             '-o'+str(target), str(input_path)], capture_output=True)
    assert result.returncode == 0 or metadata_control and result.returncode == 1, (result.stdout, result.stderr)


for variant in manifest['originals']:
    source = FIXTURES / variant['archive']
    assert sha(source.read_bytes()) == variant['sha256']
    expected = {e['name']: e for e in variant['entries']}
    with zipfile.ZipFile(source) as archive:
        for info in archive.infolist():
            entry = expected[info.filename.rstrip('/')]
            utc = [body for kind, body in fields(info.extra) if kind == 0x5455]
            assert len(utc) == 1 and len(utc[0]) == 5 and utc[0][0] & 1
            assert struct.unpack_from('<i', utc[0], 1)[0] == entry['seconds']
            assert list(info.date_time) == entry['dos']
            if not variant['encrypted']:
                assert sha(archive.read(info)) == entry['sha256']

checks = []
for case in load(BASE / 'native.json'):
    source, input_path = Path(case['source']), Path(case['input'])
    assert sha(source.read_bytes()) == case['sourceSha256']
    exact(Path(case['output']), case['entries'])
    if case['numeric']:
        joined = input_path.parent / 'oracle.zip'
        joined.write_bytes(b''.join(Path(p).read_bytes() for p in case['parts']))
        assert joined.read_bytes() == source.read_bytes()
        input_path = joined
    out = Path(case['output']).parent / 'external'
    external(input_path, out, case['encrypted'])
    exact(out, case['entries'])
    for actual in case['listing']['entries']:
        expected = next(e for e in case['entries'] if e['name'] == actual['path'].rstrip('/'))
        assert int(dt.datetime.fromisoformat(actual['modified']).timestamp()) == expected['seconds']
    checks.append(dict(kind='real-bsdtar', numeric=case['numeric'], standard=case['standard'],
                       encrypted=case['encrypted'], entries=len(case['entries']), exactTreeContentAndUtc=True))

for case in load(BASE / 'controls.json'):
    assert sha(Path(case['source']).read_bytes()) == case['sourceSha256']
    exact(Path(case['output']), case['entries'], case['fallbackAll'])
    for actual in case['listing']['entries']:
        entry = next(e for e in case['entries'] if e['name'] == actual['path'].rstrip('/'))
        expected = dos_seconds(entry) if case['fallbackAll'] or entry['seconds'] is None else entry['seconds']
        assert int(dt.datetime.fromisoformat(actual['modified']).timestamp()) == expected
    # Other tools may prefer local or NTFS time for these deliberately
    # conflicting/invalid metadata controls. Content remains independently read.
    out = Path(case['output']).parent / ('external-' + Path(case['source']).stem)
    external(Path(case['source']), out, case['encrypted'], metadata_control=True)
    for e in case['entries']:
        p = out / e['name']
        if not e.get('directory', False): assert sha(p.read_bytes()) == e['sha256']
    checks.append(dict(kind='metadata-controls', entries=len(case['entries']),
                       declaredUtcOrDosPolicy=True, independentContent=True))

case = load(BASE / 'choices.json')
assert case['workerThreads'] > 1
exact(Path(case['parallelOutput']), case['parallelEntries'])
for choice in case['choices']:
    path = Path(choice['target'])
    assert sha(path.read_bytes()) == choice['sha256']
    assert path.stat().st_mtime_ns == choice['seconds'] * 1_000_000_000 or choice['mode'] == 'dates-off'
    if choice['mode'] != 'dates-off': assert choice['seconds'] == 1700001731
checks.append(dict(kind='parallel-and-choices', actualWorkers=case['workerThreads'], choices=len(case['choices'])))

case = load(BASE / 'zipx.json')
source = Path(case['source'])
assert sha(source.read_bytes()) == case['sourceSha256']
with zipfile.ZipFile(source) as archive:
    info = archive.infolist()[0]
    extra = dict(fields(info.extra))
    assert struct.unpack_from('<H', extra[0x9901], 5)[0] == 95  # XZ behind AES
    assert struct.unpack_from('<i', extra[0x5455], 1)[0] == case['seconds'] == 1700001731
    assert info.date_time == (2001,2,3,4,5,6)
out = source.parent / 'external'
external(source, out, True)
for folder in [out, Path(case['output'])]:
    assert (folder/'note.txt').read_bytes() == b'ZIPX XZ UTC source 1731\n'
    assert (folder/'note.txt').stat().st_mtime_ns == 1700001731 * 1_000_000_000
checks.append(dict(kind='controlled-zipx-xz-aes', contentAndUtc=True))

for case in load(BASE / 'conversion.json'):
    assert sha(Path(case['source']).read_bytes()) == case['sourceSha256']
    exact(Path(case['inputContents']), case['entries'])
    source = Path(case['output'])
    if case['format'] == 'tar.gz':
        inner = source.parent / 'oracle.tar'
        inner.write_bytes(gzip.decompress(source.read_bytes()))
        source = inner
    out = source.parent / ('external-converted-' + case['format'])
    external(source, out, case['encrypted'])
    exact(out, case['entries'])
    checks.append(dict(kind='conversion', format=case['format'], entries=len(case['entries']), contentAndUtc=True))

case = load(BASE / 'consistency.json')
assert sha(Path(case['source']).read_bytes()) == case['sourceSha256']
folder = Path(case['passwordFallbackOutput'])
for e in case['entries']:
    if not e['directory']:
        p = folder / e['name']
        assert sha(p.read_bytes()) == e['sha256'] and p.stat().st_mtime_ns == e['seconds'] * 1_000_000_000
for e in manifest['originals'][0]['entries']:
    actual = next(a for a in case['comparison']['entries'] if a['path'] == e['name'])
    assert actual['modified'] == dt.datetime.fromtimestamp(e['seconds']).strftime('%Y-%m-%d %H:%M:%S')
checks.append(dict(kind='password-fallback-and-comparison', contentAndUtc=True, localComparisonCalendar=True))

receipt = dict(version='1.7.31', passed=True, producer=manifest['producer'], realOriginals=3,
               readbackOutputs=17, checks=checks, externalReader='7-Zip 26.04', externalReaderSha256=sha(SEVEN.read_bytes()))
(BASE/'verified.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:receipt[k]for k in ('passed','realOriginals','readbackOutputs')}))

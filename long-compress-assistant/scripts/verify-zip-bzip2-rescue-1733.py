"""Independent original/content hashes and external readback of rescued ZIPs."""
from pathlib import Path
import hashlib
import json
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/zip-bzip2-rescue-1733'
FIXTURES = ROOT / 'tests/fixtures/archives/zip-bzip2-rescue-1733'
SEVEN = ROOT / 'src-tauri/resources/archive-engine/7z.exe'
load = lambda p: json.loads(p.read_text(encoding='utf-8'))
sha = lambda b: hashlib.sha256(b).hexdigest()
producer = load(FIXTURES / 'producer.json')


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


for original in producer['originals']:
    path = FIXTURES / original['name']
    assert sha(path.read_bytes()) == original['sha256']
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None
        assert len(archive.infolist()) == len(original['entries'])
        for entry in original['entries']:
            member = archive.getinfo(entry['name'])
            assert member.flag_bits & 8 and member.compress_type == entry['method']
            assert sha(archive.read(member)) == entry['sha256']
    external(path, original['entries'], original['name'])
for control in producer['controls']:
    assert sha((FIXTURES / control['name']).read_bytes()) == control['sha256']

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
    with zipfile.ZipFile(archive_path) as archive:
        assert archive.testzip() is None
        assert set(archive.namelist()) == {e['name'] for e in expected}
        for entry in expected:
            member = archive.getinfo(entry['name'])
            assert member.is_dir() == entry['directory']
            content = archive.read(member)
            assert len(content) == entry['size'] and sha(content) == entry['sha256']
    external(archive_path, expected, str(index))
    checked.append({'variant': case['variant'], 'entries': len(expected), 'files': result['recoveredFiles'],
                    'partial': True, 'unknownTotalRetained': True, 'originalUnchanged': True, 'externalReadback': True})
failures = load(BASE / 'failures.json')
assert all(c['noNewOutput'] and c['originalUnchanged'] and c['stagingCleaned'] for c in failures)
assert all(c['missingMiddleRejected'] for c in load(BASE / 'numeric.json'))
receipt = {'version': '1.7.33', 'passed': True, 'independentOriginals': len(producer['originals']),
           'recoveredArchives': len(cases), 'recoveredEntries': sum(c['entries'] for c in checked),
           'externalReadbacks': len(cases) + len(producer['originals']), 'controlledStops': len(load(BASE / 'stops.json')),
           'noPublicationCases': len(failures), 'checks': checked}
(BASE / 'verified.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: v for k, v in receipt.items() if k != 'checks'}))

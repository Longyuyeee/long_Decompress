"""Read fresh 7Z outputs with external 7-Zip, then compare native final results.

Run after `cargo test --lib sevenzdir1727`; no desktop control or tool install.
"""
import datetime
import hashlib
import json
import subprocess
import tarfile
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/seven-zip-directory-layout-1727'
ENGINE = ROOT / 'src-tauri/resources/archive-engine/7z.exe'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def run(*args):
    result = subprocess.run([str(ENGINE), *map(str, args)], capture_output=True)
    assert result.returncode == 0, (args, result.stderr.decode('utf-8', errors='replace'))
    return result.stdout.decode('utf-8')


def records_tar(path):
    result = {}
    with tarfile.open(path) as archive:
        for item in archive:
            result[item.name.rstrip('/')] = dict(directory=item.isdir(), seconds=item.mtime,
                nanoseconds=0, sha256=None if item.isdir() else sha(archive.extractfile(item).read()))
    return result


def tree(path, expected):
    actual = {p.relative_to(path).as_posix(): p for p in path.rglob('*')}
    assert set(actual) == set(expected), ('exact tree', str(path), sorted(actual), sorted(expected))
    for name, source in expected.items():
        target = actual[name]
        assert target.is_dir() == source['directory'], name
        assert target.stat().st_mtime_ns == source['seconds'] * 1_000_000_000 + source['nanoseconds'], ('date', name, target.stat().st_mtime_ns, source)
        if not source['directory']:
            assert sha(target.read_bytes()) == source['sha256'], ('content', name)


def listing_date(value):
    whole, _, fraction = value.partition('.')
    local = datetime.datetime.strptime(whole, '%Y-%m-%d %H:%M:%S')
    return int(time.mktime(local.timetuple())) * 1_000_000_000 + int(fraction.ljust(9, '0') or '0')


cases = load(BASE / 'native.json') + load(BASE / 'conversion.json')
checks = []
encoded = 0
for index, case in enumerate(cases):
    expected = {e['name']: {key: e[key] for key in ('directory', 'seconds', 'nanoseconds', 'sha256')}
                for e in case['sources']}
    combined = b''.join(Path(part).read_bytes() for part in case['parts'])
    joined = BASE / f'independent-{index}.7z'
    joined.write_bytes(combined)
    password = '-p' + (case['password'] or '-')
    listing = run('l', '-slt', '-sccUTF-8', password, joined)
    entries = {}
    for block in listing.replace('\r\n', '\n').split('\n\n'):
        fields = dict(line.split(' = ', 1) for line in block.splitlines() if ' = ' in line)
        # 7-Zip does not emit Folder/Attributes for these native 7Z headers.
        # Entry blocks expose Encrypted; directory type is independently
        # verified by the external extraction below, including empty folders.
        if 'Path' in fields and 'Encrypted' in fields:
            entries[fields['Path'].replace('\\', '/').rstrip('/')] = fields
    assert set(entries) == set(expected), ('encoded names', case['name'])
    for name, source in expected.items():
        assert listing_date(entries[name]['Modified']) == source['seconds'] * 1_000_000_000 + source['nanoseconds'], ('encoded date', name)
    external = Path(tempfile.mkdtemp(prefix=f'external-{index}-', dir=BASE))
    run('x', '-y', '-sccUTF-8', password, '-o' + str(external), joined)
    tree(external, expected)
    tree(Path(case['restored']), expected)
    if 'sourceTar' in case:
        source = Path(case['sourceTar'])
        fixture = next(v for v in load(source.parent / 'manifest.json')['variants'] if v['tar'] == source.name)
        assert sha(source.read_bytes()) == fixture['tarSha256']
        assert sha(source.with_suffix('.tar.lzo').read_bytes()) == fixture['sha256'] == case['sourceSha256']
        assert records_tar(source) == expected, 'conversion changed logical contents/dates'
    else:
        for source in case['sources']:
            path = Path(source['path'])
            assert path.stat().st_mtime_ns == source['seconds'] * 1_000_000_000 + source['nanoseconds'], 'source date changed'
            if not source['directory']:
                assert sha(path.read_bytes()) == source['sha256'], 'source content changed'
    encoded += len(expected)
    checks.append(dict(name=case['name'], input=case['input'], archiveSha256=sha(combined),
                       volumes=len(case['parts']), entries=len(expected),
                       explicitDirectories=sum(e['directory'] for e in expected.values()),
                       encodedDates=True, externalAndNativeExactTreesContentDates=True, sourceUnchanged=True))

selection = load(BASE / 'selection.json')
assert [(c['mode'], len(c['times'])) for c in selection] == [
    ('directory', 2), ('file', 0), ('filter', 2), ('flat', 0), ('off', 0)]
for case in selection:
    for name, date in case['times']:
        assert (Path(case['restored']) / name).stat().st_mtime_ns == date['seconds'] * 1_000_000_000 + date['nanoseconds']
flat = Path(next(c for c in selection if c['mode'] == 'flat')['restored'])
assert (flat / 'note.txt').read_bytes() == b'a' and not (flat / '资料').exists()

receipt = dict(version='1.7.27', passed=True, archives=len(cases), encodedEntries=encoded,
               externalReader=run('i').splitlines()[1], checks=checks,
               selectionModes=[c['mode'] for c in selection])
(BASE / 'verified.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(dict(passed=True, archives=len(cases), encodedEntries=encoded)))

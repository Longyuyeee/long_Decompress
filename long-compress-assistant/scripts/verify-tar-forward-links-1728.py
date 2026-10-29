"""Independent GNU/Python TAR oracle and final filesystem checks for v1.7.28.

Originals come from the separately recorded GNU create/delete/concatenate
recipe. Decode each original once with official lzop, then verify every native
full/selected output and its numeric read source, without project readers.
"""
import hashlib
import json
import subprocess
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/tar-forward-links-1728'
FIXTURES = ROOT / 'tests/fixtures/archives/tar-forward-links-1728'
OFFICIAL = ROOT / 'test-results/lzop-read-convert-1657/official-lzop/lzop103w/lzop.exe'
PACKAGE = ROOT / 'test-results/lzop-read-convert-1657/lzop103w.zip'
sha = lambda data: hashlib.sha256(data).hexdigest()
load = lambda path: json.loads(path.read_text(encoding='utf-8'))

assert sha(PACKAGE.read_bytes()) == '0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a'
with zipfile.ZipFile(PACKAGE) as package:
    assert package.read('lzop103w/lzop.exe') == OFFICIAL.read_bytes()
producer = load(BASE / 'producer.json')
assert producer['passed']
manifest = load(FIXTURES / 'manifest.json')
oracles = {}
encoded = 0
for variant in manifest['variants']:
    source = FIXTURES / variant['archive']
    raw = (FIXTURES / variant['tar']).read_bytes()
    assert sha(raw) == variant['tarSha256']
    assert sha(source.read_bytes()) == variant['sha256']
    external = subprocess.run([str(OFFICIAL), '-d', '-c', str(source)], capture_output=True, check=True)
    assert external.stdout == raw
    expected = {e['name']: e for e in variant['entries']}
    oracle = {}
    with tarfile.open(FIXTURES / variant['tar']) as archive:
        members = list(archive)
        by_name = {m.name.rstrip('/'): m for m in members}
        assert len(by_name) == len(members)
        for index, member in enumerate(members):
            name = member.name.rstrip('/')
            target = by_name[member.linkname] if member.islnk() else member
            assert target.isreg() or target.isdir()
            data = b'' if member.isdir() else archive.extractfile(target).read()
            e = expected[name]
            assert member.isdir() == e['isDirectory']
            assert sha(data) == e['sha256'] and len(data) == e['bytes']
            assert int(member.mtime) == e['seconds']
            assert (member.linkname if member.islnk() else None) == e['linkTarget']
            if member.islnk():
                assert members.index(target) == e['targetId']
            oracle[name] = dict(directory=member.isdir(), seconds=int(member.mtime), sha256=sha(data))
    assert set(oracle) == set(expected)
    oracles[variant['format']] = oracle
    encoded += len(oracle)

checks = []
files = 0
entries = 0
for case in load(BASE / 'native.json'):
    original = Path(case['source'])
    assert sha(original.read_bytes()) == case['sourceSha256']
    read_source = Path(case['readSource'])
    if read_source != original:
        stem = read_source.name.rsplit('.', 1)[0]
        parts = sorted(read_source.parent.glob(stem + '.[0-9][0-9][0-9]'))
        assert b''.join(part.read_bytes() for part in parts) == original.read_bytes()
    expected = {e['name']: oracles[case['format']][e['name']] for e in case['entries']}
    target = Path(case['outputDirectory'])
    names = set(expected)
    for name in expected:
        names.update(p.as_posix() for p in Path(name).parents if p != Path('.'))
    actual = {p.relative_to(target).as_posix(): p for p in target.rglob('*')}
    assert set(actual) == names, ('exact output tree', case['format'], case['mode'])
    for name, e in expected.items():
        path = actual[name]
        assert path.is_dir() == e['directory'], name
        assert path.stat().st_mtime_ns == e['seconds'] * 1_000_000_000, ('own encoded date', name)
        if not e['directory']:
            assert sha(path.read_bytes()) == e['sha256'], ('content', name)
            files += 1
    entries += len(expected)
    checks.append(dict(format=case['format'], mode=case['mode'], outputDirectory=str(target),
                       entries=len(expected), exactTree=True, contentAndOwnDates=True, sourceUnchanged=True))

receipt = dict(version='1.7.28', passed=True, originals=len(oracles), encodedEntries=encoded,
               outputs=len(checks), outputEntries=entries, outputFiles=files,
               officialLzopSha256=sha(OFFICIAL.read_bytes()),
               producer=manifest['producer'], oracle=manifest['oracle'], checks=checks)
(BASE / 'verified.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: receipt[k] for k in ('passed', 'originals', 'encodedEntries', 'outputs', 'outputEntries', 'outputFiles')}))

"""Independent TAR/7-Zip oracle for the v1.7.29 forward-content conversions.

Uses the frozen GNU tar originals from v1.7.28, not project archive readers.
The original forward alias is compared with its direct target's logical bytes,
while its modification date comes from its own source header.
"""
import gzip
import hashlib
import json
import subprocess
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/tar-forward-convert-1729'
FIXTURES = ROOT / 'tests/fixtures/archives/tar-forward-links-1728'
SEVEN = ROOT / 'src-tauri/resources/archive-engine/7z.exe'
sha = lambda data: hashlib.sha256(data).hexdigest()
load = lambda path: json.loads(path.read_text(encoding='utf-8'))
manifest = load(FIXTURES / 'manifest.json')
oracles = {}
for variant in manifest['variants']:
    assert sha((FIXTURES / variant['tar']).read_bytes()) == variant['tarSha256']
    assert sha((FIXTURES / variant['archive']).read_bytes()) == variant['sha256']
    recorded = {e['name']: e for e in variant['entries']}
    expected = {}
    with tarfile.open(FIXTURES / variant['tar']) as archive:
        members = list(archive)
        by_name = {m.name.rstrip('/'): m for m in members}
        assert len(members) == len(by_name)
        for member in members:
            name = member.name.rstrip('/')
            target = by_name[member.linkname] if member.islnk() else member
            assert target.isreg() or target.isdir()
            data = b'' if member.isdir() else archive.extractfile(target).read()
            e = dict(directory=member.isdir(), bytes=len(data), sha256=sha(data), seconds=int(member.mtime), linkTarget=member.linkname if member.islnk() else None)
            assert e['directory'] == recorded[name]['isDirectory']
            for field in ('bytes', 'sha256', 'seconds'):
                assert e[field] == recorded[name][field], (name, field)
            expected[name] = e
    oracles[variant['format']] = expected


def exact(folder, expected):
    actual = {p.relative_to(folder).as_posix(): p for p in folder.rglob('*')}
    assert set(actual) == set(expected), ('exact tree', folder)
    for name, e in expected.items():
        path = actual[name]
        assert path.is_dir() == e['directory'], name
        assert path.stat().st_mtime_ns == e['seconds'] * 1_000_000_000, ('own date', name, path)
        if not e['directory']:
            assert path.stat().st_size == e['bytes'], ('size', name)
            assert sha(path.read_bytes()) == e['sha256'], ('content', name)
            if e['linkTarget']:
                assert not path.samefile(folder / e['linkTarget']), ('independent alias', name)


checks = []
for case in load(BASE / 'native.json'):
    expected = oracles[case['variant']]
    source = Path(case['source'])
    assert sha(source.read_bytes()) == case['sourceSha256']
    if case['inputParts']:
        assert b''.join(Path(p).read_bytes() for p in case['inputParts']) == source.read_bytes()
        assert case['readSource'] == case['inputParts'][-1]
    exact(Path(case['nativeRestored']), expected)
    receipt = case['output']
    for part in receipt['files']:
        assert Path(part['path']).stat().st_size == part['bytes'] > 0
    primary = Path(receipt['primaryPath'])
    work = primary.parent
    external_input = primary
    if case['split'] and not case['standard']:
        external_input = work / ('oracle.' + case['format'])
        with external_input.open('wb') as joined:
            for part in receipt['files']:
                joined.write(Path(part['path']).read_bytes())
    if case['format'] == 'tar.gz':
        external_input = work / 'oracle.tar'
        external_input.write_bytes(gzip.decompress(primary.read_bytes()))
    target = work / 'external-restored'
    args = [str(SEVEN), 'x', '-y', '-sccUTF-8', '-pForwardConvert1729!' if case['encrypted'] else '-p', '-o' + str(target), str(external_input)]
    result = subprocess.run(args, capture_output=True)
    assert result.returncode == 0, (args, result.stdout.decode('utf-8', 'replace'), result.stderr.decode('utf-8', 'replace'))
    exact(target, expected)
    assert case['privateFilesCleaned'] and not list(work.glob('.long-*'))
    checks.append(dict(variant=case['variant'], format=case['format'], encrypted=case['encrypted'],
                       split=case['split'], standard=case['standard'], numericSource=bool(case['inputParts']),
                       parts=len(receipt['files']), entries=len(expected), files=sum(not e['directory'] for e in expected.values()),
                       nativeAndExternalReadback=True, ownDates=True, originalUnchanged=True,
                       independentCopies=True, privateFilesCleaned=True))
receipt = dict(version='1.7.29', passed=True, producer=manifest['producer'], originalGroups=len(oracles),
               outputs=len(checks), outputEntries=sum(c['entries'] for c in checks), outputFiles=sum(c['files'] for c in checks),
               externalReader='7-Zip 26.04', externalReaderSha256=sha(SEVEN.read_bytes()), checks=checks)
(BASE / 'verified.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: receipt[k] for k in ('passed', 'originalGroups', 'outputs', 'outputEntries', 'outputFiles')}))

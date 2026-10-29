"""Independent GNU/Python TAR bytes and external 7-Zip readback for v1.7.30."""
from pathlib import Path
import hashlib
import io
import json
import subprocess
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/tar-forward-nested-1730'
FIXTURES = ROOT / 'tests/fixtures/archives/tar-forward-nested-1730'
SEVEN = ROOT / 'src-tauri/resources/archive-engine/7z.exe'
sha = lambda data: hashlib.sha256(data).hexdigest()
load = lambda path: json.loads(path.read_text(encoding='utf-8'))
manifest = load(FIXTURES / 'manifest.json')
producer = load(BASE / 'producer.json')
assert producer['passed']
oracles = {}
for variant in manifest['variants']:
    assert sha((FIXTURES / variant['archive']).read_bytes()) == variant['sha256']
    assert sha((FIXTURES / variant['tar']).read_bytes()) == variant['tarSha256']
    recorded = {e['name']: e for e in variant['entries']}
    expected = {}
    with tarfile.open(FIXTURES / variant['tar']) as archive:
        members = list(archive)
        by_name = {m.name.rstrip('/'): m for m in members}
        assert len(members) == len(by_name)
        for index, member in enumerate(members):
            name = member.name.rstrip('/')
            target = by_name[member.linkname] if member.islnk() else member
            data = b'' if member.isdir() else archive.extractfile(target).read()
            assert sha(data) == recorded[name]['sha256']
            assert len(data) == recorded[name]['bytes']
            assert int(member.mtime) == recorded[name]['seconds']
            if member.islnk():
                assert members.index(target) > index
            expected[name] = dict(data=data, seconds=int(member.mtime))
    oracles[variant['format']] = expected

checks = []
third_count = 0
for case in load(BASE / 'native.json'):
    source = Path(case['source'])
    assert sha(source.read_bytes()) == case['sourceSha256']
    if case['parts']:
        assert b''.join(Path(p).read_bytes() for p in case['parts']) == source.read_bytes()
        assert case['readSource'] == case['parts'][-1]
    entry = oracles[case['variant']][case['alias']]
    retained = Path(case['retained'])
    assert retained.read_bytes() == entry['data']
    receipt = case['receipt']
    assert receipt['contentSha256'] == sha(entry['data'])
    assert receipt['parentSha256'] == sha(Path(case['readSource']).read_bytes())
    assert receipt['depth'] == 2 and receipt['entryPath'] == case['alias']
    assert case['seconds'] == entry['seconds']
    assert retained.stat().st_nlink == 1
    target = retained.parent / 'external'
    result = subprocess.run([str(SEVEN), 'x', '-y', '-sccUTF-8', '-pNested1730!' if case['encrypted'] else '-p',
                             '-o' + str(target), str(retained)], capture_output=True)
    assert result.returncode == 0, (result.stdout, result.stderr)
    selected = Path(case['selected'])
    selected_files = {p.relative_to(selected).as_posix(): p.read_bytes() for p in selected.rglob('*') if p.is_file()}
    if case['encrypted']:
        expected_files = {'secret.txt', 'cover.png'}
        assert (target / 'secret.txt').read_bytes() == b'Inner password is independent: Nested1730!\n'
    else:
        expected_files = {'readme.txt', 'gallery/cover.png'}
        with zipfile.ZipFile(io.BytesIO(entry['data'])) as child:
            assert (target / 'readme.txt').read_bytes() == child.read('readme.txt')
            third = Path(case['thirdRetained'])
            assert third.read_bytes() == child.read('docs/next.zip')
            assert (target / 'docs/next.zip').read_bytes() == third.read_bytes()
        with zipfile.ZipFile(third) as archive:
            assert archive.read('third.txt') == b'Nested third layer 1730\n'
        assert case['thirdReceipt']['depth'] == 3
        assert case['thirdReceipt']['parentSha256'] == receipt['contentSha256']
        assert case['thirdReceipt']['contentSha256'] == sha(third.read_bytes()) == manifest['thirdSha256']
        assert not Path(case['thirdReceipt']['cachePath']).exists()
        third_count += 1
    assert set(selected_files) == expected_files
    for name, data in selected_files.items():
        assert data == (target / name).read_bytes()
    png_name = 'cover.png' if case['encrypted'] else 'gallery/cover.png'
    assert sha((target / png_name).read_bytes()) == manifest['imageSha256'] == case['imageSha256']
    assert sha(selected_files['secret.txt' if case['encrypted'] else 'readme.txt']) == case['textSha256']
    assert case['cacheCleaned'] and not Path(receipt['cachePath']).exists()
    checks.append(dict(variant=case['variant'], alias=case['alias'], encrypted=case['encrypted'], numericSource=bool(case['parts']),
                       sourceUnchanged=True, independentTarBytes=True, external7zipReadback=True,
                       nativeSelectedFiles=len(selected_files), ownAliasDate=True, thirdLayer=not case['encrypted'], cacheCleaned=True))
receipt = dict(version='1.7.30', passed=True, producer=manifest['producer'], originals=len(oracles),
               childArchives=len(checks), thirdLayerArchives=third_count, nativeSelectedFiles=sum(c['nativeSelectedFiles'] for c in checks),
               externalReader='7-Zip 26.04', externalReaderSha256=sha(SEVEN.read_bytes()), checks=checks)
(BASE / 'verified.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: receipt[k] for k in ('passed', 'originals', 'childArchives', 'thirdLayerArchives', 'nativeSelectedFiles')}))

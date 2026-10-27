"""GNU tar 1.35 + independently pinned official lzop; no application writer is used."""
from pathlib import Path
import hashlib, io, json, random, subprocess, tarfile

root = Path(__file__).resolve().parents[1]
fixture = root / 'tests/fixtures/archives/tar-lzo-gnu-178'
base = root / 'test-results/tar-lzo-gnu-178'
source = base / 'source'
fixture.mkdir(parents=True, exist_ok=True)
source.mkdir(parents=True, exist_ok=True)
sha = lambda data: hashlib.sha256(data).hexdigest()
unix = lambda path: '/mnt/' + str(path.resolve())[0].lower() + str(path.resolve())[2:].replace('\\', '/')
lzop = root / 'test-results/lzop-multi-create-172/research/lzop-linux'
provenance = json.loads((root / 'tests/fixtures/archives/lzop-prefix-174/manifest.json').read_text(encoding='utf-8'))
assert sha(lzop.read_bytes()) == provenance['producerSha256']

def linux(*args):
    return subprocess.run(['wsl', '-d', 'Ubuntu', '--exec', *args], capture_output=True, check=True).stdout

tar_version = linux('tar', '--version').decode().splitlines()[0]
assert tar_version == 'tar (GNU tar) 1.35', tar_version
long_dir = '资料目录' * 9 + '/' + '原始文件' * 8
long_leaf = '长文件名' * 9 + '-notes.txt'
archive = fixture / 'complete.tar.lzo'
if not archive.exists():
    for directory in ['tree/empty', 'other']:
        (source / directory).mkdir(parents=True, exist_ok=True)
    data = {
        'prefix.txt': b'first independently confirmed file\n',
        'tree/readme.txt': '长目录里的说明\n'.encode(),
        'tree/data.bin': random.Random(178).randbytes(200033),
        'other/readme.txt': b'same leaf name, different folder\n',
        'long.txt': b'GNU long UTF-8 file name\n',
        'after.txt': b'long name must not leak into the next file\n',
        'late.bin': random.Random(1781).randbytes(1100178),
    }
    for name, content in data.items():
        (source / name).write_bytes(content)
    tar_path = base / 'original.tar'
    linux('tar', '--format=gnu', '--no-recursion', '--mtime=@1700000000', '--owner=0', '--group=0',
          '--transform', f's@^tree@{long_dir}@;s@^long.txt$@{long_leaf}@',
          '-cf', unix(tar_path), '-C', unix(source),
          'prefix.txt', 'tree', 'tree/empty', 'tree/readme.txt', 'tree/data.bin',
          'other/readme.txt', 'long.txt', 'after.txt', 'late.bin')
    compressed = linux(unix(lzop), '-c', '-N', unix(tar_path))
    archive.write_bytes(compressed)
    decoded = linux(unix(lzop), '-d', '-c', unix(archive))
    assert decoded == tar_path.read_bytes()
    entries = []
    with tarfile.open(fileobj=io.BytesIO(decoded)) as tar:
        for member in tar:
            content = tar.extractfile(member).read() if member.isfile() else b''
            entries.append({'name': member.name.rstrip('/'), 'isDirectory': member.isdir(), 'bytes': len(content), 'sha256': sha(content)})
    records = []
    at = 0
    while any(decoded[at:at+512]):
        block = decoded[at:at+512]
        size = int(block[124:136].rstrip(b'\0 ').lstrip(b' ') or b'0', 8)
        records.append({'offset': at, 'type': chr(block[156]), 'size': size})
        at += 512 + (size + 511) // 512 * 512
    assert sum(record['type'] == 'L' for record in records) >= 4
    manifest = {'archive': archive.name, 'sha256': sha(compressed), 'tarSha256': sha(decoded),
                'tarProducer': tar_version, 'lzopProducerSha256': provenance['producerSha256'],
                'lzopPackageSha256': provenance['packageSha256'], 'sources': 'Self-authored test files; no user files',
                'longDirectory': long_dir, 'longFile': long_leaf, 'entries': entries, 'records': records}
    (fixture / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')

manifest = json.loads((fixture / 'manifest.json').read_text(encoding='utf-8'))
assert sha(archive.read_bytes()) == manifest['sha256']
decoded = linux(unix(lzop), '-d', '-c', unix(archive))
assert sha(decoded) == manifest['tarSha256']
(base / 'official.tar').write_bytes(decoded)
listing = linux('tar', '-tf', unix(base / 'official.tar')).decode().splitlines()
assert [name.rstrip('/') for name in listing] == [entry['name'] for entry in manifest['entries']]
with tarfile.open(fileobj=io.BytesIO(decoded)) as tar:
    members = list(tar)
    assert len(members) == len(manifest['entries'])
    for member, expected in zip(members, manifest['entries']):
        assert member.name.rstrip('/') == expected['name']
        assert member.isdir() == expected['isDirectory']
        if member.isfile():
            data = linux('tar', '-xOf', unix(base / 'official.tar'), member.name)
            assert len(data) == expected['bytes'] and sha(data) == expected['sha256']
            assert tar.extractfile(member).read() == data
(base / 'official.json').write_text(json.dumps({'passed': True, 'original': manifest,
    'officialLzopDecoded': True, 'gnuTarListingAndContents': True, 'pythonTarfileAgrees': True}, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(f"Independent GNU/LZOP original verified: {len(manifest['entries'])} entries, SHA {manifest['sha256']}")

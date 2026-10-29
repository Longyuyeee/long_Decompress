"""GNU tar create/delete/concatenate originals: links precede their data.

No header rewriting. Two ordered GNU extraction passes and inode/SHA checks
form the oracle (GNU's ordinary single pass rejects these forward references).
"""
from pathlib import Path
import hashlib
import json
import struct
import subprocess
import tarfile
import uuid
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'tests/fixtures/archives/tar-forward-links-1728'
BASE = ROOT / 'test-results/tar-forward-links-1728'
OUT.mkdir(parents=True, exist_ok=True)
BASE.mkdir(parents=True, exist_ok=True)
sha = lambda data: hashlib.sha256(data).hexdigest()
unix = lambda path: '/mnt/' + str(path.resolve())[0].lower() + str(path.resolve())[2:].replace('\\', '/')


def linux(*args):
    result = subprocess.run(['wsl.exe', '-d', 'Ubuntu', '--exec', *map(str, args)], capture_output=True)
    assert result.returncode == 0, (args, result.stderr.decode('utf-8', errors='replace'))
    return result.stdout


lzop = ROOT / 'test-results/lzop-multi-create-172/research/lzop-linux'
assert sha(lzop.read_bytes()) == 'd9b46cf6fa44f4d657e5a3943d361dac24040d0acda37019b19ba285e52e2c53'


def chunk(name, data):
    return struct.pack('>I', len(data)) + name + data + struct.pack('>I', zlib.crc32(name + data))


png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 32, 24, 8, 2, 0, 0, 0))
png += chunk(b'IDAT', zlib.compress((b'\0' + bytes([90, 150, 210]) * 32) * 24)) + chunk(b'IEND', b'')
long = 'docs/' + '长文档' * 20 + '.txt'
long_alias = 'docs/' + '链接副本' * 18 + '.txt'
source = '/tmp/long-forward-1728-' + uuid.uuid4().hex
linux('mkdir', '-p', source + '/docs/empty', source + '/gallery')
payloads = [('ordinary.txt', b'prior target remains supported\n'), ('empty.txt', b''),
            ('docs/readme.txt', b'INFO forward TAR.LZO document 1728\n' * 34000),
            ('gallery/cover.png', png), (long, 'GNU 长名称前向内容。\n'.encode()),
            ('after.txt', b'unrelated tail preserved\n')]
for name, data in payloads:
    temporary = BASE / 'producer-input.bin'
    temporary.write_bytes(data)
    linux('cp', unix(temporary), source + '/' + name)
links = [('ordinary-copy.txt', 'ordinary.txt'), ('阅读副本.txt', 'docs/readme.txt'),
         ('gallery/封面副本.png', 'gallery/cover.png'), ('empty-copy.txt', 'empty.txt'), (long_alias, long), ('sparse-copy.bin', 'sparse.bin')]
linux('python3', '-c', 'import sys; f=open(sys.argv[1],"wb"); f.write(b"sparse-start"); f.seek(3*1024*1024); f.write(b"sparse-end"); f.close()', source + '/sparse.bin')
linux('python3', '-c', 'import os,json,sys;[(os.link(sys.argv[1]+"/"+b,sys.argv[1]+"/"+a)) for a,b in json.loads(sys.argv[2])]', source, json.dumps(links))
variants = []
for fmt in ('ustar', 'gnu', 'pax'):
    targets = ['empty.txt', 'docs/readme.txt', 'gallery/cover.png'] + ([long, 'sparse.bin'] if fmt != 'ustar' else [])
    aliases = [a for a, b in links if b in targets or b == 'ordinary.txt']
    names = ['docs', 'docs/empty', 'gallery', 'ordinary.txt', *targets, *aliases, 'after.txt']
    tar_path = OUT / (fmt + '.tar')
    archive_path = OUT / (fmt + '.tar.lzo')
    extra = ['--pax-option=delete=atime,delete=ctime'] if fmt == 'pax' else []
    sparse = ['--sparse'] if fmt != 'ustar' else []
    # Delete regular content records, then use the GNU sparse writer only for
    # the final contents. GNU 1.35's PAX-sparse --delete can otherwise crash.
    linux('tar', '--format=' + fmt, '--no-recursion', '--mtime=@1700001728', *extra, '-cf', unix(tar_path), '-C', source, *names)
    linux('tar', '--delete', '-f', unix(tar_path), *targets)
    contents = BASE / (fmt + '-contents.tar')
    linux('tar', '--format=' + fmt, '--no-recursion', '--mtime=@1700001828', *sparse, *extra, '-cf', unix(contents), '-C', source, *targets)
    linux('tar', '--concatenate', '-f', unix(tar_path), unix(contents))
    raw = tar_path.read_bytes()
    archive_path.write_bytes(linux(unix(lzop), '-c', '-N', unix(tar_path)))
    assert linux(unix(lzop), '-dc', unix(archive_path)) == raw
    extracted = source + '-' + fmt
    linux('mkdir', extracted)
    # Materialize contents before references with GNU itself. Nothing in the
    # original archive is reordered or rewritten for the application's reader.
    linux('tar', '--no-recursion', '-xf', unix(tar_path), '-C', extracted, *targets)
    linux('tar', '--no-recursion', '-xf', unix(tar_path), '-C', extracted, *[n for n in names if n not in targets])
    entries = []
    with tarfile.open(tar_path) as archive:
        members = list(archive)
        assert len({m.name.rstrip('/') for m in members}) == len(members)
        by_name = {m.name.rstrip('/'): m for m in members}
        for index, member in enumerate(members):
            name = member.name.rstrip('/')
            target = by_name[member.linkname] if member.islnk() else member
            data = b'' if member.isdir() else archive.extractfile(target).read()
            if not member.isdir():
                assert linux('sha256sum', source + '/' + name).decode().split()[0] == sha(data)
                assert linux('sha256sum', extracted + '/' + name).decode().split()[0] == sha(data)
            if member.islnk():
                assert linux('stat', '-c', '%i', extracted + '/' + name) == linux('stat', '-c', '%i', extracted + '/' + member.linkname)
            entries.append(dict(name=name, bytes=len(data), isDirectory=member.isdir(), sha256=sha(data),
                prefixSha256=sha(data[:1048576]), linkTarget=member.linkname if member.islnk() else None,
                targetId=members.index(target) if member.islnk() else None, seconds=int(member.mtime),
                headerOffset=member.offset, dataOffset=member.offset_data))
    assert sum(e['targetId'] > index for index, e in enumerate(entries) if e['targetId'] is not None) == len(targets)
    variants.append(dict(format=fmt, tar=tar_path.name, tarSha256=sha(raw), archive=archive_path.name,
                         sha256=sha(archive_path.read_bytes()), entries=entries))
manifest = dict(producer=linux('tar', '--version').decode().splitlines()[0], recipeVersion=1, variants=variants,
                recipe='GNU tar creates real Linux hardlinks, deletes their original content records, then concatenates a second GNU-created TAR containing those contents; no header rewriting.',
                oracle='Two ordered GNU extraction passes (contents, then remaining entries) with inode/content checks and independent Python TAR reads; ordinary GNU single-pass extraction rejects forward references. Links use their own encoded mtime, targets have a different encoded mtime.')
(OUT / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
(BASE / 'producer.json').write_text(json.dumps(dict(passed=True, **manifest), ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print('GNU ustar/GNU/PAX forward-hardlink producer originals verified.')

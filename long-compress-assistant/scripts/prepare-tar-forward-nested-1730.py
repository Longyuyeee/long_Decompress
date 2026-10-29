"""Real GNU tar forward references to ZIP/7Z children, without header rewriting.

Run once to freeze producer originals. ZIP content is project authored; encrypted
7Z is made by bundled 7-Zip, TAR hardlinks by GNU tar, and wrapping by pinned lzop.
"""
from pathlib import Path
import hashlib
import io
import json
import os
import struct
import subprocess
import tarfile
import uuid
import zipfile
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'tests/fixtures/archives/tar-forward-nested-1730'
BASE = ROOT / 'test-results/tar-forward-nested-1730'
OUT.mkdir(parents=True, exist_ok=True)
BASE.mkdir(parents=True, exist_ok=True)
sha = lambda data: hashlib.sha256(data).hexdigest()
unix = lambda p: '/mnt/' + str(p.resolve())[0].lower() + str(p.resolve())[2:].replace('\\', '/')


def linux(*args):
    return subprocess.run(['wsl.exe', '-d', 'Ubuntu', '--exec', *map(str, args)],
                          capture_output=True, check=True).stdout


lzop = ROOT / 'test-results/lzop-multi-create-172/research/lzop-linux'
assert sha(lzop.read_bytes()) == 'd9b46cf6fa44f4d657e5a3943d361dac24040d0acda37019b19ba285e52e2c53'


def zip_bytes(files):
    b = io.BytesIO()
    with zipfile.ZipFile(b, 'w') as archive:
        for name, data in files:
            info = zipfile.ZipInfo(name, (2026, 10, 29, 12, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)
    return b.getvalue()


def chunk(name, data):
    return struct.pack('>I', len(data)) + name + data + struct.pack('>I', zlib.crc32(name + data))


png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 32, 24, 8, 2, 0, 0, 0))
png += chunk(b'IDAT', zlib.compress((b'\0' + bytes([90, 150, 210]) * 32) * 24)) + chunk(b'IEND', b'')
text = '前向链接内层包，继续在原双栏浏览。\n'.encode()
secret = b'Inner password is independent: Nested1730!\n'
third = zip_bytes([('third.txt', b'Nested third layer 1730\n')])
plain = zip_bytes([('readme.txt', text), ('gallery/cover.png', png), ('docs/next.zip', third)])
input_dir = BASE / ('child-input-' + uuid.uuid4().hex)
input_dir.mkdir()
for name, data in [('secret.txt', secret), ('cover.png', png)]:
    p = input_dir / name
    p.write_bytes(data)
    os.utime(p, (1700001830, 1700001830))
encrypted = OUT / 'locked.7z'
encrypted.unlink(missing_ok=True)
subprocess.run([str(ROOT / 'src-tauri/resources/archive-engine/7z.exe'), 'a', '-t7z', '-m0=LZMA2',
                '-mhe=on', '-mtc=off', '-mta=off', '-pNested1730!', str(encrypted), 'secret.txt', 'cover.png'],
               cwd=input_dir, capture_output=True, check=True)
long_target = 'payload/' + '长目标内容' * 12 + '.zip'
long_alias = 'links/' + '长名称副本' * 12 + '.zip'
contents = [('payload/plain.data', plain), ('payload/locked.data', encrypted.read_bytes()),
            ('payload/fake.data', b'Not an archive despite a ZIP suffix.\n'), (long_target, plain),
            ('ordinary.zip', plain), ('after.txt', b'Outer archive stays unchanged.\n')]
links = [('links/copy.zip', 'payload/plain.data'), ('links/加密副本.7z', 'payload/locked.data'),
         ('links/fake.zip', 'payload/fake.data'), (long_alias, long_target)]
source = '/tmp/long-forward-nested-1730-' + uuid.uuid4().hex
linux('mkdir', '-p', source + '/payload', source + '/links')
for name, data in contents:
    transfer = BASE / 'producer-input.bin'
    transfer.write_bytes(data)
    linux('cp', unix(transfer), source + '/' + name)
linux('python3', '-c', 'import os,json,sys;[(os.link(sys.argv[1]+"/"+b,sys.argv[1]+"/"+a)) for a,b in json.loads(sys.argv[2])]', source, json.dumps(links))
variants = []
for fmt in ['ustar', 'gnu', 'pax']:
    selected_links = links if fmt != 'ustar' else links[:-1]
    targets = [target for _, target in selected_links]
    names = ['payload', 'links', *targets, 'ordinary.zip', *[a for a, _ in selected_links], 'after.txt']
    rawpath = OUT / (fmt + '.tar')
    archivepath = OUT / (fmt + '.tar.lzo')
    extra = ['--pax-option=delete=atime,delete=ctime'] if fmt == 'pax' else []
    linux('tar', '--format=' + fmt, '--no-recursion', '--mtime=@1700001730', *extra, '-cf', unix(rawpath), '-C', source, *names)
    linux('tar', '--delete', '-f', unix(rawpath), *targets)
    tail = BASE / (fmt + '-contents.tar')
    linux('tar', '--format=' + fmt, '--no-recursion', '--mtime=@1700001830', *extra, '-cf', unix(tail), '-C', source, *targets)
    linux('tar', '--concatenate', '-f', unix(rawpath), unix(tail))
    raw = rawpath.read_bytes()
    archivepath.write_bytes(linux(unix(lzop), '-c', '-N', unix(rawpath)))
    assert linux(unix(lzop), '-dc', unix(archivepath)) == raw
    extracted = source + '-' + fmt
    linux('mkdir', extracted)
    linux('tar', '--no-recursion', '-xf', unix(rawpath), '-C', extracted, *targets)
    linux('tar', '--no-recursion', '-xf', unix(rawpath), '-C', extracted, *[n for n in names if n not in targets])
    entries = []
    with tarfile.open(rawpath) as archive:
        members = list(archive)
        by_name = {m.name.rstrip('/'): m for m in members}
        assert len(members) == len(by_name)
        for index, member in enumerate(members):
            name = member.name.rstrip('/')
            target = by_name[member.linkname] if member.islnk() else member
            data = b'' if member.isdir() else archive.extractfile(target).read()
            if not member.isdir():
                assert linux('sha256sum', source + '/' + name).decode().split()[0] == sha(data)
                assert linux('sha256sum', extracted + '/' + name).decode().split()[0] == sha(data)
            if member.islnk():
                assert members.index(target) > index
                assert linux('stat', '-c', '%i', extracted + '/' + name) == linux('stat', '-c', '%i', extracted + '/' + member.linkname)
            entries.append(dict(name=name, isDirectory=member.isdir(), bytes=len(data), sha256=sha(data),
                                linkTarget=member.linkname if member.islnk() else None,
                                targetId=members.index(target) if member.islnk() else None,
                                seconds=int(member.mtime), headerOffset=member.offset, dataOffset=member.offset_data))
    variants.append(dict(format=fmt, archive=archivepath.name, sha256=sha(archivepath.read_bytes()),
                         tar=rawpath.name, tarSha256=sha(raw), entries=entries,
                         aliases=[a for a, _ in selected_links]))
manifest = dict(producer=linux('tar', '--version').decode().splitlines()[0], recipeVersion=1, variants=variants,
                recipe='GNU creates real Linux hardlinks, deletes the content records, then concatenates GNU-created targets; no header rewriting.',
                oracle='Two ordered GNU extraction passes with inode/SHA checks; independent Python TAR bytes and own header dates.',
                childSha256=sha(plain), thirdSha256=sha(third), encryptedSha256=sha(encrypted.read_bytes()),
                textSha256=sha(text), imageSha256=sha(png), secretSha256=sha(secret), credential='Nested1730!')
(OUT / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
(BASE / 'producer.json').write_text(json.dumps(dict(passed=True, **manifest), ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print('GNU USTAR/GNU/PAX forward nested originals independently verified.')

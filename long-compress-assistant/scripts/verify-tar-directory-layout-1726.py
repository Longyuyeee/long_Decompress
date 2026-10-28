"""Independently read fresh TAR outputs and the final native filesystem results.

Run after `cargo test --lib tardir1726`. Uses cached external codec tools from
the earlier compatibility work; does not install tools or control the desktop.
"""
import bz2
import gzip
import hashlib
import io
import json
import lzma
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/tar-directory-layout-1726'
sys.path.insert(0, str(ROOT / 'test-results/aes-1521/python'))
import brotli
import lz4.frame


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding='utf-8'))


def run(args):
    result = subprocess.run([str(a) for a in args], capture_output=True)
    assert result.returncode == 0, (args, result.stderr.decode('utf-8', errors='replace'))
    return result.stdout


def linux(path):
    value = str(path.resolve())
    return '/mnt/' + value[0].lower() + value[2:].replace('\\', '/')


official_zip = ROOT / 'test-results/lzop-read-convert-1657/lzop103w.zip'
official_lzop = ROOT / 'test-results/lzop-read-convert-1657/official-lzop/lzop103w/lzop.exe'
assert sha(official_zip.read_bytes()) == '0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a'
with zipfile.ZipFile(official_zip) as package:
    assert official_lzop.read_bytes() == package.read('lzop103w/lzop.exe')
official_lzip = ROOT / 'test-results/lzip-tools/unpacked/usr/bin/lzip.lzip'
seven_zip = ROOT / 'src-tauri/resources/archive-engine/7z.exe'


def decode(case, index):
    raw = b''.join(Path(part).read_bytes() for part in case['parts'])
    suffix = case['format'].removeprefix('tar').removeprefix('.')
    if not suffix:
        return raw
    if suffix == 'gz':
        return gzip.decompress(raw)
    if suffix == 'bz2':
        return bz2.decompress(raw)
    if suffix in ('xz', 'lzma'):
        return lzma.decompress(raw)
    if suffix == 'lz4':
        return lz4.frame.decompress(raw)
    if suffix == 'br':
        return brotli.decompress(raw)
    joined = BASE / f'independent-{index}.{case["format"]}'
    joined.write_bytes(raw)
    if suffix == 'zst':
        return run([seven_zip, 'x', '-so', joined])
    if suffix == 'lz':
        return run(['wsl.exe', '-d', 'Ubuntu', '--exec', linux(official_lzip), '-d', '-c', linux(joined)])
    assert suffix == 'lzo', suffix
    return run([official_lzop, '-d', '-c', joined])


def logical_tar(raw):
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:') as archive:
        result = {}
        for member in archive:
            name = member.name.rstrip('/')
            assert name not in result, ('duplicate member', name)
            result[name] = dict(directory=member.isdir(), seconds=member.mtime,
                                sha256=None if member.isdir() else sha(archive.extractfile(member).read()))
        return result


cases = read_json(BASE / 'native.json') + read_json(BASE / 'conversion.json')
checks = []
encoded_entries = 0
for index, case in enumerate(cases):
    expected = {e['name']: {key: e[key] for key in ('directory', 'seconds', 'sha256')}
                for e in case['sources']}
    decoded = logical_tar(decode(case, index))
    assert decoded == expected, (case['format'], 'encoded tree/content/dates', decoded, expected)
    encoded_entries += len(decoded)
    target = Path(case['restored'])
    actual = {p.relative_to(target).as_posix(): p for p in target.rglob('*')}
    assert set(actual) == set(expected), ('final exact tree', case['format'])
    for name, source in expected.items():
        path = actual[name]
        assert path.is_dir() == source['directory'], name
        assert path.stat().st_mtime_ns == source['seconds'] * 1_000_000_000, ('final date', name)
        if not source['directory']:
            assert sha(path.read_bytes()) == source['sha256'], ('final content', name)
    if 'sourceTar' in case:
        original = Path(case['sourceTar'])
        manifest = read_json(original.parent / 'manifest.json')
        fixture = next(v for v in manifest['variants'] if v['tar'] == original.name)
        assert sha(original.read_bytes()) == fixture['tarSha256']
        assert sha(original.with_suffix('.tar.lzo').read_bytes()) == fixture['sha256'] == case['sourceSha256']
        assert logical_tar(original.read_bytes()) == expected, 'conversion changed logical source'
    else:
        for source in case['sources']:
            path = Path(source['path'])
            assert path.stat().st_mtime_ns == source['seconds'] * 1_000_000_000, 'source date changed'
            if not source['directory']:
                assert sha(path.read_bytes()) == source['sha256'], 'source content changed'
    checks.append(dict(format=case['format'], input=case['input'],
                       archiveSha256=sha(b''.join(Path(p).read_bytes() for p in case['parts'])),
                       volumeCount=len(case['parts']), entries=len(expected),
                       explicitDirectories=sum(e['directory'] for e in expected.values()),
                       exactTreeContentDates=True, sourceUnchanged=True))

selection = read_json(BASE / 'selection.json')
assert [(c['mode'], len(c['times'])) for c in selection] == [
    ('directory', 2), ('file', 0), ('filter', 2), ('flat', 0), ('off', 0)]
for case in selection:
    for name, date in case['times']:
        assert (Path(case['restored']) / name).stat().st_mtime_ns == date['seconds'] * 1_000_000_000 + date['nanoseconds']
flat = Path(next(c for c in selection if c['mode'] == 'flat')['restored'])
assert (flat / 'note.txt').is_file() and not (flat / '资料').exists()

receipt = dict(version='1.7.26', passed=True, archives=len(checks), encodedEntries=encoded_entries,
               tools=dict(python=sys.version.split()[0], brotli=brotli.__version__, lz4=lz4.__version__,
                          lzip=run(['wsl.exe', '-d', 'Ubuntu', '--exec', linux(official_lzip), '--version']).decode().splitlines()[0],
                          lzopSha256=sha(official_lzop.read_bytes()),
                          zstandardReader=run([seven_zip, 'i']).decode(errors='replace').splitlines()[1]),
               checks=checks, selectionModes=[c['mode'] for c in selection])
(BASE / 'verified.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(dict(passed=True, archives=len(checks), encodedEntries=encoded_entries)))

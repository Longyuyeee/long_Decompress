"""Test-only official lzop 1.03 originals; pass the official Windows executable.
No compressor, executable, or GPL source is shipped with the application.
"""
import hashlib, io, json, pathlib, random, subprocess, sys, tarfile, zlib

root = pathlib.Path(__file__).resolve().parent
tool = pathlib.Path(sys.argv[1]).resolve()
source = root.parent / 'aes-codecs-1521' / '原文件'
text = (source / '说明.txt').read_bytes()
payloads = {
    'method1.txt.lzo': (['-3'], text),
    'method2.txt.lzo': (['-1'], text),
    'method3.txt.lzo': (['-9'], text),
    'crc.txt.lzo': (['-9', '--crc32', '-CC'], text * 9000),
    'unchecked.txt.lzo': (['-3', '--no-checksum'], text),
    'stored.bin.lzo': (['-1', '-CC'], random.Random(1657).randbytes(600000)),
    'picture.png.lzo': (['-9'], (source / '图标.png').read_bytes()),
    'empty.txt.lzo': (['-3'], b''),
    'large.txt.lzo': (['-3'], b'large text line\n' * 650000),
}
tar = io.BytesIO()
with tarfile.open(fileobj=tar, mode='w') as archive:
    entry = tarfile.TarInfo('inside.txt'); entry.size = len(text)
    archive.addfile(entry, io.BytesIO(text))
payloads['directory.tar.lzo'] = (['-3'], tar.getvalue())
proof = []
for name, (options, plain) in payloads.items():
    # stdin avoids machine-specific absolute filenames/timestamps in headers.
    packed = subprocess.run([str(tool), *options, '-c'], input=plain, capture_output=True, check=True).stdout
    (root / name).write_bytes(packed)
    subprocess.run([str(tool), '-t', str(root / name)], capture_output=True, check=True)
    restored = subprocess.run([str(tool), '-d', '-c', str(root / name)], capture_output=True, check=True).stdout
    assert plain == restored
    proof.append({'file': name, 'options': options, 'size': len(plain),
                  'sha256': hashlib.sha256(plain).hexdigest(),
                  'archiveSha256': hashlib.sha256(packed).hexdigest(), 'officialTestAndReadback': True})
(root / 'manifest.json').write_text(json.dumps({
    'producer': 'official Windows lzop 1.03 / LZO 2.04',
    'url': 'https://www.lzop.org/download/lzop103w.zip',
    'downloadSha256': '0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a',
    'producerVersion': subprocess.run([str(tool), '--version'], capture_output=True, check=True).stdout.decode('ascii'),
    'cases': proof,
    'mutations': 'Malformed headers, data, truncation, flags and appended members are derived in Rust tests; they are not valid official originals.',
}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'{len(proof)} official originals tested and independently read back')

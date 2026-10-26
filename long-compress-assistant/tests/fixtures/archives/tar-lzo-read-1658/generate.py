"""Independent TAR producers + official lzop 1.03. Test tools are not shipped.
Usage: python generate.py /path/to/official/lzop.exe
"""
import hashlib, json, pathlib, random, subprocess, sys, tarfile

root = pathlib.Path(__file__).resolve().parent
app = root.parents[3]
work = app / 'test-results/tar-lzo-read-convert-1658/fixtures'
work.mkdir(parents=True, exist_ok=True)
source = work / 'source'
source.mkdir(exist_ok=True)
tool = pathlib.Path(sys.argv[1]).resolve()
text = '收到 TAR.LZO 资料，直接浏览目录，只取回需要的文件。\n'.encode()
payloads = {
    'notes/说明 [1].txt': text,
    'notes/large.txt': b'TAR.LZO directory prefix preview\n' * 50000,
    'notes/' + '长路径' * 22 + '.txt': text,
    'gallery/cover.png': (root.parent / 'aes-codecs-1521/原文件/图标.png').read_bytes(),
    'data.bin': random.Random(1658).randbytes(300000),
    'empty.txt': b'',
}
for name, data in payloads.items():
    p = source / name; p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(data)
(source / 'empty-dir').mkdir(exist_ok=True)
tar = work / 'directory.tar'
subprocess.run(['tar', '--format=pax', '-cf', str(tar), '-C', str(source),
                'notes', 'gallery', 'data.bin', 'empty.txt', 'empty-dir'], check=True, capture_output=True)
expected = [{'path': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
            for name, data in payloads.items()]
cases = []

def wrap(name, content, options, files, producer):
    path = root / name
    plain = work / (name + '.tar')
    output = work / (name + '-restored')
    packed = subprocess.run([str(tool), *options, '-c'], input=content, capture_output=True, check=True).stdout
    path.write_bytes(packed)
    subprocess.run([str(tool), '-t', str(path)], capture_output=True, check=True)
    restored = subprocess.run([str(tool), '-d', '-c', str(path)], capture_output=True, check=True).stdout
    assert restored == content
    plain.write_bytes(restored); output.mkdir(exist_ok=True)
    subprocess.run(['tar', '--options', 'hdrcharset=UTF-8', '-xf', str(plain), '-C', str(output)], capture_output=True, check=True)
    for file in files:
        data = (output / file['path']).read_bytes()
        assert len(data) == file['bytes'] and hashlib.sha256(data).hexdigest() == file['sha256']
    cases.append({'archive': name, 'tarProducer': producer, 'lzopOptions': options,
                  'archiveSha256': hashlib.sha256(packed).hexdigest(),
                  'tarSha256': hashlib.sha256(content).hexdigest(), 'tarBytes': len(content),
                  'expected': files, 'officialCheckAndReadback': True, 'independentTarReadback': True})

version = subprocess.run(['tar', '--version'], capture_output=True, check=True).stdout.decode().strip()
for method in ['1', '3', '9']:
    wrap(f'directory-{method}.tar.lzo', tar.read_bytes(), ['-' + method, '-CC'], expected, version)
old = root.parent / 'tar-1513'
manifest = json.loads((old / 'manifest.json').read_text(encoding='utf-8'))
for name in ['old-gnu.tar', 'pax00.tar', 'pax01.tar', 'pax10.tar', 'pax-ordinary.tar']:
    case = next(c for c in manifest['cases'] if c['archive'] == name)
    wrap(name + '.lzo', (old / name).read_bytes(), ['-3', '--crc32', '-CC'], case['expected'], 'GNU tar 1.35; tar-1513 provenance')
empty_list = work / 'empty-list'; empty_list.write_bytes(b'')
subprocess.run(['tar', '-cf', str(work / 'empty.tar'), '-T', str(empty_list)], capture_output=True, check=True)
wrap('empty.tar.lzo', (work / 'empty.tar').read_bytes(), ['-3'], [], version)
# Valid LZOP envelopes around deliberately invalid TAR; not healthy originals.
content = tar.read_bytes()
with tarfile.open(tar) as parsed:
    offset = parsed.getmember('notes/large.txt').offset_data
bad_header = bytearray(content); bad_header[0] ^= 1
rejected = []
for name, data in [('bad-header.tar.lzo', bytes(bad_header)),
                   ('short-file.tar.lzo', content[:offset + 100]),
                   ('after-eof.tar.lzo', content + b'not TAR padding'),
                   ('not-tar.tar.lzo', b'not a tar archive')]:
    target = root / name
    target.write_bytes(subprocess.run([str(tool), '-3', '-CC', '-c'], input=data, capture_output=True, check=True).stdout)
    subprocess.run([str(tool), '-t', str(target)], capture_output=True, check=True)
    rejected.append({'archive': name, 'kind': 'derived invalid TAR / valid LZOP envelope',
                     'archiveSha256': hashlib.sha256(target.read_bytes()).hexdigest()})
(root / 'manifest.json').write_text(json.dumps({
    'lzopProducer': 'official Windows lzop 1.03 / LZO 2.04',
    'lzopDownload': 'https://www.lzop.org/download/lzop103w.zip',
    'lzopDownloadSha256': '0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a',
    'sources': 'Self-authored filesystem data and previously committed GNU TAR originals; no user files',
    'cases': cases, 'rejected': rejected,
}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'{len(cases)} TAR.LZO originals: official lzop and independent TAR readback passed')

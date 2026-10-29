"""UTC/DOS recovery controls plus recipes referencing frozen independent originals."""
from pathlib import Path
import hashlib
import io
import json
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'tests/fixtures/archives/zip-recovery-time-1735'
OUT.mkdir(parents=True, exist_ok=True)
sha = lambda b: hashlib.sha256(b).hexdigest()
field = lambda b: struct.pack('<HH', 0x5455, len(b)) + b
ut = lambda seconds, flags=1: field(bytes([flags]) + struct.pack('<I', seconds))
local = lambda seconds, flags=7: field(bytes([flags]) + b''.join(struct.pack('<I', s) for s in [seconds, seconds + 2, seconds + 4][:flags.bit_count()]))
DOS = [2001, 2, 3, 4, 5, 6]
FALLBACK = [1980, 1, 1, 0, 0, 0]
S = 1700001735
controls = [
    ('valid', ut(S), ut(S), S, S),
    ('flags7', local(S), ut(S, 7), S, S),
    ('flags5', local(S, 5), ut(S, 5), S, S),
    ('central-only', b'', ut(S), S, None),
    ('local-only', local(S, 3), b'', None, S),
    ('central-wins', ut(S - 30), ut(S), S, S - 30),
    ('short-local', ut(S, 7), ut(S, 7), S, None),
    ('empty', field(b''), field(b''), None, None),
    ('malformed', field(b'\x01abc'), field(b'\x01abc'), None, None),
    ('missing-mtime', field(b'\x01'), field(b'\x01'), None, None),
    ('access-only', ut(S, 2), ut(S, 2), None, None),
    ('reserved', ut(S, 9), ut(S, 9), None, None),
    ('duplicate', ut(S) * 2, ut(S) * 2, None, None),
    ('conflict', ut(S) + ut(S + 2), ut(S) + ut(S + 2), None, None),
    ('high-bit', ut(0xffffffff), ut(0xffffffff), None, None),
    ('zero', ut(0), ut(0), 0, 0),
    ('signed-max', ut(0x7fffffff), ut(0x7fffffff), 0x7fffffff, 0x7fffffff),
    ('unknown-before', b'\xef\xbe\x03\x00abc' + ut(S), b'\xef\xbe\x03\x00abc' + ut(S), S, S),
    ('dos-only', b'', b'', None, None),
    ('invalid-dos', b'', b'', None, None),
    ('invalid-dos-utc', ut(S), ut(S), S, S),
    ('bad-content', ut(S), ut(S), S, S),
]
sink = io.BytesIO()
expected_full, expected_lost = [], []
with zipfile.ZipFile(sink, 'w') as z:
    rows = [('资料/', b'', local(S - 100, 3), ut(S - 100, 3), S - 100, S - 100),
            ('资料/空目录/', b'', local(S - 90), ut(S - 90, 7), S - 90, S - 90)]
    rows += [('资料/' + name + '.txt', ('source-' + name + '\n').encode(), a, b, c, d) for name, a, b, c, d in controls]
    for name, content, a, b, c, d in rows:
        info = zipfile.ZipInfo(name, tuple(DOS)); info.extra = a
        z.writestr(info, content); info.extra = b
        record = {'name': name.rstrip('/'), 'directory': name.endswith('/'), 'sha256': sha(content), 'bytes': len(content),
                  'dos': FALLBACK if '/invalid-dos' in name else DOS}
        if not name.endswith('bad-content.txt'):
            expected_full.append(dict(record, utc=c)); expected_lost.append(dict(record, utc=d))
raw = bytearray(sink.getvalue())
with zipfile.ZipFile(io.BytesIO(raw)) as z:
    central = z.start_dir
    for info in z.infolist():
        start = info.header_offset + 30 + struct.unpack_from('<H', raw, info.header_offset + 26)[0] + struct.unpack_from('<H', raw, info.header_offset + 28)[0]
        if '/invalid-dos' in info.filename:
            struct.pack_into('<HH', raw, info.header_offset + 10, 0, 0)
        if info.filename.endswith('bad-content.txt'): raw[start] ^= 1
pos = central
while raw[pos:pos + 4] == b'PK\x01\x02':
    n, x, c = struct.unpack_from('<HHH', raw, pos + 28)
    name = raw[pos + 46:pos + 46 + n].decode('utf-8')
    if '/invalid-dos' in name: struct.pack_into('<HH', raw, pos + 12, 0, 0)
    pos += 46 + n + x + c

cases = []
for name, value, entries in [('controls-full', bytes(raw), expected_full), ('controls-lost', bytes(raw[:central]), expected_lost)]:
    archive = name + '.zip'; (OUT / archive).write_bytes(value)
    cases.append({'name': name, 'archive': archive, 'recipe': {'kind': 'copy'}, 'sha256': sha(value), 'entries': entries,
                  'encrypted': False, 'rebuilt': name.endswith('lost'), 'skipped': ['资料/bad-content.txt']})

old = ROOT / 'tests/fixtures/archives/zip-utc-read-1731'
manifest = json.loads((old / 'manifest.json').read_text(encoding='utf-8'))
for label, filename, encrypted, recipe_kind in [('plain-lost', 'plain.zip', False, 'prefix'),
                                              ('parallel-flat', 'parallel.zip', False, 'prefix'),
                                              ('aes-damaged', 'aes.zip', True, 'ciphertext'),
                                              ('aes-bad-ut-damaged', 'aes-malformed-ut.zip', True, 'ciphertext'),
                                              ('standard-crc', 'plain.zip', False, 'crc')]:
    original = (old / filename).read_bytes(); value = bytearray(original)
    source = next(x for x in manifest['originals'] if x['name'] == ('aes' if encrypted else 'parallel' if filename == 'parallel.zip' else 'plain'))
    expected = [dict(x, utc=None if 'bad-ut' in label else x['seconds']) for x in source['entries']]
    with zipfile.ZipFile(io.BytesIO(original)) as z:
        if recipe_kind == 'prefix':
            recipe = {'kind': 'prefix', 'length': z.start_dir}; value = value[:z.start_dir]
        else:
            member = z.getinfo('docs/nested/large-b.bin')
            if recipe_kind == 'ciphertext':
                at = member.header_offset + 30 + struct.unpack_from('<H', value, member.header_offset + 26)[0] + struct.unpack_from('<H', value, member.header_offset + 28)[0] + 25
            else:
                pos = z.start_dir
                while True:
                    n, x, c = struct.unpack_from('<HHH', value, pos + 28)
                    if value[pos + 46:pos + 46 + n] == b'docs/nested/large-b.bin': break
                    pos += 46 + n + x + c
                at = pos + 16
            value[at] ^= 1; recipe = {'kind': 'xor', 'position': at}
            expected = [x for x in expected if x['name'] != 'docs/nested/large-b.bin']
    for entry in expected: entry['bytes'] = entry.get('bytes', 0)
    cases.append({'name': label, 'archive': '../zip-utc-read-1731/' + filename, 'recipe': recipe,
                  'originalSha256': sha(original), 'sha256': sha(value), 'entries': expected,
                  'encrypted': encrypted, 'password': manifest['credential'] if encrypted else None,
                  'rebuilt': recipe_kind == 'prefix', 'skipped': [] if recipe_kind == 'prefix' else ['docs/nested/large-b.bin']})
numeric = dict(next(x for x in cases if x['name'] == 'plain-lost'), name='numeric-lost')
cases.append(numeric)
for method, version in [('bzip2', 1733), ('lzma', 1734)]:
    path = ROOT / f'tests/fixtures/archives/zip-{method}-rescue-{version}'
    p = json.loads((path / 'producer.json').read_text(encoding='utf-8'))
    original = next(x for x in p['originals'] if x['zip64'] == (method == 'bzip2'))
    entries = [dict(name=e['name'].rstrip('/'), directory=e['directory'], sha256=e['sha256'], bytes=e['size'],
                    dos=[2024, 5, 6, 7, 8, 10], utc=None) for e in original['entries']]
    archive = f'../zip-{method}-rescue-{version}/lost{64 if method == "bzip2" else 32}.zip'
    value = (OUT / archive).read_bytes()
    cases.append({'name': method + '-stream', 'archive': archive, 'recipe': {'kind': 'copy'}, 'sha256': sha(value),
                  'entries': entries, 'encrypted': False, 'rebuilt': True, 'skipped': []})
(OUT / 'manifest.json').write_text(json.dumps({'version': '1.7.35', 'cases': cases,
    'provenance': 'Independent Python metadata controls; recipes reference frozen bsdtar/libarchive 3.8.9 and CPython streaming originals. Mutations are labelled, not new external originals.'}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'cases': len(cases), 'expectedEntries': sum(len(x['entries']) for x in cases)}))

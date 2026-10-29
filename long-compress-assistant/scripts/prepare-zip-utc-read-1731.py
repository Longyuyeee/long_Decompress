"""Freeze real bsdtar UTC ZIP originals and explicitly labelled metadata variants."""
from pathlib import Path
import hashlib
import json
import os
import random
import struct
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'tests/fixtures/archives/zip-utc-read-1731'
BASE = ROOT / 'test-results/zip-utc-read-1731'
OUT.mkdir(parents=True, exist_ok=True)
BASE.mkdir(parents=True, exist_ok=True)
INPUT = BASE / 'producer-input'
INPUT.mkdir(exist_ok=True)
sha = lambda b: hashlib.sha256(b).hexdigest()
def field(value): return struct.pack('<HH', 0x5455, len(value)) + value
def ut(seconds, flags=1): return field(bytes([flags]) + struct.pack('<I', seconds))

data = random.Random(1731).randbytes(310000)
files = {'docs/note.txt': ('UTC 修改时间，保留奇数秒。\n'.encode(), 1700001731),
         'docs/nested/large-a.bin': (data, 1700001833),
         'docs/nested/large-b.bin': (data[::-1], 1710054133),
         'docs/zero.txt': (b'', 1700001935)}
dirs = {'docs/empty': 1700001537, 'docs/nested': 1700001549, 'docs': 1700001551}
for name, (body, seconds) in files.items():
    p = INPUT / name
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_bytes(body)
    os.utime(p, (seconds, seconds))
for name, seconds in dirs.items():
    p = INPUT / name
    p.mkdir(parents=True, exist_ok=True)
    os.utime(p, (seconds, seconds))
flat = INPUT / 'flat'
flat.mkdir(exist_ok=True)
for name, seconds, body in [('a.bin', 1700001731, data), ('b.bin', 1710054133, data[::-1])]:
    p = flat / name
    p.write_bytes(body)
    os.utime(p, (seconds, seconds))
version = subprocess.check_output(['tar', '--version'], text=True).splitlines()[0]
variants = []
for name, encrypted, flat_source in [('plain', False, False), ('aes', True, False), ('parallel', False, True)]:
    path = OUT / (name + '.zip')
    args = ['tar', '-c', '--format', 'zip', '-f', str(path), '-C', str(flat if flat_source else INPUT)]
    if encrypted:
        args += ['--options', 'zip:encryption=aes256', '--passphrase', 'UtcRead1731!']
    args += ['a.bin', 'b.bin'] if flat_source else ['docs']
    result = subprocess.run(args, capture_output=True)
    assert result.returncode == 0, (args, result.stderr)
    expected = {n: (b, s, False) for n, (b, s) in files.items()}
    expected.update({n: (b'', s, True) for n, s in dirs.items()})
    if flat_source:
        expected = {'a.bin': (data, 1700001731, False), 'b.bin': (data[::-1], 1710054133, False)}
    entries = []
    with zipfile.ZipFile(path) as archive:
        for info in archive.infolist():
            n = info.filename.rstrip('/')
            body, seconds, directory = expected[n]
            extra = info.extra
            times = []
            while extra:
                kind, size = struct.unpack_from('<HH', extra)
                value, extra = extra[4:4+size], extra[4+size:]
                if kind == 0x5455:
                    assert len(value) == 5 and value[0] & 1
                    times.append(struct.unpack_from('<i', value, 1)[0])
            assert times == [seconds], (name, n, times, seconds)
            if not encrypted:
                assert archive.read(info) == body
            entries.append(dict(name=n, bytes=len(body), directory=directory, sha256=sha(body), seconds=seconds, dos=list(info.date_time)))
    assert len(entries) == len(expected)
    variants.append(dict(name=name, archive=path.name, sha256=sha(path.read_bytes()), encrypted=encrypted, entries=entries))

# Independent Python ZIP writer supplies control cases, not external certification.
controls = OUT / 'controls.zip'
records = []
control_cases = [('valid', ut(1700001731), ut(1700001731), 1700001731),
                 ('central-flags7', field(b'\x07'+struct.pack('<III',1700001731,1700001733,1700001735)), ut(1700001731,7), 1700001731),
                 ('central-only', b'', ut(1700001731), 1700001731),
                 ('local-only', ut(1700001731), b'', None),
                 ('central-wins', ut(1700001701), ut(1700001731), 1700001731),
                 ('malformed', field(b'\x01abc'), field(b'\x01abc'), None),
                 ('empty', field(b''), field(b''), None),
                 ('missing-mtime', field(b'\x01'), field(b'\x01'), None),
                 ('access-only', ut(1700001731,2), ut(1700001731,2), None),
                 ('reserved-bits', ut(1700001731,9), ut(1700001731,9), None),
                 ('duplicate', ut(1700001731)*2, ut(1700001731)*2, None),
                 ('conflicting', ut(1700001731)+ut(1700001733), ut(1700001731)+ut(1700001733), None),
                 ('high-bit', ut(0xffffffff), ut(0xffffffff), None),
                 ('zero', ut(0), ut(0), 0), ('signed-max', ut(0x7fffffff), ut(0x7fffffff), 0x7fffffff),
                 ('时间', ut(1700001731), ut(1700001731), 1700001731),
                 ('unknown-before', struct.pack('<HH',0xbeef,3)+b'abc'+ut(1700001731), struct.pack('<HH',0xbeef,3)+b'abc'+ut(1700001731),1700001731)]
with zipfile.ZipFile(controls, 'w') as archive:
    for name, local, central, seconds in control_cases:
        info = zipfile.ZipInfo(name + '.txt', (2001,2,3,4,5,6))
        info.extra = local
        body = (name + '\n').encode()
        archive.writestr(info, body)
        info.extra = central
        records.append(dict(name=info.filename, sha256=sha(body), seconds=seconds, dos=list(info.date_time), bytes=len(body)))

# A bounded malformed UT before existing required encryption metadata. Payloads,
# names, local headers and AES fields are unchanged from the real AES original.
raw = (OUT / 'aes.zip').read_bytes()
end = raw.rfind(b'PK\x05\x06')
directory_size, start = struct.unpack_from('<II', raw, end+12)
directory = bytearray()
pos = start
while pos < start + directory_size:
    assert raw[pos:pos+4] == b'PK\x01\x02'
    n, x, c = struct.unpack_from('<HHH', raw, pos+28)
    extra = raw[pos+46+n:pos+46+n+x]
    retained = bytearray()
    while extra:
        kind, size = struct.unpack_from('<HH', extra)
        if kind != 0x5455: retained.extend(extra[:4+size])
        extra = extra[4+size:]
    extra = field(b'\x01abc') + retained
    header = bytearray(raw[pos:pos+46])
    struct.pack_into('<H', header, 30, len(extra))
    directory.extend(header + raw[pos+46:pos+46+n] + extra + raw[pos+46+n+x:pos+46+n+x+c])
    pos += 46+n+x+c
tail = bytearray(raw[end:])
struct.pack_into('<I', tail, 12, len(directory))
derived = OUT / 'aes-malformed-ut.zip'
derived.write_bytes(raw[:start] + directory + tail)
manifest = dict(producer=version, originals=variants, controls=dict(archive=controls.name,sha256=sha(controls.read_bytes()),entries=records),
                derived=dict(archive=derived.name,sha256=sha(derived.read_bytes()),source='aes.zip',recipe='Only central optional UT replaced with bounded invalid UT before retained AES/other fields; encrypted payloads and local headers unchanged.'),
                credential='UtcRead1731!', utcPolicy='Single central UT mtime, flags low bits only, 0..2147483647 seconds; otherwise existing DOS local fallback. No local-only inference.')
(OUT / 'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(BASE / 'producer.json').write_text(json.dumps(dict(passed=True,**manifest),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Three real bsdtar ZIP originals and labelled metadata control archives prepared.')

"""Real libarchive frames in explicitly labelled multi-frame ZIP controls."""
from pathlib import Path
import copy
import hashlib
import io
import json
import os
import struct
import subprocess
import zipfile
import zlib

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'tests/fixtures/archives/zip-zstd-rescue-1736'
DEST = ROOT / 'tests/fixtures/archives/zip-zstd-multiframe-1737'
BASE = ROOT / 'test-results/zip-zstd-multiframe-1737'
DEST.mkdir(parents=True, exist_ok=True)
BASE.mkdir(parents=True, exist_ok=True)
sha = lambda b: hashlib.sha256(b).hexdigest()
load = lambda p: json.loads(p.read_text(encoding='utf-8'))
source_manifest = load(SOURCE / 'producer.json')
original = source_manifest['variants'][0]
source = (SOURCE / original['name']).read_bytes()
assert sha(source) == original['sha256']
plain = BASE / 'producer-source'
plain.mkdir(exist_ok=True)
r = subprocess.run([str(ROOT / 'src-tauri/resources/archive-engine/7z.exe'), 'x', '-y', '-bd',
                    '-o' + str(plain), str(SOURCE / original['name'])], capture_output=True)
assert r.returncode == 0, r.stdout


def read_records(raw, expected):
    records = []
    with zipfile.ZipFile(io.BytesIO(raw)) as z:
        at = z.start_dir
        for info, metadata in zip(z.infolist(), expected):
            assert info.filename == metadata['name']
            n, x = struct.unpack_from('<HH', raw, info.header_offset + 26)
            data = info.header_offset + 30 + n + x
            cn, cx, cc = struct.unpack_from('<HHH', raw, at + 28)
            end = at + 46 + cn + cx + cc
            records.append(dict(header=raw[info.header_offset:data], central=raw[at:end],
                                payload=raw[data:data + info.compress_size], expected=copy.deepcopy(metadata)))
            at = end
    return records


records = read_records(source, original['entries'])
for record in records:
    e = record['expected']
    content = b'' if e['directory'] else (plain / e['name']).read_bytes()
    assert sha(content) == e['sha256'] and len(content) == e['size']
    record['content'] = content
    record['frames'] = [] if e['directory'] else [record['payload']]
empty = records[2]['payload']
tail_frame = records[5]['payload']
tail_content = records[5]['content']
records[1]['frames'].append(empty)
records[2]['frames'].append(empty)
records[3]['frames'] = [empty, records[3]['payload'], tail_frame]
records[3]['content'] += tail_content
# An independently encoded BZip2 tail checks that the next method still starts
# at the member end, rather than at one of the intermediate frame ends.
bzip = (SOURCE / 'bzip2-tail.zip').read_bytes()
with zipfile.ZipFile(io.BytesIO(bzip)) as z:
    i = z.infolist()[0]
    metadata = dict(name=i.filename, directory=False, size=i.file_size, sha256=sha(z.read(i)),
                    method=i.compress_type, dos=list(i.date_time), utc=None)
records[5] = read_records(bzip, [metadata])[0]
records[5].update(content=tail_content, frames=[])


def make_control(name, records, wide, signed):
    local = bytearray()
    central = bytearray()
    entries = []
    for record in records:
        e = copy.deepcopy(record['expected'])
        header = bytearray(record['header'])
        payload = b''.join(record['frames']) if record['frames'] else record['payload']
        crc = zlib.crc32(record['content'])
        expanded = len(record['content'])
        offset = len(local)
        if not e['directory']:
            struct.pack_into('<III', header, 14, 0, 0xffffffff if wide else 0, 0xffffffff if wide else 0)
            if wide:
                extra = struct.unpack_from('<H', header, 28)[0]
                struct.pack_into('<H', header, 28, extra + 20)
                header += struct.pack('<HHQQ', 1, 16, 0, 0)
        local += header
        data = len(local)
        local += payload
        descriptor = len(local)
        if not e['directory']:
            if signed: local += b'PK\x07\x08'
            local += struct.pack('<IQQ' if wide else '<III', crc, len(payload), expanded)
        frame_offsets = []
        at = data
        for frame in record['frames']:
            assert frame[:4] == b'\x28\xb5\x2f\xfd'
            frame_offsets.append(dict(data=at, length=len(frame)))
            at += len(frame)
        e.update(size=expanded, compressed=len(payload), crc=crc, sha256=sha(record['content']),
                 local=offset, data=data, descriptor=descriptor, end=len(local), frames=frame_offsets)
        entries.append(e)
        c = bytearray(record['central'])
        struct.pack_into('<III', c, 16, crc, len(payload), expanded)
        struct.pack_into('<I', c, 42, offset)
        central += c
    end = len(local)
    raw = local + central + struct.pack('<4s4H2IH', b'PK\x05\x06', 0, 0, len(entries), len(entries), len(central), end, 0)
    (DEST / name).write_bytes(raw)
    return dict(name=name, zip64=wide, signed=signed, sha256=sha(raw), lostSha256=sha(local),
                central=end, entries=entries, classification='metadata-derived control; actual encoded frames preserved')


manifest = dict(sources=[dict(path='../zip-zstd-rescue-1736/stream32.zip', sha256=sha(source)),
                         dict(path='../zip-zstd-rescue-1736/bzip2-tail.zip', sha256=sha(bzip))],
                recipe='Concatenate actual frames; recompute cumulative CRC/size; patch descriptor and central values/offsets; optionally widen local ZIP64 placeholders or remove optional descriptor signature.', cases=[])
for wide in [False, True]:
    for signed in [True, False]:
        name = f'multi{64 if wide else 32}-{"signed" if signed else "unsigned"}.zip'
        manifest['cases'].append(make_control(name, records, wide, signed))

# Exercise unsigned-descriptor ambiguity with an actual independently encoded
# payload whose CRC is exactly the ordinary Zstandard frame magic.
zero = zlib.crc32(bytes(4))
basis = {}
for bit in range(32):
    value, mask = zlib.crc32((1 << bit).to_bytes(4, 'little')) ^ zero, 1 << bit
    while value:
        pivot = value.bit_length() - 1
        if pivot in basis:
            v, m = basis[pivot]; value ^= v; mask ^= m
        else:
            basis[pivot] = (value, mask); break
target, answer = 0xfd2fb528 ^ zero, 0
while target:
    v, m = basis[target.bit_length() - 1]; target ^= v; answer ^= m
magic_content = answer.to_bytes(4, 'little')
assert zlib.crc32(magic_content) == 0xfd2fb528
input_path = BASE / 'crc-magic.bin'
input_path.write_bytes(magic_content)
os.utime(input_path, (1700000001, 1700000001))
r = subprocess.run(['tar', '--format', 'zip', '--options', 'zip:compression=zstd', '-cf', '-',
                    '-C', str(BASE), input_path.name], capture_output=True)
assert r.returncode == 0, r.stderr
(DEST / 'crc-magic-stream.zip').write_bytes(r.stdout)
with zipfile.ZipFile(io.BytesIO(r.stdout)) as z:
    i = z.infolist()[0]
    assert i.compress_type == 93 and i.flag_bits & 8 and i.CRC == 0xfd2fb528
    magic_metadata = dict(name=i.filename, directory=False, method=93, size=4,
                          sha256=sha(magic_content), dos=list(i.date_time), utc=1700000001)
magic_record = read_records(r.stdout, [magic_metadata])[0]
magic_record.update(content=magic_content, frames=[magic_record['payload']])
manifest['sources'].append(dict(path='crc-magic-stream.zip', sha256=sha(r.stdout),
                                producer=subprocess.run(['tar', '--version'], capture_output=True, text=True).stdout.strip()))
manifest['cases'].append(make_control('crc-magic-unsigned.zip', [magic_record], False, False))
(DEST / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(dict(controls=len(manifest['cases']), sources=len(manifest['sources']), crcMagic=hex(0xfd2fb528))))

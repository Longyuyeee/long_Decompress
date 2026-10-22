"""Controlled ALZ Bzip2 streams, NOT historical-producer interoperability evidence.

Encode self-authored deterministic data with Python/libbzip2, retain its actual
single-block Huffman/BWT payload, then frame repeated blocks as ALZ. Large mode
contains >256 MiB of real compressed data, not a rewritten size declaration.
No downloaded producer is run. Output stays in ignored test-results/.
"""
import argparse
import bz2
import hashlib
import json
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--small', action='store_true')
args = parser.parse_args()
out = ROOT / 'test-results/alz-bounded-168/fixtures'
out.mkdir(parents=True, exist_ok=True)
plain = hashlib.shake_256(b'Long v1.6.8 self-authored bounded ALZ block, public fixture v1').digest(800000)
standard = bz2.compress(plain, compresslevel=9)
bits = format(int.from_bytes(standard, 'big'), f'0{len(standard)*8}b')
magic = format(0x314159265359, '048b')
eos = bits.rfind(format(0x177245385090, '048b'))
assert bits.startswith(''.join(format(x, '08b') for x in b'BZh9') + magic)
assert bits.count(magic) == 1 and 0 <= len(bits)-eos-80 < 8
assert bits[112] == '0'  # not a randomized block
block = format(0x444c5a01, '032b') + bits[113:eos]
ending = format(0x444c5a02, '032b')
password = 'Public ALZ Bounded 168!'


def inventory(file):
    digest = hashlib.sha256()
    with file.open('rb') as reader:
        for data in iter(lambda: reader.read(1024*1024), b''):
            digest.update(data)
    return dict(name=file.name, bytes=file.stat().st_size, sha256=digest.hexdigest())


def header(name, size, packed, crc, method=1, key=False):
    name = name.encode('ascii')
    return (b'BLZ\x01' + struct.pack('<HBIBB', len(name), 0x20, 0x5cfb18c3, 0x40 | key, 0)
            + struct.pack('<BBIII', method, 0, crc, packed, size) + name)


def stream(writer, repeats):
    carry = ''
    for segment in [block]*repeats + [ending]:
        combined = carry + segment
        count = len(combined)//8
        writer.write(int(combined[:count*8], 2).to_bytes(count, 'big'))
        carry = combined[count*8:]
    if carry:
        writer.write(int(carry.ljust(8, '0'), 2).to_bytes(1, 'big'))


def encrypt(data):
    keys = [305419896, 591751049, 878082192]
    table = []
    for i in range(256):
        c = i
        for _ in range(8):
            c = (c >> 1) ^ (0xedb88320 if c & 1 else 0)
        table.append(c)

    def update(byte):
        keys[0] = ((keys[0] >> 8) ^ table[(keys[0] ^ byte) & 255]) & 0xffffffff
        keys[1] = ((keys[1] + (keys[0] & 255))*134775813+1) & 0xffffffff
        keys[2] = ((keys[2] >> 8) ^ table[(keys[2] ^ (keys[1] >> 24)) & 255]) & 0xffffffff
    for byte in password.encode():
        update(byte)
    result = bytearray()
    for byte in data:
        temp = (keys[2] | 2) & 65535
        result.append(byte ^ (((temp*(temp ^ 1)) & 65535) >> 8))
        update(byte)
    return result


def make(name, repeats, key=False):
    from io import BytesIO
    crc, sha = 0, hashlib.sha256()
    for _ in range(repeats):
        crc = zlib.crc32(plain, crc)
        sha.update(plain)
    size, packed = len(plain)*repeats, (len(block)*repeats+32+7)//8
    file = out / (name+'.alz')
    note = b'Self-authored controlled ALZ stream; not an ALZip producer original.\r\n'
    with file.open('wb') as writer:
        writer.write(b'ALZ\x01\x0a\x00\x00\x00')
        writer.write(header('payload.bin', size, packed, crc, key=key))
        if key:
            payload = BytesIO(); stream(payload, repeats)
            writer.write(encrypt(b'\x35'*11 + bytes([crc >> 24]) + payload.getvalue()))
        else:
            stream(writer, repeats)
        writer.write(header('note.txt', len(note), len(note), zlib.crc32(note), method=0))
        writer.write(note)
        writer.write(b'CLZ\x01' + b'\0'*8 + b'CLZ\x02')
    result = dict(name=name, archive=str(file.relative_to(ROOT)).replace('\\', '/'),
                  controlled=True, historicalProducer=False, repeats=repeats, compressedBytes=packed,
                  password=password if key else None, members=[inventory(file)],
                  expected=[dict(path='payload.bin', bytes=size, sha256=sha.hexdigest()),
                            dict(path='note.txt', bytes=len(note), sha256=hashlib.sha256(note).hexdigest())])
    return result


cases = [make('small', 4), make('key', 4, True)]
key_source = out/'key.alz'
data = key_source.read_bytes()
payload = data[8:-16]
pieces = [payload[:120001], payload[120001:1600003], payload[1600003:]]
members = []
for i, piece in enumerate(pieces):
    file = out/('key-volumes.' + ('alz' if i == 0 else f'a{i-1:02d}'))
    file.write_bytes(b'ALZ\x01\x0a\x00'+struct.pack('<H', i)+piece+b'CLZ\x01'+b'\0'*8+(b'CLZ\x03' if i < 2 else b'CLZ\x02'))
    members.append(inventory(file))
cases.append(dict(cases[1], name='key-volumes', archive=str((out/'key-volumes.a00').relative_to(ROOT)).replace('\\','/'), members=members))
if not args.small:
    large = make('large', 352)
    assert large['compressedBytes'] > 256*1024*1024
    cases.append(large)
manifest = dict(version='1.6.8', interpretation=__doc__.strip(),
                encoder='Python bz2 / libbzip2 standard single block, explicitly reframed by this generator',
                plainBlockBytes=len(plain), plainBlockSha256=hashlib.sha256(plain).hexdigest(),
                standardBlockSha256=hashlib.sha256(standard).hexdigest(), blockBits=len(block),
                cases=cases)
(out/'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps(dict(fixtures=str(out), cases=[dict(name=c['name'],compressedBytes=c['compressedBytes']) for c in cases]), indent=2))

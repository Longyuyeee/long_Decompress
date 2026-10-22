"""Self-authored ALZ image regression inputs; controlled framing, not producer originals."""
import bz2
import hashlib
import json
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT/'tests/fixtures/archives/alz-image-1610'
OUT.mkdir(parents=True, exist_ok=True)
HEAD = b'ALZ\x01\x0a\0\0\0'
TAIL = b'CLZ\x01'+b'\0'*8+b'CLZ\x02'
PASSWORD = '图片密码1610!'


def chunk(kind, data):
    return struct.pack('>I', len(data))+kind+data+struct.pack('>I', zlib.crc32(kind+data))


def png(width=320, height=240):
    pixels = b''.join(b'\0'+bytes(channel for x in range(width) for channel in
        (20+x*120//width, 80+y*150//height, 180+(x+y)*70//(width+height))) for y in range(height))
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))+chunk(b'IDAT', zlib.compress(pixels))+chunk(b'IEND', b'')


def bmp():
    data = bytes(channel for y in range(256) for x in range(256) for channel in
                 ((x^y), y, x))
    return b'BM'+struct.pack('<IHHI', 54+len(data), 0, 0, 54)+struct.pack('<IiiHHIIiiII', 40, 256, 256, 1, 24, 0, len(data), 2835, 2835, 0, 0)+data


def alz_bzip(data):
    standard = bz2.compress(data, compresslevel=9)
    bits = ''.join(f'{byte:08b}' for byte in standard)
    eos = bits.rfind(f'{0x177245385090:048b}')
    assert bits.count(f'{0x314159265359:048b}') == 1 and bits[112] == '0'
    framed = f'{0x444c5a01:032b}'+bits[113:eos]+f'{0x444c5a02:032b}'
    return int(framed.ljust((len(framed)+7)//8*8, '0'), 2).to_bytes((len(framed)+7)//8, 'big')


def encrypt(data):
    keys = [305419896, 591751049, 878082192]
    table = []
    for i in range(256):
        value = i
        for _ in range(8): value = (value >> 1) ^ (0xedb88320 if value & 1 else 0)
        table.append(value)
    def update(byte):
        keys[0] = ((keys[0] >> 8) ^ table[(keys[0] ^ byte) & 255]) & 0xffffffff
        keys[1] = ((keys[1]+(keys[0] & 255))*134775813+1) & 0xffffffff
        keys[2] = ((keys[2] >> 8) ^ table[(keys[2] ^ (keys[1] >> 24)) & 255]) & 0xffffffff
    for byte in PASSWORD.encode(): update(byte)
    out = bytearray()
    for byte in data:
        temp = (keys[2] | 2) & 65535
        out.append(byte ^ (((temp*(temp ^ 1)) & 65535) >> 8))
        update(byte)
    return out


def entry(name, data, method=2, key=False, crc_bad=False, truncated=False):
    name = name.encode()
    packed = zlib.compress(data)[2:-4] if method == 2 else alz_bzip(data) if method == 1 else data
    if truncated: packed = packed[:-4]
    crc = zlib.crc32(data) ^ crc_bad
    header = b'BLZ\x01'+struct.pack('<HBIBB', len(name), 32, 0x5cfb18c3, 0x40 | key, 0)+struct.pack('<BBIII', method, 0, crc, len(packed), len(data))+name
    return header+(encrypt(b'\x35'*11+bytes([crc >> 24])+packed) if key else packed)


cover, grid = png(), bmp()
items = [('相册/01-封面.png', cover), ('相册/02-网格.bmp', grid)]
cases = []
for name, method, key in [('store', 0, False), ('deflate', 2, False), ('bzip2', 1, False), ('password', 2, True)]:
    data = HEAD+entry('unrelated.bin', b'unsupported unrelated method', 3)+b''.join(entry(n, b, method, key) for n, b in items)+TAIL
    file = OUT/(name+'.alz'); file.write_bytes(data)
    cases.append(dict(name=name, method=method, controlled=True, historicalProducer=False, password=PASSWORD if key else None, path=file.name))

payload = (OUT/'password.alz').read_bytes()[8:-16]
# Real native headers/footers, split through the selected image's compressed stream.
pieces = [payload[:20001], payload[20001:40003], payload[40003:]]
assert all(pieces)
for i, part in enumerate(pieces):
    (OUT/('password-volumes.'+('alz' if i == 0 else f'a{i-1:02d}'))).write_bytes(
        b'ALZ\x01\x0a\0'+struct.pack('<H', i)+part+b'CLZ\x01'+b'\0'*8+(b'CLZ\x03' if i < 2 else b'CLZ\x02'))

at_limit = bytearray(grid)+b'\0'*(8*1024*1024-len(grid));at_limit[2:6] = struct.pack('<I', len(at_limit))
(OUT/'at-limit.alz').write_bytes(HEAD+entry('preview.bmp', at_limit)+TAIL)

negative = [('crc', cover, dict(crc_bad=True)), ('missing-end', cover, dict(method=1, truncated=True)),
            ('not-image', b'not an image', {}), ('oversize', b'x'*(8*1024*1024+1), {})]
too_wide = bytearray(cover); too_wide[16:20] = struct.pack('>I', 8193)
negative.append(('dimensions', too_wide, {}))
for name, data, options in negative:
    (OUT/(name+'.alz')).write_bytes(HEAD+entry('preview.png', data, **options)+TAIL)

ui = HEAD+entry('gallery/00-bad.png', b'not an image')+entry('gallery/01-cover.png', cover)+entry('gallery/02-grid.bmp', grid)+TAIL
(OUT/'ui.alz').write_bytes(ui)
source = ROOT/'test-results/alz-image-1610/self-authored';source.mkdir(parents=True, exist_ok=True)
for name, data in items:
    p = source/name;p.parent.mkdir(parents=True, exist_ok=True);p.write_bytes(data)
manifest = dict(version='1.6.10', controlled=True, interpretation=__doc__, password=PASSWORD,
    encoding='utf8', cases=cases, images=[dict(path=n, bytes=len(b), sha256=hashlib.sha256(b).hexdigest()) for n, b in items],
    files=[dict(path=f.name, bytes=f.stat().st_size, sha256=hashlib.sha256(f.read_bytes()).hexdigest()) for f in sorted(OUT.glob('*')) if f.name != 'manifest.json'])
(OUT/'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print('Prepared controlled image fixtures and self-authored producer input.')

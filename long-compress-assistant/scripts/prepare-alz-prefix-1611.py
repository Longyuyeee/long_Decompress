"""Controlled ALZ text prefixes; no new historical-producer interoperability claim."""
import ast
import bz2
import hashlib
import json
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT/'tests/fixtures/archives/alz-prefix-1611'
OUT.mkdir(parents=True, exist_ok=True)
CAP = 1024*1024
PASSWORD = '日志密码1611!'
HEAD = b'ALZ\x01\x0a\0\0\0'
TAIL = b'CLZ\x01'+b'\0'*8+b'CLZ\x02'
# Reuse only the self-authored cipher fixture function, with no module side effects.
tree = ast.parse((ROOT/'scripts/prepare-alz-image-1610.py').read_text(encoding='utf-8'))
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'encrypt'], type_ignores=[]), '<fixture-cipher>', 'exec'))


def bzip(data):
    blocks = []
    for pos in range(0, len(data), 500000):
        bits = ''.join(f'{b:08b}' for b in bz2.compress(data[pos:pos+500000], compresslevel=9))
        eos = bits.rfind(f'{0x177245385090:048b}')
        assert bits.count(f'{0x314159265359:048b}') == 1 and bits[112] == '0'
        blocks.append(f'{0x444c5a01:032b}'+bits[113:eos])
    bits = ''.join(blocks)+f'{0x444c5a02:032b}'
    return int(bits.ljust((len(bits)+7)//8*8, '0'), 2).to_bytes((len(bits)+7)//8, 'big')


def header(name, size, packed, crc, method=2, key=False):
    name = name.encode()
    return b'BLZ\x01'+struct.pack('<HBIBB', len(name), 32, 0x5cfb18c3, 0x40 | key, 0)+struct.pack('<BBIII', method, 0, crc, packed, size)+name


def entry(data, method=2, key=False, bad_crc=False, packed=None, declared=None):
    if packed is None:
        packed = zlib.compress(data)[2:-4] if method == 2 else bzip(data) if method == 1 else data
    crc = zlib.crc32(data) ^ bad_crc
    return header('logs/session.txt', declared or len(data), len(packed), crc, method, key)+(encrypt(b'\x35'*11+bytes([crc >> 24])+packed) if key else packed)


log = ('INFO 2026-10-23 02:00:00 服务已就绪，读取日志开头。\n'.encode()*40000)
cases = []
for name, data, method, key, bad, packed, declared in [
    ('store', log, 0, False, False, None, None),
    ('deflate', log, 2, False, False, None, None),
    ('bzip2', log, 1, False, False, None, None),
    ('password', log, 2, True, False, None, None),
    ('utf8-boundary', b'A'*(CAP-1)+'中'.encode()+b'next'*1000, 2, False, False, None, None),
    ('utf16-boundary', b'\xff\xfe'+b'A\0'*((CAP-4)//2)+'😀后续'.encode('utf-16-le'), 2, False, False, None, None),
    ('gbk-boundary', b'G'+'中文日志'.encode('gbk')*(CAP//4), 2, False, False, None, None),
    ('bad-crc-later', log, 2, False, True, None, None),
    ('short-before-prefix', b'Short body\n'*10000, 2, False, False, None, len(log)),
]:
    # List the unrelated unsupported entry; never decode it for a selected text.
    unrelated = header('unrelated.bin', 7, 7, zlib.crc32(b'ignored'), 3)+b'ignored'
    file = OUT/(name+'.alz');file.write_bytes(HEAD+unrelated+entry(data, method, key, bad, packed, declared)+TAIL)
    cases.append(dict(name=name, method=method, bytes=len(data), totalSize=declared or len(data), prefixSha256=hashlib.sha256(data[:CAP]).hexdigest(), controlled=True, password=PASSWORD if key else None))
payload = (OUT/'password.alz').read_bytes()[8:-16]
points = [0, len(payload)//3, len(payload)*2//3, len(payload)]
for i in range(3):
    (OUT/('password-volumes.'+('alz' if i == 0 else f'a{i-1:02d}'))).write_bytes(
        b'ALZ\x01\x0a\0'+struct.pack('<H', i)+payload[points[i]:points[i+1]]+b'CLZ\x01'+b'\0'*8+(b'CLZ\x03' if i < 2 else b'CLZ\x02'))

manifest = dict(version='1.6.11', controlled=True, historicalProducer=False, interpretation=__doc__, password=PASSWORD, encoding='utf8', cases=cases,
    files=[dict(path=p.name, bytes=p.stat().st_size, sha256=hashlib.sha256(p.read_bytes()).hexdigest()) for p in sorted(OUT.glob('*')) if p.name != 'manifest.json'])
(OUT/'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')

# Actual larger compressed inputs stay outside Git. One returns its early prefix;
# another must stop at the scan budget before any useful text is produced.
large = ROOT/'test-results/alz-prefix-1611';large.mkdir(parents=True, exist_ok=True)
plain = b'INFO large stored text\n'*(70*1024*1024//23+1)
(large/'large-store.alz').write_bytes(HEAD+entry(plain, 0)+TAIL)
empty_blocks = b'\0\0\0\xff\xff'*(64*1024*1024//5+1)
(large/'scan-budget.alz').write_bytes(HEAD+entry(log, packed=empty_blocks+zlib.compress(log)[2:-4])+TAIL)
print('Prepared controlled prefixes and two actual >64 MiB compressed-input cases.')

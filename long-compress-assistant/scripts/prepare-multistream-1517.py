"""Generate real concatenated codec streams using independent system producers.

Run with WSL Python; only writes the specified new fixture directory.
"""
import bz2
import ctypes
import gzip
import hashlib
import io
import json
import lzma
from pathlib import Path
import platform
import subprocess
import sys
import tarfile
import tempfile

dest = Path(sys.argv[1]).resolve()
dest.mkdir(parents=True, exist_ok=True)
if (dest / 'manifest.json').exists():
    raise SystemExit('Original manifest exists; preserve it or use a fresh directory')
sha = lambda data: hashlib.sha256(data).hexdigest()
lib = ctypes.CDLL('libzstd.so.1')
lib.ZSTD_versionString.restype = ctypes.c_char_p
lib.ZSTD_compressBound.argtypes = [ctypes.c_size_t]
lib.ZSTD_compressBound.restype = ctypes.c_size_t
lib.ZSTD_createCCtx.restype = ctypes.c_void_p
lib.ZSTD_CCtx_setParameter.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
lib.ZSTD_CCtx_setParameter.restype = ctypes.c_size_t
lib.ZSTD_compress2.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t]
lib.ZSTD_compress2.restype = ctypes.c_size_t
lib.ZSTD_freeCCtx.argtypes = [ctypes.c_void_p]
lib.ZSTD_isError.argtypes = [ctypes.c_size_t]
lib.ZSTD_isError.restype = ctypes.c_uint

def zstd(data):
    ctx = lib.ZSTD_createCCtx()
    assert ctx
    try:
        # ZSTD_c_checksumFlag, from the system zstd.h API. No custom frame headers.
        assert not lib.ZSTD_isError(lib.ZSTD_CCtx_setParameter(ctx, 201, 1))
        out = ctypes.create_string_buffer(lib.ZSTD_compressBound(len(data)))
        size = lib.ZSTD_compress2(ctx, out, len(out), data, len(data))
        assert not lib.ZSTD_isError(size)
        return out.raw[:size]
    finally:
        lib.ZSTD_freeCCtx(ctx)

encoders = {'gz': lambda b: gzip.compress(b, mtime=0), 'bz2': bz2.compress,
            'xz': lzma.compress, 'zst': zstd}
text = ('开头必须保留。\n' + ''.join(f'第 {i:04d} 行：连续压缩流的完整内容。\n' for i in range(900)) + '最后一段：用户需要的末尾内容。\n').encode()
state = 1517
payload = bytearray()
for _ in range(131089):
    state ^= (state << 13) & 0xffffffff
    state ^= state >> 17
    state ^= (state << 5) & 0xffffffff
    payload.append(state & 255)
tail = '末尾文件：跨越压缩流边界，内容不能丢。\n'.encode() * 47
with tempfile.TemporaryDirectory(prefix='long-multistream1517-') as scratch:
    source = Path(scratch)
    (source / '开头.txt').write_text('第一份说明。\n', encoding='utf-8')
    (source / '数据.bin').write_bytes(payload)
    (source / '末尾.txt').write_bytes(tail)
    (source / '空目录').mkdir()
    archive = source / 'original.tar'
    producer = subprocess.check_output(['tar', '--version'], text=True).splitlines()[0]
    args = ['tar', '--format=pax', '--mtime=@1577934245', '--owner=0', '--group=0', '--numeric-owner',
            '--pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime',
            '-cf', str(archive), '-C', str(source), '开头.txt', '数据.bin', '末尾.txt', '空目录']
    subprocess.run(args, check=True)
    raw = archive.read_bytes()
    with tarfile.open(fileobj=io.BytesIO(raw)) as tf:
        cut1 = tf.getmember('数据.bin').offset_data + 3073
        cut2 = tf.getmember('末尾.txt').offset_data + 13
    tar_parts = [raw[:cut1], raw[cut1:cut2], raw[cut2:]]
    plain_parts = [text[:7], b'', text[7:len(text)-31], text[len(text)-31:]]
    (dest / 'original.tar').write_bytes(raw)
    (dest / '说明.txt').write_bytes(text)
    expected = [{'path': n, 'bytes': len(b), 'sha256': sha(b)} for n, b in
                [('开头.txt', (source / '开头.txt').read_bytes()), ('数据.bin', payload), ('末尾.txt', tail)]]
    cases = []
    for codec, encode in encoders.items():
        for kind, parts, expected_raw, name in [('plain', plain_parts, text, f'说明.txt.{codec}'),
                                               ('tar', tar_parts, raw, f'跨段.tar.{codec}')]:
            members = [encode(p) for p in parts]
            # XZ stream padding is in multiples of four bytes.
            pad = b'\0' * 4 if codec == 'xz' else b''
            data = pad.join(members)
            (dest / name).write_bytes(data)
            truncated = name + '.truncated'
            (dest / truncated).write_bytes(data[:-5])
            corrupt = name + '.corrupt'
            bad = bytearray(data)
            bad[-9 if codec == 'xz' else -6] ^= 0x40
            (dest / corrupt).write_bytes(bad)
            cases.append({'codec': codec, 'kind': kind, 'archive': name, 'bytes': len(data), 'sha256': sha(data),
                          'members': len(members), 'xzPaddingBytes': 4 if pad else 0,
                          'decodedBytes': len(expected_raw), 'decodedSha256': sha(expected_raw),
                          'truncated': truncated, 'corrupt': corrupt})
        concat = f'两个完整TAR.tar.{codec}'
        (dest / concat).write_bytes(encode(raw) + (b'\0'*4 if codec == 'xz' else b'') + encode(raw))
    (dest / 'manifest.json').write_text(json.dumps({'python': platform.python_version(), 'tarProducer': producer,
        'zstdProducer': lib.ZSTD_versionString().decode(), 'tarArguments': args, 'tarCuts': [cut1,cut2],
        'expected': expected, 'emptyDirectory': '空目录', 'cases': cases}, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print('Generated 8 real multistream originals, 16 derived faults and 4 separate-TAR concatenations.')

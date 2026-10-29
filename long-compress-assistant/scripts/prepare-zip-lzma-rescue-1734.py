"""Independent CPython streaming ZIP originals and clearly labelled controls."""
from pathlib import Path
import lzma
import hashlib
import io
import json
import random
import struct
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'tests/fixtures/archives/zip-lzma-rescue-1734'
DEST.mkdir(parents=True, exist_ok=True)
sha = lambda b: hashlib.sha256(b).hexdigest()


class Streaming(io.BytesIO):
    def seek(self, *args):
        raise io.UnsupportedOperation('deliberately non-seekable ZIP producer')


payloads = [
    ('资料/', b'', zipfile.ZIP_DEFLATED),
    ('资料/说明.txt', ('流式 ZIP 完整内容。PK\x03\x04PK\x07\x08\n' * 100000).encode(), zipfile.ZIP_LZMA),
    ('资料/empty.txt', b'', zipfile.ZIP_LZMA),
    ('资料/blob.bin', random.Random(1734).randbytes(200000), zipfile.ZIP_LZMA),
    ('空目录/', b'', zipfile.ZIP_LZMA),
    ('tail.txt', b'existing BZip2 after LZMA\n' * 300, zipfile.ZIP_BZIP2),
]
receipt = {'producer': f'CPython {sys.version.split()[0]} zipfile + stdlib liblzma; non-seekable output', 'originals': [], 'controls': []}
for zip64 in [False, True]:
    sink = Streaming()
    with zipfile.ZipFile(sink, 'w', allowZip64=True) as writer:
        for name, content, method in payloads:
            info = zipfile.ZipInfo(name, (2024, 5, 6, 7, 8, 10))
            info.compress_type = method
            info.external_attr = ((0o40755 << 16) | 0x10) if name.endswith('/') else (0o100644 << 16)
            with writer.open(info, 'w', force_zip64=zip64) as member:
                # The producer cannot backpatch local sizes, including empty directories.
                for at in range(0, len(content), 7777):
                    member.write(content[at:at + 7777])
    raw = sink.getvalue()
    name = f'stream{64 if zip64 else 32}.zip'
    (DEST / name).write_bytes(raw)
    entries = []
    with zipfile.ZipFile(io.BytesIO(raw)) as source:
        assert source.testzip() is None
        for info, (_, content, method) in zip(source.infolist(), payloads):
            local = info.header_offset
            flags, actual_method = struct.unpack_from('<HH', raw, local + 6)
            assert flags & 8 and not flags & 1 and actual_method == method
            assert bool(flags & 2) == (method == zipfile.ZIP_LZMA)
            compressed, expanded = struct.unpack_from('<II', raw, local + 18)
            assert (compressed, expanded) == ((0xffffffff, 0xffffffff) if zip64 else (0, 0))
            name_len, extra_len = struct.unpack_from('<HH', raw, local + 26)
            data = local + 30 + name_len + extra_len
            descriptor = data + info.compress_size
            assert raw[descriptor:descriptor + 4] == b'PK\x07\x08'
            descriptor_values = struct.unpack_from('<IQQ' if zip64 else '<III', raw, descriptor + 4)
            assert descriptor_values == (info.CRC, info.compress_size, info.file_size)
            assert source.read(info) == content
            if method == zipfile.ZIP_LZMA:
                assert struct.unpack_from('<H', raw, data + 2)[0] == 5
                props = lzma._decode_filter_properties(lzma.FILTER_LZMA1, raw[data + 4:data + 9])
                decoder = lzma.LZMADecompressor(lzma.FORMAT_RAW, filters=[props])
                assert decoder.decompress(raw[data + 9:]) == content and decoder.eof
                assert len(raw[data + 9:]) - len(decoder.unused_data) + 9 == info.compress_size
            entries.append({'name': info.filename, 'directory': info.is_dir(), 'method': method,
                            'size': info.file_size, 'compressed': info.compress_size, 'crc': info.CRC,
                            'sha256': sha(content), 'local': local, 'data': data, 'descriptor': descriptor,
                            'end': descriptor + (24 if zip64 else 16)})
        central = source.start_dir
    receipt['originals'].append({'name': name, 'zip64': zip64, 'sha256': sha(raw), 'central': central, 'entries': entries})
    lost = raw[:central]
    assert entries[-1]['end'] == central
    lost_name = f'lost{64 if zip64 else 32}.zip'
    (DEST / lost_name).write_bytes(lost)
    unsigned = b''.join(raw[e['local']:e['descriptor']] + raw[e['descriptor'] + 4:e['end']] for e in entries)
    unsigned_name = f'unsigned{64 if zip64 else 32}.zip'
    (DEST / unsigned_name).write_bytes(unsigned)
    for control, value, description in [(lost_name, lost, 'central directory removed; signed descriptors unchanged'),
                                        (unsigned_name, unsigned, 'central directory and optional descriptor signatures removed')]:
        receipt['controls'].append({'name': control, 'original': name, 'sha256': sha(value), 'description': description})
(DEST / 'producer.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'originals': len(receipt['originals']), 'controls': len(receipt['controls']), 'entriesPerOriginal': len(payloads)}))

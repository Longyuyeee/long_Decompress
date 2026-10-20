"""Independent Python LZMA-alone originals, not application-produced round trips."""
import hashlib, json, lzma, random
from pathlib import Path

root = Path(__file__).resolve().parents[1]
base = root / 'tests/fixtures/archives/lzma-1523'
base.mkdir(parents=True, exist_ok=True)
text = ('收到 LZMA 分卷后，可从任意卷查看、预览和提取。\n' * 2048).encode()
payloads = {'说明.txt': text, '空文件.txt': b'', '图标.png': (root / 'src-tauri/icons/32x32.png').read_bytes(),
            '数据.bin': random.Random(1523).randbytes(33001), '大文本.txt': b'LZMA bounded preview\n' * (9 * 1024 * 1024 // 21 + 1)}
cases = []
for name, data in payloads.items():
    encoded = lzma.compress(data, format=lzma.FORMAT_ALONE, preset=1)
    (base / (name + '.lzma')).write_bytes(encoded)
    cases.append({'archive': name + '.lzma', 'name': name, 'size': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
for name, known, filters in [('其他属性.txt', False, [{'id': lzma.FILTER_LZMA1, 'dict_size': 1 << 20, 'lc': 2, 'lp': 0, 'pb': 1}])]:
    encoded = bytearray(lzma.compress(text, format=lzma.FORMAT_ALONE, filters=filters))
    if known: encoded[5:13] = len(text).to_bytes(8, 'little')
    assert lzma.decompress(encoded, format=lzma.FORMAT_ALONE) == text
    (base / (name + '.lzma')).write_bytes(encoded)
    cases.append({'archive': name + '.lzma', 'name': name, 'size': len(text), 'sha256': hashlib.sha256(text).hexdigest()})
(base / 'manifest.json').write_text(json.dumps({'producer': 'Python stdlib lzma FORMAT_ALONE', 'cases': cases}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

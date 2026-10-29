"""Actual libarchive streaming Zstandard ZIP; labelled ZIP64/unsigned/mixed controls."""
from pathlib import Path
import hashlib, io, json, os, random, struct, subprocess, zipfile
ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'tests/fixtures/archives/zip-zstd-rescue-1736'
WORK = ROOT / 'test-results/zip-zstd-rescue-1736/producer-input'
DEST.mkdir(parents=True, exist_ok=True)
WORK.mkdir(parents=True, exist_ok=True)
sha = lambda b: hashlib.sha256(b).hexdigest()
payloads = [('docs/', b''), ('docs/note.txt', b'recovered Zstandard content PK\x03\x04PK\x07\x08\n' * 100000),
            ('docs/empty.txt', b''), ('docs/blob.bin', random.Random(1736).randbytes(200000)),
            ('empty-dir/', b''), ('tail.txt', b'last real Zstandard member\n' * 300)]
for index, (name, data) in enumerate(payloads):
    path = WORK / name
    if name.endswith('/'): path.mkdir(exist_ok=True)
    else: path.write_bytes(data)
# Apply directories last, so child creation cannot change their intended dates.
for index, (name, _) in reversed(list(enumerate(payloads))):
    os.utime(WORK / name, (1700000001 + index * 70, 1700000001 + index * 70))
command = ['tar', '--format', 'zip', '--options', 'zip:compression=zstd', '--no-recursion',
           '-cf', '-', '-C', str(WORK), *[n.rstrip('/') for n, _ in payloads]]
r = subprocess.run(command, capture_output=True)
assert r.returncode == 0, repr(r.stderr)
raw = r.stdout
version = subprocess.run(['tar', '--version'], capture_output=True, text=True).stdout.strip()
(DEST / 'stream32.zip').write_bytes(raw)

def records(raw, contents):
    rows = []
    with zipfile.ZipFile(io.BytesIO(raw)) as z:
        for info, (name, content) in zip(z.infolist(), contents):
            assert info.filename == name
            local = info.header_offset
            n, x = struct.unpack_from('<HH', raw, local + 26)
            data = local + 30 + n + x
            descriptor = data + info.compress_size
            is64 = struct.unpack_from('<I', raw, local + 18)[0] == 0xffffffff
            end = descriptor + ((24 if is64 else 16) if info.flag_bits & 8 else 0)
            if info.flag_bits & 8:
                assert raw[descriptor:descriptor+4] == b'PK\x07\x08'
                assert struct.unpack_from('<IQQ' if is64 else '<III', raw, descriptor+4) == (info.CRC, info.compress_size, info.file_size)
                assert info.compress_type == 93
                assert raw[data:data+4] == b'\x28\xb5\x2f\xfd'
            rows.append(dict(name=name, directory=info.is_dir(), method=info.compress_type, size=len(content),
                             compressed=info.compress_size, crc=info.CRC, sha256=sha(content), local=local,
                             data=data, descriptor=descriptor, end=end, dos=list(info.date_time), utc=1700000001+len(rows)*70))
        central = z.start_dir
    assert rows[-1]['end'] == central
    return rows, central

entries, central = records(raw, payloads)
assert [e['method'] for e in entries] == [0,93,93,93,0,93]
receipt = dict(producer=version, command=command[:9]+['<fixed input>', '<explicit ordered entries>'],
               originals=[dict(name='stream32.zip', sha256=sha(raw), central=central, entries=entries)], variants=[], controls=[])
# ZIP64-local control: real payload bytes and CRC stay untouched. Widen only
# local unknown-size placeholders and descriptors; patch central offsets.
local64 = bytearray(); offsets = []
for e in entries:
    offsets.append(len(local64))
    h = bytearray(raw[e['local']:e['data']]); n, x = struct.unpack_from('<HH',h,26)
    if not e['directory']:
        struct.pack_into('<II',h,18,0xffffffff,0xffffffff)
        struct.pack_into('<H',h,28,x+20)
        h += struct.pack('<HHQQ',1,16,0,0)
    local64 += h + raw[e['data']:e['descriptor']]
    if not e['directory']: local64 += b'PK\x07\x08'+struct.pack('<IQQ',e['crc'],e['compressed'],e['size'])
cd = bytearray(); at = central
for offset in offsets:
    n,x,c=struct.unpack_from('<HHH',raw,at+28); end=at+46+n+x+c
    h=bytearray(raw[at:end]); struct.pack_into('<I',h,42,offset); cd+=h; at=end
full64=bytes(local64+cd+struct.pack('<4s4H2IH',b'PK\x05\x06',0,0,len(entries),len(entries),len(cd),len(local64),0))
(DEST/'stream64-control.zip').write_bytes(full64)
receipt['controls'].append(dict(name='stream64-control.zip',original='stream32.zip',sha256=sha(full64),description='real payloads; local unknown-size ZIP64 placeholders and wide descriptors; patched central offsets'))
for is64, original, name in [(False,raw,'stream32.zip'),(True,full64,'stream64-control.zip')]:
    rows, end = records(original,payloads)
    receipt['variants'].append(dict(name=name,zip64=is64,sha256=sha(original),central=end,entries=rows))
    lost=original[:end]
    unsigned=b''.join(original[e['local']:e['descriptor']]+(original[e['descriptor']+4:e['end']] if not e['directory'] else b'') for e in rows)
    for prefix, value, recipe in [('lost',lost,'remove central directory'),('unsigned',unsigned,'remove central directory and optional data descriptor signatures')]:
        control=f'{prefix}{64 if is64 else 32}.zip';(DEST/control).write_bytes(value)
        receipt['controls'].append(dict(name=control,original=name,sha256=sha(value),description=recipe))
# Real CPython BZip2 tail demonstrates method changes after Zstandard.
class Streaming(io.BytesIO):
    def seek(self,*args): raise io.UnsupportedOperation('non-seekable')
sink=Streaming(); tail=payloads[-1][1]
with zipfile.ZipFile(sink,'w') as z:
    i=zipfile.ZipInfo('tail.txt',(2024,5,6,7,8,10));i.compress_type=zipfile.ZIP_BZIP2
    z.writestr(i,tail)
bzip=sink.getvalue();(DEST/'bzip2-tail.zip').write_bytes(bzip)
with zipfile.ZipFile(io.BytesIO(bzip)) as z: bzip_end=z.start_dir; assert z.read('tail.txt')==tail
receipt['originals'].append(dict(name='bzip2-tail.zip',sha256=sha(bzip),producer='CPython zipfile + stdlib bz2; non-seekable'))
mixed=raw[:entries[-1]['local']]+bzip[:bzip_end]
(DEST/'mixed.zip').write_bytes(mixed)
receipt['controls'].append(dict(name='mixed.zip',original=['stream32.zip','bzip2-tail.zip'],sha256=sha(mixed),description='last local member replaced by actual BZip2 local record; no central directory'))
(DEST/'producer.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(originals=len(receipt['originals']),controls=len(receipt['controls']),entries=len(entries),producer=version)))

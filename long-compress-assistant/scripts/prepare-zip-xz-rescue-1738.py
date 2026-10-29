"""Independent 7-Zip stdout ZIPX and clearly labelled stream/descriptor controls."""
from pathlib import Path
import copy,hashlib,io,json,lzma,os,random,struct,subprocess,zipfile,zlib
ROOT=Path(__file__).resolve().parents[1]
DEST=ROOT/'tests/fixtures/archives/zip-xz-rescue-1738'
BASE=ROOT/'test-results/zip-xz-rescue-1738'
INPUT=BASE/'producer-input'
DEST.mkdir(parents=True,exist_ok=True);INPUT.mkdir(parents=True,exist_ok=True)
SEVEN=ROOT/'src-tauri/resources/archive-engine/7z.exe'
sha=lambda b:hashlib.sha256(b).hexdigest()
payloads=[('docs/',b''),('docs/01-note.txt',b'XZ ZIPX complete content PK\x03\x04PK\x07\x08\n'*100000),
 ('docs/02-empty.txt',b''),('docs/03-blob.bin',random.Random(1738).randbytes(200000)),('empty-dir/',b''),('tail.txt',b'existing BZip2 after XZ\n'*300)]
for name,data in payloads:
 p=INPUT/name
 if name.endswith('/'):p.mkdir(exist_ok=True)
 else:p.write_bytes(data)
for index,(name,_) in reversed(list(enumerate(payloads))):os.utime(INPUT/name,(1700000001+index*70,1700000001+index*70))
args=[str(SEVEN),'a','-tzip','-so','-mm=XZ','-bso2','-bsp0','-mtc=off','-mta=off','unused.zipx','docs','empty-dir','tail.txt']
r=subprocess.run(args,cwd=INPUT,capture_output=True);assert r.returncode==0,r.stderr
raw=r.stdout;(DEST/'stream-xz.zipx').write_bytes(raw)
metadata=[]
with zipfile.ZipFile(io.BytesIO(raw)) as z:
 for info in z.infolist():
  content=dict(payloads)[info.filename]
  metadata.append(dict(name=info.filename,directory=info.is_dir(),method=info.compress_type,size=info.file_size,
   crc=info.CRC,sha256=sha(content),dos=list(info.date_time),utc=None))

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
            struct.pack_into('<HHH',header,4,63,8,e['method'])
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
        if not e['directory']: struct.pack_into('<HHH',c,6,63,8,e['method'])
        struct.pack_into('<III', c, 16, crc, len(payload), expanded)
        struct.pack_into('<I', c, 42, offset)
        central += c
    end = len(local)
    raw = local + central + struct.pack('<4s4H2IH', b'PK\x05\x06', 0, 0, len(entries), len(entries), len(central), end, 0)
    (DEST / name).write_bytes(raw)
    return dict(name=name, zip64=wide, signed=signed, sha256=sha(raw), lostSha256=sha(local),
                central=end, entries=entries, classification='metadata-derived control; actual encoded XZ streams preserved')


records_by_name={r['expected']['name']:r for r in read_records(raw,metadata)}
records=[]
for name,content in payloads:
 r=records_by_name[name];r.update(content=content,frames=[])
 e=r['expected'];assert e['size']==len(content) and e['crc']==zlib.crc32(content)
 if e['method']==95:
  decoder=lzma.LZMADecompressor(lzma.FORMAT_XZ);decoded=decoder.decompress(r['payload']+b'descriptor lookahead')
  assert decoder.eof and decoder.unused_data==b'descriptor lookahead' and decoded==content
 records.append(r)
assert records[1]['expected']['method']==records[3]['expected']['method']==95
empty=lzma.compress(b'',format=lzma.FORMAT_XZ,check=lzma.CHECK_CRC64)
(DEST/'empty.xz').write_bytes(empty)
records[2]['payload']=empty;records[2]['expected']['method']=95
# Keep a separately encoded BZip2 tail so the next member uses a different decoder.
class Streaming(io.BytesIO):
 def seek(self,*args):raise io.UnsupportedOperation('non-seekable')
sink=Streaming()
with zipfile.ZipFile(sink,'w') as z:
 i=zipfile.ZipInfo('tail.txt',(2024,5,6,7,8,10));i.compress_type=zipfile.ZIP_BZIP2;z.writestr(i,payloads[5][1])
bzip=sink.getvalue();(DEST/'bzip-tail.zip').write_bytes(bzip)
with zipfile.ZipFile(io.BytesIO(bzip)) as z:
 i=z.infolist()[0];e=dict(name=i.filename,directory=False,method=12,size=i.file_size,crc=i.CRC,sha256=sha(z.read(i)),dos=list(i.date_time),utc=None)
records[5]=read_records(bzip,[e])[0];records[5].update(content=payloads[5][1],frames=[])
manifest=dict(sources=[dict(path='stream-xz.zipx',sha256=sha(raw),producer='7-Zip 26.04; stdout non-seekable; actual method 95 with zero local sizes'),
 dict(path='empty.xz',sha256=sha(empty),producer='CPython stdlib liblzma FORMAT_XZ/CHECK_CRC64 actual empty stream'),
 dict(path='bzip-tail.zip',sha256=sha(bzip),producer='CPython zipfile/bz2 non-seekable')],
 recipe='Preserve actual compressed streams; reorder records; replace empty Store payload with real empty XZ stream; recalculate descriptors/central offsets; widen ZIP64 local placeholders or omit descriptor signatures.',cases=[])
for wide in [False,True]:
 for signed in [True,False]:
  name=f'xz{64 if wide else 32}-{"signed" if signed else "unsigned"}.zipx'
  manifest['cases'].append(make_control(name,records,wide,signed))
(DEST/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(sources=3,controls=4,methods=[r['expected']['method'] for r in records])))

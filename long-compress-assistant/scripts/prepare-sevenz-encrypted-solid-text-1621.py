"""Original 7-Zip shared text, independent readback; preserve existing receipts."""
import hashlib, json, subprocess
from pathlib import Path

base=Path('tests/fixtures/archives/sevenz-encrypted-solid-text-1621');work=Path('test-results/sevenz-encrypted-solid-text-1621')
base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=base/'manifest.json'
if manifest.exists():
    m=json.loads(manifest.read_text(encoding='utf-8'))
    for c in m['cases']:
        for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
    if 'controlledBzip2Header' in m:assert sha(base/'controlled-bzip2-header.7z')==m['controlledBzip2Header']['sha256']
    print('Existing encrypted originals retained');raise SystemExit

def run(exe,args,cwd=None):
    r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=90)
    assert r.returncode==0,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace')
    return r.stdout.decode('utf-8','replace')

source=(work/'source').resolve();(source/'docs/empty-directory').mkdir(parents=True,exist_ok=True)
text=('7Z 固实包里的说明、日志，直接在双栏阅读。\r\n'*180).encode()
files={'docs/01-before.txt':b'Before selected\r\n'*300,'docs/02-说明 [1].txt':text,
       'docs/02-说明 1.txt':b'NOT the selected bracket path','docs/03-after.txt':b'After selected\r\n'*500,
       'docs/empty.txt':b'','payload.bin':hashlib.shake_256(b'encrypted1621').digest(48000)}
for name,b in files.items():(source/name).write_bytes(b)
key='7Z加密固实1621!';cases=[]
common=['a','-t7z','-y','-sccUTF-8','-mtc=off','-mta=off','-mtm=off','-mhc=on','-ms=128m']
for name,method,header,volume in [('content-lzma','LZMA',False,False),('content-lzma2','LZMA2',False,False),('headers','LZMA2',True,False),('headers-volumes','LZMA',True,True)]:
    password=key
    dest=(base/name).resolve();dest.mkdir(exist_ok=True);archive=dest/'资料.7z'
    args=[*common,'-m0='+method,*(['-p'+password,'-mhe='+('on' if header else 'off')] if password else []),*(['-v12k'] if volume else []),archive,'.']
    run(producer,args,source)
    members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())]
    out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True)
    run(independent,['x','-y','-sccUTF-8','-o'+str(out),*(['-p'+password] if password else []),dest/members[0]['name']])
    for f,b in files.items():assert sha(out/f)==hashlib.sha256(b).hexdigest()
    listing=run(producer,['l','-slt','-sccUTF-8',*(['-p'+password] if password else []),dest/members[0]['name']])
    assert 'Solid = +' in listing and 'Blocks = 1' in listing
    cases.append({'name':name,'method':method,'password':password,'members':members,'originalArgs':list(map(str,args)),'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
# Ciphertext damage with untouched metadata; no partial text may be published.
original=base/'content-lzma2/资料.7z';data=bytearray(original.read_bytes());data[48]^=1;(base/'damaged-data.7z').write_bytes(data)
large=(work/'large-source').resolve();large.mkdir(exist_ok=True)
with (large/'huge-outside.bin').open('wb') as f:f.truncate(65*1024*1024)
(large/'readme.txt').write_bytes(text);over=base/'oversize-shared.7z';run(producer,[*common,'-p'+key,'-m0=LZMA2',over.resolve(),'.'],large);listing=run(producer,['l','-slt','-p'+key,over.resolve()]);assert 'Blocks = 1' in listing;run(independent,['t','-p'+key,over.resolve()])
other=(work/'unrelated.7z').resolve();run(producer,['a','-t7z','-y','-m0=Copy','-ms=off',other,'huge-outside.bin'],large);run(producer,[*common,'-p'+key,'-m0=LZMA2',other,'.'],source);run(independent,['t','-p'+key,other])
# A corrupt unrelated stored block still must not be read during metadata/text preview.
with other.open('r+b') as f:f.seek(64);b=f.read(1);f.seek(64);f.write(bytes([b[0]^1]))
# Controlled Bzip2 encoded-header compatibility, independently decoded by 7-Zip.
# This is not claimed to be a newly acquired original Bzip2-header producer file.
import bz2,zlib
raw=(work/'raw-header.7z').resolve();run(producer,['a','-t7z','-y','-p'+key,'-m0=LZMA2','-ms=on','-mhc=off',raw,'.'],source)
b=bytearray(raw.read_bytes());offset=int.from_bytes(b[12:20],'little');size=int.from_bytes(b[20:28],'little');metadata=bytes(b[32+offset:32+offset+size]);assert metadata[0]==1
compressed=bz2.compress(metadata)
def uint(n):
 for i in range(9):
  if i==8:return bytes([255])+n.to_bytes(8,'little')
  if n<1<<(7+7*i):return bytes([((255<<(8-i))&255)|(n>>(8*i))])+n.to_bytes(8,'little')[:i]
desc=b'\x17\x06'+uint(offset)+b'\x01\x09'+uint(len(compressed))+b'\x00\x07\x0b\x01\x00\x01\x03\x04\x02\x02\x0c'+uint(len(metadata))+b'\x0a\x01'+zlib.crc32(metadata).to_bytes(4,'little')+b'\x00\x00'
start=(offset+len(compressed)).to_bytes(8,'little')+len(desc).to_bytes(8,'little')+zlib.crc32(desc).to_bytes(4,'little')
controlled=base/'controlled-bzip2-header.7z';controlled.write_bytes(bytes(b[:8])+zlib.crc32(start).to_bytes(4,'little')+start+bytes(b[32:32+offset])+compressed+desc)
run(independent,['t','-p'+key,controlled.resolve()])
manifest.write_text(json.dumps({'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'cases':cases,'files':[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()],'textSha256':hashlib.sha256(text).hexdigest(),'controlledBzip2Header':{'sha256':sha(controlled),'independentReadback':True,'provenance':'Controlled encoding of an owned original header, not an original producer case'},'damagedDataSha256':sha(base/'damaged-data.7z'),'oversizeSharedSha256':sha(over),'largeUnrelatedIgnored':str(other)},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('4 encrypted solid originals independently read back; shared limit and damaged unrelated scope prepared')

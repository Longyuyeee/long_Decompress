"""Original 7-Zip shared text, independent readback; preserve existing receipts."""
import hashlib, json, subprocess
from pathlib import Path

base=Path('tests/fixtures/archives/sevenz-solid-text-1619');work=Path('test-results/sevenz-solid-text-1619')
base.mkdir(parents=True,exist_ok=True);work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe');independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=base/'manifest.json'
if manifest.exists():
    m=json.loads(manifest.read_text(encoding='utf-8'))
    for c in m['cases']:
        for f in c['members']:assert sha(base/c['name']/f['name'])==f['sha256']
    print('Existing shared originals retained');raise SystemExit

def run(exe,args,cwd=None):
    r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=90)
    assert r.returncode==0,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace')
    return r.stdout.decode('utf-8','replace')

source=(work/'source').resolve();(source/'docs/empty-directory').mkdir(parents=True,exist_ok=True)
text=('7Z 固实包里的说明、日志，直接在双栏阅读。\r\n'*180).encode()
files={'docs/01-before.txt':b'Before selected\r\n'*300,'docs/02-说明 [1].txt':text,
       'docs/02-说明 1.txt':b'NOT the selected bracket path','docs/03-after.txt':b'After selected\r\n'*500,
       'docs/empty.txt':b'','payload.bin':hashlib.shake_256(b'shared1619').digest(48000)}
for name,b in files.items():(source/name).write_bytes(b)
key='7Z固实公开测试1619!';cases=[]
common=['a','-t7z','-y','-sccUTF-8','-mtc=off','-mta=off','-mtm=off','-mhc=off','-ms=on']
for name,method,password,volume in [('lzma','LZMA',None,False),('lzma2','LZMA2',None,False),('volumes','LZMA2',None,True),('encrypted','LZMA2',key,False)]:
    dest=(base/name).resolve();dest.mkdir(exist_ok=True);archive=dest/'资料.7z'
    args=[*common,'-m0='+method,*(['-p'+password,'-mhe=off'] if password else []),*(['-v12k'] if volume else []),archive,'.']
    run(producer,args,source)
    members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())]
    out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True)
    run(independent,['x','-y','-sccUTF-8','-o'+str(out),*(['-p'+password] if password else []),dest/members[0]['name']])
    for f,b in files.items():assert sha(out/f)==hashlib.sha256(b).hexdigest()
    listing=run(producer,['l','-slt','-sccUTF-8',*(['-p'+password] if password else []),dest/members[0]['name']])
    assert 'Solid = +' in listing and 'Blocks = 1' in listing
    cases.append({'name':name,'method':method,'password':password,'members':members,'originalArgs':list(map(str,args)),'independentReadback':True,'listing':[s for s in listing.splitlines() if s.startswith(('Method =','Solid =','Blocks ='))]})
# Corrupt only the LZMA2 terminal marker; headers and all decoded text stay intact.
original=base/'lzma2/资料.7z';data=bytearray(original.read_bytes());end=32+int.from_bytes(data[12:20],'little');assert data[end-1]==0;data[end-1]=1;(base/'bad-end.7z').write_bytes(data)
# A tiny selected text with 65 MiB shared dependency is rejected before decoding.
large=(work/'large-source').resolve();large.mkdir(exist_ok=True)
with (large/'shared.bin').open('wb') as f:f.truncate(65*1024*1024)
(large/'readme.txt').write_bytes(text)
over=base/'oversize-shared.7z';run(producer,[*common,'-m0=LZMA2',over.resolve(),'.'],large)
run(independent,['t',over.resolve()])
# A separate 65 MiB COPY block must not be consulted for the selected solid block.
other=(work/'unrelated.7z').resolve()
run(producer,['a','-t7z','-y','-m0=Copy','-ms=off',other,'shared.bin'],large)
run(producer,[*common,'-m0=LZMA2',other,'.'],source)
run(independent,['t',other])
manifest.write_text(json.dumps({'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'cases':cases,'files':[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()],'textSha256':hashlib.sha256(text).hexdigest(),'badEndSha256':sha(base/'bad-end.7z'),'oversizeSharedSha256':sha(over),'largeUnrelatedIgnored':str(other)},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('4 original solid groups independently read back; whole-block end, oversize and unrelated scope prepared')

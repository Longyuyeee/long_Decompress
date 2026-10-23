"""New original 7-Zip inputs, independent official readback; preserve prior receipts."""
import hashlib, json, subprocess
from pathlib import Path

base=Path('tests/fixtures/archives/sevenz-text-1617'); work=Path('test-results/sevenz-text-1617'); work.mkdir(parents=True,exist_ok=True)
producer=Path('E:/7-Zip/7z.exe'); independent=Path('src-tauri/resources/archive-engine/7z.exe').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=base/'manifest.json'
if manifest.exists():
    m=json.loads(manifest.read_text(encoding='utf-8'))
    for c in m['cases']:
        for f in c['members']: assert sha(base/c['name']/f['name'])==f['sha256']
    print('Existing 7Z text originals retained');raise SystemExit
base.mkdir(parents=True,exist_ok=True)
source=(work/'source').resolve();(source/'notes').mkdir(parents=True,exist_ok=True)
text='7Z 包内说明与日志，不需要先解压。\r\n'.encode()*80
files={'notes/说明 [1].txt':text,'notes/说明 1.txt':b'NOT the selected bracket name','empty.txt':b'',
       'utf16.txt':b'\xff\xfe'+'完整 UTF16 文本'.encode('utf-16le'),'gbk.txt':'中文编码说明'.encode('gbk'),
       'binary.txt':b'\0\1\2\3'*20,'other.bin':hashlib.shake_256(b'7z1617').digest(48000)}
for name,data in files.items():(source/name).write_bytes(data)
expected=[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in files.items()]
key='7Z预览密码1617!';cases=[]
def run(exe,args,cwd=None):
    r=subprocess.run([str(exe),*map(str,args)],cwd=cwd,capture_output=True,timeout=90)
    assert r.returncode==0,r.stdout.decode('utf-8','replace')+r.stderr.decode('utf-8','replace')
    return r.stdout.decode('utf-8','replace')
for name,method,password,header,volume,solid in [('lzma','LZMA',None,False,False,False),('lzma2','LZMA2',None,False,False,False),('contents','LZMA2',key,False,False,False),('headers-volumes','LZMA2',key,True,True,False),('solid','LZMA2',None,False,False,True)]:
    dest=(base/name).resolve();dest.mkdir(exist_ok=True);archive=dest/'资料.7z'
    args=['a','-t7z','-y','-sccUTF-8','-mtc=off','-mta=off','-m0='+method,'-ms='+('on' if solid else 'off'),*( ['-p'+password,'-mhe='+('on' if header else 'off')] if password else []),*(['-v24k'] if volume else []),archive,'.']
    run(producer,args,source)
    members=[{'name':p.name,'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(dest.iterdir())]
    out=(work/'readback'/name).resolve();out.mkdir(parents=True,exist_ok=True)
    run(independent,['x','-y','-sccUTF-8','-o'+str(out),*(['-p'+password] if password else []),dest/members[0]['name']])
    for e in expected:assert sha(out/e['path'])==e['sha256']
    listing=run(producer,['l','-slt','-sccUTF-8',*(['-p'+password] if password else []),dest/members[0]['name']])
    cases.append({'name':name,'method':method,'password':password,'solid':solid,'members':members,'expected':expected,'originalArgs':list(map(str,args)),'independentReadback':True,'methods':[l for l in listing.splitlines() if l.startswith(('Method =','Solid =','Blocks ='))]})
# Single stream with an intact header: corrupt only the LZMA2 end marker.
one=(work/'one').resolve();one.mkdir(exist_ok=True);(one/'readme.txt').write_bytes(text)
original=base/'terminal.7z';run(producer,['a','-t7z','-y','-m0=LZMA2','-mhc=off','-ms=off',original.resolve(),'readme.txt'],one)
b=bytearray(original.read_bytes());end=32+int.from_bytes(b[12:20],'little');assert b[end-1]==0;b[end-1]=1;(base/'bad-end.7z').write_bytes(b)
# An unrelated stored file bigger than the input budget must not block a tiny selected text.
large=(work/'large-source').resolve();large.mkdir(exist_ok=True);blob=large/'unrelated.bin'
with blob.open('wb') as f:f.truncate(65*1024*1024)
(large/'readme.txt').write_bytes(text);big=(work/'large-unrelated.7z').resolve()
run(producer,['a','-t7z','-y','-m0=Copy','-ms=off',big,'unrelated.bin'],large)
run(producer,['a','-t7z','-y','-m0=LZMA2','-ms=off',big,'readme.txt'],large)
manifest.write_text(json.dumps({'producer':str(producer),'producerSha256':sha(producer),'independent':str(independent),'independentSha256':sha(independent),'cases':cases,'terminalOriginalSha256':sha(original),'badEndSha256':sha(base/'bad-end.7z'),'largeUnrelatedIgnored':str(big),'textSha256':hashlib.sha256(text).hexdigest()},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('5 original cases independently decoded; terminal mutation and ignored large unrelated archive prepared')

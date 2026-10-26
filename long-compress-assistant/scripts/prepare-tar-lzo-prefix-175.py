"""Independent PAX TAR producer + official lzop; validate full original before deriving damage."""
from pathlib import Path
import hashlib,io,json,subprocess,tarfile,random
root=Path(__file__).resolve().parents[1]
fixture=root/'tests/fixtures/archives/tar-lzo-prefix-175';fixture.mkdir(parents=True,exist_ok=True)
base=root/'test-results/tar-lzo-prefix-rescue-175';base.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
exe=root/'test-results/lzop-multi-create-172/research/lzop-linux'
old=json.loads((root/'tests/fixtures/archives/lzop-prefix-174/manifest.json').read_text(encoding='utf-8'))
assert sha(exe.read_bytes())==old['producerSha256']
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
archive=fixture/'complete.tar.lzo'
if not archive.exists():
    png=(root/'src-tauri/icons/icon.png').read_bytes()
    members=[('资料/',None),('资料/空目录/',None),('资料/说明.txt',('独立 TAR 与官方 LZOP 完整内容依据。\n'*6000).encode()),('资料/空文件.txt',b''),('资料/'+('长路径资料'*22)+'/说明.txt',b'PAX long UTF-8 path\n'),('资料/图标.png',png),('late.bin',random.Random(175).randbytes(1200173))]
    tarpath=base/'independent.tar';entries=[]
    with tarfile.open(tarpath,'w',format=tarfile.PAX_FORMAT) as out:
        for name,data in members:
            h=tarfile.TarInfo(name);h.mtime=1700000000;h.mode=0o755 if data is None else 0o644
            if data is None:h.type=tarfile.DIRTYPE
            else:h.size=len(data)
            out.addfile(h,None if data is None else io.BytesIO(data))
            entries.append({'name':name.rstrip('/'),'isDirectory':data is None,'bytes':len(data or b''),'sha256':sha(data or b'')})
    compressed=subprocess.run(['wsl','-d','Ubuntu','--exec',unix(exe),'-c','-N',unix(tarpath)],capture_output=True,check=True).stdout
    archive.write_bytes(compressed)
    (fixture/'manifest.json').write_text(json.dumps({'archive':archive.name,'sha256':sha(compressed),'producerSha256':old['producerSha256'],'packageSha256':old['packageSha256'],'tarProducer':'Python stdlib tarfile PAX_FORMAT','entries':entries},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
manifest=json.loads((fixture/'manifest.json').read_text(encoding='utf-8'));assert sha(archive.read_bytes())==manifest['sha256']
decoded=subprocess.run(['wsl','-d','Ubuntu','--exec',unix(exe),'-d','-c',unix(archive)],capture_output=True,check=True).stdout
(base/'official.tar').write_bytes(decoded)
with tarfile.open(fileobj=io.BytesIO(decoded)) as inp:
    actual=[{'name':h.name.rstrip('/'),'isDirectory':h.isdir(),'bytes':h.size,'sha256':sha(inp.extractfile(h).read() if h.isfile() else b'')} for h in inp]
assert actual==manifest['entries']
listing=subprocess.run(['wsl','-d','Ubuntu','--exec','tar','-tf',unix(base/'official.tar')],capture_output=True,check=True)
assert len(listing.stdout.splitlines())==len(actual)
(base/'official.json').write_text(json.dumps({'original':manifest,'officialDecoded':True,'independentTarListing':True,'passed':True},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'Official lzop readback and independent TAR listing: {len(actual)} entries, original SHA {manifest["sha256"]}')

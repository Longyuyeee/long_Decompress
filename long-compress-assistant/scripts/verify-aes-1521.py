"""Independent full readback for new AES-codec outputs, with existing published V2 decoder."""
from pathlib import Path
import hashlib,importlib.util,io,json,subprocess,sys,tarfile
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'test-results/aes-1521/python'))
import lz4.frame,brotli,cryptography
spec=importlib.util.spec_from_file_location('existing_aes_verifier',Path(__file__).with_name('verify-aes-1518.py'));existing=importlib.util.module_from_spec(spec);spec.loader.exec_module(existing)
sha=lambda b:hashlib.sha256(b).hexdigest()

def main():
    request=json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
    if request.get('type')=='zip': existing.main();return
    root=Path(__file__).resolve().parents[1];results=[]
    for c in request['cases']:
        raw=existing.decrypt(c['parts'],request['password']);inner=Path(c['innerPath']);inner.write_bytes(raw)
        codec=c['format'].removesuffix('.aes').removeprefix('tar.')
        if codec=='lz4':
            decoded,consumed=lz4.frame.decompress(raw,return_bytes_read=True);assert consumed==len(raw)
        elif codec=='br':decoded=brotli.decompress(raw)
        else:
            assert codec=='lz'
            linux=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
            tool=root/'test-results/lzip-tools/unpacked/usr/bin/lzip.lzip'
            decoded=subprocess.run(['wsl.exe','-d','Ubuntu','--exec',linux(tool),'-d','-c',linux(inner)],check=True,capture_output=True).stdout
        if c['format'].startswith('tar.'):
            with tarfile.open(fileobj=io.BytesIO(decoded),mode='r:') as tar:
                files={m.name:sha(tar.extractfile(m).read()) for m in tar if m.isfile()}
                directories=[m.name.rstrip('/') for m in tar if m.isdir()]
            assert files==request['expectedFiles'],(c['format'],files)
            assert request['emptyDirectory'] in directories,(c['format'],directories)
        else:assert sha(decoded)==c.get('plainSha256',request['plainSha256']),c['format']
        results.append(dict(format=c['format'],volumeCount=len(c['parts']),innerSha256=sha(raw),fullContentMatches=True,emptyDirectoryPreserved=c['format'].startswith('tar.')))
    result=dict(tool=dict(cryptography=cryptography.__version__,lz4=lz4.__version__,brotli=brotli.__version__,lzip='GNU lzip 1.24.1'),cases=results)
    Path(sys.argv[2]).write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(verified=len(results),allFullContentMatched=True)))
if __name__=='__main__':main()

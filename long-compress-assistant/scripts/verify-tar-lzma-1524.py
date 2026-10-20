"""Independent complete readback of application TAR/LZMA/AES outputs."""
from pathlib import Path
import hashlib, importlib.util, io, json, lzma, sys, tarfile
root=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(root/'test-results/aes-1521/python'))
spec=importlib.util.spec_from_file_location('existing_aes_verifier',Path(__file__).with_name('verify-aes-1518.py'))
existing=importlib.util.module_from_spec(spec);spec.loader.exec_module(existing)
sha=lambda b:hashlib.sha256(b).hexdigest()
request=json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
if request.get('type')=='zip':
    existing.main();sys.exit(0)
results=[]
for c in request['cases']:
    raw=existing.decrypt(c['parts'],request['password']) if c['format'].endswith('.aes') else b''.join(Path(p).read_bytes() for p in c['parts'])
    decoder=lzma.LZMADecompressor(format=lzma.FORMAT_ALONE);decoded=decoder.decompress(raw)
    assert decoder.eof and not decoder.unused_data
    if c['format'].startswith('tar.'):
        with tarfile.open(fileobj=io.BytesIO(decoded),mode='r:') as tar:
            files={m.name:sha(tar.extractfile(m).read()) for m in tar if m.isfile()}
            directories=[m.name.rstrip('/') for m in tar if m.isdir()]
        assert files==request['expectedFiles'],files
        assert request['emptyDirectory'] in directories,directories
    else:
        assert sha(decoded)==c.get('plainSha256',request['plainSha256'])
    results.append(dict(format=c['format'],volumeCount=len(c['parts']),fullContentMatches=True,innerSha256=sha(raw)))
Path(sys.argv[2]).write_text(json.dumps(dict(tool='Python tarfile/lzma + cryptography authentication',cases=results),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(verified=len(results),fullContentMatches=True)))

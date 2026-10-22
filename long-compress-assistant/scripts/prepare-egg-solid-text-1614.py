"""Small controlled solid negatives and native framing of an unchanged original stream.
Not genuine producer-native volumes. Never modify the frozen original archives.
"""
import hashlib,json,struct,zlib
from pathlib import Path
root=Path(__file__).resolve().parents[1];out=root/'tests/fixtures/archives/egg-solid-text-1614';out.mkdir(parents=True,exist_ok=True)
end=struct.pack('<I',0x08e28222)
def extra(sig,data):return struct.pack('<IBH',sig,0,len(data))+data
def head(id=1614):return struct.pack('<IHII',0x41474745,256,id,0)
text=('固实包内说明：保持双栏，只显示所选文本。\n'*1200).encode()
tail=bytes(range(256))*20
def file(name,size,id,blocks=b'',encrypt=False):
 return struct.pack('<IIQ',0x0a8590e3,id,size)+extra(0x0a8591ac,name.encode())+(extra(0x08d1470f,b'\0'+b'\0'*16) if encrypt else b'')+end+blocks
def archive(name,data=text,expanded=None,badcrc=False,packed=None,method=1,encrypted=False):
 payload=data+tail;compressed=packed if packed is not None else zlib.compress(payload,wbits=-15)
 size=expanded if expanded is not None else len(payload)
 block=struct.pack('<IBBIII',0x02b50c13,method,0,size,len(compressed),zlib.crc32(payload)^int(badcrc))+end+compressed
 body=file('readme.txt',len(data),1)+file('tail.bin',size-len(data),2,block,encrypted)+end
 (out/(name+'.egg')).write_bytes(head()+extra(0x24e5a060,b'')+end+body)
archive('ui')
archive('shared-crc',badcrc=True)
archive('shared-output-limit',expanded=64*1024*1024+1)
archive('selected-limit',data=b'A'*(1024*1024+1))
archive('unknown-dependency',method=127)
archive('encrypted-dependency',encrypted=True)
packed=zlib.compress(text+tail,wbits=-15)
archive('truncated',packed=packed[:-3]);archive('trailing',packed=packed+b'X')
# Exact selected-text bound, independent of the 64 MiB shared bound.
archive('exact-limit',data=b'A'*(1024*1024))
source=root/'tests/fixtures/archives/egg-153/upstream/solid.egg'
original=source.read_bytes();at=original.index(end,14);extension=original[14:at];body=original[at+4:]
parts=[body[:len(body)//3],body[len(body)//3:2*len(body)//3],body[2*len(body)//3:]]
ids=[161401,161402,161403]
for i,part in enumerate(parts):
 prefix=head(ids[i])+extension+extra(0x24f5a262,struct.pack('<II',ids[i-1] if i else 0,ids[i+1] if i<2 else 0))+end
 (out/f'controlled-native.vol{i+1}.egg').write_bytes(prefix+part)
manifest={'version':'1.6.14','controlled':True,'historicalProducer':False,'textBytes':len(text),'textSha256':hashlib.sha256(text).hexdigest(),'nativeVolumeEvidence':'controlled native headers around the unchanged genuine Deflate payload; not original producer volumes','nativePayloadOriginalSha256':hashlib.sha256(original).hexdigest(),'files':[{'name':f.name,'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in sorted(out.glob('*.egg'))]}
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
# Input >64 MiB stays ignored, not a tracked huge fixture.
base=root/'test-results/egg-solid-text-1614';base.mkdir(parents=True,exist_ok=True)
data=b'X'*(64*1024*1024+1)
block=struct.pack('<IBBIII',0x02b50c13,1,0,1,len(data),0)+end+data
(base/'input-limit.egg').write_bytes(head()+extra(0x24e5a060,b'')+end+file('readme.txt',1,1,block)+end)
print('Prepared small controlled solid cases and ignored input bound.')

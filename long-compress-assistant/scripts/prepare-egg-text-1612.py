"""Self-authored EGG text/negative cases; not original-producer interop samples."""
from pathlib import Path
import hashlib,json,struct,zlib
root=Path(__file__).resolve().parents[1]
out=root/'tests/fixtures/archives/egg-text-1612';out.mkdir(parents=True,exist_ok=True)
end=struct.pack('<I',0x08e28222)
head=struct.pack('<IHII',0x41474745,256,1612,0)+end
def entry(name,data,method=0,crc=None,declared=None,packed=None):
 name=name.encode();body=data if packed is None else packed
 return struct.pack('<IIQ',0x0a8590e3,1 if name==b'readme.txt' else 2,len(data) if declared is None else declared)+struct.pack('<IBH',0x0a8591ac,0,len(name))+name+end+struct.pack('<IBBIII',0x02b50c13,method,0,len(data) if declared is None else declared,len(body),zlib.crc32(data) if crc is None else crc)+end+body
text=('EGG 文本预览：保持双栏，直接查看所选说明。\n'*1500).encode()
pack=zlib.compress(text,wbits=-15)
values={'plain':entry('readme.txt',text)+entry('unrelated.bin',b'',127),'crc-damage':entry('readme.txt',text,crc=zlib.crc32(text)^1),'trailing-deflate':entry('readme.txt',text,1,packed=pack+b'X'),'short-deflate':entry('readme.txt',text,1,packed=pack[:-2]),'exact-limit':entry('readme.txt',b'L'*1048576),'too-large':entry('readme.txt',b'L'*1048577),'binary':entry('readme.txt',b'\x00'*200),'empty':entry('readme.txt',b''),'utf16-odd':entry('readme.txt',b'\xff\xfeA\x00B')}
manifest={'version':'1.6.12','controlled':True,'historicalProducer':False,'payloadSha256':hashlib.sha256(text).hexdigest(),'payloadBytes':len(text),'cases':[]}
for name,value in values.items():
 data=head+value+end;(out/(name+'.egg')).write_bytes(data);manifest['cases'].append({'name':name,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
large=root/'test-results/egg-text-1612';large.mkdir(parents=True,exist_ok=True)
(large/'input-limit.egg').write_bytes(head+entry('readme.txt',b'A',declared=1,packed=b'X'*(64*1024*1024+1))+end)

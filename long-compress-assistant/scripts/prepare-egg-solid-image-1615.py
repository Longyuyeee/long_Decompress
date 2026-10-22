"""Self-authored solid image streams and native framing; not original producer archives."""
import ast,hashlib,json,lzma,struct,zlib
from pathlib import Path
root=Path(__file__).resolve().parents[1];out=root/'tests/fixtures/archives/egg-solid-image-1615';out.mkdir(parents=True,exist_ok=True)
tree=ast.parse((root/'scripts/prepare-alz-image-1610.py').read_text(encoding='utf-8'))
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in ['chunk','png','bmp']],type_ignores=[]),'<owned-pixels>','exec'))
end=struct.pack('<I',0x08e28222)
def extra(sig,data):return struct.pack('<IBH',sig,0,len(data))+data
def head(id=1615):return struct.pack('<IHII',0x41474745,256,id,0)
def file(name,size,id,blocks=b'',encrypted=False):
 return struct.pack('<IIQ',0x0a8590e3,id,size)+extra(0x0a8591ac,name.encode())+(extra(0x08d1470f,b'\0'+b'\0'*16) if encrypted else b'')+end+blocks
p=png();b=bmp();large=b+b'\0'*(2*1024*1024-len(b));tail=bytes(range(256))*8
images=[('gallery/01-cover.png',p),('gallery/02-grid.bmp',large)]
base_entries=[('gallery/00-broken.png',b'not an image'),*images,('readme.txt','固实相册说明：只读取用户当前所选图片。'.encode()),('tail.bin',tail)]
def archive(name,entries=base_entries,method=1,crc_bad=False,expanded_extra=0,encrypted=False,change=None):
 clear=b''.join(data for _,data in entries)
 if method==4:
  encoded=lzma.compress(clear,format=lzma.FORMAT_ALONE,filters=[{'id':lzma.FILTER_LZMA1,'dict_size':65536}]);packed=b'\0'*4+encoded[:5]+encoded[13:]
 else:packed=zlib.compress(clear,wbits=-15)
 if change=='truncated':packed=packed[:-3]
 if change=='trailing':packed+=b'X'
 # Two segments of one shared stream, with separate CRC regions.
 split=len(entries[0][1])+len(entries[1][1]) if len(entries)>1 else len(clear)
 cp=max(1,len(packed)//2)
 blocks=b''
 for raw,encoded,bad in [(clear[:split],packed[:cp],False),(clear[split:],packed[cp:],crc_bad)]:
  size=len(raw)+(expanded_extra if bad or not blocks else 0)
  blocks+=struct.pack('<IBBIII',0x02b50c13,method,0,size,len(encoded),zlib.crc32(raw)^int(bad))+end+encoded
 body=b''
 for i,(n,d) in enumerate(entries):
  declared=len(d)+(expanded_extra if i==len(entries)-1 else 0)
  body+=file(n,declared,i+1,blocks if i==len(entries)-1 else b'',encrypted and i==len(entries)-1)
 data=head()+extra(0x24e5a060,b'')+end+body+end;(out/(name+'.egg')).write_bytes(data)
 return body+end
for name,method in [('deflate',1),('lzma',4)]:archive(name,method=method)
body=archive('ui')
archive('shared-crc',crc_bad=True);archive('shared-output-limit',expanded_extra=64*1024*1024);archive('encrypted-dependency',encrypted=True);archive('unknown-dependency',method=127)
archive('truncated',change='truncated');archive('trailing',change='trailing')
limit=b+b'\0'*(8*1024*1024-len(b));archive('exact-limit',entries=[('preview.bmp',limit),('tail.bin',tail)])
archive('oversize',entries=[('preview.bmp',limit+b'X'),('tail.bin',tail)])
bad=p[:16]+struct.pack('>II',9000,2)+p[24:];archive('dimensions',entries=[('preview.png',bad),('tail.bin',tail)])
# Three complete native members cut inside the actual shared payload.
parts=[body[:len(body)//3],body[len(body)//3:2*len(body)//3],body[2*len(body)//3:]];ids=[161501,161502,161503]
for i,part in enumerate(parts):
 prefix=head(ids[i])+extra(0x24e5a060,b'')+extra(0x24f5a262,struct.pack('<II',ids[i-1] if i else 0,ids[i+1] if i<2 else 0))+end
 (out/f'controlled-native.vol{i+1}.egg').write_bytes(prefix+part)
manifest={'version':'1.6.15','controlled':True,'historicalProducer':False,'description':'Two blocks of one shared Deflate/LZMA stream. Only selected image retained. All pixels are self-authored; native headers and shared framing controlled.','images':[{'path':n,'bytes':len(d),'sha256':hashlib.sha256(d).hexdigest()} for n,d in images],'exactLimitSha256':hashlib.sha256(limit).hexdigest(),'files':[{'name':f.name,'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in sorted(out.glob('*.egg'))]}
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
base=root/'test-results/egg-solid-image-1615';base.mkdir(parents=True,exist_ok=True)
packed=b'X'*(64*1024*1024+1);block=struct.pack('<IBBIII',0x02b50c13,1,0,len(p),len(packed),0)+end+packed
(base/'input-limit.egg').write_bytes(head()+extra(0x24e5a060,b'')+end+file('preview.png',len(p),1,block)+end)
print('Prepared controlled solid image streams, native group and ignored input bound.')

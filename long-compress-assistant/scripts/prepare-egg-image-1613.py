"""Controlled EGG images, keys and native framing; not original producer archives."""
import ast,bz2,hashlib,hmac,json,lzma,struct,zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher,algorithms,modes
root=Path(__file__).resolve().parents[1];out=root/'tests/fixtures/archives/egg-image-1613';out.mkdir(parents=True,exist_ok=True)
# Reuse only self-authored pixel functions, without running the old generator.
tree=ast.parse((root/'scripts/prepare-alz-image-1610.py').read_text(encoding='utf-8'))
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in ['chunk','png','bmp']],type_ignores=[]),'<owned-pixels>','exec'))
end=struct.pack('<I',0x08e28222);password='图片密码1613!';key=password.encode('gbk');identity=0
head=lambda id=1613:struct.pack('<IHII',0x41474745,256,id,0)
def extra(sig,data):return struct.pack('<IBH',sig,0,len(data))+data

def zipcrypto(data,crc):
 table=[]
 for i in range(256):
  for _ in range(8):i=(i>>1)^(0xedb88320 if i&1 else 0)
  table.append(i)
 keys=[0x12345678,0x23456789,0x34567890]
 def update(b):
  keys[0]=(keys[0]>>8)^table[(keys[0]^b)&255];keys[1]=((keys[1]+(keys[0]&255))*134775813+1)&0xffffffff;keys[2]=(keys[2]>>8)^table[(keys[2]^(keys[1]>>24))&255]
 for b in key:update(b)
 clear=b'image1613!!'+bytes([crc>>24]);assert len(clear)==12
 result=bytearray()
 for b in clear+data:
  t=keys[2]|2;result.append(b^(((t*(t^1))&0xffffffff)>>8&255));update(b)
 return b'\0'+bytes(result[:12])+struct.pack('<I',crc),bytes(result[12:])

def aes(data):
 salt=bytes(range(16));derived=hashlib.pbkdf2_hmac('sha1',key,salt,1000,66)
 ctr=b''.join(i.to_bytes(16,'little') for i in range(1,(len(data)+15)//16+1));enc=Cipher(algorithms.AES(derived[:32]),modes.ECB()).encryptor();stream=enc.update(ctr)+enc.finalize();encrypted=bytes(a^b for a,b in zip(data,stream))
 return b'\2'+salt+derived[64:66]+hmac.digest(derived[32:64],encrypted,'sha1')[:10],encrypted

def entry(name,data,method=1,cipher=None,crc_bad=False,packed=None):
 global identity
 identity+=1;crc=zlib.crc32(data)
 if packed is None:
  if method==0:packed=data
  elif method==1:packed=zlib.compress(data,wbits=-15)
  elif method==2:packed=bz2.compress(data)
  elif method==4:
   p=lzma.compress(data,format=lzma.FORMAT_ALONE,filters=[{'id':lzma.FILTER_LZMA1,'dict_size':65536}]);packed=b'\0'*4+p[:5]+p[13:]
  else:packed=data
 info=b''
 if cipher:
  payload,packed=zipcrypto(packed,crc) if cipher=='zipcrypto' else aes(packed);info=extra(0x08d1470f,payload)
 return struct.pack('<IIQ',0x0a8590e3,identity,len(data))+extra(0x0a8591ac,name.encode())+info+end+struct.pack('<IBBIII',0x02b50c13,method,0,len(data),len(packed),crc^int(crc_bad))+end+packed

def write(name,entries,solid=False):
 data=head()+(extra(0x24e5a060,b'') if solid else b'')+end+entries+end;(out/(name+'.egg')).write_bytes(data)
p=png();b=bmp();images=[('gallery/01-cover.png',p),('gallery/02-grid.bmp',b)]
for name,method in [('store',0),('deflate',1),('bzip2',2),('lzma',4)]:write(name,b''.join(entry(n,d,method) for n,d in images)+entry('unrelated.bin',b'',127))
write('ui',entry('gallery/00-broken.png',b'not an image')+b''.join(entry(n,d) for n,d in images))
for cipher in ['zipcrypto','aes256']:write(cipher,entry('相册/01-封面.png',p,cipher=cipher)+entry('unrelated.bin',b'',127))
body=entry('相册/01-封面.png',p,cipher='aes256')+end
chunks=[body[:len(body)//3],body[len(body)//3:2*len(body)//3],body[2*len(body)//3:]]
for i,part in enumerate(chunks):
 ids=[161301,161302,161303];prefix=head(ids[i])+extra(0x24f5a262,struct.pack('<II',ids[i-1] if i else 0,ids[i+1] if i<2 else 0))+end
 (out/f'password-volumes.vol{i+1}.egg').write_bytes(prefix+part)
write('crc',entry('preview.png',p,crc_bad=True));packed=zlib.compress(p,wbits=-15);write('truncated',entry('preview.png',p,packed=packed[:-3]));write('trailing',entry('preview.png',p,packed=packed+b'X'))
write('solid',entry('preview.png',p),solid=True)
write('not-image',entry('preview.png',b'not a picture'))
large=bytearray(b);large.extend(b'\0'*(8*1024*1024-len(large)));write('at-limit',entry('preview.bmp',bytes(large)));write('oversize',entry('preview.png',bytes(large)+b'X'))
# Header-only dimensions are enough for the product metadata check; never a positive displayed image.
bad=p[:16]+struct.pack('>II',9000,2)+p[24:];write('dimensions',entry('preview.png',bad))
manifest={'version':'1.6.13','controlled':True,'historicalProducer':False,'password':password,'passwordEncoding':'cp936','images':[{'path':n,'bytes':len(d),'sha256':hashlib.sha256(d).hexdigest()} for n,d in images],'files':[{'name':f.name,'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in sorted(out.glob('*.egg'))]}
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
largeout=root/'test-results/egg-image-1613';largeout.mkdir(parents=True,exist_ok=True)
write_data=head()+end+entry('preview.png',p,packed=b'X'*(64*1024*1024+1))+end;(largeout/'input-limit.egg').write_bytes(write_data)

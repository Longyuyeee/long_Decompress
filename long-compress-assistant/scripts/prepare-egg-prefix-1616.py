"""Owned non-solid EGG prefix fixtures, never original-producer interop claims."""
import ast,bz2,hashlib,hmac,json,lzma,struct,zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher,algorithms,modes
root=Path(__file__).resolve().parents[1];out=root/'tests/fixtures/archives/egg-prefix-1616';out.mkdir(parents=True,exist_ok=True)
tree=ast.parse((root/'scripts/prepare-egg-image-1613.py').read_text(encoding='utf-8'))
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in ['zipcrypto','aes']],type_ignores=[]),'<existing-owned-ciphers>','exec'))
password='日志密码1616!';key=password.encode('gbk');end=struct.pack('<I',0x08e28222);head=struct.pack('<IHII',0x41474745,256,1616,0)+end;serial=0;limit=1024*1024
payload=('INFO 2026-10-23 EGG 包内日志；只看开头，保持双栏。\n'*40000).encode()
def extra(sig,data):return struct.pack('<IBH',sig,0,len(data))+data
def compress(data,method):
 if method==0:return data
 if method==1:return zlib.compress(data,wbits=-15)
 if method==2:return bz2.compress(data)
 if method==4:
  v=lzma.compress(data,format=lzma.FORMAT_ALONE,filters=[{'id':lzma.FILTER_LZMA1,'dict_size':65536}]);return b'\0'*4+v[:5]+v[13:]
 return data
def entry(data=payload,method=1,cipher=None,crc_bad=False,packed=None,declared=None,name='logs/session.txt',chunks=None):
 global serial
 serial+=1;size=len(data) if declared is None else declared;crc=zlib.crc32(data);packed=compress(data,method) if packed is None else packed;info=b''
 if cipher:
  encrypted,packed=zipcrypto(packed,crc) if cipher=='zipcrypto' else aes(packed);info=extra(0x08d1470f,encrypted)
 blocks=struct.pack('<IBBIII',0x02b50c13,method,0,size,len(packed),crc^int(crc_bad))+end+packed
 if chunks is not None:
  blocks=b''
  for i,chunk in enumerate(chunks):
   v=compress(chunk,method);blocks+=struct.pack('<IBBIII',0x02b50c13,method,0,len(chunk),len(v),zlib.crc32(chunk)^int(crc_bad and i==0))+end+v
 return struct.pack('<IIQ',0x0a8590e3,serial,size)+extra(0x0a8591ac,name.encode())+info+end+blocks
def write(name,body):(out/(name+'.egg')).write_bytes(head+body+end)
for name,method in [('store',0),('deflate',1),('bzip2',2),('lzma',4)]:write(name,entry(method=method)+entry(b'',127,name='unrelated.bin'))
for cipher in ['zipcrypto','aes256']:write(cipher,entry(cipher=cipher)+entry(b'',127,name='unrelated.bin'))
body=entry(cipher='aes256')+end;ids=[161601,161602,161603]
for i in range(3):
 chunk=body[i*len(body)//3:(i+1)*len(body)//3]
 prefix=struct.pack('<IHII',0x41474745,256,ids[i],0)+extra(0x24f5a262,struct.pack('<II',ids[i-1] if i else 0,ids[i+1] if i<2 else 0))+end
 (out/f'password-volumes.vol{i+1}.egg').write_bytes(prefix+chunk)
utf8=b'A'*(limit-1)+'中文日志'.encode()*1000
utf16=b'\xff\xfe'+b'A\0'*((limit-4)//2)+'😀继续'.encode('utf-16le')*1000
gbk='中文'.encode('gbk')+b'A'*(limit-5)+'中文日志'.encode('gbk')*1000
for name,data in [('utf8-boundary',utf8),('utf16-boundary',utf16),('gbk-boundary',gbk)]:write(name,entry(data))
write('bad-crc-later',entry(crc_bad=True));write('short-before-prefix',entry(b'short data'*100,declared=len(payload)))
write('bad-first-block',entry(data=b'A'*limit+payload,chunks=[b'A'*limit,payload],crc_bad=True))
write('binary',entry(b'\0'*(limit+4096)));write('unknown-method',entry(method=127))
write('multi-block',entry(data=b'A'*limit+payload,chunks=[b'A'*(limit//2),b'A'*(limit//2)+payload]))
rendered=payload[:limit].decode('utf-8',errors='ignore').encode()
manifest={'renderedPrefixBytes':len(rendered),'renderedPrefixSha256':hashlib.sha256(rendered).hexdigest(),'version':'1.6.16','controlled':True,'historicalProducer':False,'password':password,'passwordEncoding':'cp936','payloadBytes':len(payload),'prefixSha256':hashlib.sha256(payload[:limit]).hexdigest(),'characterBoundaryCases':{'utf8-boundary':limit-1,'utf16-boundary':limit-2,'gbk-boundary':limit-1},'files':[{'name':f.name,'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in sorted(out.glob('*.egg'))]}
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
base=root/'test-results/egg-prefix-1616';base.mkdir(parents=True,exist_ok=True)
# A large stored entry can yield a prefix without reading the entire >64 MiB payload.
(base/'large-store.egg').write_bytes(head+entry(b'L'*(64*1024*1024+1),0)+end)
# Legitimate empty Deflate blocks before text require >64 MiB actual input before any prefix.
padding=b'\0\0\0\xff\xff'*(64*1024*1024//5+1)
(base/'input-limit.egg').write_bytes(head+entry(packed=padding+compress(payload,1))+end)
print('Prepared owned prefix methods, keys, native members, boundaries and ignored budgets.')

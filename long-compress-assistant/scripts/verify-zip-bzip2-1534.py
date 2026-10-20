"""Independent ZIP method-12 decoding, including WinZip AES and ZipCrypto."""
from pathlib import Path
import bz2,hashlib,hmac,io,json,struct,sys,zipfile,zlib
root=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(root/'test-results/aes-1521/python'))
from cryptography.hazmat.primitives.ciphers import Cipher,algorithms,modes
request=json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'));data=Path(request['file']).read_bytes();password=request.get('password','').encode('utf-8');files={};directories=[];methods=[]
with zipfile.ZipFile(io.BytesIO(data)) as archive:
 for info in archive.infolist():
  if info.is_dir():directories.append(info.filename.rstrip('/'));continue
  method=info.compress_type
  if method==99:
   offset=info.header_offset;assert data[offset:offset+4]==b'PK\x03\x04';n,e=struct.unpack_from('<HH',data,offset+26);payload=data[offset+30+n+e:offset+30+n+e+info.compress_size]
   extra=info.extra;aes=None
   while extra:
    tag,size=struct.unpack_from('<HH',extra);value=extra[4:4+size];extra=extra[4+size:]
    if tag==0x9901:aes=value
   assert aes is not None;version,vendor,strength,method=struct.unpack('<H2sBH',aes);assert vendor==b'AE' and strength==3 and version in (1,2)
   keys=hashlib.pbkdf2_hmac('sha1',password,payload[:16],1000,66);assert hmac.compare_digest(payload[16:18],keys[64:]);encrypted=payload[18:-10];assert hmac.compare_digest(hmac.new(keys[32:64],encrypted,'sha1').digest()[:10],payload[-10:])
   counters=b''.join(i.to_bytes(16,'little') for i in range(1,(len(encrypted)+15)//16+1));enc=Cipher(algorithms.AES(keys[:32]),modes.ECB()).encryptor();stream=enc.update(counters)+enc.finalize();raw=bytes(a^b for a,b in zip(encrypted,stream));assert method==12;decoded=bz2.decompress(raw)
   if version==1:assert (zlib.crc32(decoded)&0xffffffff)==info.CRC
  else:
   assert method==12;decoded=archive.read(info,pwd=password or None)
  assert len(decoded)==info.file_size;methods.append(method);files[info.filename]=hashlib.sha256(decoded).hexdigest()
assert files==request['expectedFiles'],files
assert request['emptyDirectory'] in directories,directories
result=dict(tool='Python zipfile/bz2 + independent WinZip AES authentication/decryption',methods=methods,allMethods12=True,allIndependentHashes=True,emptyDirectory=True)
Path(sys.argv[2]).write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(json.dumps(result))

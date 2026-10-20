"""Independent ZIP LZMA decode: Python/liblzma; WinZip AES uses independent HMAC/CTR."""
import hashlib, hmac, io, json, struct, sys, zipfile, zlib
from pathlib import Path
request=json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'));data=Path(request['file']).read_bytes();password=request.get('password','').encode('utf-8');files={};directories=[];methods=[]
with zipfile.ZipFile(io.BytesIO(data)) as archive:
 for info in archive.infolist():
  if info.is_dir(): directories.append(info.filename.rstrip('/'));continue
  method=info.compress_type;offset=info.header_offset;n,e=struct.unpack_from('<HH',data,offset+26)
  assert info.flag_bits&2 and info.extract_version>=63 and struct.unpack_from('<H',data,offset+6)[0]&2
  payload=data[offset+30+n+e:offset+30+n+e+info.compress_size];aes_version=None
  if method==99:
   from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
   extra=info.extra;aes=None
   while extra:
    tag,size=struct.unpack_from('<HH',extra);value=extra[4:4+size];extra=extra[4+size:]
    if tag==0x9901: aes=value
   assert aes is not None;aes_version,vendor,strength,method=struct.unpack('<H2sBH',aes);assert vendor==b'AE' and strength==3 and aes_version in (1,2)
   keys=hashlib.pbkdf2_hmac('sha1',password,payload[:16],1000,66);assert hmac.compare_digest(payload[16:18],keys[64:]);encrypted=payload[18:-10];assert hmac.compare_digest(hmac.new(keys[32:64],encrypted,'sha1').digest()[:10],payload[-10:])
   counters=b''.join(i.to_bytes(16,'little') for i in range(1,(len(encrypted)+15)//16+1));enc=Cipher(algorithms.AES(keys[:32]),modes.ECB()).encryptor();stream=enc.update(counters)+enc.finalize();payload=bytes(a^b for a,b in zip(encrypted,stream))
  assert method==14
  if info.flag_bits&1 and aes_version is None:
   decoded=archive.read(info,pwd=password)
  else:
   assert payload[:4]==bytes([9,4,5,0]);decoder=zipfile.LZMADecompressor();decoded=decoder.decompress(payload);assert decoder.eof
   if aes_version!=2: assert zlib.crc32(decoded)&0xffffffff==info.CRC
   if aes_version is None: assert archive.read(info)==decoded
  assert len(decoded)==info.file_size;files[info.filename]=hashlib.sha256(decoded).hexdigest();methods.append(14)
assert files==request['expectedFiles'];assert request['emptyDirectory'] in directories
result=dict(tool='Python zipfile/liblzma with independent AES HMAC/CTR when needed',methods=methods,headerAndEos=True,allIndependentHashes=True,emptyDirectory=True)
if len(sys.argv)>2: Path(sys.argv[2]).write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(result))

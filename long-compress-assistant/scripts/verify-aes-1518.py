"""Independent readback of application-created AES numeric volumes."""
from pathlib import Path
import hashlib, io, json, struct, subprocess, sys, tarfile
import cryptography
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.argon2 import Argon2id

def sha(b): return hashlib.sha256(b).hexdigest()
def decrypt(paths,password):
    data=b"".join(Path(p).read_bytes() for p in paths)
    assert data[:8] in (b"AESENC02",b"TARAES02")
    header=data[:76]
    size,flags,chunk,length,kdf,cipher,reserved,memory,iterations,lanes=struct.unpack("<HHIQBBHIII",header[8:40])
    assert (size,flags,kdf,cipher,reserved,memory,iterations,lanes)==(76,0,1,1,0,65536,3,1)
    count=max(1,(length+chunk-1)//chunk);assert len(data)==76+length+16*count
    key=Argon2id(salt=header[40:72],length=32,iterations=iterations,lanes=lanes,memory_cost=memory).derive(password.encode())
    aes=AESGCM(key);decoded=[];offset=76
    for i in range(count):
        n=min(chunk,length-i*chunk)+16
        decoded.append(aes.decrypt(header[72:76]+i.to_bytes(8,"big"),data[offset:offset+n],header+i.to_bytes(8,"big")))
        offset+=n
    return b"".join(decoded)

def main():
    request=json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    results=[]
    for c in request["cases"]:
        raw=decrypt(c["parts"],request["password"])
        inner=Path(c["innerPath"]);inner.write_bytes(raw)
        if c["format"]!="tar.aes":
            decoded=subprocess.run([request["sevenZip"],"x","-so","-bd",str(inner)],check=True,capture_output=True).stdout
        else: decoded=raw
        if c["format"].startswith("tar"):
            with tarfile.open(fileobj=io.BytesIO(decoded),mode="r:") as tar:
                files={m.name:sha(tar.extractfile(m).read()) for m in tar if m.isfile()}
                empty=[m.name.rstrip("/") for m in tar if m.isdir()]
            assert files==request["expectedFiles"],(c["format"],files)
            assert request["emptyDirectory"] in empty,(c["format"],empty)
        else: assert sha(decoded)==request["plainSha256"],c["format"]
        results.append(dict(format=c["format"],fullContentMatches=True,innerSha256=sha(raw),
            volumeCount=len(c["parts"]),emptyDirectory=c["format"].startswith("tar")))
    result=dict(tool="cryptography "+cryptography.__version__+" with independent AESGCM/Argon2id decoder and 7-Zip/TAR readback",cases=results)
    Path(sys.argv[2]).write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(dict(verified=len(results),allFullContentMatched=True)))
if __name__=="__main__": main()

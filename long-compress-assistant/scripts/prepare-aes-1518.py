"""Independent Long AES V1/V2 originals from the existing published layout.

No application code is imported. The source compressed streams are original
CPython/libzstd/GNU tar outputs retained by v1.5.17. Random salt/nonce is retained
in each committed original; regeneration intentionally produces new originals.
"""
from pathlib import Path
import hashlib, json, os, platform, struct
import cryptography
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.argon2 import Argon2id

root = Path(__file__).resolve().parents[1]
base = root / "tests/fixtures/archives/multistream-1517"
out = root / "tests/fixtures/archives/aes-1518"
out.mkdir(parents=True, exist_ok=True)
password = "Long 分卷-1518"
cases = []
digest = lambda b: hashlib.sha256(b).hexdigest()
for kind, codec, source in [
    ("tar", "", "original.tar"),
    *[("tar", c, "跨段.tar."+c) for c in ("gz","bz2","xz","zst")],
    *[("plain", c, "说明.txt."+c) for c in ("gz","bz2","xz","zst")],
]:
    payload = (base/source).read_bytes()
    inner = "资料.tar" + ("."+codec if codec else "") if kind == "tar" else "说明.txt."+codec
    for version in ([2,1] if not codec or kind == "plain" and codec == "gz" else [2]):
        salt, nonce = os.urandom(32), os.urandom(4 if version==2 else 12)
        key = Argon2id(salt=salt,length=32,iterations=3,lanes=1,memory_cost=65536).derive(password.encode())
        cipher = AESGCM(key)
        magic = (b"TARAES0" if not codec else b"AESENC0")+str(version).encode()
        if version == 2:
            chunk = 65536
            header = magic+struct.pack("<HHIQBBHIII",76,0,chunk,len(payload),1,1,0,65536,3,1)+salt+nonce
            assert len(header)==76
            encoded = header+b"".join(cipher.encrypt(nonce+i.to_bytes(8,"big"),payload[i*chunk:(i+1)*chunk],
                header+i.to_bytes(8,"big")) for i in range(max(1,(len(payload)+chunk-1)//chunk)))
        else:
            encoded=magic+salt+nonce+cipher.encrypt(nonce,payload,None)
        name="v"+str(version)+"-"+inner+".aes"
        (out/name).write_bytes(encoded)
        cases.append(dict(version=version,kind=kind,codec=codec,archive=name,
            innerName=inner,source=source,sha256=digest(encoded),bytes=len(encoded),
            innerSha256=digest(payload),innerBytes=len(payload)))
manifest=dict(tool="CPython "+platform.python_version()+"/cryptography "+cryptography.__version__,
    specification="Existing AESENC01/TARAES01 and AESENC02/TARAES02 in published v1.5.17 source",
    password=password,kdf="Argon2id v19, 65536 KiB, 3 iterations, 1 lane, 32-byte key",
    cipher="AES-256-GCM; V2 64 KiB chunks and header/index AAD; V1 single authenticated message",
    sourceManifestSha256=digest((base/"manifest.json").read_bytes()),cases=cases)
(out/"manifest.json").write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
print(json.dumps(dict(originals=len(cases),tool=manifest["tool"]),ensure_ascii=False))

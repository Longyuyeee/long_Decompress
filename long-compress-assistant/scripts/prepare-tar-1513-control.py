"""Small genuine PAX sparse archive with 2 GiB logical output for real controls."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

out = Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=True)
if (out / "manifest.json").exists():
    raise SystemExit("Existing control original preserved")
def digest(file):
    h=hashlib.sha256()
    with open(file,"rb") as stream:
        while block:=stream.read(1024*1024): h.update(block)
    return h.hexdigest()
size=2*1024*1024*1024+17
with tempfile.TemporaryDirectory(prefix="long-tar1513-control-") as scratch:
    source=Path(scratch)/"control.bin"
    with open(source,"wb") as stream:
        for i,offset in enumerate([0,512*1024*1024,1024*1024*1024,2*1024*1024*1024-4096]):
            stream.seek(offset);stream.write(bytes([65+i])*4096)
        stream.truncate(size)
    archive=out/"control.tar"
    args=["tar","--format=pax","--sparse","--sparse-version=1.0","--hole-detection=seek","--mtime=@1577934245","--owner=0","--group=0","--numeric-owner","--pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime","-cf",str(archive),"-C",scratch,"control.bin"]
    subprocess.run(args,check=True)
    decoded=Path(scratch)/"independent-producer-roundtrip";decoded.mkdir()
    subprocess.run(["tar","-xf",str(archive),"-C",str(decoded)],check=True)
    source_hash=digest(source);assert digest(decoded/"control.bin")==source_hash
    result={"producer":subprocess.check_output(["tar","--version"],text=True).splitlines()[0],"arguments":args,"layout":"GNU PAX sparse 1.0","logicalBytes":size,"sourceSha256":source_hash,"archiveBytes":archive.stat().st_size,"archiveSha256":digest(archive),"producerRoundtripFullHash":True,"description":"Self-authored fixed sparse file; four 4096-byte data extents, holes restored as zero content. No preallocated length accepted as processed bytes."}
    (out/"manifest.json").write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(result,ensure_ascii=False))

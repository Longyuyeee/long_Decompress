"""GNU tar originals from self-authored sparse files; run with WSL Python.

Binary fixtures are generated outputs, not hand-authored TAR headers. Existing
manifests are never overwritten. Re-generation in a fresh destination has its
own archive identity (GNU sparse names can contain the producer PID).
"""
import bz2
import gzip
import hashlib
import json
import lzma
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timezone

destination = Path(sys.argv[1]).resolve()
destination.mkdir(parents=True, exist_ok=True)
if (destination / "manifest.json").exists():
    raise SystemExit("Existing original manifest preserved; use a fresh destination")

def digest(file):
    h = hashlib.sha256()
    with open(file, "rb") as stream:
        while chunk := stream.read(1024 * 1024):
            h.update(chunk)
    return h.hexdigest()

def identity(file):
    return {"path": file.relative_to(destination).as_posix(), "bytes": file.stat().st_size, "sha256": digest(file)}

producer = subprocess.check_output(["tar", "--version"], text=True).splitlines()[0]
assert producer == "tar (GNU tar) 1.35", producer
cases = []
with tempfile.TemporaryDirectory(prefix="long-tar1513-") as scratch:
    source = Path(scratch) / "source"
    source.mkdir()
    long_name = "文档/" + "目录层级" * 9 + "/说明文件.txt"
    (source / long_name).parent.mkdir(parents=True)
    (source / long_name).write_text("用户只要这份长路径说明。\nPAX / GNU long name.\n", encoding="utf-8")
    (source / "空目录").mkdir()
    (source / "空.txt").touch()
    (source / "末尾.txt").write_text("最后一条内容不能漏掉。\n" * 40, encoding="utf-8")
    with open(source / "稀疏.bin", "wb") as stream:
        for i in range(7):
            stream.seek(i * 1024 * 1024)
            stream.write(bytes([i + 33]) * 4096)
        stream.truncate(8 * 1024 * 1024 + 33)
    with open(source / "全零.bin", "wb") as stream:
        stream.truncate(128 * 1024)
    expected = [{"path": p.relative_to(source).as_posix(), "bytes": p.stat().st_size, "sha256": digest(p)}
                for p in source.rglob("*") if p.is_file()]
    expected.sort(key=lambda p: p["path"])
    fixed = ["--sort=name", "--mtime=@1577934245", "--owner=0", "--group=0", "--numeric-owner"]
    for name, fmt, sparse_version in [("old-gnu", "gnu", None), ("pax00", "pax", "0.0"),
                                       ("pax01", "pax", "0.1"), ("pax10", "pax", "1.0"),
                                       ("pax-ordinary", "pax", "none")]:
        archive = destination / (name + ".tar")
        arguments = ["tar", "--format=" + fmt, *fixed]
        if fmt == "pax":
            arguments += ["--pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime"]
        if sparse_version != "none":
            arguments += ["--sparse", "--hole-detection=raw"]
        if sparse_version not in (None, "none"):
            arguments += ["--sparse-version=" + sparse_version]
        arguments += ["-cf", str(archive), "-C", str(source), "."]
        subprocess.run(arguments, check=True)
        decoded = Path(scratch) / ("gnu-extracted-" + name)
        decoded.mkdir()
        subprocess.run(["tar", "-xf", str(archive), "-C", str(decoded)], check=True)
        for item in expected:
            assert digest(decoded / item["path"]) == item["sha256"]
        raw = archive.read_bytes()
        files = [archive]
        for extension, data in [("gz", gzip.compress(raw, mtime=0)), ("bz2", bz2.compress(raw)), ("xz", lzma.compress(raw))]:
            wrapped = destination / (archive.name + "." + extension)
            wrapped.write_bytes(data)
            files.append(wrapped)
        if shutil.which("zstd"):
            wrapped = destination / (archive.name + ".zst")
            subprocess.run(["zstd", "-q", "-f", str(archive), "-o", str(wrapped)], check=True)
            files.append(wrapped)
        cases += [{"name": name + "-" + p.name.removeprefix(name + "."), "layout": name,
                   "archive": p.name, "original": identity(p), "expected": expected,
                   "producerArguments": arguments, "gnuFullContentHashes": True} for p in files]
    # Non-regular entries are a separate original, not positive file support.
    special = Path(scratch) / "special"
    special.mkdir()
    (special / "说明.txt").write_text("可单独提取的普通文件。\n", encoding="utf-8")
    os.link(special / "说明.txt", special / "硬链接.txt")
    os.symlink("说明.txt", special / "符号链接.txt")
    os.mkfifo(special / "管道")
    archive = destination / "non-regular.tar"
    subprocess.run(["tar", "--format=pax", *fixed, "-cf", str(archive), "-C", str(special), "."], check=True)
    non_regular = {"archive": archive.name, "original": identity(archive),
                   "selected": "硬链接.txt", "regularSha256": digest(special / "硬链接.txt")}

faults = []
def fault(name, source_name, transform, category):
    file = destination / name
    file.write_bytes(transform((destination / source_name).read_bytes()))
    faults.append({"archive": name, "from": source_name, "derivedNotOriginal": True,
                   "expectedCategory": category, "identity": identity(file)})

def unknown_version(data):
    assert b"GNU.sparse.major=1" in data
    return data.replace(b"GNU.sparse.major=1", b"GNU.sparse.major=9", 1)

def wrong_size(data):
    needle = b"GNU.sparse.realsize=8388641"
    assert needle in data
    return data.replace(needle, b"GNU.sparse.realsize=0388641", 1)

def overlapping(data):
    marker = b"GNU.sparse.map="
    search = 0
    while True:
        at = data.index(marker, search) + len(marker)
        end = data.index(b"\n", at)
        values = data[at:end].split(b",")
        if len(values) > 4:
            break
        search = end
    values[2] = b"0" * len(values[2])
    replacement = b",".join(values)
    assert len(replacement) == end - at
    return data[:at] + replacement + data[end:]

fault("derived-pax10-unknown.tar", "pax10.tar", unknown_version, "unsupported")
fault("derived-pax10-size.tar", "pax10.tar", wrong_size, "damaged")
fault("derived-pax01-overlap.tar", "pax01.tar", overlapping, "damaged")
fault("derived-pax10-footer.tar.gz", "pax10.tar.gz", lambda b: b[:-5], "damaged")
manifest = {"createdAt": datetime.now(timezone.utc).isoformat(), "producer": producer,
            "producerSha256": digest(Path(shutil.which("tar"))),
            "sources": "Self-authored fixed sparse/zero/empty/Unicode long-path payloads; no external user files",
            "longName": long_name, "cases": cases, "nonRegular": non_regular, "faults": faults}
(destination / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"producer": producer, "cases": len(cases), "faults": len(faults), "destination": str(destination)}, ensure_ascii=False))

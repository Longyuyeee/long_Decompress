"""Independently read original TAR payloads and verify actual selected publications."""
from pathlib import Path
import ctypes, hashlib, json, os, tarfile

root = Path(__file__).resolve().parents[1]
base = root / 'test-results/tar-selected-1720'
receipts = json.loads((base / 'native.json').read_text(encoding='utf-8'))
receipts.append(json.loads((base / 'single.json').read_text(encoding='utf-8')))
checks = []

def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        while data := f.read(65536): h.update(data)
    return h.hexdigest()

for receipt in receipts:
    source = Path(receipt['source']).resolve()
    manifest = json.loads((source.parent / 'manifest.json').read_text(encoding='utf-8'))
    variant = next((v for v in manifest.get('variants', []) if v['archive'] == source.name), manifest)
    assert digest(source) == variant['sha256']
    tar_path = source.with_suffix('')
    assert digest(tar_path) == variant['tarSha256']
    oracle = {e['name']: e for e in variant['entries']}
    output = Path(receipt['outputDirectory']).resolve()
    expected_files, expected_dirs, allocations, actual_files = {}, set(), [], []
    with tarfile.open(tar_path) as archive:
        members = archive.getmembers()
        by_name = {member.name: member for member in members}
        for entry in receipt['entries']:
            name = entry['name']
            original = oracle[name]
            own = by_name[name]
            expected_dirs.update(p.as_posix() for p in Path(name).parents if p != Path('.'))
            path = output / name
            # Each link's own timestamp is independent of the source header.
            assert path.stat().st_mtime_ns == int(own.mtime * 1_000_000_000), name
            if entry['isDirectory']:
                expected_dirs.add(name)
                continue
            # Use the producer-confirmed target identity for GNU sparse placeholder names.
            target = members[original['targetId']] if original.get('targetId') is not None else by_name.get(original.get('linkTarget')) if original.get('linkTarget') else own
            assert target is not None and target.isfile()
            h = hashlib.sha256()
            with archive.extractfile(target) as stream:
                while data := stream.read(65536): h.update(data)
            assert h.hexdigest() == original['sha256'] == digest(path), name
            assert path.stat().st_size == original['bytes'] and path.stat().st_nlink == 1, name
            expected_files[name] = original['sha256']
            actual_files.append({'path':str(path),'name':name,'is_dir':False,'size':path.stat().st_size,'modified_time':int(own.mtime)})
            if target.sparse and os.name == 'nt':
                assert path.stat().st_file_attributes & 0x200, name
                high = ctypes.c_ulong()
                fn = ctypes.windll.kernel32.GetCompressedFileSizeW
                fn.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_ulong)]
                fn.restype = ctypes.c_ulong
                low = fn(str(path), ctypes.byref(high))
                assert low != 0xffffffff
                allocated = (high.value << 32) | low
                if original['bytes'] >= 1048576: assert allocated < original['bytes'] // 2
                allocations.append({'name':name,'logicalBytes':original['bytes'],'allocatedBytes':allocated})
    assert {p.relative_to(output).as_posix():digest(p) for p in output.rglob('*') if p.is_file()} == expected_files
    assert {p.relative_to(output).as_posix() for p in output.rglob('*') if p.is_dir()} == expected_dirs
    checks.append({'source':str(source),'outputDirectory':str(output),'files':len(expected_files),'directories':len(expected_dirs),'exactLogicalHashesAndOwnTimes':True,'allOutputsIndependent':True,'allocations':allocations,'actualFiles':actual_files})

(base / 'verified.json').write_text(json.dumps({'passed':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'{len(checks)} actual publication trees / {sum(c["files"] for c in checks)} files verified against independent TAR contents, paths, own times, sparse allocation and link counts.')

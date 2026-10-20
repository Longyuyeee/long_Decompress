"""Maintainer-only native SWM originals; independent pinned wimlib, never shipped."""
from pathlib import Path
import argparse, hashlib, json, subprocess, tempfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--tool', type=Path, required=True)
parser.add_argument('--output', type=Path, default=root/'tests/fixtures/archives/native-swm-1529')
args = parser.parse_args()
tool = args.tool.resolve()
sha = lambda data: hashlib.sha256(data).hexdigest()
assert sha(tool.read_bytes()) == '34c0c4165591ad1f592837ed99d08273c58d6ed3fe0ed6360cf34e7b0739b353'
def run(*parts):
    return subprocess.run([str(tool), *map(str, parts)], check=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout
def tree(directory):
    return {p.relative_to(directory).as_posix(): sha(p.read_bytes()) for p in directory.rglob('*') if p.is_file()}

manifest = dict(generator='wimlib-imagex 1.14.5 x64', toolSha256=sha(tool.read_bytes()),
    packageUrl='https://wimlib.net/downloads/wimlib-1.14.5-windows-x86_64-bin.zip',
    packageSha256='2f446d6fa3866582175f1a22a7be198eeee0aec7aba5b4e04ad25c99eae2d265', cases=[])
args.output.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='long-swm-1529-original-') as temporary:
    work = Path(temporary)
    for index in [1, 2]:
        source = work/f'image{index}'
        (source/'项目').mkdir(parents=True); (source/'空目录').mkdir()
        (source/'说明.txt').write_text(f'原生 SWM 独立镜像 {index}，读取整组资源。\r\n', encoding='utf-8')
        (source/'空文件.txt').write_bytes(b'')
        (source/'项目/同名.txt').write_text(f'image {index}', encoding='utf-8')
        (source/'图标.png').write_bytes((root/'src-tauri/icons/32x32.png').read_bytes())
        # Each resource mixes fresh entropy and repetition in EVERY 32 KiB chunk.
        # Compressed methods must really compress, while the set still spans 3+ parts.
        for resource in range(18):
            chunks = []
            for chunk in range(8):
                entropy = b''.join(hashlib.sha256(f'{index}:{resource}:{chunk}:{n}'.encode()).digest() for n in range(512))
                chunks.append(entropy + bytes([65 + resource]) * 16384)
            (source/f'项目/数据{resource:02}.bin').write_bytes(b''.join(chunks))
    for method in ['none', 'xpress', 'lzx']:
        folder = args.output/method
        assert not folder.exists(), f'Refusing to replace fixed originals: {folder}'
        folder.mkdir()
        wim = work/(method+'.wim')
        run('capture', work/'image1', wim, '镜像一', '--compress='+method, '--check', '--no-acls', '--threads=1')
        count = 2 if method == 'lzx' else 1
        if count == 2: run('append', work/'image2', wim, '镜像二', '--check', '--no-acls', '--threads=1')
        first = folder/'install.swm'
        run('split', wim, first, 1, '--check')
        run('verify', first, '--ref='+str(folder/'install*.swm'))
        expected = {}; directories = []
        for index in range(1, count+1):
            output = work/f'{method}-readback-{index}'
            run('apply', first, index, output, '--ref='+str(folder/'install*.swm'), '--no-acls')
            source = work/f'image{index}'
            assert tree(output) == tree(source)
            prefix = str(index)+'/' if count > 1 else ''
            expected.update({prefix+name:digest for name,digest in tree(source).items()})
            wanted_dirs = [p.relative_to(source).as_posix() for p in source.rglob('*') if p.is_dir()]
            assert all((output/p).is_dir() for p in wanted_dirs)
            directories.extend(prefix+p for p in wanted_dirs)
        xml = work/(method+'.xml'); run('info', first, '--extract-xml='+str(xml))
        if count > 1: expected['[1].xml'] = sha(xml.read_bytes())
        members = []
        compressed_resources = 0
        for part in sorted(folder.glob('*.swm'), key=lambda p:int.from_bytes(p.read_bytes()[40:42], 'little')):
            data = part.read_bytes()
            # Non-solid WIM blob table entries: 24-byte resource, part/refcount/SHA1.
            table_size = int.from_bytes(data[48:55], 'little'); offset = int.from_bytes(data[56:64], 'little')
            assert not data[55] & 4, 'Generator expects an uncompressed blob table'
            for entry in range(offset, offset+table_size, 50):
                stored = int.from_bytes(data[entry:entry+7], 'little')
                unpacked = int.from_bytes(data[entry+16:entry+24], 'little')
                if data[entry+7] & 4 and stored < unpacked and unpacked >= 262144:
                    compressed_resources += 1
            members.append(dict(file=part.relative_to(args.output).as_posix(), sha256=sha(data), bytes=len(data),
                number=int.from_bytes(data[40:42], 'little'), total=int.from_bytes(data[42:44], 'little'), guid=data[24:40].hex()))
        assert len(members) >= 3 and all(p['total'] == len(members) for p in members)
        assert method == 'none' or compressed_resources >= 18
        manifest['cases'].append(dict(method=method, images=count, members=members, expected=expected,
            directories=directories, independentApply=True, compressedResources=compressed_resources))
(args.output/'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps(dict(cases=len(manifest['cases']), members=[len(c['members']) for c in manifest['cases']],
    compressedResources=[c['compressedResources'] for c in manifest['cases']], independentApply=True)))

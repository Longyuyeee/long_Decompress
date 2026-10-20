"""Maintainer-only: independent WIM originals, using pinned standalone wimlib."""
from pathlib import Path
import argparse, hashlib, json, subprocess, tempfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--tool', type=Path, required=True)
parser.add_argument('--output', type=Path, default=root/'tests/fixtures/archives/wim-volumes-1527')
args = parser.parse_args()
tool = args.tool.resolve()
sha = lambda data: hashlib.sha256(data).hexdigest()
def run(*parts):
    return subprocess.run([str(tool), *map(str, parts)], check=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout
manifest = dict(generator='wimlib-imagex 1.14.5 x64', toolSha256=sha(tool.read_bytes()),
    packageUrl='https://wimlib.net/downloads/wimlib-1.14.5-windows-x86_64-bin.zip',
    packageSha256='2f446d6fa3866582175f1a22a7be198eeee0aec7aba5b4e04ad25c99eae2d265', cases=[])
args.output.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='long-wim-1527-original-') as temporary:
    work=Path(temporary)
    for index in [1,2]:
        source=work/f'image{index}'
        (source/'项目').mkdir(parents=True);(source/'空目录').mkdir()
        (source/'说明.txt').write_text(f'独立 WIM 镜像 {index}，目录与内容保持一致。\r\n',encoding='utf-8')
        (source/'空文件.txt').write_bytes(b'')
        (source/'项目/同名.txt').write_text(f'image {index}',encoding='utf-8')
        random=b''.join(hashlib.sha256(f'{index}:{n}'.encode()).digest() for n in range(256))
        (source/'数据.bin').write_bytes((b'Independent WIM chunk data '*128+random)*32)
        (source/'图标.png').write_bytes((root/'src-tauri/icons/32x32.png').read_bytes())
    for method in ['none','xpress','lzx','lzms']:
        file=args.output/(method+'.wim')
        assert not file.exists(), f'Refusing to replace fixed original: {file}'
        run('capture',work/'image1',file,'镜像一','--compress='+method,'--check','--no-acls','--threads=1',*(['--solid'] if method=='lzms' else []))
        count=2 if method=='lzx' else 1
        if count==2:run('append',work/'image2',file,'镜像二','--check','--no-acls','--threads=1')
        run('verify',file)
        expected={};directories=[]
        for index in range(1,count+1):
            output=work/(method+'-readback-'+str(index));run('apply',file,index,output,'--no-acls')
            source=work/f'image{index}'
            actual={p.relative_to(output).as_posix():sha(p.read_bytes()) for p in output.rglob('*') if p.is_file()}
            wanted={p.relative_to(source).as_posix():sha(p.read_bytes()) for p in source.rglob('*') if p.is_file()}
            assert actual==wanted
            prefix=str(index)+'/' if count>1 else ''
            expected.update({prefix+name:digest for name,digest in wanted.items()})
            directories.extend(prefix+p.relative_to(source).as_posix() for p in source.rglob('*') if p.is_dir())
        xml=file.with_suffix('.xml');run('info',file,'--extract-xml='+str(xml))
        if count>1: expected['[1].xml']=sha(xml.read_bytes())
        manifest['cases'].append(dict(file=file.name,sha256=sha(file.read_bytes()),method=method,images=count,expected=expected,directories=directories,independentApply=True))
(args.output/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(originals=len(manifest['cases']),independentApply=True)))

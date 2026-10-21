"""Maintainer-only, independent official ZPAQ 7.15 three-transaction originals."""
from pathlib import Path
import argparse, hashlib, json, os, subprocess, tempfile
root=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser();parser.add_argument('--tool',type=Path,required=True)
parser.add_argument('--output',type=Path,default=root/'tests/fixtures/archives/zpaq-native-1564');args=parser.parse_args()
tool=args.tool.resolve();sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha(tool.read_bytes())=='7a94b4f1d6323a758c7b0b6344036f166bff0fd44f1c3c86f05b3688023496cb'
args.output.mkdir(parents=True,exist_ok=True)
manifest=dict(upstream='ZPAQ 7.15 official x64',toolSha256=sha(tool.read_bytes()),packageUrl='https://www.mattmahoney.net/dc/zpaq715.zip',packageSha256='e85ec2529eb0ba22ceaeabd461e55357ef099b80f61c14f377b429ea3d49d418',password='多部分 密码1564',cases=[])
with tempfile.TemporaryDirectory(prefix='long-zpaq-1530-original-') as temp:
 work=Path(temp)
 def run(*parts):return subprocess.run([str(tool),*map(str,parts)],cwd=work,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=True).stdout
 for encrypted in [False,True]:
  source=work/'资料';source.mkdir();(source/'空目录').mkdir();(source/'项目').mkdir()
  stem='journal-key' if encrypted else 'journal'
  file=args.output/(stem+'??.zpaq');reuse=len(list(args.output.glob(stem+'[0-9]*.zpaq')))==3
  key=['-key',manifest['password']] if encrypted else [];snapshots=[]
  for version in [1,2,3]:
   (source/'说明.txt').write_text(f'ZPAQ 历史快照 {version}：这份内容属于第 {version} 次更新。\r\n',encoding='utf-8')
   (source/'空文件.txt').write_bytes(b'')
   (source/'图标.png').write_bytes((root/'src-tauri/icons'/('32x32.png' if version==1 else '128x128.png')).read_bytes())
   if version==1:(source/'后来删除.txt').write_text('第一版独有，之后删除。',encoding='utf-8')
   if version==2:
    (source/'后来删除.txt').unlink();(source/'新增.txt').write_text('第二版新增，第三版改名。',encoding='utf-8')
   if version==3:(source/'新增.txt').rename(source/'已改名.txt')
   data=b''.join(hashlib.sha256(f'{version}:{n}'.encode()).digest() for n in range(4096))
   (source/'项目/数据.bin').write_bytes(data)
   for p in source.rglob('*'):os.utime(p,(1792490400+version*3600,1792490400+version*3600))
   if not reuse:run('a',file,'资料','-method','14','-threads','1','-force',*key)
   expected={'资料/'+p.relative_to(source).as_posix():sha(p.read_bytes()) for p in source.rglob('*') if p.is_file()}
   directories=['资料']+['资料/'+p.relative_to(source).as_posix() for p in source.rglob('*') if p.is_dir()]
   snapshots.append(dict(version=version,expected=expected,directories=directories))
  for snap in snapshots:
   output=work/f'readback-{encrypted}-{snap["version"]}'
   run('x',file,'-until',snap['version'],'-to',output,'-threads','1','-noattributes',*key)
   actual={p.relative_to(output).as_posix():sha(p.read_bytes()) for p in output.rglob('*') if p.is_file()}
   assert actual==snap['expected'];assert all((output/d).is_dir() for d in snap['directories'])
  parts=sorted(args.output.glob(stem+'[0-9]*.zpaq'));assert len(parts)==3
  listing=run('l',file,'-all','-summary','100',*key).decode('utf-8',errors='replace')
  import re
  dates=sorted(re.findall(r'^- (20\d\d-\d\d-\d\d \d\d:\d\d:\d\d)\s+\d+\s+(\d{4})/ \+',listing,re.M),key=lambda row:int(row[1]))
  if len(dates)!=3:raise RuntimeError(listing)
  for snap,(date,version) in zip(snapshots,dates):assert snap['version']==int(version);snap['date']=int(re.sub(r'\D','',date))
  manifest['cases'].append(dict(stem=stem,parts=[dict(file=p.name,sha256=sha(p.read_bytes()),size=p.stat().st_size)for p in parts],encrypted=encrypted,snapshots=snapshots,independentAllSnapshots=True))
  # Only this generator-owned temporary source is reset between credentials.
  import shutil
  assert source.resolve().parent==work.resolve();shutil.rmtree(source)
(args.output/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(cases=len(manifest['cases']),snapshots=3,independentAllSnapshots=True)))

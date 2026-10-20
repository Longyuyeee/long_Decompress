"""Pinned independent solid-WIM/ESD framing and complete-content probe."""
from pathlib import Path
import hashlib,json,subprocess,time,struct
root=Path(__file__).resolve().parents[1];base=root/'test-results/wim-solid-1539';base.mkdir(parents=True,exist_ok=True)
wim=root/'src-tauri/resources/wim-engine/wimlib-imagex.exe';seven=root/'src-tauri/resources/archive-engine/7z.exe'
def run(args):
 p=subprocess.run([str(x) for x in args],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 assert p.returncode==0,(args,p.stdout.decode('utf-8',errors='replace'),p.stderr.decode('utf-8',errors='replace'))
 return p.stdout.decode('utf-8',errors='replace')
def tree(p):return {x.relative_to(p).as_posix():hashlib.sha256(x.read_bytes()).hexdigest() for x in p.rglob('*') if x.is_file()}
source=base/'source';data=source/'资料';(data/'项目/空目录').mkdir(parents=True,exist_ok=True)
(data/'说明.txt').write_text('固实 WIM / ESD 中文内容。\n'*200,encoding='utf-8');(data/'项目/说明.txt').write_text('同名层级保持。\n'*100,encoding='utf-8');(data/'空文件.txt').write_bytes(b'');(data/'图标.png').write_bytes((root/'src-tauri/icons/32x32.png').read_bytes());(data/'pagefile.sys').write_bytes(b'ordinary selected file')
payload=bytearray()
for c in range(128):
 for n in range(512):payload.extend(hashlib.sha256(f'{c}:{n}'.encode()).digest())
 payload.extend(bytes([65+c%26])*16384)
(data/'项目/数据.bin').write_bytes(payload)
config=base/'capture.ini';config.write_text('[ExclusionList]\n',encoding='utf-8');expected=tree(source);cases=[]
for extension in ['wim','esd']:
 output=base/('oracle.'+extension)
 reused=output.exists();began=time.monotonic();log=(base/('capture-'+extension+'.log')).read_text(encoding='utf-8') if reused else run([wim,'capture',source,output,'First','--solid','--solid-compress=LZMS','--solid-chunk-size=67108864','--threads=1','--no-acls','--check','--config='+str(config)])
 raw=output.read_bytes();assert raw[:8]==b'MSWIM\0\0\0';version=struct.unpack_from('<I',raw,12)[0];assert version==0xe00
 table_size=struct.unpack_from('<Q',raw,48)[0]&0xffffffffffffff;offset=struct.unpack_from('<Q',raw,56)[0];solid=sum(bool(raw[i+7]&16) for i in range(offset,offset+table_size,50));assert solid>0
 listing=(base/('listing-'+extension+'.log')).read_text(encoding='utf-8') if reused else run([seven,'l','-slt','-sccUTF-8',output]);assert 'LZMS' in listing
 target=base/('decoded-'+extension);
 if not target.exists(): run([seven,'x','-y','-sccUTF-8',output,'-o'+str(target)])
 assert tree(target)==expected;assert (target/'资料/项目/空目录').is_dir()
 cases.append(dict(extension=extension,version=version,solidReferences=solid,bytes=len(raw),elapsedSeconds=None if reused else time.monotonic()-began,reusedOutput=reused,independentHashes=True,emptyDirectory=True,sha256=hashlib.sha256(raw).hexdigest()))
 (base/('capture-'+extension+'.log')).write_text(log,encoding='utf-8');(base/('listing-'+extension+'.log')).write_text(listing,encoding='utf-8')
# Real second image in the same solid ESD, with different contents and a separate empty directory.
second=base/'second';(second/'第二镜像/空目录').mkdir(parents=True,exist_ok=True);(second/'第二镜像/说明.txt').write_text('第二个镜像独立内容',encoding='utf-8')
multi=base/'multi.esd'
if not multi.exists():
 multi.write_bytes((base/'oracle.esd').read_bytes());run([wim,'append',second,multi,'Second','--solid','--solid-compress=LZMS','--solid-chunk-size=67108864','--threads=1','--no-acls','--check','--config='+str(config)])
listing=run([seven,'l','-slt','-sccUTF-8',multi]);(base/'listing-multi.log').write_text(listing,encoding='utf-8');decoded=base/'decoded-multi'
if not decoded.exists():run([seven,'x','-y','-sccUTF-8',multi,'-o'+str(decoded)])
raw=multi.read_bytes();xml_size=struct.unpack_from('<Q',raw,72)[0]&0xffffffffffffff;xml_offset=struct.unpack_from('<Q',raw,80)[0];xml=raw[xml_offset:xml_offset+xml_size]
multi_expected={**{'1/'+k:v for k,v in expected.items()},**{'2/'+k:v for k,v in tree(second).items()},'[1].xml':hashlib.sha256(xml).hexdigest()};assert tree(decoded)==multi_expected;assert (decoded/'1/资料/项目/空目录').is_dir() and (decoded/'2/第二镜像/空目录').is_dir()
result=dict(tool='wimlib 1.14.5 capture/append; independent 7-Zip 26.04 decode',source=str(source),expected=expected,multiExpected=multi_expected,cases=cases,multipleImages=True,solidChunkBytes=67108864,threads=1)
(base/'probe.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(json.dumps({k:v for k,v in result.items() if k not in ['expected','multiExpected']},ensure_ascii=True))

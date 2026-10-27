"""Python producer originals, independent content and directory-time oracles."""
from pathlib import Path
from decimal import Decimal, ROUND_FLOOR
import hashlib,io,json,random,subprocess,tarfile,datetime,re
root=Path(__file__).resolve().parents[1];fixture=root/'tests/fixtures/archives/tar-directory-time-1712';base=root/'test-results/tar-directory-time-1712'
fixture.mkdir(parents=True,exist_ok=True);base.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
def linux(*args):return subprocess.run(['wsl','-d','Ubuntu','--exec',*args],capture_output=True,check=True).stdout
lzop=root/'test-results/lzop-multi-create-172/research/lzop-linux';provenance=json.loads((root/'tests/fixtures/archives/lzop-prefix-174/manifest.json').read_text(encoding='utf-8'));assert sha(lzop.read_bytes())==provenance['producerSha256']
cases=[];differences=[]
for kind in ('gnu','pax'):
    source=[('资料',True,'1700001712.125'),('资料/deep/nested',True,'1700001713.123456700'),('资料/deep/nested/first.txt',False,'1700000001'),('资料/deep',True,'-1.25'),('资料/deep/nested/empty',True,'0'),('solo-empty',True,'-12000000000'),('other',True,'1700001715.123456789'),('资料/deep/nested/late.bin',False,'1700001717.5'),('implicit-only/file.txt',False,'1700001716')]
    if kind=='gnu':source=[(n,d,str(int(Decimal(t))))for n,d,t in source]
    data=io.BytesIO();entries=[]
    with tarfile.open(fileobj=data,mode='w',format=tarfile.GNU_FORMAT if kind=='gnu'else tarfile.PAX_FORMAT,pax_headers={'mtime':source[0][2]}if kind=='pax'else None)as writer:
        for index,(name,directory,time)in enumerate(source):
            content=b''if directory else random.Random(1712).randbytes(1100178)if name.endswith('late.bin')else f'original file {name}\n'.encode()
            info=tarfile.TarInfo(name);info.type=tarfile.DIRTYPE if directory else tarfile.REGTYPE;info.mode=0o755 if directory else 0o644;info.mtime=int(Decimal(time));info.size=len(content)
            if kind=='pax'and index:info.pax_headers={'mtime':time}
            writer.addfile(info,None if directory else io.BytesIO(content))
            seconds=int(Decimal(time).to_integral_value(rounding=ROUND_FLOOR));nanos=int((Decimal(time)-seconds)*1_000_000_000)
            entries.append({'name':name,'isDirectory':directory,'bytes':len(content),'sha256':sha(content),'modifiedTime':{'seconds':seconds,'nanoseconds':nanos}})
    path=fixture/f'{kind}.tar';archive=fixture/f'{kind}.tar.lzo';raw=data.getvalue();path.write_bytes(raw);archive.write_bytes(linux(unix(lzop),'-c','-N',unix(path)));assert linux(unix(lzop),'-d','-c',unix(archive))==raw
    listing=linux('tar','--full-time','--utc','-tvf',unix(path)).decode()
    with tarfile.open(fileobj=io.BytesIO(raw))as reader:
        members=list(reader);assert len(members)==len(entries)
        for member,e in zip(members,entries):
            assert member.name.rstrip('/')==e['name']and member.isdir()==e['isDirectory']
            if member.isfile():assert sha(reader.extractfile(member).read())==e['sha256']and sha(linux('tar','-xOf',unix(path),member.name))==e['sha256']
            t=e['modifiedTime'];assert abs(member.mtime-(t['seconds']+t['nanoseconds']/1e9))<.000001
            line=next(line for line in listing.splitlines()if line.rstrip('/').endswith(' '+e['name']));match=re.search(r'(\d{4}-\d\d-\d\d) (\d\d:\d\d:\d\d)(?:\.(\d+))?',line);assert match,line
            dt=datetime.datetime.fromisoformat(match[1]+'T'+match[2]).replace(tzinfo=datetime.timezone.utc);observed={'seconds':int(dt.timestamp()),'nanoseconds':int((match[3]or'').ljust(9,'0'))}
            if observed!=t:
                assert member.pax_headers['mtime'].startswith('-');differences.append({'name':e['name'],'value':member.pax_headers['mtime'],'decimalPython':t,'gnu135':observed})
    cases.append({'name':kind,'tar':path.name,'archive':archive.name,'tarSha256':sha(raw),'sha256':sha(archive.read_bytes()),'entries':entries,'gnuFullTimeListing':listing,'independentContentAgree':True})
manifest={'sources':'Self-authored Python GNU/PAX producer originals, fixed-SHA official lzop. Python/Decimal directory times and GNU/Python contents independently checked. GNU 1.35 negative-fraction disagreement recorded as in v1.7.11, not claimed as consensus. Explicit state-deletion boundaries are separate tests. No application writer.','lzopProducerSha256':provenance['producerSha256'],'gnuVersion':linux('tar','--version').decode().splitlines()[0],'negativeFractionDifferences':differences,'cases':cases}
(fixture/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');(base/'official.json').write_text(json.dumps({'passed':True,**manifest},ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('2 independent originals: directory times, ordering, paths and contents verified; GNU negative-fraction difference retained.')

"""Independent Python producer + GNU time/content oracle. No application writer."""
from pathlib import Path
from decimal import Decimal, ROUND_FLOOR
import hashlib, io, json, random, re, subprocess, tarfile, datetime
root=Path(__file__).resolve().parents[1]
fixture=root/'tests/fixtures/archives/tar-time-1711';base=root/'test-results/tar-time-1711'
fixture.mkdir(parents=True,exist_ok=True);base.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
def linux(*args):return subprocess.run(['wsl','-d','Ubuntu','--exec',*args],capture_output=True,check=True).stdout
lzop=root/'test-results/lzop-multi-create-172/research/lzop-linux'
provenance=json.loads((root/'tests/fixtures/archives/lzop-prefix-174/manifest.json').read_text(encoding='utf-8'))
assert sha(lzop.read_bytes())==provenance['producerSha256']
cases=[];differences=[]
for kind in ('gnu','pax'):
    expected=[];data=io.BytesIO()
    times=['1700001711.123456700','-1.123456700','0','1700001712.123456789','-12000000000','1700001713.500000000','1700001714'] if kind=='pax' else ['1700001711','-1','0','1700001712','1','42','1700001714']
    names=['global.txt','资料/负时间.txt','zero.txt','precision.txt','outside-windows.txt','empty-file.txt','late.bin']
    with tarfile.open(fileobj=data,mode='w',format=tarfile.PAX_FORMAT if kind=='pax' else tarfile.GNU_FORMAT,pax_headers={'mtime':times[0]}if kind=='pax'else None)as writer:
        directory=tarfile.TarInfo('empty');directory.type=tarfile.DIRTYPE;directory.mtime=1700000000;writer.addfile(directory)
        expected.append({'name':'empty','isDirectory':True,'bytes':0,'sha256':sha(b'')})
        for index,(name,time)in enumerate(zip(names,times)):
            content=random.Random(1711).randbytes(180033)if name=='late.bin'else b''if name=='empty-file.txt'else f'independent original time: {time}\n'.encode()
            info=tarfile.TarInfo(name);info.size=len(content);info.mode=0o644;info.mtime=int(Decimal(time))
            if kind=='pax'and index:info.pax_headers={'mtime':time}
            writer.addfile(info,io.BytesIO(content))
            seconds=int(Decimal(time).to_integral_value(rounding=ROUND_FLOOR));nanos=int((Decimal(time)-seconds)*1_000_000_000)
            expected.append({'name':name,'isDirectory':False,'bytes':len(content),'sha256':sha(content),'modifiedTime':{'seconds':seconds,'nanoseconds':nanos}})
    raw=data.getvalue();path=fixture/f'{kind}.tar';archive=fixture/f'{kind}.tar.lzo';path.write_bytes(raw)
    archive.write_bytes(linux(unix(lzop),'-c','-N',unix(path)))
    assert linux(unix(lzop),'-d','-c',unix(archive))==raw
    listing=linux('tar','--full-time','--utc','-tvf',unix(path)).decode()
    with tarfile.open(fileobj=io.BytesIO(raw))as reader:
        members=list(reader);assert len(members)==len(expected)
        for member,e in zip(members,expected):
            assert member.name.rstrip('/')==e['name']and member.isdir()==e['isDirectory']
            if not member.isfile():continue
            assert sha(reader.extractfile(member).read())==e['sha256']
            assert sha(linux('tar','-xOf',unix(path),member.name))==e['sha256']
            t=e['modifiedTime'];assert abs(member.mtime-(t['seconds']+t['nanoseconds']/1e9))<0.000001
            line=next(line for line in listing.splitlines()if line.endswith(' '+e['name']))
            match=re.search(r'(\d{4}-\d\d-\d\d) (\d\d:\d\d:\d\d)(?:\.(\d+))?',line);assert match,line
            dt=datetime.datetime.fromisoformat(match[1]+'T'+match[2]).replace(tzinfo=datetime.timezone.utc)
            observed={'seconds':int(dt.timestamp()),'nanoseconds':int((match[3]or'').ljust(9,'0'))}
            if observed!=t:
                assert kind=='pax'and member.pax_headers['mtime'].startswith('-')
                differences.append({'case':kind,'name':e['name'],'paxValue':member.pax_headers['mtime'],'decimalAndPythonTime':t,'gnu135Time':observed})
    cases.append({'name':kind,'tar':path.name,'archive':archive.name,'tarSha256':sha(raw),'sha256':sha(archive.read_bytes()),'entries':expected,'gnuFullTimeListing':listing,'independentContentAgree':True})
manifest={'sources':'Self-authored Python tarfile GNU/PAX producer originals; GNU/Python contents agree. Exact PAX times use Decimal plus Python reader and Go parsePAXTime signed semantics; GNU 1.35 negative-fraction disagreement is recorded, not claimed as consensus. Global/local deletion reuses v1.7.9 explicit POSIX-state boundary specimen. No application writer.','signedTimeReference':'https://go.dev/src/archive/tar/strconv.go','gnuVersion':linux('tar','--version').decode().splitlines()[0],'lzopProducerSha256':provenance['producerSha256'],'negativeFractionDifferences':differences,'cases':cases}
(fixture/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(base/'official.json').write_text(json.dumps({'passed':True,**manifest},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('2 independent producer originals: exact Decimal/Python times and GNU/Python contents checked; GNU negative-fraction differences recorded.')

"""Independent GNU records + Python PAX records, validated by GNU/Python readers."""
from pathlib import Path
import hashlib,io,json,subprocess,tarfile
root=Path(__file__).resolve().parents[1]
base=root/'test-results/tar-mixed-1710'; fixture=root/'tests/fixtures/archives/tar-mixed-1710'
base.mkdir(parents=True,exist_ok=True);fixture.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
unix=lambda p:'/mnt/'+str(p.resolve())[0].lower()+str(p.resolve())[2:].replace('\\','/')
def linux(*args):return subprocess.run(['wsl','-d','Ubuntu','--exec',*args],capture_output=True,check=True).stdout
lzop=root/'test-results/lzop-multi-create-172/research/lzop-linux'
provenance=json.loads((root/'tests/fixtures/archives/lzop-prefix-174/manifest.json').read_text(encoding='utf-8'))
assert sha(lzop.read_bytes())==provenance['producerSha256']
source=root/'tests/fixtures/archives/tar-lzo-gnu-178'
original=json.loads((source/'manifest.json').read_text(encoding='utf-8'))
raw=linux(unix(lzop),'-d','-c',unix(source/'complete.tar.lzo'))
assert sha(raw)==original['tarSha256']
version=linux('tar','--version').decode().splitlines()[0]
assert version=='tar (GNU tar) 1.35'
def pax(fields,global_record=False):return tarfile.TarInfo._create_pax_generic_header(fields,tarfile.XGLTYPE if global_record else tarfile.XHDTYPE,'utf-8')
previous=json.loads((fixture/'manifest.json').read_text(encoding='utf-8'))if(fixture/'manifest.json').exists()else{}
cached={c['name']:c for c in previous.get('cases',[])}
cases=[];reused=0
for order in ('lx','xl','alternating','overrides','padded'):
    data=bytearray();records=[];logical=0;at=0;mixes=0
    if order in ('alternating','overrides'):data.extend(pax({'comment':'mixed state remains local','mtime':'1700001710'},True))
    pending=False
    while any(raw[at:at+512]):
        header=raw[at:at+512];size=int(header[124:136].rstrip(b'\0 ').lstrip(b' ')or b'0',8)
        end=at+512+(size+511)//512*512
        if header[156:157]==b'L':
            # The following physical member is ordinary file/dir from GNU's
            # original. Its contents and physical size are kept unchanged.
            entry=original['entries'][logical]
            fields={'size':str(entry['bytes']),'mtime':'1700001710'}
            if order not in ('alternating','overrides') or mixes%2==0:fields['path']=entry['name']+('/'if entry['isDirectory']else'')
            extension=pax(fields)
            layout=('lx'if order=='padded'else order) if order not in ('alternating','overrides') else ('lx'if mixes%2==0 else'xl')
            long_record=bytearray(raw[at:end])
            if order=='padded':
                long_record.extend(b'\0'*(1024-(len(long_record)-512)));size=1024
                long_record[124:136]=f'{size:011o}\0'.encode();long_record[148:156]=b'        '
                long_record[148:156]=f'{sum(long_record[:512]):06o}\0 '.encode()
            start=len(data)
            if layout=='lx':
                records.extend([{'offset':len(data),'type':'L','size':size}]);data.extend(long_record)
                records.append({'offset':len(data),'type':'x','fields':fields});data.extend(extension)
            else:
                records.append({'offset':len(data),'type':'x','fields':fields});data.extend(extension)
                records.append({'offset':len(data),'type':'L','size':size});data.extend(long_record)
            records[-1]['mixedStart']=start
            mixes+=1
            pending=True
        else:
            chunk=bytearray(raw[at:end]);stored=size
            if order=='overrides'and pending and size>0:
                chunk[124:136]=b'00000000001\0';chunk[148:156]=b'        '
                chunk[148:156]=f'{sum(chunk[:512]):06o}\0 '.encode();stored=1
            records.append({'offset':len(data),'type':chr(header[156]),'logicalIndex':logical,'size':stored,'logicalBytes':size})
            data.extend(chunk);logical+=1;pending=False
        at=end
    data.extend(raw[at:])
    path=fixture/f'{order}.tar';archive=fixture/f'{order}.tar.lzo'
    old=cached.get(order)
    if old and path.exists()and archive.exists()and sha(data)==old['tarSha256']and sha(path.read_bytes())==old['tarSha256']and sha(archive.read_bytes())==old['sha256']:
        cases.append(old);reused+=1;continue
    path.write_bytes(data)
    compressed=linux(unix(lzop),'-c','-N',unix(path));archive.write_bytes(compressed)
    assert linux(unix(lzop),'-d','-c',unix(archive))==data
    listing=[n.rstrip('/')for n in linux('tar','-tf',unix(path)).decode().splitlines()]
    assert listing==[e['name']for e in original['entries']]
    with tarfile.open(fileobj=io.BytesIO(data))as reader:
        members=list(reader);assert len(members)==len(original['entries'])
        for member,expected in zip(members,original['entries']):
            assert member.name.rstrip('/')==expected['name']
            assert member.isdir()==expected['isDirectory']
            if member.isfile():
                content=linux('tar','-xOf',unix(path),member.name)
                assert sha(content)==expected['sha256']and len(content)==expected['bytes']
                assert reader.extractfile(member).read()==content
    cases.append({'name':order,'tar':path.name,'archive':archive.name,'tarSha256':sha(data),'sha256':sha(compressed),'entries':original['entries'],'records':records,'mixedPairs':mixes,'gnuAndPythonListingContentAgree':True})
manifest={'sources':'Self-authored GNU tar original with independently inserted Python tarfile PAX records; a derived interoperability specimen, not a third-party mixed archive. No application writer.',
    'gnuVersion':version,'sourceGnuTarSha256':original['tarSha256'],'lzopProducerSha256':provenance['producerSha256'],'lzopPackageSha256':provenance['packageSha256'],'cases':cases}
(fixture/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(base/'official.json').write_text(json.dumps({'passed':True,**manifest},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
probes=[]
for case in cases[:2]:
    record=next(r for r in case['records']if r['type']=='x');fields={**record['fields'],'path':'pax-conflicting-directory/'}
    data=bytearray((fixture/case['tar']).read_bytes());offset=record['offset'];old_size=int(data[offset+124:offset+136].rstrip(b'\0 '),8)
    length=512+(old_size+511)//512*512;replacement=pax(fields);data[offset:offset+length]=replacement
    path=base/f"{case['name']}-conflict.tar";path.write_bytes(data)
    gnu=[n.rstrip('/')for n in linux('tar','-tf',unix(path)).decode().splitlines()]
    with tarfile.open(fileobj=io.BytesIO(data))as reader:python=[m.name.rstrip('/')for m in reader]
    probes.append({'order':case['name'],'gnuNames':gnu,'pythonNames':python,'readersAgree':gnu==python})
(base/'conflict-probes.json').write_text(json.dumps(probes,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'GNU/Python agree on {len(cases)} mixed-order specimens; {reused} prior successful cases reused. Conflict probes recorded separately.')

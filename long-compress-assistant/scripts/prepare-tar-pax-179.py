"""Independent GNU/Python producers and GNU tar content oracle; no app writer."""
from pathlib import Path
import hashlib, io, json, random, subprocess, tarfile
root = Path(__file__).resolve().parents[1]
base = root / 'test-results/tar-pax-179'
fixture = root / 'tests/fixtures/archives/tar-pax-179'
source = base / 'source'
for path in (base, fixture, source): path.mkdir(parents=True, exist_ok=True)
sha = lambda b: hashlib.sha256(b).hexdigest()
unix = lambda p: '/mnt/' + str(p.resolve())[0].lower() + str(p.resolve())[2:].replace('\\','/')
lzop = root / 'test-results/lzop-multi-create-172/research/lzop-linux'
provenance = json.loads((root / 'tests/fixtures/archives/lzop-prefix-174/manifest.json').read_text(encoding='utf-8'))
assert sha(lzop.read_bytes()) == provenance['producerSha256']
def linux(*args): return subprocess.run(['wsl','-d','Ubuntu','--exec',*args], capture_output=True, check=True).stdout
version = linux('tar','--version').decode().splitlines()[0]
assert version == 'tar (GNU tar) 1.35'
payloads = {'prefix.txt': b'independent global PAX prefix\n', 'first.bin': random.Random(179).randbytes(700),
    'local.txt': b'local PAX override\n', 'second.bin': random.Random(1792).randbytes(700),
    'after.txt': b'global deletion restores raw header\n', 'local-empty.txt': b'local deletion is scoped to one entry\n',
    'again.bin': random.Random(1793).randbytes(700), 'late.bin': random.Random(1794).randbytes(1100179)}
for name, data in payloads.items(): (source/name).write_bytes(data)
(source/'empty').mkdir(exist_ok=True)
(source/'说明.txt').write_text('全局 PAX 与局部路径覆盖\n', encoding='utf-8')
def raw_header(name, size, kind=tarfile.REGTYPE):
    info = tarfile.TarInfo(name); info.size=size; info.type=kind; info.mtime=1700000000; info.mode=0o755 if kind==tarfile.DIRTYPE else 0o644
    return info.tobuf(format=tarfile.USTAR_FORMAT)
def metadata(fields, kind): return tarfile.TarInfo._create_pax_generic_header(fields, kind, 'utf-8')
state = bytearray(); expected = []; records=[]
def control(fields, kind):
    records.append({'offset': len(state), 'type': kind.decode(), 'fields': fields})
    state.extend(metadata(fields,kind))
def entry(raw_name, data, name=None, size_field=None, directory=False, mtime=1700000000):
    records.append({'offset': len(state), 'type': '5' if directory else '0', 'name': raw_name})
    state.extend(raw_header(raw_name,len(data) if size_field is None else size_field,tarfile.DIRTYPE if directory else tarfile.REGTYPE))
    state.extend(data); state.extend(b'\0' * (-len(data)%512))
    expected.append({'name':name or raw_name,'isDirectory':directory,'bytes':len(data),'sha256':sha(data),'mtime':mtime})
entry('prefix.txt',payloads['prefix.txt'])
control({'path':'global/first.bin','size':'700','mtime':'1700000179','uid':'179','comment':'persistent global state'},tarfile.XGLTYPE)
entry('raw-first',payloads['first.bin'],name='global/first.bin',size_field=1,mtime=1700000179)
control({'path':'local/说明.txt','size':str(len(payloads['local.txt'])),'mtime':'-1.25'},tarfile.XHDTYPE)
entry('raw-local',payloads['local.txt'],name='local/说明.txt',size_field=2,mtime=-1.25)
control({'path':'global/second.bin'},tarfile.XGLTYPE)
entry('raw-second',payloads['second.bin'],name='global/second.bin',size_field=0,mtime=1700000179)
control({'path':'','size':'','mtime':''},tarfile.XGLTYPE)
entry('after.txt',payloads['after.txt'])
control({'path':'hidden-global','size':'700','mtime':'1700000180'},tarfile.XGLTYPE)
control({'path':'','size':'','mtime':''},tarfile.XHDTYPE)
entry('local-empty.txt',payloads['local-empty.txt'])
control({'path':'global/again.bin'},tarfile.XGLTYPE)
entry('raw-again',payloads['again.bin'],name='global/again.bin',size_field=3,mtime=1700000180)
control({'path':'','size':'','mtime':'','uid':'','comment':''},tarfile.XGLTYPE)
entry('empty',b'',directory=True)
entry('late.bin',payloads['late.bin'])
state.extend(b'\0'*1024)
state.extend(b'\0'*(-len(state)%10240))
(fixture/'state.tar').write_bytes(state)
gnu = fixture/'gnu.tar'
linux('tar','--format=pax','--no-recursion','--mtime=@1700000179',
    '--pax-option=comment=GNU-global,uid=179,delete=atime,delete=ctime',
    '-cf',unix(gnu),'-C',unix(source),'prefix.txt','empty','说明.txt','late.bin')
python = fixture/'python.tar'
with tarfile.open(python,'w',format=tarfile.PAX_FORMAT,pax_headers={'comment':'Python-global','mtime':'1700000179','uname':'pax-user'}) as writer:
    for name in ('prefix.txt','empty','说明.txt','late.bin'):
        info=writer.gettarinfo(str(source/name),arcname=name); info.mtime=1700000000
        if info.isfile():
            with (source/name).open('rb') as data: writer.addfile(info,data)
        else: writer.addfile(info)
cases=[]
for path in (fixture/'state.tar',gnu,python):
    raw=path.read_bytes(); compressed=linux(unix(lzop),'-c','-N',unix(path)); target=path.with_suffix('.tar.lzo'); target.write_bytes(compressed)
    assert linux(unix(lzop),'-d','-c',unix(target)) == raw
    if path.stem=='state':
        # GNU tar 1.35 does not merge successive globals or accept empty numeric
        # deletion; Python 3.12 applies global size to intervening x records.
        # Keep this explicitly constructed POSIX-state boundary specimen separate
        # from the two producer originals with independently matching readers.
        entries=expected
        listing=[item['name'] for item in entries]
    else:
        listing=[name.rstrip('/') for name in linux('tar','-tf',unix(path)).decode().splitlines()]
        with tarfile.open(path) as archive:
            entries=[{'name':item.name.rstrip('/'),'isDirectory':item.isdir(),'bytes':item.size,
                'sha256':sha(archive.extractfile(item).read() if item.isfile() else b''),'mtime':item.mtime} for item in archive]
    assert listing == [item['name'] for item in entries], (path,listing,entries)
    for item in entries:
        if not item['isDirectory'] and path.stem!='state':
            actual=linux('tar','-xOf',unix(path),item['name'])
            assert len(actual)==item['bytes'] and sha(actual)==item['sha256'], (path,item)
    cases.append({'name':path.stem,'tar':path.name,'archive':target.name,'tarSha256':sha(raw),'sha256':sha(compressed),'entries':entries,
        'gnuListingAndContent':path.stem!='state','pythonReaderAgrees':path.stem!='state',
        'constructedStateBoundary':path.stem=='state'})
manifest={'tarProducer':version,'lzopProducerSha256':provenance['producerSha256'],'lzopPackageSha256':provenance['packageSha256'],
    'sources':'Self-authored files. GNU/Python producer originals independently agree. The constructed state specimen uses Python TarInfo headers, explicit source payload hashes and POSIX field semantics; GNU/Python readers do not agree on its boundary semantics.',
    'stateRecords':records,'cases':cases}
(fixture/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(base/'official.json').write_text(json.dumps({'passed':True,**manifest},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Two GNU/Python originals independently verified; one explicit POSIX-state boundary specimen prepared.')

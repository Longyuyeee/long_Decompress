"""Independent source epochs, both ZIP headers, and external UTC readback."""
from pathlib import Path
import datetime as dt
import hashlib
import json
import struct
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/zip-utc-write-1732'
SEVEN = ROOT / 'src-tauri/resources/archive-engine/7z.exe'
sha = lambda b: hashlib.sha256(b).hexdigest()
load = lambda p: json.loads(p.read_text(encoding='utf-8'))
cases = [load(BASE/'bounds.json')] + load(BASE/'native.json') + load(BASE/'choices.json')['cases'] + load(BASE/'conversion.json')


def fields(raw):
    result = []
    while raw:
        kind, size = struct.unpack_from('<HH', raw)
        assert len(raw) >= 4 + size
        result.append((kind, raw[4:4+size]))
        raw = raw[4+size:]
    return result


def legacy(seconds):
    if seconds < 0:
        return (1980,1,1,0,0,0)
    calendar = (dt.datetime(1970,1,1,tzinfo=dt.timezone.utc) + dt.timedelta(seconds=seconds)).astimezone()
    if calendar.year < 1980:
        return (1980,1,1,0,0,0)
    if calendar.year > 2107:
        return (2107,12,31,23,59,58)
    return (calendar.year,calendar.month,calendar.day,calendar.hour,calendar.minute,calendar.second//2*2)


def dos(date, clock):
    return (1980+(date>>9),(date>>5)&15,date&31,clock>>11,(clock>>5)&63,(clock&31)*2)


def exact(folder, sources):
    actual = {p.relative_to(folder).as_posix(): p for p in folder.rglob('*')}
    assert set(actual) == {s['name'] for s in sources}, ('exact tree', folder)
    for s in sources:
        p = actual[s['name']]
        assert p.is_dir() == s['directory'], p
        if s['utcExpected']:
            assert p.stat().st_mtime_ns == s['seconds']*1_000_000_000, ('UTC seconds',p)
        else:
            expected = dt.datetime(*legacy(s['seconds'])).timestamp()
            assert p.stat().st_mtime_ns == int(expected)*1_000_000_000, ('DOS fallback',p,expected,p.stat().st_mtime_ns)
        if not s['directory']:
            assert sha(p.read_bytes()) == s['sha256'], p


checks = []
native_outputs = 0
for index, case in enumerate(cases):
    sources = {s['name']: s for s in case['sources']}
    parts = [Path(p) for p in case['parts']]
    chunks = [p.read_bytes() for p in parts]
    raw = b''.join(chunks)
    starts, length = [], 0
    for chunk in chunks:
        starts.append(length)
        length += len(chunk)
    end = raw.rfind(b'PK\x05\x06')
    assert end >= 0
    size = struct.unpack_from('<I',raw,end+12)[0]
    central_end = end
    if raw[end-20:end-16] == b'PK\x06\x07':
        central_end = raw.rfind(b'PK\x06\x06',0,end)
        assert central_end >= 0 and central_end+12+struct.unpack_from('<Q',raw,central_end+4)[0] == end-20
        size = struct.unpack_from('<Q',raw,central_end+40)[0]
    position = central_end-size
    checked = []
    while position < central_end:
        assert raw[position:position+4] == b'PK\x01\x02'
        clock, date = struct.unpack_from('<HH',raw,position+12)
        n,x,c,disk = struct.unpack_from('<HHHH',raw,position+28)
        offset = struct.unpack_from('<I',raw,position+42)[0]
        name = raw[position+46:position+46+n].decode('utf-8').rstrip('/')
        s = sources[name]
        want_utc = 0 <= s['seconds'] <= 0x7fffffff
        assert want_utc == s['utcExpected']
        want_dos = legacy(s['seconds'])
        assert dos(date,clock) == want_dos, (name,'central DOS',dos(date,clock),want_dos)
        local = offset if case['numeric'] else starts[disk]+offset
        assert raw[local:local+4] == b'PK\x03\x04'
        lc,ld = struct.unpack_from('<HH',raw,local+10)
        assert dos(ld,lc) == want_dos
        ln,lx = struct.unpack_from('<HH',raw,local+26)
        central_utc = [v for k,v in fields(raw[position+46+n:position+46+n+x]) if k==0x5455]
        local_utc = [v for k,v in fields(raw[local+30+ln:local+30+ln+lx]) if k==0x5455]
        if want_utc:
            expected = b'\x01'+struct.pack('<I',s['seconds'])
            assert central_utc == local_utc == [expected], (name,'both UT headers')
        else:
            assert not central_utc and not local_utc, (name,'no ambiguous UT')
        if 'originalSource' not in case:
            p = Path(s['path'])
            assert p.stat().st_mtime_ns == s['seconds']*1_000_000_000+s['nanos'], ('unchanged source timestamp',p)
            if not s['directory']: assert sha(p.read_bytes()) == s['sha256']
        checked.append(dict(name=name,utcSeconds=s['seconds'] if want_utc else None,dos=want_dos))
        position += 46+n+x+c
    assert len(checked) == len(sources)
    for output in case['restored']:
        exact(Path(output),case['sources'])
        native_outputs += 1
    oracle_input = Path(case['input'])
    folder = BASE / f'external-{index}'
    if case['numeric']:
        oracle_input = BASE / f'joined-{index}.zip'
        oracle_input.write_bytes(raw)
    elif len(parts)>1 and oracle_input.suffix.lower()=='.zipx':
        family = BASE / f'zipx-family-{index}'
        family.mkdir(exist_ok=True)
        for i,chunk in enumerate(chunks):
            (family/('same.zip' if i==len(chunks)-1 else f'same.z{i+1:02d}')).write_bytes(chunk)
        oracle_input = family/'same.zip'
    result = subprocess.run([str(SEVEN),'x','-y','-sccUTF-8','-pUtcWrite1732!' if case['encrypted'] else '-p',str(oracle_input),'-o'+str(folder)],capture_output=True)
    assert result.returncode==0,(result.stdout,result.stderr)
    exact(folder,case['sources'])
    if 'originalSource' in case:
        original = Path(case['originalSource'])
        assert sha(original.read_bytes()) == case['originalSourceSha256']
        with zipfile.ZipFile(original) as archive:
            for info in archive.infolist():
                s=sources[info.filename.rstrip('/')]
                utc=[v for k,v in fields(info.extra) if k==0x5455]
                assert len(utc)==1 and struct.unpack_from('<i',utc[0],1)[0]==s['seconds']
                if not info.is_dir(): assert sha(archive.read(info))==s['sha256']
        assert case['privateFilesCleaned']
        for published in case['published']['files']:
            assert Path(published['path']).stat().st_size == published['bytes'] > 0
    checks.append(dict(name=case['name'],input=str(oracle_input),encodedEntries=len(checked),headers=checked,
                       localAndCentralUtcMatch=True,legacyDosCompatible=True,external7zipReadback=True,
                       sourceUnchanged=True,encrypted=case['encrypted']))
assert load(BASE/'choices.json')['workers']>1
receipt=dict(version='1.7.32',passed=True,archives=len(checks),encodedEntries=sum(c['encodedEntries'] for c in checks),
             nativeReadbackOutputs=native_outputs,actualParallelWorkers=load(BASE/'choices.json')['workers'],checks=checks,
             policy='Single UT modification field in both headers for 0..2147483647; integer seconds only, legacy DOS retained.')
(BASE/'verified.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:receipt[k]for k in ('passed','archives','encodedEntries','nativeReadbackOutputs')}))

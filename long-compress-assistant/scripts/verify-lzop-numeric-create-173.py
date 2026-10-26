"""Join fresh numeric outputs, then restore unique streams with official independent tools."""
import hashlib, json, subprocess, tarfile, tempfile, zipfile
from pathlib import Path
root = Path(__file__).resolve().parents[1]
base = root / 'test-results/lzop-numeric-create-173'
sha = lambda data: hashlib.sha256(data).hexdigest()
win = root / 'test-results/lzop-read-convert-1657/official-lzop/lzop103w/lzop.exe'
win_zip = root / 'test-results/lzop-read-convert-1657/lzop103w.zip'
assert sha(win_zip.read_bytes()) == '0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a'
with zipfile.ZipFile(win_zip) as z: assert z.read('lzop103w/lzop.exe') == win.read_bytes()
linux = root / 'test-results/lzop-multi-create-172/research/lzop-linux'
linux_package = linux.parent / 'lzop-1.03-i386_linux.tar.gz'
assert sha(linux_package.read_bytes()) == 'd4cd71272cd18ef2061ffe2a5a75dd11182ed9015c53a2283eeb41acb9a21370'
with tarfile.open(linux_package) as z:
    member = next(f for f in z.getmembers() if f.isfile() and Path(f.name).name == 'lzop')
    assert z.extractfile(member).read() == linux.read_bytes()
def unix(path):
    p = str(path.resolve()).replace('\\','/')
    return '/mnt/' + p[0].lower() + p[2:]
def canonical(entries): return sorted(entries, key=lambda e:e['path'])
cases = json.loads((base/'generated/manifest.json').read_text(encoding='utf-8'))
if (base/'desktop-outputs.json').exists(): cases += json.loads((base/'desktop-outputs.json').read_text(encoding='utf-8'))
tar_version = subprocess.run(['tar.exe','--version'],capture_output=True,text=True,check=True).stdout.strip()
receipt = {'version':'1.7.3','windowsVerifierSha256':sha(win.read_bytes()),'linuxVerifierSha256':sha(linux.read_bytes()),'tar':tar_version,'checks':[]}
old_file = base/'official.json'
previous = json.loads(old_file.read_text(encoding='utf-8')) if old_file.exists() else {}
compatible = all(previous.get(k)==receipt[k] for k in ['windowsVerifierSha256','linuxVerifierSha256','tar'])
new_count = 0
for case in cases:
    parts = [Path(p) for p in case['parts']]
    assert parts and all(p.is_file() and 0 < p.stat().st_size <= case['splitSize'] for p in parts)
    data = b''.join(p.read_bytes() for p in parts)
    assert sha(data) == case['streamSha256']
    assert len(parts) == (len(data)+case['splitSize']-1)//case['splitSize']
    prior = next((row for row in receipt['checks']+(previous.get('checks',[]) if compatible else []) if row['format']==case['format'] and row['streamSha256']==case['streamSha256'] and canonical(row['entries'])==canonical(case['entries']) and row.get('exactNamesAndHashes')),None)
    if prior:
        receipt['checks'].append({**case,'exactNamesAndHashes':True,'reusedReadback':True,'verifiedStreamSource':prior.get('verifiedStreamSource',prior['archive'])})
        continue
    work = Path(tempfile.mkdtemp(prefix='official-',dir=base)); joined = work/('stream.'+case['format']); joined.write_bytes(data)
    tested = subprocess.run([str(win),'-t','-N','-P',str(joined)],capture_output=True)
    assert tested.returncode==0,repr(tested.stderr)
    out = work/'restored';out.mkdir()
    result = subprocess.run(['wsl','-d','Ubuntu','--exec',unix(linux),'-d','-N','-P','-k','-p'+unix(out),unix(joined)],capture_output=True)
    assert result.returncode==0,repr(result.stderr)
    if case['format']=='tar.lzo':
        tar_path=out/'stream.tar'; assert tar_path.is_file()
        out=work/'tar-files';out.mkdir()
        subprocess.run(['tar.exe','-xf',str(tar_path),'-C',str(out)],capture_output=True,check=True)
    actual = {p.relative_to(out).as_posix():sha(p.read_bytes()) for p in out.rglob('*') if p.is_file()}
    expected = {e['path']:e['sha256'] for e in case['entries'] if not e['directory']}
    assert actual==expected,(actual,expected)
    for entry in case['entries']:
        if entry['directory']: assert (out/entry['path']).is_dir(),entry
    receipt['checks'].append({**case,'exactNamesAndHashes':True,'windowsTestExit':tested.returncode,'linuxRestoreExit':result.returncode,'reusedReadback':False,'verifiedStreamSource':case['archive']})
    new_count+=1
receipt['passed']=True
(base/'official.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'{len(cases)} numeric output groups have exact joined identities, names and hashes; {new_count} new independent readbacks, {len(cases)-new_count} reused stream receipts.')

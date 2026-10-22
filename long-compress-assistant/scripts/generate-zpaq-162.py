"""Official writer interruption originals. Never truncate or edit generated archives."""
from pathlib import Path
import hashlib, json, os, shutil, subprocess, tempfile, time

root = Path(__file__).resolve().parents[1]
tool = root/'test-results/zpaq-volumes-1526/upstream/zpaq64.exe'
base = root/'tests/fixtures/archives/zpaq-native-1564'
output = root/'tests/fixtures/archives/zpaq-interrupted-162'
manifest = json.loads((base/'manifest.json').read_text(encoding='utf-8'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(tool) == manifest['toolSha256']
output.mkdir(parents=True, exist_ok=True)
assert not (output/'manifest.json').exists(), 'Use frozen originals; do not regenerate silently'
cases = []
with tempfile.TemporaryDirectory(prefix='long-zpaq-162-writer-') as temp:
    work = Path(temp)
    added = work/'interrupted.bin'
    with added.open('wb') as f:
        for _ in range(64): f.write(os.urandom(1024*1024))
    for original in manifest['cases']:
        key = ['-key', manifest['password']] if original['encrypted'] else []
        for native in [False, True]:
            name = ('key-' if key else 'plain-') + ('native' if native else 'single')
            folder = output/name; folder.mkdir()
            members = [base/part['file'] for part in original['parts']]
            if native:
                for i, p in enumerate(members, 1): shutil.copyfile(p, folder/f'backup.part{i}.zpaq')
                archive = folder/'backup.part*.zpaq'; tail = folder/'backup.part4.zpaq'; start = 0
            else:
                archive = folder/'backup.zpaq'
                archive.write_bytes(b''.join(p.read_bytes() for p in members))
                tail = archive; start = archive.stat().st_size
            prefix = hashlib.sha256(b''.join(p.read_bytes() for p in members)).hexdigest()
            command = [str(tool), 'a', str(archive), added.name, '-method', '10', '-threads', '1', '-force', *key]
            with (folder/'writer.log').open('wb') as log:
                proc = subprocess.Popen(command, cwd=work, stdout=log, stderr=subprocess.STDOUT)
                deadline = time.monotonic()+60
                while proc.poll() is None:
                    if tail.exists() and tail.stat().st_size-start >= 32768:
                        proc.kill(); break
                    assert time.monotonic() < deadline, 'Writer did not emit transaction bytes'
                    time.sleep(.001)
                code = proc.wait()
            assert code != 0, 'Writer completed; this is not an interruption original'
            parts = sorted(folder.glob('*.zpaq'))
            if native:
                assert [sha(p) for p in parts[:3]] == [sha(p) for p in members]
            else: assert hashlib.sha256(tail.read_bytes()[:start]).hexdigest() == prefix
            restored = []
            for snapshot in original['snapshots']:
                dest = work/f'{name}-{snapshot["version"]}'
                r = subprocess.run([str(tool), 'x', str(archive), '-until', str(snapshot['version']), '-to', str(dest), '-threads', '1', '-noattributes', *key], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                assert r.returncode == 0, r.stdout.decode('utf-8','replace')
                actual = {p.relative_to(dest).as_posix(): sha(p) for p in dest.rglob('*') if p.is_file()}
                assert actual == snapshot['expected']
                restored.append(snapshot['version'])
            listed = subprocess.run([str(tool), 'l', str(archive), *key], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            text = listed.stdout.decode('utf-8','replace').replace('\r\n','\n')
            assert 'Incomplete transaction ignored' in text, text
            (folder/'official-list.log').write_text(text, encoding='utf-8', newline='\n')
            cases.append(dict(name=name, encrypted=bool(key), native=native, writerExit=code,
                interruption='TerminateProcess after appended data bytes became visible; no truncation or modification',
                transactionOffset=sum(p.stat().st_size for p in members), tailBytes=tail.stat().st_size-start,
                parts=[dict(file=p.relative_to(output).as_posix(), size=p.stat().st_size, sha256=sha(p)) for p in parts],
                snapshots=original['snapshots'], independentSnapshots=restored, originalPrefixUnchanged=True))
(output/'manifest.json').write_text(json.dumps(dict(upstream=manifest['upstream'], toolSha256=sha(tool), packageUrl=manifest['packageUrl'], password=manifest['password'], cases=cases), ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps([dict(name=c['name'],tailBytes=c['tailBytes'],independentSnapshots=c['independentSnapshots']) for c in cases]))

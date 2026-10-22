"""Opt-in local boundary exercise; requires generated fixtures and Bandizip portable.
Does not run in CI or claim historical-producer compatibility.
"""
import ctypes
from ctypes import wintypes
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT/'test-results/alz-bounded-168'
MANIFEST = json.loads((BASE/'fixtures/manifest.json').read_text(encoding='utf-8'))
report_file = BASE/'boundary.json'
report = dict(version='1.6.8', controlled=True, historicalProducer=False, checks=[])
if report_file.exists():
    report = json.loads(report_file.read_text(encoding='utf-8'))


def sha(file):
    with file.open('rb') as reader:
        return hashlib.file_digest(reader, 'sha256').hexdigest()


class Counters(ctypes.Structure):
    _fields_ = [('cb', wintypes.DWORD), ('PageFaultCount', wintypes.DWORD)] + [
        (name, ctypes.c_size_t) for name in ['PeakWorkingSetSize', 'WorkingSetSize',
        'QuotaPeakPagedPoolUsage', 'QuotaPagedPoolUsage', 'QuotaPeakNonPagedPoolUsage',
        'QuotaNonPagedPoolUsage', 'PagefileUsage', 'PeakPagefileUsage']]


memory = ctypes.WinDLL('psapi').GetProcessMemoryInfo
memory.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
memory.restype = wintypes.BOOL


def run(exe, arguments, request=None):
    startup = subprocess.STARTUPINFO()
    startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
    startup.wShowWindow = 0
    started = time.monotonic()
    process = subprocess.Popen([str(exe), *map(str, arguments)], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               startupinfo=startup, creationflags=subprocess.CREATE_NO_WINDOW)
    process.stdin.write(json.dumps(request or {}).encode())
    process.stdin.close()
    peak, private = 0, 0
    while True:
        counter = Counters(); counter.cb = ctypes.sizeof(counter)
        if memory(int(process._handle), ctypes.byref(counter), ctypes.sizeof(counter)):
            peak = max(peak, counter.PeakWorkingSetSize)
            private = max(private, counter.PeakPagefileUsage)
        status = process.poll()
        if status is not None:
            break
        if time.monotonic()-started > 180:
            process.kill(); raise TimeoutError(str(arguments))
        time.sleep(.02)
    return dict(executableSha256=sha(exe), arguments=list(map(str,arguments)), exitCode=status,
                seconds=round(time.monotonic()-started,3), peakWorkingSet=peak,
                peakPrivateBytes=private, stdout=process.stdout.read().decode('utf-8',errors='replace'),
                stderr=process.stderr.read().decode('utf-8',errors='replace'))


def record(name, execute):
    if any(c['name'] == name for c in report['checks']):
        return
    result = execute()
    report['checks'].append(dict(name=name, **result))
    report_file.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(name, json.dumps(result), flush=True)


old = BASE/'v1.6.7-long-alz.exe'
if not old.exists():
    old.write_bytes(subprocess.check_output(['git','show','94361cad361d342dc253bedc8c730c95bc80b325:long-compress-assistant/src-tauri/resources/alz-engine/long-alz.exe'],cwd=ROOT))
assert sha(old) == 'f5e74b6420da0f46239c943700fbe53e39efde6b178592b96b56a4366a304cd7'
new = ROOT/'tools/alz-engine/target/release/long-alz.exe'
independent = BASE/'independent/Bandizip.x64.exe'
large = next(c for c in MANIFEST['cases'] if c['name'] == 'large')
for item in report['checks']:
    if item['name'].startswith('candidate-'): assert item['executableSha256'] == sha(new)
    if item['name'].startswith('independent-'): assert item['executableSha256'] == sha(independent)
manifest_sha = sha(BASE/'fixtures/manifest.json')
assert report.get('fixtureManifestSha256', manifest_sha) == manifest_sha
report['fixtureManifestSha256'] = manifest_sha


def old_boundary():
    result = run(old, ['verify', ROOT/large['archive']], dict(encoding='cp949'))
    assert result['exitCode'] != 0 and 'compressed size exceeds limit' in result['stdout']+result['stderr'], result
    return result


record('old-whole-entry-limit', old_boundary)
for case in MANIFEST['cases']:
    archive = ROOT/case['archive']
    request = dict(encoding='cp949', password=case['password'])

    def verify():
        result = run(new, ['verify', archive], request)
        assert result['exitCode'] == 0 and json.loads(result['stdout'])['result']['verified'], result
        # This is a helper process measurement, not the entire desktop app.
        assert result['peakWorkingSet'] < 64*1024*1024, result
        return result
    record('candidate-'+case['name'], verify)

    def compare():
        folder = BASE/('independent-'+case['name'])
        assert not folder.exists(), 'Do not accept stale extracted files'
        args = ['x', '-y', '-cp:949', '-o:'+str(folder)]
        if case['password']:
            args.append('-p:'+case['password'])
        result = run(independent, [*args, archive])
        assert result['exitCode'] == 0, result
        actual = [dict(path=p.name, bytes=p.stat().st_size, sha256=sha(p)) for p in folder.iterdir() if p.is_file()]
        assert sorted(actual,key=lambda a:a['path']) == sorted(case['expected'],key=lambda a:a['path']), actual
        return dict(**result, outputHashes=actual)
    record('independent-'+case['name'], compare)
report['passed'] = True
report_file.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')

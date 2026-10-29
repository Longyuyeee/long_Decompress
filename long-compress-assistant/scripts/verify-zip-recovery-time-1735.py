"""Independent both-header timestamp inspection and final filesystem readback."""
from pathlib import Path
import datetime as dt
import hashlib
import json
import struct
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'test-results/zip-recovery-time-1735'
FIXTURES = ROOT / 'tests/fixtures/archives/zip-recovery-time-1735'
load = lambda p: json.loads(p.read_text(encoding='utf-8'))
sha = lambda b: hashlib.sha256(b).hexdigest()
cases = load(BASE / 'controls.json') + load(BASE / 'native.json') + load(BASE / 'streams.json')


def utc_fields(raw):
    fields = []
    while raw:
        kind, length = struct.unpack_from('<HH', raw)
        assert len(raw) >= 4 + length
        if kind == 0x5455: fields.append(raw[4:4 + length])
        raw = raw[4 + length:]
    return fields


def seconds(entry):
    return entry['utc'] if entry['utc'] is not None else int(dt.datetime(*entry['dos']).timestamp())


def exact(target, expected):
    actual = {p.relative_to(target).as_posix(): p for p in target.rglob('*')}
    assert set(actual) == {e['name'] for e in expected}, target
    for entry in expected:
        path = actual[entry['name']]
        assert path.is_dir() == entry['directory'], path
        if not entry['directory']:
            content = path.read_bytes()
            assert len(content) == entry['bytes'] and sha(content) == entry['sha256'], path
        assert path.stat().st_mtime_ns == seconds(entry) * 10**9, (path, path.stat().st_mtime_ns, seconds(entry))


for spec in load(FIXTURES / 'manifest.json')['cases']:
    raw = (FIXTURES / spec['archive']).read_bytes()
    if 'originalSha256' in spec: assert sha(raw) == spec['originalSha256']
    recipe = spec['recipe']
    if recipe['kind'] == 'prefix': raw = raw[:recipe['length']]
    if recipe['kind'] == 'xor':
        raw = bytearray(raw); raw[recipe['position']] ^= 1
    assert sha(raw) == spec['sha256']

checks = []
native_outputs = 0
for index, case in enumerate(cases):
    for original in case['sources']:
        assert sha(Path(original['path']).read_bytes()) == original['sha256']
    output = Path(case['output']); raw = output.read_bytes(); expected = case['expected']
    assert case['result']['verified'] and case['result']['partial']
    with zipfile.ZipFile(output) as archive:
        assert set(n.rstrip('/') for n in archive.namelist()) == {e['name'] for e in expected}
        for info in archive.infolist():
            entry = next(e for e in expected if e['name'] == info.filename.rstrip('/'))
            assert list(info.date_time) == entry['dos']
            local = info.header_offset
            name_len, extra_len = struct.unpack_from('<HH', raw, local + 26)
            extra = raw[local + 30 + name_len:local + 30 + name_len + extra_len]
            fields = [] if entry['utc'] is None else [b'\x01' + struct.pack('<I', entry['utc'])]
            assert utc_fields(extra) == utc_fields(info.extra) == fields, (case['name'], info.filename)
            clock, date = struct.unpack_from('<HH', raw, local + 10)
            dos = [1980 + (date >> 9), (date >> 5) & 15, date & 31, clock >> 11, (clock >> 5) & 63, (clock & 31) * 2]
            assert dos == entry['dos']
            if not case['password']: assert sha(archive.read(info)) == entry['sha256']
    for target in case['restored']:
        exact(Path(target), expected); native_outputs += 1
    target = BASE / ('external-' + str(index)); target.mkdir(exist_ok=True)
    args = [str(ROOT / 'src-tauri/resources/archive-engine/7z.exe'), 'x', '-y', '-bd', '-o' + str(target), str(output)]
    if case['password']: args.insert(2, '-p' + case['password'])
    result = subprocess.run(args, capture_output=True)
    assert result.returncode == 0, result.stdout.decode(errors='replace')
    exact(target, expected)
    checks.append({'name': case['name'], 'entries': len(expected), 'localAndCentralTimesMatch': True,
                   'independentContentAndDates': True, 'sourceUnchanged': True, 'encrypted': bool(case['password'])})
assert next(c for c in cases if c['name'] == 'parallel-flat')['workers'] > 1
receipt = {'version': '1.7.35', 'passed': True, 'recoveredArchives': len(cases),
           'encodedEntries': sum(c['entries'] for c in checks), 'nativeReadbackOutputs': native_outputs,
           'external7zipReadbacks': len(cases), 'checks': checks}
(BASE / 'verified.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: v for k, v in receipt.items() if k != 'checks'}))

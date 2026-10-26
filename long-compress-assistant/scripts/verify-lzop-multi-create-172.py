"""Official Windows CRC checks and Linux per-member name/content restoration in WSL."""
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile

root = Path(__file__).resolve().parents[1]
base = root / 'test-results/lzop-multi-create-172'
sha = lambda data: hashlib.sha256(data).hexdigest()
win = root / 'test-results/lzop-read-convert-1657/official-lzop/lzop103w/lzop.exe'
package = root / 'test-results/lzop-read-convert-1657/lzop103w.zip'
assert sha(package.read_bytes()) == '0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a'
with zipfile.ZipFile(package) as z:
    assert z.read('lzop103w/lzop.exe') == win.read_bytes()
linux = base / 'research/lzop-linux'
assert sha((base / 'research/lzop-1.03-i386_linux.tar.gz').read_bytes()) == 'd4cd71272cd18ef2061ffe2a5a75dd11182ed9015c53a2283eeb41acb9a21370'

def unix(path):
    p = str(path.resolve()).replace('\\', '/')
    return '/mnt/' + p[0].lower() + p[2:]

manifest = base / 'generated/manifest.json'
cases = json.loads(manifest.read_text(encoding='utf-8'))
# A GUI run can append its freshly created archive to a separate manifest.
desktop_manifest = base / 'desktop-outputs.json'
if desktop_manifest.exists():
    cases += json.loads(desktop_manifest.read_text(encoding='utf-8'))
receipt = {'version': '1.7.2', 'windowsVerifierSha256': sha(win.read_bytes()), 'linuxVerifierSha256': sha(linux.read_bytes()), 'linuxDownload': 'https://www.lzop.org/download/lzop-1.03-i386_linux.tar.gz', 'checks': []}
previous_file = base / 'official.json'
previous = json.loads(previous_file.read_text(encoding='utf-8')) if previous_file.exists() else {}
reused = 0
for index, case in enumerate(cases):
    archive = Path(case['archive'])
    assert sha(archive.read_bytes()) == case['archiveSha256']
    old = next((item for item in previous.get('checks', []) if item['archive'] == case['archive'] and item['archiveSha256'] == case['archiveSha256'] and item['entries'] == case['entries'] and item.get('exactNamesAndHashes')), None)
    if old and previous.get('windowsVerifierSha256') == receipt['windowsVerifierSha256'] and previous.get('linuxVerifierSha256') == receipt['linuxVerifierSha256']:
        receipt['checks'].append(old)
        reused += 1
        continue
    tested = subprocess.run([str(win), '-t', '-N', '-P', str(archive)], capture_output=True)
    assert tested.returncode == 0, repr(tested.stderr)
    out = base / f'official-restored-{index}'
    out.mkdir(exist_ok=True)
    # -k keeps the archive; names are restored as distinct files, never stdout concatenation.
    result = subprocess.run(['wsl', '-d', 'Ubuntu', '--exec', str(unix(linux)), '-d', '-N', '-P', '-k', '-p' + unix(out), unix(archive)], capture_output=True)
    assert result.returncode == 0, repr(result.stderr)
    actual = sorted(p.name for p in out.iterdir())
    assert actual == sorted(e['path'] for e in case['entries']), (actual, case)
    for entry in case['entries']:
        data = (out / entry['path']).read_bytes()
        assert len(data) == entry['bytes'] and sha(data) == entry['sha256'], entry
    receipt['checks'].append({**case, 'windowsTestExit': tested.returncode, 'linuxRestoreExit': result.returncode, 'exactNamesAndHashes': True})
receipt['passed'] = True
(base / 'official.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'Official lzop verified {len(cases)-reused} additional fresh outputs; reused {reused} unchanged output receipts. All {len(cases)} outputs have exact restored names and hashes.')

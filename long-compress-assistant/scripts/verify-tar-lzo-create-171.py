"""Official LZOP outer readback plus independent Windows bsdtar extraction."""
import hashlib, json, subprocess, tempfile, zipfile
from pathlib import Path

root = Path(__file__).resolve().parents[1]
base = root / 'test-results/tar-lzo-create-171'
official_zip = root / 'test-results/lzop-read-convert-1657/lzop103w.zip'
official = root / 'test-results/lzop-read-convert-1657/official-lzop/lzop103w/lzop.exe'
sha = lambda data: hashlib.sha256(data).hexdigest()
assert sha(official_zip.read_bytes()) == '0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a'
with zipfile.ZipFile(official_zip) as package:
    assert package.read('lzop103w/lzop.exe') == official.read_bytes()
cases = json.loads((base / 'generated/manifest.json').read_text(encoding='utf-8'))
receipt = {'version': '1.7.1', 'lzopSha256': sha(official.read_bytes()), 'tar': subprocess.run(['tar.exe', '--version'], capture_output=True, text=True, check=True).stdout.strip(), 'checks': []}
for case in cases:
    archive = Path(case['archive'])
    assert sha(archive.read_bytes()) == case['archiveSha256']
    subprocess.run([str(official), '-t', '-P', str(archive)], capture_output=True, check=True)
    tar_bytes = subprocess.run([str(official), '-d', '-c', str(archive)], capture_output=True, check=True).stdout
    tar_path = archive.with_suffix('')
    tar_path.write_bytes(tar_bytes)
    restored = Path(tempfile.mkdtemp(prefix='independent-', dir=base))
    assert restored.resolve().is_relative_to(base.resolve())
    subprocess.run(['tar.exe', '-xf', str(tar_path), '-C', str(restored)], capture_output=True, check=True)
    actual_files = {str(path.relative_to(restored)).replace('\\', '/'): sha(path.read_bytes()) for path in restored.rglob('*') if path.is_file()}
    expected_files = {entry['path']: entry['sha256'] for entry in case['entries'] if not entry['directory']}
    assert actual_files == expected_files, (actual_files, expected_files)
    for entry in case['entries']:
        if entry['directory']:
            assert (restored / entry['path']).is_dir(), entry
    receipt['checks'].append({'archive': archive.name, 'archiveSha256': case['archiveSha256'], 'files': len(actual_files), 'directories': sum(entry['directory'] for entry in case['entries']), 'allHashesMatch': True})
receipt['passed'] = True
(base / 'official.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print('Fresh TAR.LZO outputs pass official lzop and independent bsdtar extraction; all files, hashes and empty directories match.')

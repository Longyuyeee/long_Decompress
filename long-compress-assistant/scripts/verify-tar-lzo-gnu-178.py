"""Compare newly recovered GNU paths and bytes to the independent full original."""
from pathlib import Path
import hashlib, json, os

root = Path(__file__).resolve().parents[1]
base = root / 'test-results/tar-lzo-gnu-178'
oracle = json.loads((base / 'official.json').read_text(encoding='utf-8'))
assert oracle['passed'] and oracle['gnuTarListingAndContents'] and oracle['officialLzopDecoded'] and oracle['pythonTarfileAgrees']
sha = lambda data: hashlib.sha256(data).hexdigest()
assert sha((root / 'tests/fixtures/archives/tar-lzo-gnu-178/complete.tar.lzo').read_bytes()) == oracle['original']['sha256']
original = {entry['name']: entry for entry in oracle['original']['entries']}
checks = []
for result in json.loads((base / 'native-outputs.json').read_text(encoding='utf-8')):
    directory = Path(result['outputDirectory'])
    if os.name == 'nt':
        directory = Path('\\\\?\\' + str(directory.resolve()).removeprefix('\\\\?\\'))
    selected = result['entries']
    expected_files, expected_dirs = {}, set()
    for entry in selected:
        name = entry['name']
        assert entry['sha256'] == original[name]['sha256']
        if entry['isDirectory']:
            expected_dirs.add(name)
        else:
            expected_files[name] = original[name]['sha256']
        expected_dirs.update(parent.as_posix() for parent in Path(name).parents if parent != Path('.'))
    actual_files = {path.relative_to(directory).as_posix(): sha(path.read_bytes()) for path in directory.rglob('*') if path.is_file()}
    actual_dirs = {path.relative_to(directory).as_posix() for path in directory.rglob('*') if path.is_dir()}
    assert actual_files == expected_files
    assert actual_dirs == expected_dirs
    checks.append({'source': result['source'], 'entries': [entry['name'] for entry in selected], 'exactNamesDirectoriesAndHashes': True})
oracle['recoveryChecks'] = checks
(base / 'verified.json').write_text(json.dumps(oracle, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(f'{len(checks)} actual recovery results match the independent GNU original, including long paths, directories and SHA-256.')

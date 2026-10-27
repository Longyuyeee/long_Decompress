"""Check actual full/recovered trees against the independent mixed-record oracle."""
from pathlib import Path
import hashlib,json,os,sys
root=Path(__file__).resolve().parents[1];base=root/'test-results/tar-mixed-1710'
manifest=json.loads((root/'tests/fixtures/archives/tar-mixed-1710/manifest.json').read_text(encoding='utf-8'))
sha=lambda b:hashlib.sha256(b).hexdigest()
originals={case['name']:case for case in manifest['cases']}
for case in originals.values():
    assert case['gnuAndPythonListingContentAgree']
    assert sha((root/'tests/fixtures/archives/tar-mixed-1710'/case['archive']).read_bytes())==case['sha256']
    assert sha((root/'tests/fixtures/archives/tar-mixed-1710'/case['tar']).read_bytes())==case['tarSha256']
full_only='--full-only'in sys.argv
checks=[c for c in json.loads((base/'verified.json').read_text(encoding='utf-8'))['checks']if c['kind']!='full']if full_only else[]
for kind in(('full',)if full_only else('full','recovery','padded')):
    results=json.loads((base/f'{kind}-outputs.json').read_text(encoding='utf-8'))if kind!='padded'else[json.loads((base/'padded-output.json').read_text(encoding='utf-8'))]
    for result in results:
        oracle={e['name']:e for e in originals[result['case']]['entries']}
        output=Path(result['outputDirectory'])
        if os.name=='nt':output=Path('\\\\?\\'+str(output.resolve()).removeprefix('\\\\?\\'))
        expected_files={};expected_dirs=set()
        for entry in result['entries']:
            expected=oracle[entry['name']];assert expected==entry
            if entry['isDirectory']:expected_dirs.add(entry['name'])
            else:expected_files[entry['name']]=entry['sha256']
            expected_dirs.update(p.as_posix()for p in Path(entry['name']).parents if p!=Path('.'))
        actual_files={p.relative_to(output).as_posix():sha(p.read_bytes())for p in output.rglob('*')if p.is_file()}
        actual_dirs={p.relative_to(output).as_posix()for p in output.rglob('*')if p.is_dir()}
        assert actual_files==expected_files and actual_dirs==expected_dirs,result
        checks.append({'kind':kind,'case':result['case'],'source':result['source'],'files':len(actual_files),'directories':len(actual_dirs),'exactPathsAndHashes':True})
(base/'verified.json').write_text(json.dumps({'passed':True,'independentMixedSpecimens':len(originals),'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'{len(checks)} actual full/recovered trees match independent paths, directories and SHA-256.'+(' Prior recovery checks reused; only changed full-read outputs checked.'if full_only else''))

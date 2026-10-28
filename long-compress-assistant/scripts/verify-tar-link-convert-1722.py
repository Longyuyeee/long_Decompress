"""Check independent 7-Zip readback against the original TAR, not the intermediate tree."""
from pathlib import Path
import hashlib,json,os,subprocess,tarfile
root=Path(__file__).resolve().parents[1]
base=root/'test-results/tar-link-convert-1722'
checks=[]
def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as stream:
        while data:=stream.read(65536):h.update(data)
    return h.hexdigest()
for case in json.loads((base/'native.json').read_text(encoding='utf-8')):
    source=Path(case['originalSource']).resolve()
    manifest=json.loads((source.parent/'manifest.json').read_text(encoding='utf-8'))
    variant=next((v for v in manifest.get('variants',[]) if v['archive']==source.name),manifest)
    assert digest(source)==variant['sha256']==case['sourceSha256']
    tar_path=source.with_suffix('');assert digest(tar_path)==variant['tarSha256']
    output=case['output'];paths=[Path(e['path']).resolve() for e in output['files']]
    assert output['primaryPath'] in [e['path'] for e in output['files']]
    assert all(p.is_file() and p.stat().st_size==e['bytes']>0 for p,e in zip(paths,output['files']))
    assert sum(p.stat().st_size for p in paths)==output['totalBytes']
    assert all('.long-convert' not in p.name for p in paths)
    assert not list(paths[0].parent.glob('.long-*')) and case['privateFilesCleaned']
    if case['inputParts']:
        h=hashlib.sha256()
        for part in case['inputParts']:h.update(Path(part).read_bytes())
        assert h.hexdigest()==case['sourceSha256']
    restored=Path(case['restored']).resolve();expected={};dirs=set();times=[]
    with tarfile.open(tar_path) as archive:
        members=archive.getmembers();by_name={m.name:m for m in members}
        for i,entry in enumerate(variant['entries']):
            name=entry['name'];own=members[i]
            assert own.name==name or (own.sparse and own.pax_headers.get('GNU.sparse.name')==name)
            dirs.update(p.as_posix() for p in Path(name).parents if p!=Path('.'))
            path=restored/name
            if entry['isDirectory']:dirs.add(name)
            else:
                target=members[entry['targetId']] if entry.get('targetId') is not None else by_name.get(entry.get('linkTarget')) if entry.get('linkTarget') else own
                assert target is not None and target.isfile()
                h=hashlib.sha256()
                with archive.extractfile(target) as data:
                    while chunk:=data.read(65536):h.update(chunk)
                assert h.hexdigest()==entry['sha256']==digest(path),name
                assert path.stat().st_size==entry['bytes'] and path.stat().st_nlink==1
                expected[name]=entry['sha256']
            # Existing creators omit nonempty top-level directory headers; check the
            # timestamps they actually represent, including files and empty directories.
            implicit_root=entry['isDirectory'] and '/' not in name and any(e['name'].startswith(name+'/') for e in variant['entries'])
            if case['format'] not in ('zip','zipx') and not implicit_root:
                assert path.stat().st_mtime_ns==int(own.mtime*1_000_000_000),name
                times.append(name)
    assert {p.relative_to(restored).as_posix():digest(p) for p in restored.rglob('*') if p.is_file()}==expected
    assert {p.relative_to(restored).as_posix() for p in restored.rglob('*') if p.is_dir()}==dirs
    wrong_password_rejected=None
    if case['encrypted']:
        primary=Path(output['primaryPath'])
        if case['format']=='zip' and case['split'] and not case['standard']:primary=primary.parent/'oracle.zip'
        result=subprocess.run([str(root/'src-tauri/resources/archive-engine/7z.exe'),'t','-pIncorrect1722!',str(primary)],capture_output=True,creationflags=subprocess.CREATE_NO_WINDOW if os.name=='nt' else 0)
        assert result.returncode!=0
        wrong_password_rejected=True
    checks.append({'format':case['format'],'encrypted':case['encrypted'],'split':case['split'],'standard':case['standard'],'files':len(expected),'directories':len(dirs),'publishedParts':len(paths),'publishedBytes':output['totalBytes'],'wholeTarContentsMatched':True,'allOutputsIndependent':True,'nativeInputTimesRetained':True,'targetTimePolicy':('existing ZIP dates' if case['format'] in ('zip','zipx') else 'file/encoded directory seconds; nonempty top-level roots implicit'),'targetOwnTimesChecked':len(times),'wrongPasswordRejected':wrong_password_rejected,'privateFilesCleaned':True})
(base/'verified.json').write_text(json.dumps({'passed':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'{len(checks)} converted archives / {sum(c["files"] for c in checks)} actual readback files independently verified; encrypted cases reject an incorrect password.')

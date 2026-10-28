from pathlib import Path
import hashlib,json,subprocess,tarfile,zipfile
root=Path(__file__).resolve().parents[1];base=root/'test-results/tar-nested-links-1723';fixtures=root/'tests/fixtures/archives/tar-nested-links-1723'
manifest=json.loads((fixtures/'manifest.json').read_text(encoding='utf-8'));records=json.loads((base/'native.json').read_text(encoding='utf-8'));sha=lambda b:hashlib.sha256(b).hexdigest();checks=[]
for v in manifest['variants']:
 assert sha((fixtures/v['archive']).read_bytes())==v['sha256'];assert sha((fixtures/v['tar']).read_bytes())==v['tarSha256']
for i,r in enumerate(records):
 actual=Path(r['retained']);data=actual.read_bytes();assert actual.stat().st_nlink==1;assert sha(data)==r['receipt']['contentSha256']
 if not r.get('numericOuter'):
  v=next(v for v in manifest['variants'] if Path(r['source']).name==v['archive'])
  with tarfile.open(fixtures/v['tar'])as t:expected=t.extractfile(r['alias']).read()
  assert data==expected;assert r['receipt']['entryPath']==r['alias'];assert r['outer']['entries'][next(j for j,e in enumerate(r['outer']['entries'])if e['path']==r['alias'])]['linkTarget']
 else:assert sha(data)==manifest['childSha256']
 out=base/f'oracle-{i}';out.mkdir(exist_ok=True);encrypted=r.get('encrypted',False)
 args=[str(root/'src-tauri/resources/archive-engine/7z.exe'),'x','-y',str(actual),'-o'+str(out),'-p'+('Nested1723!'if encrypted else 'unused')]
 result=subprocess.run(args,capture_output=True);assert result.returncode==0,result.stdout
 if encrypted:
  assert (out/'secret.txt').read_text(encoding='utf-8')=='Inner password belongs only to this archive.\n'
  bad=subprocess.run([str(root/'src-tauri/resources/archive-engine/7z.exe'),'t',str(actual),'-pwrong'],capture_output=True);assert bad.returncode!=0
 else:
  assert (out/'readme.txt').read_bytes()=='包内链接压缩包内容，保持原双栏浏览。\n'.encode()
  with zipfile.ZipFile(out/'docs/next.zip')as z:assert z.read('third.txt')==b'Nested third layer 1723\n'
 checks.append({'retained':str(actual),'sha256':sha(data),'independentTarRead':not r.get('numericOuter',False),'external7zipReadback':True,'encrypted':encrypted})
(base/'verified.json').write_text(json.dumps({'passed':True,'archives':len(checks),'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(f'{len(checks)} materialized archives independently matched to producer TAR bytes and external 7-Zip readback.')

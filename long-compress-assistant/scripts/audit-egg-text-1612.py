"""Check only the new preview operation against frozen independent original payloads."""
import json,subprocess,hashlib
from pathlib import Path
root=Path(__file__).resolve().parents[1]
helper=root/'src-tauri/resources/egg-engine/long-egg.exe';fixtures=root/'tests/fixtures/archives';file=root/'test-results/egg-text-1612/helper-originals.json'
identity=hashlib.sha256(helper.read_bytes()).hexdigest()
report=json.loads(file.read_text(encoding='utf-8')) if file.exists() else {'version':'1.6.12','helperSha256':identity,'checks':[]}
assert report['helperSha256']==identity
cases=[('egg-154','../egg-153/upstream/azo.egg','azo_fix.sys',None,None),('egg-153','upstream/store.egg','hello.txt',None,None),('egg-153','upstream/deflate5.egg','text.txt',None,None),('egg-153','upstream/lzma5.egg','text.txt',None,None),('egg-153','upstream/enc_zip20_bzip2.egg','text.txt','test1234',None)]
for cipher in ['zipcrypto','aes256','lea256']:
 name='ordinary-'+cipher+'-m3-volumes';cases.append(('egg-1511',f'originals/{name}/{name}.vol3.egg','文档/99最后.txt','中文密码1511!','cp936'))
for family,archive,entry,key,encoding in cases:
 source=fixtures/family/archive;relative=source.relative_to(root).as_posix()
 if any(c['archive']==relative and c['entry']==entry for c in report['checks']):continue
 listing=json.loads(subprocess.check_output([str(helper),'list',str(source)],input=b'{}'))['result'];selected=next(e for e in listing['entries'] if e['path']==entry)
 r=subprocess.run([str(helper),'preview-text',str(source)],input=json.dumps({'selected':[entry],'password':key,'encoding':encoding}).encode(),capture_output=True);reply=json.loads(r.stdout);assert r.returncode==0,(archive,reply)
 data=bytes(reply['result']['bytes']);sha=hashlib.sha256(data).hexdigest();m=json.loads((fixtures/family/'manifest.json').read_text(encoding='utf-8'))
 expected=next(e for e in m['expected'] if e['path']==entry)['sha256'] if family=='egg-1511' else next(e for c in m['cases'] if c['archive']==archive.removeprefix('upstream/') for e in c['expected'] if e['path']==entry)['sha256']
 assert sha==expected,(archive,sha,expected)
 report['checks'].append({'archive':relative,'entry':entry,'methods':selected['methods'],'encryption':selected['encryption'],'bytes':len(data),'sha256':sha,'matchesFrozenIndependentPayload':True,'inputBytes':reply['result']['inputBytes']});file.parent.mkdir(parents=True,exist_ok=True);file.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('New preview operation matched',len(report['checks']),'frozen independent original payloads')

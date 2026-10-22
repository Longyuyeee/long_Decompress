"""Only the new prefix path; checkpoint receipts avoid repeating completed cases."""
import json,subprocess,hashlib
from pathlib import Path
root=Path(__file__).resolve().parents[1];exe=root/'src-tauri/resources/egg-engine/long-egg.exe';sample=root/'tests/fixtures/archives/egg-prefix-1616';base=root/'test-results/egg-prefix-1616';file=base/'helper.json';m=json.loads((sample/'manifest.json').read_text(encoding='utf-8'))
identity=hashlib.sha256(exe.read_bytes()).hexdigest()
report=json.loads(file.read_text(encoding='utf-8')) if file.exists() else {'version':'1.6.16','helperSha256':identity,'controlled':True,'checks':[]}
assert report['helperSha256']==identity
def save():file.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def run(source,password=None,encoding='cp936'):
 r=subprocess.run([str(exe),'preview-text',str(source)],input=json.dumps({'selected':['logs/session.txt'],'password':password,'encoding':encoding}).encode(),capture_output=True,timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
 return r,json.loads(r.stdout)
for name in ['store','deflate','bzip2','lzma','zipcrypto','aes256','password-volumes.vol2','bad-crc-later','multi-block','large-store']:
 if any(c['name']==name for c in report['checks']):continue
 source=(base if name=='large-store' else sample)/(name+'.egg');key=m['password'] if name in ['zipcrypto','aes256','password-volumes.vol2'] else None
 r,v=run(source,key);assert r.returncode==0,(name,v)
 data=bytes(v['result']['bytes']);expected=hashlib.sha256((b'L' if name=='large-store' else b'A')*1048576).hexdigest() if name in ['large-store','multi-block'] else m['prefixSha256']
 assert len(data)==1048576 and v['result']['partial'] is True and v['result']['totalSize']>1048576 and hashlib.sha256(data).hexdigest()==expected,(name,v.keys())
 report['checks'].append({'name':name,'bytes':len(data),'totalSize':v['result']['totalSize'],'sha256':hashlib.sha256(data).hexdigest(),'inputBytes':v['result']['inputBytes'],'partial':True});save()
for name,source,key,encoding,reason in [
 ('bad-first-block',sample/'bad-first-block.egg',None,'cp936','CRC'),
 ('short-before-prefix',sample/'short-before-prefix.egg',None,'cp936','大小或 CRC'),
 ('unknown-method',sample/'unknown-method.egg',None,'cp936','目前支持'),
 ('actual-input-limit',base/'input-limit.egg',None,'cp936','64 MiB'),
 ('wrong-password',sample/'aes256.egg','wrong','cp936','密码'),
 ('wrong-codepage',sample/'aes256.egg',m['password'],'utf8','密码')]:
 if any(c['name']==name for c in report['checks']):continue
 r,v=run(source,key,encoding);assert r.returncode!=0 and reason in v.get('error',''),(name,v)
 report['checks'].append({'name':name,'rejected':True,'reason':v['error']});save()
# Authentication and payload reads share one counter, rather than resetting on seek.
aes=next(c for c in report['checks'] if c['name']=='aes256')
listing=json.loads(subprocess.check_output([str(exe),'list',str(sample/'aes256.egg')],input=b'{}',creationflags=subprocess.CREATE_NO_WINDOW))['result']
packed=next(e['compressedSize'] for e in listing['entries'] if e['path']=='logs/session.txt')
assert aes['inputBytes']>packed
report['authenticatedReReadCounted']={'packed':packed,'actual':aes['inputBytes']}
report['passed']=True;save();print('Prefix helper:',len(report['checks']),'checks; authentication re-read counted.')

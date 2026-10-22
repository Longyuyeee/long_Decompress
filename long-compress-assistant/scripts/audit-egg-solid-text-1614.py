"""Only the new solid-text operation; frozen original payload hashes, scoped negatives."""
import hashlib,json,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1];fixtures=root/'tests/fixtures/archives';helper=root/'src-tauri/resources/egg-engine/long-egg.exe';base=root/'test-results/egg-solid-text-1614';file=base/'helper.json'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
report=json.loads(file.read_text(encoding='utf-8')) if file.exists() else {'version':'1.6.14','helperSha256':sha(helper),'checks':[]}
assert report['helperSha256']==sha(helper)
def run(source,entry):
 r=subprocess.run([str(helper),'preview-text',str(source)],input=json.dumps({'selected':[entry]}).encode(),capture_output=True,timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
 return r,json.loads(r.stdout)
def save():file.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def positive(name,source,entry,expected,original):
 if any(c['name']==name for c in report['checks']):return
 r,v=run(source,entry);assert r.returncode==0,(name,v)
 b=bytes(v['result']['bytes']);actual=hashlib.sha256(b).hexdigest();assert actual==expected,(name,actual,expected)
 report['checks'].append({'name':name,'originalArchive':original,'entry':entry,'archiveSha256':sha(source),'bytes':len(b),'sha256':actual,'matchesFrozenIndependentPayload':True,'inputBytes':v['result']['inputBytes']});save()
hello='b47cc0f104b62d4c7c30bcd68fd8e67613e287dc4ad8c310ef10cbadea9c4380'
for name,entry,expected in [('solid','hello.txt',hello),('solid-empty','empty.txt',hashlib.sha256(b'').hexdigest())]:
 positive(name,fixtures/'egg-153/upstream/solid.egg',entry,expected,True)
for method in ['deflate','high']:
 positive('original-'+method,fixtures/f'egg-154/original-alzip/solid-{method}.egg','文档/说明 文件.txt','fa0a640e110667fc47056120b2dd20a7faa377f49afb19af557ad1463f912021',True)
positive('controlled-native-middle',fixtures/'egg-solid-text-1614/controlled-native.vol2.egg','hello.txt',hello,False)
for name,reason in [('shared-crc','CRC'),('shared-output-limit','64 MiB'),('selected-limit','1 MiB'),('unknown-dependency','Deflate / LZMA'),('encrypted-dependency','加密固实'),('truncated','校验失败'),('trailing','校验失败'),('input-limit','64 MiB')]:
 if any(c['name']==name for c in report['checks']):continue
 source=(base if name=='input-limit' else fixtures/'egg-solid-text-1614')/(name+'.egg')
 r,v=run(source,'readme.txt');assert r.returncode!=0 and reason in v.get('error',''),(name,v)
 report['checks'].append({'name':name,'controlled':True,'rejected':True,'reason':v['error']});save()
report['passed']=True;save();print('Solid text:',len(report['checks']),'scoped helper checks.')

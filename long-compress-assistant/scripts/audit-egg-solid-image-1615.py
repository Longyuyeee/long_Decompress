"""New solid image operation against owned pixels; no old algorithm matrix."""
import json,subprocess,hashlib
from pathlib import Path
root=Path(__file__).resolve().parents[1];sample=root/'tests/fixtures/archives/egg-solid-image-1615';helper=root/'src-tauri/resources/egg-engine/long-egg.exe';base=root/'test-results/egg-solid-image-1615';file=base/'helper.json'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
report=json.loads(file.read_text(encoding='utf-8')) if file.exists() else {'version':'1.6.15','helperSha256':sha(helper),'controlledImageSources':True,'newProducerOriginal':False,'checks':[]}
assert report['helperSha256']==sha(helper)
m=json.loads((sample/'manifest.json').read_text(encoding='utf-8'))
def save():file.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def run(source,entry,op='preview-image'):
 r=subprocess.run([str(helper),op,str(source)],input=json.dumps({'selected':[entry]}).encode(),capture_output=True,timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
 return r,json.loads(r.stdout)
def positive(name,source,entry,expected):
 if any(c['name']==name for c in report['checks']):return
 r,v=run(source,entry);assert r.returncode==0,(name,v)
 data=bytes(v['result']['bytes']);actual=hashlib.sha256(data).hexdigest();assert actual==expected,(name,actual,expected)
 report['checks'].append({'name':name,'archiveSha256':sha(source),'entry':entry,'bytes':len(data),'sha256':actual,'inputBytes':v['result']['inputBytes'],'matchesOwnedPixelPayload':True});save()
for method in ['deflate','lzma']:
 for e in m['images']:positive(method+'-'+e['path'],sample/(method+'.egg'),e['path'],e['sha256'])
positive('controlled-middle-member',sample/'controlled-native.vol2.egg',m['images'][1]['path'],m['images'][1]['sha256'])
positive('exact-8MiB',sample/'exact-limit.egg','preview.bmp',m['exactLimitSha256'])
for name,entry,reason,op in [
 ('shared-crc','gallery/01-cover.png','CRC','preview-image'),
 ('shared-output-limit','gallery/01-cover.png','64 MiB','preview-image'),
 ('encrypted-dependency','gallery/01-cover.png','加密固实图片','preview-image'),
 ('unknown-dependency','gallery/01-cover.png','Deflate / LZMA','preview-image'),
 ('truncated','gallery/01-cover.png','校验失败','preview-image'),
 ('trailing','gallery/01-cover.png','校验失败','preview-image'),
 ('oversize','preview.bmp','8 MiB','preview-image'),
 ('input-limit','preview.png','64 MiB','preview-image'),
 ('ui','gallery/02-grid.bmp','1 MiB','preview-text')]:
 key=name+'-'+op
 if any(c['name']==key for c in report['checks']):continue
 source=(base if name=='input-limit' else sample)/(name+'.egg');r,v=run(source,entry,op);assert r.returncode!=0 and reason in v.get('error',''),(key,v)
 report['checks'].append({'name':key,'rejected':True,'reason':v['error']});save()
report['passed']=True;save();print('New shared-image checks:',len(report['checks']))

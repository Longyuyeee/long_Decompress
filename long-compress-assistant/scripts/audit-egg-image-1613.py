"""New image operation checks against fixed owned pixel hashes; resume completed cases."""
import hashlib,json,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1];fixtures=root/'tests/fixtures/archives/egg-image-1613';base=root/'test-results/egg-image-1613';base.mkdir(parents=True,exist_ok=True);file=base/'helper.json';helper=root/'src-tauri/resources/egg-engine/long-egg.exe';identity=hashlib.sha256(helper.read_bytes()).hexdigest()
report=json.loads(file.read_text(encoding='utf-8')) if file.exists() else {'version':'1.6.13','helperSha256':identity,'controlled':True,'historicalProducer':False,'checks':[]}
assert report['helperSha256']==identity
m=json.loads((fixtures/'manifest.json').read_text(encoding='utf-8'));cases=[]
for method in ['store','deflate','bzip2','lzma']:
 for i in [0,1]:cases.append((method+'.egg',m['images'][i]['path'],None,i))
for source in ['zipcrypto.egg','aes256.egg','password-volumes.vol2.egg']:cases.append((source,'相册/01-封面.png',m['password'],0))
for name,entry,key,index in cases:
 label=name+':'+entry
 if any(c['name']==label for c in report['checks']):continue
 proc=subprocess.run([str(helper),'preview-image',str(fixtures/name)],input=json.dumps({'selected':[entry],'password':key,'encoding':'cp936'}).encode(),capture_output=True);reply=json.loads(proc.stdout);assert proc.returncode==0,(label,reply)
 data=bytes(reply['result']['bytes']);sha=hashlib.sha256(data).hexdigest();assert sha==m['images'][index]['sha256'];assert reply['result']['totalSize']==len(data);assert reply['result']['inputBytes']<=64*1024*1024
 report['checks'].append({'name':label,'bytes':len(data),'sha256':sha,'inputBytes':reply['result']['inputBytes'],'matchesOwnedPixels':True});file.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Complete controlled image byte checks:',len(report['checks']))

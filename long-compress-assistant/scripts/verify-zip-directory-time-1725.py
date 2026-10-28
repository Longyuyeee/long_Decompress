"""Independent ZIP local/central dates, source times and external 7-Zip readback."""
from pathlib import Path
import hashlib,json,os,struct,subprocess,tarfile,time
root=Path(__file__).resolve().parents[1];base=root/'test-results/zip-directory-time-1725';sha=lambda b:hashlib.sha256(b).hexdigest()
cases=json.loads((base/'native.json').read_text(encoding='utf-8'))+json.loads((base/'conversion.json').read_text(encoding='utf-8'))
def expected(seconds):
 t=time.localtime(seconds)
 if t.tm_year<1980:return(1980,1,1,0,0,0)
 if t.tm_year>2107:return(2107,12,31,23,59,58)
 return(*t[:5],t.tm_sec//2*2)
def dos(date,clock):return(1980+(date>>9),date>>5&15,date&31,clock>>11,clock>>5&63,(clock&31)*2)
checks=[]
for i,c in enumerate(cases):
 parts=[Path(p)for p in c['parts']];chunks=[p.read_bytes()for p in parts];raw=b''.join(chunks);starts=[];n=0
 for chunk in chunks:starts.append(n);n+=len(chunk)
 eocd=raw.rfind(b'PK\x05\x06');assert eocd>=0
 cdsize=struct.unpack_from('<I',raw,eocd+12)[0];central_end=eocd
 if raw[eocd-20:eocd-16]==b'PK\x06\x07':
  central_end=raw.rfind(b'PK\x06\x06',0,eocd);assert central_end>=0 and central_end+12+struct.unpack_from('<Q',raw,central_end+4)[0]==eocd-20;cdsize=struct.unpack_from('<Q',raw,central_end+40)[0]
 pos=central_end-cdsize;entries=[]
 sources={e['name'].rstrip('/'):e for e in c['sources']}
 while pos<central_end:
  assert raw[pos:pos+4]==b'PK\x01\x02';clock,date=struct.unpack_from('<HH',raw,pos+12);nl,xl,cl,disk=struct.unpack_from('<HHHH',raw,pos+28);offset=struct.unpack_from('<I',raw,pos+42)[0];name=raw[pos+46:pos+46+nl].decode('utf-8');key=name.rstrip('/');s=sources[key];want=expected(s['seconds']);assert dos(date,clock)==want,(name,dos(date,clock),want)
  local=starts[disk]+offset if not c.get('numeric')else offset;assert raw[local:local+4]==b'PK\x03\x04';lt,ld=struct.unpack_from('<HH',raw,local+10);assert dos(ld,lt)==want
  if s.get('directory')is False and 'sourceTar'not in c:
   p=Path(s['path']);assert sha(p.read_bytes())==s['sha256'];assert p.stat().st_mtime_ns==s['seconds']*10**9+s.get('nanos',0)
  entries.append({'name':name,'date':want,'sourceSeconds':s['seconds']});pos+=46+nl+xl+cl
 assert len(entries)==len(sources)
 out=base/f'independent-{i}';out.mkdir(exist_ok=True);input=Path(c['input'])
 if c.get('numeric'):input=base/f'joined-{i}.zip';input.write_bytes(raw)
 elif len(parts)>1 and input.suffix.lower()=='.zipx':
  family=base/f'zipx-family-{i}';family.mkdir(exist_ok=True)
  for j,chunk in enumerate(chunks):(family/('same.zip'if j==len(chunks)-1 else f'same.z{j+1:02d}')).write_bytes(chunk)
  input=family/'same.zip'
 args=[str(root/'src-tauri/resources/archive-engine/7z.exe'),'x','-y','-sccUTF-8',str(input),'-o'+str(out),'-pZipDir1725!'];result=subprocess.run(args,capture_output=True);assert result.returncode==0,result.stdout
 for e in entries:
  key=e['name'].rstrip('/');s=sources[key];p=out/key;assert p.exists()
  assert int(p.stat().st_mtime)==int(time.mktime((*e['date'],0,0,-1))),(key,p.stat().st_mtime,e['date'])
  if not s['directory']:
   data=p.read_bytes();assert sha(data)==s['sha256']
   if 'sourceTar'in c:
    with tarfile.open(c['sourceTar'])as t:assert data==t.extractfile(key).read();assert int(t.getmember(key).mtime)//2*2==s['seconds']
 if c.get('encrypted'):
  wrong=subprocess.run([str(root/'src-tauri/resources/archive-engine/7z.exe'),'t',str(input),'-pwrong'],capture_output=True);assert wrong.returncode!=0
 checks.append({'input':str(input),'entries':entries,'localAndCentralDatesMatch':True,'external7zipReadback':True,'encrypted':c.get('encrypted',False)})
# Native final outputs must retain the same explicit tree and directory dates.
for c in cases:
 target=Path(c['restored']);expected_paths={s['name'].rstrip('/')for s in c['sources']}
 actual={str(p.relative_to(target)).replace('\\','/')for p in target.rglob('*')};assert actual==expected_paths,(actual,expected_paths)
 for s in c['sources']:
  p=target/s['name'];assert p.is_dir()==s['directory'];assert int(p.stat().st_mtime)==int(time.mktime((*expected(s['seconds']),0,0,-1)))
  if not s['directory']:assert sha(p.read_bytes())==s['sha256']
 if 'sourceTar'in c:
  tpath=Path(c['sourceTar']);manifest=json.loads((tpath.parent/'manifest.json').read_text(encoding='utf-8'));v=next(v for v in manifest['variants']if v['tar']==tpath.name);assert sha(tpath.read_bytes())==v['tarSha256']
selection=json.loads((base/'selection.json').read_text(encoding='utf-8'))
for c in selection:
 for name,t in c['times']:assert int((Path(c['restored'])/name).stat().st_mtime)==t['seconds']
 if c['mode']=='file':
  assert not c['times'];assert int((Path(c['restored'])/'资料').stat().st_mtime)!=int(time.mktime((2000,6,1,12,34,56,0,0,-1)))
result={'passed':True,'archives':len(checks),'encodedEntries':sum(len(c['entries'])for c in checks),'checks':checks,'nativeFinalTreesAndDates':True,'selection':selection,'timePolicy':'Explicit ZIP directory headers only; local two-second DOS calendar. No fabricated implicit parent dates.'}
(base/'verified.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(f"{result['archives']} archives / {result['encodedEntries']} entries: independent header dates, exact native output trees, directory dates and external content/time readback passed.")

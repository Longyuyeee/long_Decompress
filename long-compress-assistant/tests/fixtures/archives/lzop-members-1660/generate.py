"""Official lzop -o multi-file originals, not concatenated stand-ins. Test tool only."""
import hashlib,json,pathlib,subprocess,sys,random
root=pathlib.Path(__file__).resolve().parent;tool=pathlib.Path(sys.argv[1]).resolve()
work=root.parents[3]/'test-results/lzop-members-read-1660/fixture-work';work.mkdir(parents=True,exist_ok=True)
source=work/'source';source.mkdir(exist_ok=True)
payloads={'notes.txt':'收到多成员 LZOP 资料，先查看，再取回需要的文件。\n'.encode(),'large.txt':b'LZOP member prefix preview\n'*60000,'cover.png':(root.parent/'aes-codecs-1521/原文件/图标.png').read_bytes(),'data.bin':random.Random(1660).randbytes(300000),'empty.txt':b''}
for name,data in payloads.items():(source/name).write_bytes(data)
proof=[]
for method in [1,3,9]:
 name=f'members-{method}.lzo';target=root/name
 r=subprocess.run([str(tool),'-f',f'-{method}','--crc32','-CC','-o',str(target),*payloads],cwd=source,capture_output=True,check=True)
 checked=subprocess.run([str(tool),'-t','-N',str(target)],capture_output=True,check=True)
 out=work/f'restored-{method}';out.mkdir(exist_ok=True)
 restored=subprocess.run([str(tool),'-d','-N','-f','-p'+str(out),str(target)],capture_output=True,check=True)
 expected=[]
 for n,data in payloads.items():
  assert (out/n).read_bytes()==data
  expected.append({'path':n,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
 proof.append({'archive':name,'archiveSha256':hashlib.sha256(target.read_bytes()).hexdigest(),'options':[f'-{method}','--crc32','-CC','-o'],'officialTestExit':checked.returncode,'officialRestoreExit':restored.returncode,'expected':expected})
(root/'manifest.json').write_text(json.dumps({'producer':'official Windows lzop 1.03 / LZO 2.04','source':'https://www.lzop.org/download/lzop103w.zip','downloadSha256':'0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a','cases':proof,'derivedTests':'Concatenated stdin members, name collisions/invalid names, damaged CRC, truncation and byte slices are derived tests, not native multipart or external application originals.'},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('3 official multi-file originals: -t -N and independent -d -N -p readback passed, each file SHA256 matched')

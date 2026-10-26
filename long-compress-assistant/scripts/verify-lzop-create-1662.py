"""Independent lzop executable reads newly created application outputs, not fixture copies."""
import hashlib,json,subprocess,zipfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
base=root/'test-results/lzop-create-1662'
exe=root/'test-results/lzop-read-convert-1657/official-lzop/lzop103w/lzop.exe'
sha=lambda b:hashlib.sha256(b).hexdigest()
download=root/'test-results/lzop-read-convert-1657/lzop103w.zip'
assert sha(download.read_bytes())=='0e55b444069eb5eb8addda2379cf12450f8ee68248a125b44bac140f6cdf7d4a'
with zipfile.ZipFile(download) as package:
 assert package.read('lzop103w/lzop.exe')==exe.read_bytes()
cases=json.loads((base/'generated/manifest.json').read_text(encoding='utf-8'))
assert len(cases)==7
receipt={'version':'1.6.62','verifierDownloadSha256':sha(download.read_bytes()),'verifierSha256':sha(exe.read_bytes()),'verifier':'official lzop 1.03 Windows / LZO 2.04','checks':[]}
for case in cases:
 archive=Path(case['archive']);source=Path(case['source'])
 assert sha(source.read_bytes())==case['sourceSha256']
 assert sha(archive.read_bytes())==case['archiveSha256']
 test=subprocess.run([str(exe),'-t','-P',str(archive)],capture_output=True)
 assert test.returncode==0,(test.returncode,test.stderr)
 read=subprocess.run([str(exe),'-d','-c',str(archive)],capture_output=True)
 assert read.returncode==0,(read.returncode,read.stderr)
 assert len(read.stdout)==case['sourceBytes'] and sha(read.stdout)==case['sourceSha256']
 receipt['checks'].append({**case,'officialTestExit':test.returncode,'officialDecompressExit':read.returncode,'officialDecompressedSha256':sha(read.stdout)})
receipt['passed']=True
(base/'official.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Official lzop verified and fully decompressed 7 fresh LZOP outputs, including empty, UTF-8, long-name and stored/compressed-block cases; all content SHA256 values match.')

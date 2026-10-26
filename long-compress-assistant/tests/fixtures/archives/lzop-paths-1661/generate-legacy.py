"""Genuine official Windows CP936 -P single-member original, separate from derived tests."""
import ctypes,hashlib,json,pathlib,subprocess,sys
assert ctypes.windll.kernel32.GetACP()==936,'This original requires a real CP936 Windows process environment'
root=pathlib.Path(__file__).resolve().parent;tool=pathlib.Path(sys.argv[1]).resolve()
work=root.parents[3]/'test-results/lzop-paths-names-1661/legacy-official';source=work/'source';source.mkdir(parents=True,exist_ok=True)
name='docs/资料.txt';content='旧 Windows 名称按来源代码页读取，正文保持原样。\n'.encode('utf8');p=source/name;p.parent.mkdir(exist_ok=True);p.write_bytes(content)
archive=root/'official-cp936.lzo'
subprocess.run([str(tool),'-f','-1','-P','--crc32','-CC','-o',str(archive),name],cwd=source,check=True,capture_output=True)
checked=subprocess.run([str(tool),'-t','-P',str(archive)],check=True,capture_output=True)
out=work/'restored';out.mkdir(exist_ok=True)
restored=subprocess.run([str(tool),'-d','-P','-f','-p'+str(out),str(archive)],check=True,capture_output=True)
assert (out/name).read_bytes()==content
data=archive.read_bytes();flags=int.from_bytes(data[17:21],'big');raw=data[34:34+data[33]]
assert raw==name.encode('gbk') and flags&0x2000 and not flags&0x400 and flags&0x00f00000==0x00300000
proof={'producer':'official Windows lzop 1.03 / LZO 2.04 in real ACP936 environment','archive':'official-cp936.lzo','archiveSha256':hashlib.sha256(data).hexdigest(),'path':name,'contentSha256':hashlib.sha256(content).hexdigest(),'rawNameHex':raw.hex(),'flags':hex(flags),'pathFlag':True,'multipartFlag':False,'officialTestExit':checked.returncode,'officialRestoreExit':restored.returncode,'scope':'Genuine legacy Windows CP936 name and single path-flag member. Other codepages and collision cases remain explicitly derived tests.'}
(root/'legacy-manifest.json').write_text(json.dumps(proof,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print('genuine Windows CP936 name: official -P single path-flag member, independent restore and SHA256 matched')

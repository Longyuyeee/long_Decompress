//! EGG basic read-only decoder. Keys arrive on stdin, not process arguments.
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, fs::{self,File}, io::{self,Read,Seek,SeekFrom,Write},path::{Path,PathBuf}};
use unegg::{archive::{EggArchive,EggFileEntry,CompressionMethod,EncryptionMethod},crypto::{ZipCrypto,Decryptor},decompress,error::EggError};
#[path = "../../../src-tauri/src/services/egg_volumes.rs"] mod egg_volumes;
mod complex;
#[derive(Default,Deserialize)]
struct Request { password:Option<String>,encoding:Option<String>,output:Option<PathBuf>,#[serde(default)]selected:Vec<String>,#[serde(default)]preserve_timestamps:bool }
fn password_bytes(password:&str,encoding:Option<&str>)->Result<Vec<u8>> {
    let codec=match encoding.unwrap_or("utf8") {"utf8"=>encoding_rs::UTF_8,"cp936"=>encoding_rs::GBK,"cp950"=>encoding_rs::BIG5,"cp932"=>encoding_rs::SHIFT_JIS,"cp949"=>encoding_rs::EUC_KR,_=>anyhow::bail!("[archive-inspection:unsupported] EGG 密码代码页不受支持")};
    let (bytes,_,loss)=codec.encode(password);let (roundtrip,invalid)=codec.decode_without_bom_handling(&bytes);
    ensure!(!loss&&!invalid&&roundtrip==password,"[archive-inspection:password-encoding] [egg-password-encoding] 此密码不能按所选 EGG 代码页无损表示，未替换字符或据此判定密码错误");Ok(bytes.into_owned())
}
fn safe_name(name:&str)->Result<String> {
    let n=name.replace('\\',"/");let n=n.trim_end_matches('/');
    ensure!(!n.is_empty()&&!n.starts_with('/')&&!n.chars().any(|c|c.is_control()||":?*|<>\"".contains(c))&&n.split('/').all(|s|!s.is_empty()&&s!="."&&s!=".."),"[archive-inspection:unsupported] EGG 条目路径不能写入 Windows：{name}");
    Ok(n.to_string())
}
fn map_error(e:EggError,encrypted:bool)->anyhow::Error {
    match e {
        EggError::UnsupportedFeature(s)=>anyhow::anyhow!("[archive-inspection:unsupported] EGG 特性当前不受支持：{s}；不应反复尝试密码"),
        EggError::UnknownCompressionMethod(m)=>anyhow::anyhow!("[archive-inspection:unsupported] EGG 压缩方法 {m} 当前不受支持"),
        EggError::UnsupportedEncryption(m)=>anyhow::anyhow!("[archive-inspection:unsupported] EGG 加密方法 {m} 当前不受支持"),
        EggError::PasswordNotSet=>anyhow::anyhow!("[archive-inspection:password-required] EGG 所选内容需要密码"),
        EggError::InvalidPassword=>anyhow::anyhow!("[egg-password] EGG 密码短头不匹配，也可能是加密头损坏；请核对密码和原文件完整性"),
        EggError::Io(ref io)|EggError::CantOpenFile(ref io)|EggError::CantOpenDestFile(ref io) if !matches!(io.kind(),io::ErrorKind::UnexpectedEof|io::ErrorKind::InvalidData)=>anyhow::anyhow!("[archive-inspection:io] [egg-io] EGG 文件读写失败：{e}"),
        _=>anyhow::anyhow!("{} EGG 内容、大小或 CRC 校验失败：{e}",if encrypted {"[archive-inspection:ambiguous-data]"}else{"[archive-inspection:damaged]"}),
    }
}
fn open(path:&Path)->Result<EggArchive<egg_volumes::Reader>> {
    let group=egg_volumes::complete(path)?;let reader=egg_volumes::Reader::open(&group)?;let len=reader.len();
    let a=EggArchive::open(reader).map_err(|e|map_error(e,false))?;
    let mut names=HashSet::new();let mut total=0u64;
    for e in &a.entries {
        let name=safe_name(&e.file_name)?;
        ensure!(!e.is_directory()||e.uncompressed_size==0,"[archive-inspection:damaged] EGG 目录声明非零文件大小：{name}");
        ensure!(names.insert(name.to_lowercase()),"[archive-inspection:unsupported] EGG 有 Windows 同名条目：{name}");
        let mut sum=0u64;
        for b in &e.blocks {
            ensure!(b.data_pos.checked_add(b.compressed_size as u64).is_some_and(|p|p<=len-4),"[archive-inspection:damaged] EGG 条目数据范围超出原卷：{name}");
            sum+=b.uncompressed_size as u64;
        }
        ensure!(a.is_solid||sum==e.uncompressed_size,"[archive-inspection:damaged] EGG 条目大小与数据块之和不符：{name}");
        total=total.checked_add(e.uncompressed_size).context("EGG 大小溢出")?;
    }
    ensure!(total<=512*1024*1024*1024,"[archive-inspection:unsupported] EGG 声明解压大小超过 512 GiB");Ok(a)
}
fn entries(a:&EggArchive<egg_volumes::Reader>,selected:&[String])->Result<Vec<EggFileEntry>> {
    let requested=selected.iter().map(|s|safe_name(s)).collect::<Result<HashSet<_>>>()?;
    let chosen=a.entries.iter().filter(|e|requested.is_empty()||requested.contains(e.file_name.trim_end_matches('/'))).cloned().collect::<Vec<_>>();
    ensure!(requested.is_empty()||chosen.len()==requested.len(),"所选 EGG 条目已变化，请刷新目录后重新选择");Ok(chosen)
}
fn plan(a:&EggArchive<egg_volumes::Reader>,entries:&[EggFileEntry])->Result<()> {
    complex::validate(a,entries)
}
fn metadata(a:&EggArchive<egg_volumes::Reader>,entries:&[EggFileEntry])->Result<Value> {
    let mut list=vec![];let mut total=0;let mut compressed=0;let mut files=0;let mut encrypted=false;
    for e in entries {
        let path=safe_name(&e.file_name)?;let is_dir=e.is_directory();let packed=e.blocks.iter().map(|b|b.compressed_size as u64).sum::<u64>();
        if !is_dir {files+=1;total+=e.uncompressed_size;encrypted|=e.encrypt_info.is_some()};compressed+=packed;
        list.push(json!({"name":path.rsplit('/').next(),"path":path,"size":e.uncompressed_size,"sizeKnown":true,"compressedSize":if a.is_solid{None}else{Some(packed)},"modified":null,"crc":null,"encrypted":e.encrypt_info.is_some(),"isDir":is_dir,"methods":e.blocks.iter().map(|b|b.compression_method.name()).collect::<Vec<_>>(),"encryption":e.encrypt_info.as_ref().map(|i|i.method.name())}));
    }
    if a.is_solid {encrypted=a.entries.iter().any(|e|e.encrypt_info.is_some()&&!e.blocks.is_empty());compressed=a.entries.iter().flat_map(|e|&e.blocks).map(|b|b.compressed_size as u64).sum();}
    Ok(json!({"format":"EGG","entries":list,"totalFiles":files,"totalDirectories":entries.len()-files,"totalUncompressedSize":total,"totalCompressedSize":compressed,"encrypted":encrypted,"solid":a.is_solid}))
}
struct CheckedWriter<W>{inner:W,bytes:u64,limit:u64}
impl<W:Write>Write for CheckedWriter<W>{
    fn write(&mut self,b:&[u8])->io::Result<usize>{if b.len() as u64>self.limit-self.bytes{return Err(io::Error::new(io::ErrorKind::InvalidData,"EGG output exceeds declared block size"))}let n=self.inner.write(b)?;self.bytes+=n as u64;Ok(n)}
    fn flush(&mut self)->io::Result<()>{self.inner.flush()}
}
fn decode<W:Write>(a:&mut EggArchive<egg_volumes::Reader>,e:&EggFileEntry,pwd:Option<&str>,encoding:Option<&str>,mut out:W)->Result<()> {
    let mut crypto=complex::cipher(&mut a.reader,e,pwd,encoding)?;
    for b in &e.blocks {
        a.reader.seek(SeekFrom::Start(b.data_pos))?;let mut input=(&mut a.reader).take(b.compressed_size as u64);
        let mut checked=CheckedWriter{inner:&mut out,bytes:0,limit:b.uncompressed_size as u64};
        let cipher=crypto.as_mut().map(|c| &mut **c as &mut dyn Decryptor);
        let crc=complex::stream(&mut input,&mut checked,b.compression_method,b.compressed_size as u64,b.uncompressed_size as u64,cipher).map_err(|err|map_error(err,e.encrypt_info.is_some()))?;
        ensure!(checked.bytes==b.uncompressed_size as u64&&crc==b.crc32,"{} EGG 条目「{}」大小或 CRC 校验失败，未记为成功",if e.encrypt_info.is_some(){"[archive-inspection:ambiguous-data]"}else{"[archive-inspection:damaged]"},e.file_name);
    }
    out.flush().context("[archive-inspection:io] [egg-io] EGG 输出刷盘失败")?;Ok(())
}
fn run()->Result<Value>{
    let args=std::env::args_os().skip(1).collect::<Vec<_>>();ensure!(args.len()==2,"Usage: long-egg list|plan|verify|extract archive (JSON stdin)");
    let op=args[0].to_str().context("Invalid operation")?;let mut data=Vec::new();io::stdin().take(32*1024*1024+1).read_to_end(&mut data)?;
    ensure!(data.len()<=32*1024*1024,"EGG 请求过大");let req:Request=serde_json::from_slice(&data)?;data.fill(0);ensure!(req.selected.len()<=100000,"EGG 所选条目过多");
    let mut a=open(Path::new(&args[1]))?;let mut chosen=entries(&a,&req.selected)?;
    if op=="list" {return metadata(&a,&chosen)}plan(&a,&chosen)?;
    if op=="plan" {return metadata(&a,&chosen)}ensure!(matches!(op,"verify"|"extract"),"Unsupported operation");
    if a.is_solid {complex::solid(&mut a,&chosen,&req,op=="extract")?;return Ok(json!({"verified":true}));}
    chosen.sort_by_key(|e|e.encrypt_info.is_some());
    for e in chosen {
        if op=="verify" {decode(&mut a,&e,req.password.as_deref(),req.encoding.as_deref(),io::sink())?;continue;}
        let dest=req.output.as_ref().context("Missing output")?.join(safe_name(&e.file_name)?);
        if e.is_directory(){fs::create_dir_all(dest).context("[archive-inspection:io] [egg-io] 无法创建 EGG 目录")?;continue;}
        fs::create_dir_all(dest.parent().context("Invalid output")?).context("[archive-inspection:io] [egg-io] 无法创建 EGG 目录")?;
        let mut f=File::options().write(true).create_new(true).open(dest).context("[archive-inspection:io] [egg-io] 无法创建 EGG 输出文件")?;
        decode(&mut a,&e,req.password.as_deref(),req.encoding.as_deref(),&mut f)?;f.sync_all().context("[archive-inspection:io] [egg-io] EGG 文件刷盘失败")?;
        complex::set_time(&f,&e,req.preserve_timestamps)?;
    }
    Ok(json!({"verified":true}))
}
fn main(){let (value,code)=match run(){Ok(v)=>(json!({"result":v}),0),Err(e)=>(json!({"error":format!("{e:#}")}),2)};println!("{value}");std::process::exit(code);}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn truncated_and_trailing_deflate_are_not_success(){
        for input in [vec![3u8],vec![3,0,0]] {let mut r=&input[..];assert!(decompress::deflate::extract_deflate(&mut r,&mut io::sink(),input.len()as u64,None).is_err());}
        assert!(decompress::deflate::extract_deflate(&mut &[3u8,0][..],&mut io::sink(),2,None).is_ok());
    }
    #[test]fn filenames_are_not_lossily_repaired(){
        assert!(unegg::encoding::decode_filename(false,None,b"bad\xff").is_err());
        for n in ["/absolute","../up","a\0b","lost?name"]{assert!(safe_name(n).is_err());}
        assert_eq!(safe_name("中文\\目录/说明.txt").unwrap(),"中文/目录/说明.txt");
    }
    #[test]fn password_codepages_reject_aliases_and_loss(){
        for s in ["¥","‾","🔑"]{assert!(password_bytes(s,Some("cp932")).is_err());}
        assert_eq!(password_bytes("中文密码123!",Some("cp936")).unwrap(),encoding_rs::GBK.encode("中文密码123!").0.as_ref());
    }
    #[test]fn solid_selection_reports_password_from_shared_dependencies(){
        let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/archives/egg-153/upstream");
        let mut a=open(&root.join("solid.egg")).unwrap();let encrypted=open(&root.join("aes128.egg")).unwrap();
        // Metadata-only dependency unit: not a synthesized positive archive.
        let owner=a.entries.iter().position(|e|!e.blocks.is_empty()).unwrap();a.entries[owner].encrypt_info=encrypted.entries.iter().find_map(|e|e.encrypt_info.clone());
        let chosen=vec![a.entries.iter().find(|e|e.file_name=="hello.txt").unwrap().clone()];assert!(chosen[0].encrypt_info.is_none());
        assert_eq!(metadata(&a,&chosen).unwrap()["encrypted"],true);assert!(plan(&a,&chosen).is_ok());
    }
    #[test]fn altered_bzip2_lzma_streams_cannot_pass_complete_content_checks(){
        let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/archives/egg-153/upstream");
        for (file,method) in [("bzip2_big.egg",CompressionMethod::Bzip2),("lzma5.egg",CompressionMethod::Lzma)] {
            let mut a=EggArchive::open(File::open(root.join(file)).unwrap()).unwrap();
            let b=a.entries.iter().flat_map(|e|&e.blocks).find(|b|b.compression_method==method).unwrap().clone();
            a.reader.seek(SeekFrom::Start(b.data_pos)).unwrap();let mut data=vec![0;b.compressed_size as usize];a.reader.read_exact(&mut data).unwrap();data.push(0);
            let n=data.len() as u64;let mut r=&data[..];let mut output=CheckedWriter{inner:io::sink(),bytes:0,limit:b.uncompressed_size as u64};
            let result=if method==CompressionMethod::Bzip2 {decompress::bzip2::extract_bzip2(&mut r,&mut output,n,None)}else{decompress::lzma::extract_lzma(&mut r,&mut output,n,b.uncompressed_size as u64,None)};
            // Decoder Ok alone is NOT the app's acceptance condition. In the
            // LZMA case extra input may produce extra/altered output; the same
            // bounded length + CRC condition used by decode must reject it.
            assert!(!matches!(result,Ok(crc) if crc==b.crc32 && output.bytes==b.uncompressed_size as u64),"{file} passed full content checks");
        }
    }
}

//! Selected text and images; metadata and payload share an input budget.
use anyhow::{ensure, Context, Result};
use std::{io::{self, Read, Seek, SeekFrom}, path::Path, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use super::numeric_volume_reader::ZipInputReader;

const MAX_INPUT: u64 = 64 * 1024 * 1024;
const MAX_TEXT: u64 = 1024 * 1024;

struct Input<R> { inner: R, read: u64, cancelled: Arc<AtomicBool>, until: Instant, control: Option<super::image_reader::Control> }
impl<R> Input<R> {
    fn check(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) { return Err(io::Error::other("7Z 预览已取消")); }
        if Instant::now() >= self.until { return Err(io::Error::new(io::ErrorKind::TimedOut,"7Z 预览超过 30 秒，请先提取后打开")); }
        Ok(())
    }
}
impl<R: Read> Read for Input<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.check()?;
        if buffer.is_empty() { return Ok(0); }
        let capacity = buffer.len().min(MAX_INPUT.saturating_sub(self.read) as usize);
        if capacity == 0 { return Err(io::Error::other("7Z 预览读取超过 64 MiB，请先提取后打开")); }
        let n = self.inner.read(&mut buffer[..capacity])?;
        self.read += n as u64;
        if let Some(control)=&self.control {control.update("reading",self.read,0);}
        Ok(n)
    }
}
impl<R: Seek> Seek for Input<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> { self.check()?; self.inner.seek(position) }
}

fn engine_error(error: sevenz_rust::Error, encrypted: bool) -> anyhow::Error {
    use sevenz_rust::Error as E;
    match error {
        E::MaybeBadPassword(e) if e.raw_os_error().is_some() || e.kind()==io::ErrorKind::TimedOut || e.to_string().contains("预览") => anyhow::anyhow!(e),
        E::PasswordRequired | E::MaybeBadPassword(_) => anyhow::anyhow!("[sevenz-preview:password] 7Z 所选内容需要正确密码，也可能是加密数据损坏，请核对密码与分卷来源"),
        E::Io(e, _) if e.raw_os_error().is_some() || e.kind()==io::ErrorKind::TimedOut || e.to_string().contains("预览") => anyhow::anyhow!(e),
        E::UnsupportedCompressionMethod(_) | E::MaxMemLimited { .. } | E::Unsupported(_) => anyhow::anyhow!("7Z 所选内容的方法暂不支持预览，请先提取后打开：{error}"),
        _ if encrypted => anyhow::anyhow!("[sevenz-preview:password] 7Z 加密内容未通过读取或校验，请核对密码与分卷来源；必要时重新下载"),
        _ => anyhow::anyhow!("7Z 所选内容未通过完整读取或 CRC 校验，请重新下载或先提取检查：{error}"),
    }
}

pub(crate) fn read(path: &Path, selected: &str, password: Option<&str>, cancelled: Arc<AtomicBool>) -> Result<(Vec<u8>, u64)> {
    read_selected(path,selected,password,cancelled,MAX_TEXT,None)
}

pub(crate) fn read_image(path:&Path, selected:&str, password:Option<&str>, control:&super::image_reader::Control) -> Result<Vec<u8>> {
    Ok(read_selected(path,selected,password,control.flag.clone(),super::archive_preview::MAX_PREVIEW_BYTES,Some(control.clone()))?.0)
}

fn read_selected(path:&Path, selected:&str, password:Option<&str>, cancelled:Arc<AtomicBool>, max_output:u64, control:Option<super::image_reader::Control>) -> Result<(Vec<u8>,u64)> {
    with_archive(path,password,cancelled,control,|archive,input,key|{
    let mut matches = archive.files.iter().enumerate().filter(|(_,e)| e.name.replace('\\',"/")==selected);
    let (index,entry) = matches.next().context("所选 7Z 内容已不存在，请刷新目录")?;
    ensure!(matches.next().is_none(),"7Z 中存在同路径条目，不能确定所选内容，请先提取并确认内容");
    ensure!(!entry.is_directory && !entry.is_anti_item,"请选择普通文件");
    let prefix = max_output==MAX_TEXT && entry.size>max_output;
    ensure!(entry.size<=max_output || prefix,"7Z 所选文件超过 {} MiB 预览范围，请先提取后打开",max_output/1024/1024);
    if !entry.has_stream {
        ensure!(entry.size==0,"7Z 所选内容大小与数据流不一致，请先提取检查");
        return Ok((vec![],0));
    }
    let folder_index = archive.stream_map.file_folder_index.get(index).copied().flatten().context("7Z 所选内容缺少数据流，请先提取检查")?;
    let folder = archive.folders.get(folder_index).context("7Z 所选内容的数据块无法确认，请先提取检查")?;
    let shared = folder.num_unpack_sub_streams != 1 || folder.get_unpack_size()!=entry.size;
    if !shared && !prefix {ensure!(entry.has_crc || folder.has_crc,"7Z 所选内容缺少完整 CRC，暂不提供预览，请先提取检查");}
    // Explicit prefixes share bounded LZMA/LZMA2/PPMd decoding; complete
    // PPMd reads retain their independent finish and CRC verification path.
    let methods: Vec<_> = folder.ordered_coder_iter().map(|(_,c)|c.decompression_method_id()).collect();
    use sevenz_rust::SevenZMethod as M;
    let codec = |id:&[u8]| id==M::ID_LZMA || id==M::ID_LZMA2;
    let ppmd=methods.as_slice()==[M::ID_PPMD] || methods.as_slice()==[M::ID_AES256SHA256,M::ID_PPMD];
    let deflate64=methods.as_slice()==[M::ID_DEFLATE64] && !shared && !prefix;
    if methods.contains(&M::ID_DEFLATE64) {
        ensure!(deflate64,"7Z Deflate64 当前仅支持普通非固实完整小图文（文本 ≤1 MiB、图片 ≤8 MiB）；密码、固实或大文件请先提取后打开");
    }
    let deflate=methods.as_slice()==[M::ID_DEFLATE] || methods.as_slice()==[M::ID_AES256SHA256,M::ID_DEFLATE];
    if methods.contains(&M::ID_DEFLATE) {
        ensure!(deflate,"7Z Deflate 预览当前支持普通/密码完整小文本和图片；大文本仅显示开头，未校验整文件");
    }
    if methods.contains(&M::ID_PPMD) {
        ensure!(ppmd,"7Z 此 PPMd 方法组合暂不支持内部预览，请先提取后打开");
    }
    ensure!(methods.len()==folder.coders.len() && (deflate64 || deflate || ppmd || methods.len()==1 && codec(methods[0]) || methods.len()==2 && methods[0]==M::ID_AES256SHA256 && codec(methods[1])),"7Z 此压缩方法组合暂不支持内部预览，请先提取后打开");
    for c in &folder.coders {
        // Bound dictionaries before constructing the existing decoder.
        if c.decompression_method_id()==M::ID_LZMA { ensure!(c.properties.len()==5 && u32::from_le_bytes(c.properties[1..5].try_into().unwrap())<=64*1024*1024,"7Z 预览字典超过预览范围，请先提取后打开"); }
        if c.decompression_method_id()==M::ID_LZMA2 { ensure!(c.properties.len()==1 && c.properties[0]<=28,"7Z 预览字典超过预览范围，请先提取后打开"); }
    }
    let encrypted = methods.contains(&M::ID_AES256SHA256);
    if shared && !prefix {
        ensure!(folder.packed_streams.len()==1,"7Z 固实共享范围无法确认，请先提取后打开");
        let pack_index=archive.stream_map.folder_first_pack_stream_index.get(folder_index).context("7Z 固实共享范围无法确认，请先提取后打开")?;
        let packed=archive.pack_sizes.get(*pack_index).context("7Z 固实共享大小无法确认，请先提取后打开")?;
        ensure!(*packed<=MAX_INPUT && folder.get_unpack_size()<=MAX_INPUT,"7Z 固实共享内容超过 64 MiB 预览范围，请先提取后打开");
    }
    let flag=input.cancelled.clone();let until=input.until;
    let decoder = sevenz_rust::BlockDecoder::new(folder_index,&archive,key.as_slice(),input);
    let check = || {
        if flag.load(Ordering::Acquire) {return Err(sevenz_rust::Error::io(io::Error::other("7Z 预览已取消")));}
        if Instant::now()>=until {return Err(sevenz_rust::Error::io(io::Error::new(io::ErrorKind::TimedOut,"7Z 预览超过 30 秒，请先提取后打开")));}Ok(())
    };
    let bytes = if deflate64 {
        decoder.read_deflate64_entry(index,max_output as usize,check)
    } else if deflate {
        if prefix && shared { decoder.read_solid_deflate_entry_prefix(index,MAX_TEXT as usize,MAX_INPUT,check) } else if prefix { decoder.read_deflate_entry_prefix(index,MAX_TEXT as usize,check) } else if shared { decoder.read_solid_deflate_entry(index,max_output as usize,MAX_INPUT,check) } else { decoder.read_deflate_entry(index,max_output as usize,check) }
    } else if prefix && shared {
        decoder.read_solid_entry_prefix(index,MAX_TEXT as usize,MAX_INPUT,check)
    } else if prefix {
        decoder.read_single_entry_prefix(index,MAX_TEXT as usize,check)
    } else if ppmd && shared {
        decoder.read_solid_ppmd_entry(index,max_output as usize,MAX_INPUT,check)
    } else if ppmd {
        decoder.read_ppmd_entry(index,max_output as usize,check)
    } else if shared {
        decoder.read_solid_entry(index,max_output as usize,MAX_INPUT,||{
            if flag.load(Ordering::Acquire) {return Err(sevenz_rust::Error::io(io::Error::other("7Z 预览已取消")));}
            if Instant::now()>=until {return Err(sevenz_rust::Error::io(io::Error::new(io::ErrorKind::TimedOut,"7Z 预览超过 30 秒，请先提取后打开")));}
            Ok(())
        })
    } else {
        ensure!(decoder.entries().len()==1 && decoder.entries()[0].name==entry.name,"7Z 所选内容的数据映射无法确认，请先提取后打开");
        decoder.read_single_entry(max_output as usize)
    }.map_err(|e|engine_error(e,encrypted))?;
    input.check()?;
    Ok((bytes,entry.size))
    })
}

pub(crate) fn metadata(path:&Path,password:Option<&str>,cancelled:Arc<AtomicBool>)->Result<sevenz_rust::Archive>{
    with_archive(path,password,cancelled,None,|archive,_,_|Ok(archive))
}

fn with_archive<T>(path:&Path,password:Option<&str>,cancelled:Arc<AtomicBool>,control:Option<super::image_reader::Control>,work:impl FnOnce(sevenz_rust::Archive,&mut Input<ZipInputReader>,sevenz_rust::Password)->Result<T>)->Result<T>{
    let mut input=Input{inner:ZipInputReader::open_seven_zip(path)?,read:0,cancelled,until:Instant::now()+Duration::from_secs(30),control};
    let flag=input.cancelled.clone();let until=input.until;
    sevenz_rust::with_preview_limits(MAX_INPUT,20,move||{
        if flag.load(Ordering::Acquire){return Err(sevenz_rust::Error::io(io::Error::other("7Z 预览已取消")));}
        if Instant::now()>=until{return Err(sevenz_rust::Error::io(io::Error::new(io::ErrorKind::TimedOut,"7Z 预览超过 30 秒，请先提取后打开")));}Ok(())
    },||{
        input.check()?;let length=input.seek(SeekFrom::End(0))?;
        super::native_extraction::seven_zip::validate_archive_extent(&mut input,length)?;input.seek(SeekFrom::Start(0))?;
        let key=password.filter(|s|!s.is_empty()).map(sevenz_rust::Password::from).unwrap_or_default();
        let archive=sevenz_rust::Archive::read(&mut input,length,key.as_slice()).map_err(|e|engine_error(e,!key.is_empty()))?;
        input.check()?;work(archive,&mut input,key)
    }).map_err(|error:anyhow::Error|match error.downcast::<sevenz_rust::Error>(){Ok(e)=>engine_error(e,false),Err(e)=>e})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{archive_preview as preview, compression_service::{CompressionService,CompressionServiceConfig}, encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
    fn fixtures()->std::path::PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-text-1617")}
    fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(fixtures().join("manifest.json")).unwrap()).unwrap()}
    fn case(name:&str)->std::path::PathBuf { let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();fixtures().join(name).join(c["members"][0]["name"].as_str().unwrap()) }
    const SELECTED:&str="notes/说明 [1].txt";
    const KEY:&str="7Z预览密码1617!";
    #[tokio::test]
    async fn sevenz1617_original_methods_passwords_and_complete_numeric_members() {
        let m=manifest();
        for name in ["lzma","lzma2","contents","headers-volumes"] {
            let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();
            for member in c["members"].as_array().unwrap() {
                let source=fixtures().join(name).join(member["name"].as_str().unwrap());
                let password=c["password"].as_str();
                let value=preview::preview_archive_text(&source,SELECTED,password).await.unwrap();
                assert_eq!(hex::encode(sha2::Sha256::digest(value.content.as_bytes())),m["textSha256"].as_str().unwrap());
                assert!(!value.truncated);assert_eq!(value.byte_size,value.total_size);
                for (entry,text) in [("empty.txt",""),("utf16.txt","完整 UTF16 文本"),("gbk.txt","中文编码说明"),("notes/说明 1.txt","NOT the selected bracket name")] {
                    assert_eq!(preview::preview_archive_text(&source,entry,password).await.unwrap().content,text);
                }
            }
        }
    }
    #[tokio::test]
    async fn sevenz1617_rejects_bad_end_binary_oversize_and_duplicate_paths() {
        for (source,entry,reason) in [(case("lzma2"),"binary.txt","binary"),(fixtures().join("bad-end.7z"),"readme.txt","CRC"),(case("lzma2"),"missing.txt","不存在")] {
            let error=preview::preview_archive_text(&source,entry,None).await.unwrap_err();assert!(format!("{error:#}").contains(reason),"{error:#}");
        }
        let temp=tempfile::tempdir().unwrap();
        for (name,entries,reason) in [("duplicates",vec![("same.txt",b"first".to_vec()),("same.txt",b"second".to_vec())],"同路径")] {
            let p=temp.path().join(format!("{name}.7z"));let mut writer=sevenz_rust::SevenZWriter::create(&p).unwrap();
            for (name,data) in &entries {let mut e=sevenz_rust::SevenZArchiveEntry::default();e.name=(*name).into();writer.push_archive_entry(e,Some(data.as_slice())).unwrap();}writer.finish().unwrap();
            let error=preview::preview_archive_text(&p,entries[0].0,None).await.unwrap_err();assert!(format!("{error:#}").contains(reason),"{error:#}");
        }
        // CRC failures are never displayed, even when all text bytes are readable.
        let mut input=std::fs::File::open(fixtures().join("terminal.7z")).unwrap();let length=input.metadata().unwrap().len();let mut a=sevenz_rust::Archive::read(&mut input,length,&[]).unwrap();a.files[0].crc^=1;
        assert!(sevenz_rust::BlockDecoder::new(0,&a,&[],&mut input).read_single_entry(MAX_TEXT as usize).is_err());
    }
    #[tokio::test]
    async fn sevenz1617_vault_is_selected_only_and_explicit_wrong_key_never_falls_back() {
        use crate::models::password::{PasswordEntry,PasswordCategory};
        let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-test-key").await.unwrap();
        for (name,key) in [("a wrong","wrong"),("b public",KEY)] {vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
        let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
        for name in ["contents","headers-volumes"] {let p=case(name);assert!(!service.preview_sevenz_text_with_vault(p.to_str().unwrap(),SELECTED,None,None).await.unwrap().truncated);assert!(service.preview_sevenz_text_with_vault(p.to_str().unwrap(),SELECTED,Some("wrong manual"),None).await.is_err());}
        assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|e|e.use_count==0));
        let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-text-1617/large-unrelated.7z");
        assert!(p.metadata().unwrap().len()>MAX_INPUT);assert!(preview::preview_archive_text(&p,"readme.txt",None).await.unwrap().content.starts_with("7Z"));
    }
    #[tokio::test]
    async fn sevenz1617_cancellation_source_changes_missing_tail_and_budget_are_distinct() {
        let temp=tempfile::tempdir().unwrap();let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="headers-volumes").unwrap();
        let members:Vec<_>=c["members"].as_array().unwrap().iter().map(|e|{let n=e["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(fixtures().join("headers-volumes").join(n),&p).unwrap();p}).collect();
        let p=&members[1];let rev=super::super::image_reader::revision(p).unwrap();
        let error=preview::preview_archive_text_controlled_at(p,SELECTED,Some(KEY),None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await.unwrap_err();assert!(error.to_string().contains("取消"));
        std::fs::remove_file(members.last().unwrap()).unwrap();
        let error=preview::preview_archive_text_controlled_at(p,SELECTED,Some(KEY),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
        assert!(preview::preview_archive_text(p,SELECTED,Some(KEY)).await.is_err());
        let mut input=Input{inner:io::Cursor::new(vec![0;4]),read:MAX_INPUT-1,cancelled:Arc::new(AtomicBool::new(false)),until:Instant::now()+Duration::from_secs(30),control:None};
        let mut b=[0;2];assert_eq!(input.read(&mut b).unwrap(),1);input.seek(SeekFrom::Start(0)).unwrap();assert!(input.read(&mut b).unwrap_err().to_string().contains("64 MiB"));
        input.until=Instant::now()-Duration::from_secs(1);assert!(input.seek(SeekFrom::Start(0)).unwrap_err().to_string().contains("30 秒"));
    }
    use sha2::Digest;
}

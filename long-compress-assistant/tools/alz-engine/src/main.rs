//! Isolated ALZ decoder. No passwords in command arguments; stdin JSON only.
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, fs::{self, File}, io::{self, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}};
use unalz::{archive::{AlzArchive, AlzFileEntry, CompressionMethod}, crypto::ZipCrypto,
    decompress::{bzip2, deflate, raw}, error::AlzError, multivolume::MultiVolumeReader};

#[path = "../../../src-tauri/src/services/alz_volumes.rs"]
mod alz_volumes;

#[derive(Deserialize, Default)]
struct Request {
    encoding: Option<String>,
    password: Option<String>,
    output: Option<PathBuf>,
    #[serde(default)] preserve_timestamps: bool,
    #[serde(default)] selected: Vec<String>,
    // The app resolves its existing regex filter against the original names.
}

fn safe_name(name: &str) -> Result<String> {
    let name = name.replace('\\', "/");
    let name = name.trim_end_matches('/');
    ensure!(!name.is_empty() && !name.starts_with('/') && !name.contains([':', '\0', '?', '*', '|', '<', '>', '"'])
        && name.split('/').all(|p| !p.is_empty() && p != "." && p != ".."),
        "[archive-inspection:unsupported] ALZ 条目路径无法安全写入 Windows：{name}");
    Ok(name.to_string())
}

fn codec(value: Option<&str>) -> Result<&'static encoding_rs::Encoding> {
    Ok(match value.unwrap_or("cp949") {
        "cp949" => encoding_rs::EUC_KR,
        "cp936" => encoding_rs::GBK,
        "cp950" => encoding_rs::BIG5,
        "cp932" => encoding_rs::SHIFT_JIS,
        "utf8" => encoding_rs::UTF_8,
        value => anyhow::bail!("[archive-inspection:unsupported] ALZ 编码 {value} 当前不受支持"),
    })
}

fn password_bytes(password: &str, codec: &'static encoding_rs::Encoding) -> Result<Vec<u8>> {
    let (bytes, _, lossy) = codec.encode(password);
    let (round_trip, invalid) = codec.decode_without_bom_handling(&bytes);
    ensure!(!lossy && !invalid && round_trip == password,
        "[archive-inspection:password-encoding] [alz-password-encoding] 此密码不能无损表示为 ALZ 所选代码页 {}；未替换字符或尝试其他密码", codec.name());
    Ok(bytes.into_owned())
}

fn open(path: &Path, codec: &'static encoding_rs::Encoding) -> Result<AlzArchive> {
    let group = alz_volumes::complete(path)?;
    let reader = MultiVolumeReader::from_paths(&group.parts).map_err(|e| map_error(e, false))?;
    let mut archive = AlzArchive::from_reader(reader).map_err(|e| map_error(e, false))?;
    for entry in &mut archive.entries {
        let (name, lossy) = codec.decode_without_bom_handling(&entry.file_name_raw);
        ensure!(!lossy, "[archive-inspection:password-encoding] [alz-encoding] ALZ 文件名不能按 {} 完整解码，请选择原工具代码页", codec.name());
        entry.file_name = name.into_owned();
    }
    ensure!(!archive.truncated, "[archive-inspection:damaged] ALZ 索引被截断，不能将部分内容记为完整成功");
    ensure!(archive.entries.len() <= 250_000, "[archive-inspection:unsupported] ALZ 条目数超出支持范围");
    let mut names = HashSet::new();
    let mut size = 0u64;
    for e in &archive.entries {
        let name = safe_name(&e.file_name)?;
        ensure!(names.insert(name.to_lowercase()), "[archive-inspection:unsupported] ALZ 存在 Windows 同名条目：{name}");
        ensure!(e.data_pos.checked_add(e.compressed_size).is_some_and(|end| end <= group.virtual_bytes - 16),
            "[archive-inspection:damaged] ALZ 条目「{name}」的数据范围超出归档，可能被截断");
        size = size.checked_add(e.uncompressed_size).context("ALZ 大小溢出")?;
    }
    ensure!(size <= 512 * 1024 * 1024 * 1024, "[archive-inspection:unsupported] ALZ 解压大小超出支持范围");
    Ok(archive)
}

fn chosen(entries: &[AlzFileEntry], selected: &[String]) -> Result<Vec<AlzFileEntry>> {
    let selected = selected.iter().map(|s| safe_name(s)).collect::<Result<Vec<_>>>()?;
    let mut found = vec![false; selected.len()];
    let mut result = Vec::new();
    for e in entries {
        let name = safe_name(&e.file_name)?;
        let mut matched = selected.is_empty();
        for (i, s) in selected.iter().enumerate() {
            // The app has already expanded directory selections and intersected
            // its file filter. Do not silently expand them a second time.
            if name == *s { found[i] = true; matched = true; }
        }
        if matched { result.push(e.clone()); }
    }
    ensure!(found.iter().all(|v| *v), "所选 ALZ 条目不存在，请刷新目录后重新选择");
    Ok(result)
}

fn metadata(entries: &[AlzFileEntry]) -> Result<Value> {
    let mut result = Vec::new();
    let mut total = 0u64;
    let mut compressed = 0u64;
    let mut files = 0usize;
    let mut encrypted = false;
    for e in entries {
        let path = safe_name(&e.file_name)?;
        let is_dir = e.is_directory();
        if !is_dir { total += e.uncompressed_size; files += 1; }
        compressed += e.compressed_size;
        encrypted |= e.is_encrypted() && !is_dir;
        result.push(json!({"name":path.rsplit('/').next(), "path":path, "size":e.uncompressed_size,
            "sizeKnown":true, "compressedSize":e.compressed_size, "modified":null,
            "crc":format!("{:08X}", e.file_crc), "encrypted":e.is_encrypted(), "isDir":is_dir}));
    }
    Ok(json!({"format":"ALZ", "entries":result, "totalFiles":files,
        "totalDirectories":entries.len()-files, "totalUncompressedSize":total,
        "totalCompressedSize":compressed, "encrypted":encrypted}))
}

fn map_error(error: AlzError, encrypted: bool) -> anyhow::Error {
    match error {
        AlzError::Io(e) | AlzError::CantOpenFile(e) | AlzError::CantOpenDestFile(e)
            if e.kind() != io::ErrorKind::UnexpectedEof => anyhow::Error::new(e).context("[archive-inspection:io] [alz-io] ALZ 文件读写失败"),
        AlzError::PasswordNotSet => anyhow::anyhow!("[archive-inspection:password-required] ALZ 所选内容需要密码"),
        AlzError::InvalidPassword => anyhow::anyhow!("[alz-password] ALZ 密码短头不匹配，也可能加密头损坏；请核对密码和来源完整性"),
        AlzError::UnknownCompressionMethod(3) => anyhow::anyhow!("[archive-inspection:unsupported] ALZ Obfuscated Deflate（方法 3）尚未通过原包验证，当前不支持；请向提供者索取 ZIP/7Z 版本，不应反复尝试密码"),
        AlzError::UnknownCompressionMethod(m) => anyhow::anyhow!("[archive-inspection:unsupported] ALZ 压缩方法 {m} 当前不受支持，不应反复尝试密码"),
        e => anyhow::anyhow!("{} ALZ 内容、大小或 CRC 校验失败：{e}",
            if encrypted { "[archive-inspection:ambiguous-data]" } else { "[archive-inspection:damaged]" }),
    }
}

struct CheckedWriter<W> { inner: W, bytes: u64, crc: crc32fast::Hasher }
impl<W: Write> Write for CheckedWriter<W> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(b)?; self.bytes += n as u64; self.crc.update(&b[..n]); Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> { self.inner.flush() }
}

fn entry_crypto(e: &AlzFileEntry, password: Option<&str>, codec: &'static encoding_rs::Encoding) -> Result<Option<ZipCrypto>> {
    let crypto = if e.is_encrypted() {
        let password = password.context("[archive-inspection:password-required] ALZ 所选内容需要密码")?;
        let bytes = password_bytes(password, codec)?;
        let mut cipher = ZipCrypto::new(&bytes);
        let header = e.enc_check.context("[archive-inspection:damaged] ALZ 加密头不完整")?;
        ensure!(cipher.check_header(&header, e.file_crc, e.file_time_date, e.has_data_descriptor()),
            "[alz-password] ALZ 密码短头不匹配，也可能加密头损坏；请核对密码和来源完整性");
        let mut cipher = ZipCrypto::new(&bytes);
        let mut check = header; cipher.decrypt(&mut check); Some(cipher)
    } else { None };
    Ok(crypto)
}


fn decode<W: Write>(archive: &mut AlzArchive, e: &AlzFileEntry, password: Option<&str>, codec: &'static encoding_rs::Encoding, writer: W) -> Result<()> {
    let mut crypto = entry_crypto(e, password, codec)?;
    archive.reader.seek(SeekFrom::Start(e.data_pos))?;
    let mut input = (&mut archive.reader).take(e.compressed_size);
    let mut out = CheckedWriter { inner: writer, bytes: 0, crc: crc32fast::Hasher::new() };
    let result = match e.compression_method {
        CompressionMethod::Store => raw::extract_raw(&mut input, &mut out, e.compressed_size, e.uncompressed_size, crypto.as_mut()),
        CompressionMethod::Deflate => deflate::extract_deflate(&mut input, &mut out, e.compressed_size, e.uncompressed_size, crypto.as_mut()),
        CompressionMethod::Bzip2 => bzip2::extract_bzip2(&mut input, &mut out, e.compressed_size, e.uncompressed_size, crypto.as_mut()),
        CompressionMethod::Unknown(m) => Err(AlzError::UnknownCompressionMethod(m)),
    };
    if let Err(error @ (AlzError::TrailingCompressedData { .. } | AlzError::Bzip2MissingEnd)) = &result {
        // Only full decoded size AND CRC establish this is not another key
        // candidate. A matching short encryption header is not sufficient.
        if out.bytes == e.uncompressed_size && out.crc.clone().finalize() == e.file_crc {
            let cause = match error {
                AlzError::TrailingCompressedData { remaining } => format!("压缩流尾部仍有 {remaining} 字节异常数据"),
                _ => "Bzip2 压缩流缺少结束标记".to_string(),
            };
            anyhow::bail!("[archive-inspection:damaged] ALZ 条目「{}」内容已通过大小和 CRC 校验，但{}；归档不完整或已损坏，请重新获取完整原包，不必反复尝试密码", e.file_name, cause);
        }
    }
    result.map_err(|err| map_error(err, e.is_encrypted()))?;
    ensure!(out.bytes == e.uncompressed_size && out.crc.finalize() == e.file_crc,
        "{} ALZ 条目「{}」大小或 CRC 校验失败，未将结果记为成功", if e.is_encrypted() { "[archive-inspection:ambiguous-data]" } else { "[archive-inspection:damaged]" }, e.file_name);
    out.inner.flush().context("[alz-io] ALZ 输出刷盘失败")?;
    Ok(())
}

const MAX_TEXT_PREVIEW: u64 = 1024 * 1024;
const MAX_TEXT_PREVIEW_INPUT: u64 = 64 * 1024 * 1024;

struct PreviewBuffer { bytes: Vec<u8>, limit: usize }
impl Write for PreviewBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit - self.bytes.len() {
            return Err(io::Error::other("ALZ 内容超过所选预览范围，请先提取后打开"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

fn preview_entry(archive: &mut AlzArchive, entries: &[AlzFileEntry], password: Option<&str>, codec: &'static encoding_rs::Encoding, image: bool) -> Result<Value> {
    ensure!(entries.len() == 1 && !entries[0].is_directory(), "请选择一个 ALZ 文件预览");
    let entry = &entries[0];
    let limit = if image { 8 * 1024 * 1024 } else { MAX_TEXT_PREVIEW };
    ensure!(entry.uncompressed_size <= limit, "{}", if image { "ALZ 图片超过 8 MiB 预览范围，请先提取后打开" } else { "ALZ 文本预览支持 1 MiB 以内的完整文件；此文件较大，请先提取后打开" });
    ensure!(entry.compressed_size <= MAX_TEXT_PREVIEW_INPUT, "所选 ALZ 内容压缩数据超过 64 MiB 预览范围，请先提取后打开");
    let mut buffer = PreviewBuffer { bytes: Vec::with_capacity(entry.uncompressed_size as usize), limit: limit as usize };
    decode(archive, entry, password, codec, &mut buffer)?;
    Ok(json!({"bytes":buffer.bytes,"totalSize":entry.uncompressed_size}))
}

// An intentional preview boundary is distinct from decoding/I/O failures.
#[derive(Debug)]
struct PrefixComplete;
impl std::fmt::Display for PrefixComplete { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("ALZ preview prefix complete") } }
impl std::error::Error for PrefixComplete {}
struct PrefixWriter(Vec<u8>);
impl Write for PrefixWriter {
    fn write(&mut self, bytes:&[u8])->io::Result<usize>{
        let count=bytes.len().min(MAX_TEXT_PREVIEW as usize-self.0.len());
        self.0.extend_from_slice(&bytes[..count]);
        if self.0.len()==MAX_TEXT_PREVIEW as usize {return Err(io::Error::other(PrefixComplete));}
        Ok(count)
    }
    fn flush(&mut self)->io::Result<()>{Ok(())}
}
struct PrefixInput<R> { inner:R, remaining:u64, read:u64 }
impl<R:Read> Read for PrefixInput<R> {
    fn read(&mut self, buffer:&mut[u8])->io::Result<usize>{
        if buffer.is_empty(){return Ok(0)}
        if self.remaining==0{return Err(io::Error::other("ALZ 文本前缀读取超过 64 MiB 范围，请先提取后打开"))}
        let count=buffer.len().min(self.remaining as usize);
        let count=self.inner.read(&mut buffer[..count])?;self.remaining-=count as u64;self.read+=count as u64;Ok(count)
    }
}
fn preview_text_prefix(archive:&mut AlzArchive,entries:&[AlzFileEntry],password:Option<&str>,codec:&'static encoding_rs::Encoding)->Result<Value>{
    ensure!(entries.len()==1&&!entries[0].is_directory(),"请选择一份 ALZ 文本预览");
    let entry=&entries[0];
    ensure!(entry.uncompressed_size>MAX_TEXT_PREVIEW,"小文本应使用完整预览");
    let mut crypto=entry_crypto(entry,password,codec)?;
    archive.reader.seek(SeekFrom::Start(entry.data_pos))?;
    let mut input=PrefixInput{inner:(&mut archive.reader).take(entry.compressed_size),remaining:MAX_TEXT_PREVIEW_INPUT,read:0};
    let mut output=PrefixWriter(Vec::with_capacity(MAX_TEXT_PREVIEW as usize));
    // Bzip2 emits one complete block at a time; bound its RLE-expanded block too.
    let limit=entry.uncompressed_size.min(64*1024*1024);
    let result=match entry.compression_method {
        CompressionMethod::Store=>raw::extract_raw(&mut input,&mut output,entry.compressed_size,limit,crypto.as_mut()),
        CompressionMethod::Deflate=>deflate::extract_deflate(&mut input,&mut output,entry.compressed_size,limit,crypto.as_mut()),
        CompressionMethod::Bzip2=>bzip2::extract_bzip2(&mut input,&mut output,entry.compressed_size,limit,crypto.as_mut()),
        CompressionMethod::Unknown(m)=>Err(AlzError::UnknownCompressionMethod(m)),
    };
    match result {
        Err(AlzError::CantOpenDestFile(error)) if error.get_ref().is_some_and(|e|e.is::<PrefixComplete>())=>{},
        Err(error)=>return Err(map_error(error,entry.is_encrypted())),
        Ok(_)=>anyhow::bail!("ALZ 文本在预览范围前已结束且与声明大小不符，请先检查或重新获取来源"),
    }
    ensure!(output.0.len()==MAX_TEXT_PREVIEW as usize,"ALZ 文本前缀未完成");
    Ok(json!({"bytes":output.0,"totalSize":entry.uncompressed_size,"partial":true,"inputBytes":input.read}))
}

fn run() -> Result<Value> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    ensure!(args.len() == 2, "Usage: long-alz list|plan|verify|extract|preview-text|preview-image|preview-text-prefix archive (request on stdin)");
    let operation = args[0].to_str().context("Invalid operation")?;
    let mut data = Vec::new(); io::stdin().take(32 * 1024 * 1024 + 1).read_to_end(&mut data)?;
    ensure!(data.len() <= 32 * 1024 * 1024, "ALZ 请求过大");
    let request: Request = serde_json::from_slice(&data)?;
    data.fill(0);
    ensure!(request.selected.len() <= 100000, "ALZ 所选条目过多");
    let codec = codec(request.encoding.as_deref())?;
    let mut archive = open(Path::new(&args[1]), codec)?;
    let entries = chosen(&archive.entries, &request.selected)?;
    if operation == "list" { return metadata(&entries); }
    for e in &entries {
        if let CompressionMethod::Unknown(m) = e.compression_method {
            return Err(map_error(AlzError::UnknownCompressionMethod(m), false));
        }
    }
    if operation == "plan" { return metadata(&entries); }
    if operation == "preview-text-prefix" {
        ensure!(request.selected.len()==1,"ALZ 前缀预览必须指定一个精确条目");
        return preview_text_prefix(&mut archive,&entries,request.password.as_deref(),codec);
    }
    if matches!(operation, "preview-text" | "preview-image") {
        ensure!(request.selected.len() == 1, "ALZ 预览必须指定一个精确条目");
        return preview_entry(&mut archive, &entries, request.password.as_deref(), codec, operation == "preview-image");
    }
    ensure!(matches!(operation, "verify" | "extract"), "Unsupported operation");
    // Plain corruption is definitive evidence; inspect it before password trials.
    let mut entries = entries;
    entries.sort_by_key(|e| e.is_encrypted());
    for e in entries {
        if e.is_directory() {
            if operation == "extract" { fs::create_dir_all(request.output.as_ref().context("Missing output")?.join(safe_name(&e.file_name)?))?; }
            continue;
        }
        if operation == "verify" { decode(&mut archive, &e, request.password.as_deref(), codec, io::sink())?; }
        else {
            let dest = request.output.as_ref().context("Missing output")?.join(safe_name(&e.file_name)?);
            fs::create_dir_all(dest.parent().context("Invalid output")?).context("[alz-io] 无法创建 ALZ 输出目录")?;
            let mut file = File::options().write(true).create_new(true).open(&dest).context("[alz-io] 无法创建 ALZ 输出文件")?;
            decode(&mut archive, &e, request.password.as_deref(), codec, &mut file)?;
            file.sync_all().context("[alz-io] ALZ 文件刷盘失败")?;
            if request.preserve_timestamps {
                if let Some(time) = unalz::dostime::dos_datetime_to_systime(e.file_time_date) { file.set_modified(time).context("[archive-inspection:io] [alz-io] ALZ 文件时间写入失败")?; }
            }
        }
    }
    Ok(json!({"verified":true}))
}

fn main() {
    let (value, code) = match run() {
        Ok(directory) => (json!({"result":directory}), 0),
        Err(error) => (json!({"error":format!("{error:#}")}), 2),
    };
    println!("{value}");
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_codepage_aliases_not_only_unrepresentable_characters() {
        for text in ["¥", "‾"] {
            // encoding_rs considers these encodable; the decoded Unicode text
            // differs. A header mismatch here must not become a wrong-key claim.
            assert!(!encoding_rs::SHIFT_JIS.encode(text).2);
            assert!(password_bytes(text, encoding_rs::SHIFT_JIS).unwrap_err().to_string().contains("[alz-password-encoding]"));
        }
    }
    #[test]
    fn preserves_representable_original_passwords_and_rejects_loss() {
        for (text, codec) in [("test1234",encoding_rs::EUC_KR),("中文密码123!",encoding_rs::GBK),("日本語123",encoding_rs::SHIFT_JIS),("繁體123",encoding_rs::BIG5),("🔑 中文",encoding_rs::UTF_8)] {
            assert_eq!(codec.decode_without_bom_handling(&password_bytes(text,codec).unwrap()).0,text);
        }
        assert!(password_bytes("🔑",encoding_rs::GBK).is_err());
    }
}

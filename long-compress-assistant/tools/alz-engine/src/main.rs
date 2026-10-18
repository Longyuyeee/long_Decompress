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

fn decode<W: Write>(archive: &mut AlzArchive, e: &AlzFileEntry, password: Option<&str>, codec: &'static encoding_rs::Encoding, writer: W) -> Result<()> {
    let mut crypto = if e.is_encrypted() {
        let password = password.context("[archive-inspection:password-required] ALZ 所选内容需要密码")?;
        let (bytes, _, lossy) = codec.encode(password);
        ensure!(!lossy, "[archive-inspection:password-encoding] [alz-password-encoding] 此密码不能无损表示为 ALZ 所选代码页 {}；未替换字符或尝试其他密码", codec.name());
        let mut cipher = ZipCrypto::new(&bytes);
        let header = e.enc_check.context("[archive-inspection:damaged] ALZ 加密头不完整")?;
        ensure!(cipher.check_header(&header, e.file_crc, e.file_time_date, e.has_data_descriptor()),
            "[alz-password] ALZ 密码短头不匹配，也可能加密头损坏；请核对密码和来源完整性");
        let mut cipher = ZipCrypto::new(&bytes);
        let mut check = header; cipher.decrypt(&mut check); Some(cipher)
    } else { None };
    archive.reader.seek(SeekFrom::Start(e.data_pos))?;
    let mut input = (&mut archive.reader).take(e.compressed_size);
    let mut out = CheckedWriter { inner: writer, bytes: 0, crc: crc32fast::Hasher::new() };
    let result = match e.compression_method {
        CompressionMethod::Store => raw::extract_raw(&mut input, &mut out, e.compressed_size, e.uncompressed_size, crypto.as_mut()),
        CompressionMethod::Deflate => deflate::extract_deflate(&mut input, &mut out, e.compressed_size, e.uncompressed_size, crypto.as_mut()),
        CompressionMethod::Bzip2 => bzip2::extract_bzip2(&mut input, &mut out, e.compressed_size, e.uncompressed_size, crypto.as_mut()),
        CompressionMethod::Unknown(m) => Err(AlzError::UnknownCompressionMethod(m)),
    };
    result.map_err(|err| map_error(err, e.is_encrypted()))?;
    ensure!(out.bytes == e.uncompressed_size && out.crc.finalize() == e.file_crc,
        "{} ALZ 条目「{}」大小或 CRC 校验失败，未将结果记为成功", if e.is_encrypted() { "[archive-inspection:ambiguous-data]" } else { "[archive-inspection:damaged]" }, e.file_name);
    out.inner.flush().context("[alz-io] ALZ 输出刷盘失败")?;
    Ok(())
}

fn run() -> Result<Value> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    ensure!(args.len() == 2, "Usage: long-alz list|plan|verify|extract archive (request on stdin)");
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

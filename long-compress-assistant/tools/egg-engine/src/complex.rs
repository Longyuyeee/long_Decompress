//! Shared stream dependencies, authenticated cipher setup and solid output routing.
use super::*;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use unegg::archive::EggBlock;

pub fn validate(a: &EggArchive<egg_volumes::Reader>, selected: &[EggFileEntry]) -> Result<()> {
    let dependencies = if a.is_solid { &a.entries[..] } else { selected };
    let mut method = None;
    let mut expanded = 0u64;
    for e in dependencies {
        if let Some(i) = &e.encrypt_info {
            let valid = match i.method {
                EncryptionMethod::ZipCrypto => i.data.len() == 16,
                EncryptionMethod::Aes128 | EncryptionMethod::Lea128 => {
                    matches!(i.data.len(), 10 | 20)
                }
                EncryptionMethod::Aes256 | EncryptionMethod::Lea256 => {
                    matches!(i.data.len(), 18 | 28)
                }
                EncryptionMethod::Unknown(_) => {
                    anyhow::bail!("[archive-inspection:unsupported] EGG 加密方法不受支持")
                }
            };
            ensure!(valid, "[archive-inspection:damaged] EGG 加密头大小无效");
        }
        for b in &e.blocks {
            ensure!(
                !matches!(b.compression_method, CompressionMethod::Unknown(_)),
                "[archive-inspection:unsupported] EGG 压缩方法不受支持"
            );
            if a.is_solid {
                ensure!(
                    method.is_none_or(|m| m == b.compression_method),
                    "[archive-inspection:unsupported] EGG 固实流内混合压缩方法尚不受支持"
                );
                method = Some(b.compression_method);
                expanded += b.uncompressed_size as u64;
            }
        }
    }
    if a.is_solid {
        ensure!(
            expanded
                == a.entries
                    .iter()
                    .filter(|e| !e.is_directory())
                    .map(|e| e.uncompressed_size)
                    .sum::<u64>(),
            "[archive-inspection:damaged] EGG 固实文件大小与数据块之和不符"
        );
    }
    ensure!(
        !dependencies.iter().any(|e| e.encrypt_info.is_some())
            || dependencies.iter().any(|e| e.encrypt_info.is_some()
                && !e.blocks.is_empty()
                && e.blocks.iter().any(|b| b.uncompressed_size > 0)),
        "[archive-inspection:unsupported] 所选 EGG 只有加密空文件，不能通过实际内容完整确认密码"
    );
    Ok(())
}

pub fn cipher<R: Read + Seek>(
    reader: &mut R,
    e: &EggFileEntry,
    password: Option<&str>,
    encoding: Option<&str>,
) -> Result<Option<Box<dyn Decryptor>>> {
    let Some(i) = &e.encrypt_info else {
        return Ok(None);
    };
    let bytes = password_bytes(
        password.ok_or_else(|| map_error(EggError::PasswordNotSet, false))?,
        encoding,
    )?;
    if i.method == EncryptionMethod::ZipCrypto {
        let mut c = ZipCrypto::new(&bytes);
        ensure!(
            c.check_password(
                &i.data[..12].try_into()?,
                u32::from_le_bytes(i.data[12..16].try_into()?)
            ),
            "[egg-password] EGG 密码短头不匹配，也可能是加密头损坏；请核对密码和原文件完整性"
        );
        return Ok(Some(Box::new(c)));
    }
    let (salt_len, mode) = match i.method {
        EncryptionMethod::Aes128 | EncryptionMethod::Lea128 => (8, 1),
        EncryptionMethod::Aes256 | EncryptionMethod::Lea256 => (16, 3),
        _ => anyhow::bail!("[archive-inspection:unsupported] EGG 加密方法不受支持"),
    };
    let salt = &i.data[..salt_len];
    let verifier = i.data[salt_len..salt_len + 2].try_into()?;
    let (c, key): (Box<dyn Decryptor>, Vec<u8>) = if matches!(
        i.method,
        EncryptionMethod::Aes128 | EncryptionMethod::Aes256
    ) {
        let c = unegg::aes_ctr::AesCtrDecryptor::new(mode, &bytes, salt, &verifier)
            .map_err(|e| map_error(e, true))?;
        let key = c.auth_key().to_vec();
        (Box::new(c), key)
    } else {
        let c = unegg::lea::LeaCtrDecryptor::new(mode, &bytes, salt, &verifier)
            .map_err(|e| map_error(e, true))?;
        let key = c.auth_key().to_vec();
        (Box::new(c), key)
    };
    // Some genuine older producers omit the authentication footer. Both layouts
    // still require all decoded sizes and CRCs; the two-byte verifier is not proof.
    if i.data.len() == salt_len + 12 {
        let mut mac = Hmac::<Sha1>::new_from_slice(&key)
            .map_err(|_| anyhow::anyhow!("EGG HMAC setup failed"))?;
        let mut buf = [0u8; 64 * 1024];
        for b in &e.blocks {
            reader.seek(SeekFrom::Start(b.data_pos))?;
            let mut left = b.compressed_size as usize;
            while left > 0 {
                let n = left.min(buf.len());
                reader
                    .read_exact(&mut buf[..n])
                    .map_err(|e| map_error(e.into(), true))?;
                mac.update(&buf[..n]);
                left -= n;
            }
        }
        let tag = mac.finalize().into_bytes();
        ensure!(
            tag[..10] == i.data[salt_len + 2..],
            "[archive-inspection:ambiguous-data] EGG 加密内容认证失败，可能是数据损坏或密码不匹配；未确认密码正确"
        );
    }
    Ok(Some(c))
}

pub fn stream<R: Read, W: Write>(
    input: &mut R,
    out: &mut W,
    method: CompressionMethod,
    packed: u64,
    expanded: u64,
    cipher: Option<&mut dyn Decryptor>,
) -> std::result::Result<u32, EggError> {
    match method {
        CompressionMethod::Store => decompress::store::extract_store(input, out, packed, cipher),
        CompressionMethod::Deflate => {
            decompress::deflate::extract_deflate(input, out, packed, cipher)
        }
        CompressionMethod::Bzip2 => decompress::bzip2::extract_bzip2(input, out, packed, cipher),
        CompressionMethod::Lzma => {
            decompress::lzma::extract_lzma(input, out, packed, expanded, cipher)
        }
        CompressionMethod::Azo => {
            decompress::azo::extract_azo(input, out, packed, expanded, cipher)
        }
        CompressionMethod::Unknown(n) => Err(EggError::UnknownCompressionMethod(n)),
    }
}

struct Segments<'a> {
    a: &'a mut EggArchive<egg_volumes::Reader>,
    blocks: Vec<(usize, EggBlock)>,
    index: usize,
    used: u64,
    owner: Option<usize>,
    crypto: Option<Box<dyn Decryptor>>,
    password: Option<&'a str>,
    encoding: Option<&'a str>,
    error: Option<anyhow::Error>,
    read: u64,
}
impl Read for Segments<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        while self.index < self.blocks.len() {
            let (owner, b) = &self.blocks[self.index];
            if self.used == b.compressed_size as u64 {
                self.index += 1;
                self.used = 0;
                continue;
            }
            if self.owner != Some(*owner) {
                match cipher(
                    &mut self.a.reader,
                    &self.a.entries[*owner],
                    self.password,
                    self.encoding,
                ) {
                    Ok(c) => self.crypto = c,
                    Err(e) => {
                        self.error = Some(e);
                        return Err(io::Error::other("EGG solid cipher setup failed"));
                    }
                }
                self.owner = Some(*owner);
            }
            self.a
                .reader
                .seek(SeekFrom::Start(b.data_pos + self.used))?;
            let size = out
                .len()
                .min((b.compressed_size as u64 - self.used) as usize);
            self.a.reader.read_exact(&mut out[..size])?;
            if let Some(c) = self.crypto.as_mut() {
                c.decrypt(&mut out[..size]);
            }
            self.used += size as u64;
            self.read += size as u64;
            return Ok(size);
        }
        Ok(0)
    }
}

pub fn set_time(file: &File, e: &EggFileEntry, preserve: bool) -> Result<()> {
    if preserve {
        if let Some(t) = e
            .file_time
            .and_then(|t| (t / 10_000_000).checked_sub(11_644_473_600))
        {
            if let Some(t) = std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(t)) {
                file.set_modified(t)
                    .context("[archive-inspection:io] [egg-io] EGG 时间写入失败")?;
            }
        }
    }
    Ok(())
}
struct SolidSink<'a> {
    files: Vec<&'a EggFileEntry>,
    blocks: Vec<&'a EggBlock>,
    selected: HashSet<String>,
    output: Option<&'a Path>,
    fi: usize,
    fp: u64,
    bi: usize,
    bp: u64,
    hash: crc32fast::Hasher,
    file: Option<File>,
    preserve: bool,
    memory: Option<Vec<u8>>,
}
impl SolidSink<'_> {
    fn advance_files(&mut self) -> Result<()> {
        while self.fi < self.files.len() {
            let e = self.files[self.fi];
            if self.file.is_none() && self.selected.contains(&e.file_name) {
                if let Some(root) = self.output {
                    let p = root.join(safe_name(&e.file_name)?);
                    fs::create_dir_all(p.parent().context("Invalid EGG parent")?)?;
                    self.file = Some(File::options().write(true).create_new(true).open(p)?);
                }
            }
            if self.fp < e.uncompressed_size {
                break;
            }
            if let Some(f) = self.file.take() {
                f.sync_all()?;
                set_time(&f, e, self.preserve)?;
            }
            self.fi += 1;
            self.fp = 0;
        }
        Ok(())
    }
    fn advance_blocks(&mut self) -> io::Result<()> {
        while self.bi < self.blocks.len()
            && self.bp == self.blocks[self.bi].uncompressed_size as u64
        {
            let actual = std::mem::replace(&mut self.hash, crc32fast::Hasher::new()).finalize();
            if actual != self.blocks[self.bi].crc32 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "EGG solid block CRC mismatch",
                ));
            }
            self.bi += 1;
            self.bp = 0;
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        self.advance_files()?;
        self.advance_blocks()?;
        ensure!(
            self.fi == self.files.len() && self.bi == self.blocks.len(),
            "[archive-inspection:damaged] EGG 固实输出提前结束，文件或块大小不符"
        );
        Ok(())
    }
}
impl Write for SolidSink<'_> {
    fn write(&mut self, mut data: &[u8]) -> io::Result<usize> {
        let original = data.len();
        while !data.is_empty() {
            self.advance_files().map_err(io::Error::other)?;
            self.advance_blocks()?;
            if self.fi == self.files.len() || self.bi == self.blocks.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "EGG solid output exceeds declared file/block size",
                ));
            }
            let n = data
                .len()
                .min((self.files[self.fi].uncompressed_size - self.fp) as usize)
                .min((self.blocks[self.bi].uncompressed_size as u64 - self.bp) as usize);
            if let Some(f) = self.file.as_mut() {
                f.write_all(&data[..n])?;
            }
            if self.selected.contains(&self.files[self.fi].file_name) {
                if let Some(memory) = self.memory.as_mut() {
                    if memory.len() + n > MAX_TEXT as usize {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "EGG selected text exceeds preview limit"));
                    }
                    memory.extend_from_slice(&data[..n]);
                }
            }
            self.hash.update(&data[..n]);
            self.fp += n as u64;
            self.bp += n as u64;
            data = &data[n..];
        }
        Ok(original)
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Some(f) = self.file.as_mut() {
            f.flush()?
        }
        Ok(())
    }
}

pub fn solid(
    a: &mut EggArchive<egg_volumes::Reader>,
    chosen: &[EggFileEntry],
    req: &Request,
    extract: bool,
) -> Result<()> {
    solid_output(a, chosen, req, extract, false).map(|_| ())
}

/// Decode the complete bounded shared stream; only the chosen text is retained.
pub fn preview_solid_text(
    a: &mut EggArchive<egg_volumes::Reader>,
    chosen: &[EggFileEntry],
    req: &Request,
) -> Result<Value> {
    ensure!(!a.entries.iter().any(|e| e.encrypt_info.is_some()),
        "EGG 加密固实文本暂不支持预览，请先提取后打开");
    let blocks = a.entries.iter().flat_map(|e| &e.blocks).collect::<Vec<_>>();
    let packed = blocks.iter().map(|b| b.compressed_size as u64).sum::<u64>();
    let expanded = blocks.iter().map(|b| b.uncompressed_size as u64).sum::<u64>();
    let files = a.entries.iter().filter(|e| !e.is_directory()).map(|e| e.uncompressed_size).sum::<u64>();
    ensure!(packed <= MAX_TEXT_INPUT && expanded <= MAX_TEXT_INPUT && files <= MAX_TEXT_INPUT,
        "EGG 固实文本预览的完整共享压缩数据和解码总量须分别不超过 64 MiB，请先提取后打开");
    ensure!(blocks.iter().all(|b| matches!(b.compression_method, CompressionMethod::Deflate | CompressionMethod::Lzma)),
        "EGG 固实文本预览目前支持 Deflate / LZMA，请先提取后打开");
    validate(a, chosen)?;
    let (bytes, read) = solid_output(a, chosen, req, false, true)?;
    ensure!(read == packed && bytes.len() as u64 == chosen[0].uncompressed_size,
        "[archive-inspection:damaged] EGG 固实文本共享数据未完整读取或所选大小不符");
    Ok(json!({"bytes": bytes, "totalSize": chosen[0].uncompressed_size, "inputBytes": read}))
}

fn solid_output(
    a: &mut EggArchive<egg_volumes::Reader>,
    chosen: &[EggFileEntry],
    req: &Request,
    extract: bool,
    capture: bool,
) -> Result<(Vec<u8>, u64)> {
    let all = a.entries.clone();
    let encrypted = all.iter().any(|e| e.encrypt_info.is_some());
    let output = if extract {
        Some(req.output.as_deref().context("Missing output")?)
    } else {
        None
    };
    for e in chosen.iter().filter(|e| e.is_directory()) {
        if let Some(root) = output {
            fs::create_dir_all(root.join(safe_name(&e.file_name)?))?;
        }
    }
    let blocks = all.iter().flat_map(|e| e.blocks.iter()).collect::<Vec<_>>();
    let method = blocks
        .first()
        .map(|b| b.compression_method)
        .unwrap_or(CompressionMethod::Store);
    let packed = blocks.iter().map(|b| b.compressed_size as u64).sum();
    let expanded = blocks.iter().map(|b| b.uncompressed_size as u64).sum();
    let segments = all
        .iter()
        .enumerate()
        .flat_map(|(i, e)| e.blocks.iter().cloned().map(move |b| (i, b)))
        .collect();
    let mut input = Segments {
        a,
        blocks: segments,
        index: 0,
        used: 0,
        owner: None,
        crypto: None,
        password: req.password.as_deref(),
        encoding: req.encoding.as_deref(),
        error: None,
        read: 0,
    };
    let mut out = SolidSink {
        files: all.iter().filter(|e| !e.is_directory()).collect(),
        blocks,
        selected: chosen.iter().map(|e| e.file_name.clone()).collect(),
        output,
        fi: 0,
        fp: 0,
        bi: 0,
        bp: 0,
        hash: crc32fast::Hasher::new(),
        file: None,
        preserve: req.preserve_timestamps,
        memory: capture.then(|| Vec::with_capacity(chosen[0].uncompressed_size as usize)),
    };
    let result = stream(&mut input, &mut out, method, packed, expanded, None);
    if let Some(error) = input.error.take() {
        return Err(error);
    }
    result.map_err(|e| map_error(e, encrypted))?;
    out.finish().map_err(|e| {
        let io = e.downcast_ref::<io::Error>().is_some_and(|e| {
            !matches!(
                e.kind(),
                io::ErrorKind::UnexpectedEof | io::ErrorKind::InvalidData
            )
        });
        anyhow::anyhow!(
            "{} EGG 固实流处理失败：{e:#}",
            if io {
                "[archive-inspection:io] [egg-io]"
            } else if encrypted {
                "[archive-inspection:ambiguous-data]"
            } else {
                "[archive-inspection:damaged]"
            }
        )
    })?;
    Ok((out.memory.unwrap_or_default(), input.read))
}

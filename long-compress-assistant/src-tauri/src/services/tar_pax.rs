//! Bounded global/local PAX state shared by complete TAR readers and prefix rescue.
use std::{collections::BTreeMap, io::{self, Cursor, Read}};

pub(crate) type Fields = Vec<(String, String)>;
pub(crate) const MAX_METADATA: usize = 1024 * 1024;
const MAX_LOCAL: usize = 8 * MAX_METADATA;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("[archive-inspection:damaged] TAR PAX {message}"))
}
fn unsupported(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, format!("[archive-inspection:unsupported] TAR PAX {message}"))
}
pub(crate) fn parse(bytes: &[u8]) -> io::Result<Fields> {
    let mut at = 0usize;
    let mut result = Vec::new();
    while at < bytes.len() {
        let space = bytes[at..].iter().position(|b| *b == b' ').ok_or_else(|| invalid("记录长度不完整"))? + at;
        let digits = &bytes[at..space];
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) { return Err(invalid("记录长度无效")); }
        let len: usize = std::str::from_utf8(digits).unwrap().parse().map_err(|_| invalid("记录长度溢出"))?;
        let end = at.checked_add(len).ok_or_else(|| invalid("记录长度溢出"))?;
        if end <= space + 2 || end > bytes.len() || bytes[end-1] != b'\n' { return Err(invalid("记录被截断")); }
        let pair = &bytes[space+1..end-1];
        let eq = pair.iter().position(|b| *b == b'=').ok_or_else(|| invalid("记录缺少属性值"))?;
        let key = std::str::from_utf8(&pair[..eq]).map_err(|_| invalid("属性名不是 UTF-8"))?;
        let value = std::str::from_utf8(&pair[eq+1..]).map_err(|_| invalid("属性值不是 UTF-8"))?;
        if key.is_empty() || key.contains(['\0','\n']) || value.contains('\0') { return Err(invalid("属性含无效字符")); }
        if result.iter().any(|(k, _)| k == key) && !matches!(key, "GNU.sparse.offset" | "GNU.sparse.numbytes") {
            return Err(invalid("属性重复"));
        }
        result.push((key.to_owned(), value.to_owned()));
        at = end;
    }
    Ok(result)
}
pub(crate) fn size(fields: &Fields) -> io::Result<Option<u64>> {
    fields.iter().rev().find(|(k,_)| k == "size").map(|(_,v)| {
        if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) { return Err(invalid("大小无效")); }
        v.parse().map_err(|_| invalid("大小溢出"))
    }).transpose()
}
pub(crate) fn path(fields: &Fields) -> Option<&str> {
    fields.iter().rev().find(|(k,_)| k == "path").map(|(_,v)| v.as_str())
}
/// A mixed GNU/PAX name is supported only when it is unambiguous. Different
/// readers give conflicting names different precedence; never guess a path.
pub(crate) fn check_mixed_path(gnu: &[u8], local: &Fields) -> io::Result<()> {
    if let Some(path) = path(local) {
        if path.as_bytes() != gnu {
            return Err(unsupported("GNU 长名称与局部 PAX 路径不一致，已停止在此处；请向提供者索取名称一致的原包"));
        }
    }
    Ok(())
}
pub(crate) fn common(fields: &Fields) -> io::Result<()> {
    for (key,value) in fields {
        if value.contains('\n') { return Err(unsupported("含换行的属性值暂不支持")); }
        if !matches!(key.as_str(), "path"|"size"|"mtime"|"atime"|"ctime"|"uid"|"gid"|"uname"|"gname"|"comment") {
            return Err(unsupported(&format!("暂不支持属性 {key}，已停止在此处")));
        }
        if value.is_empty() { continue; }
        if matches!(key.as_str(), "size"|"uid"|"gid") && (!value.bytes().all(|b| b.is_ascii_digit()) || value.parse::<u64>().is_err()) {
            return Err(invalid("数值属性无效"));
        }
        if matches!(key.as_str(), "mtime"|"atime"|"ctime") && value.parse::<f64>().map_or(true, |v| !v.is_finite() || v.abs() > 1e12) {
            return Err(invalid("时间属性无效"));
        }
    }
    Ok(())
}
#[derive(Default)]
pub(crate) struct State { globals: BTreeMap<String,String> }
impl State {
    pub(crate) fn update(&mut self, fields: Fields) -> io::Result<()> {
        common(&fields)?;
        let mut next = self.globals.clone();
        for (key,value) in fields { if value.is_empty() { next.remove(&key); } else { next.insert(key,value); } }
        if next.iter().map(|(k,v)| k.len()+v.len()).sum::<usize>() > MAX_METADATA { return Err(unsupported("全局元数据超过支持范围")); }
        self.globals = next;
        Ok(())
    }
    pub(crate) fn effective(&self, local: &Fields) -> Fields {
        let mut merged: Fields = self.globals.iter().filter(|(k,_)| !local.iter().any(|(key,_)| key == *k))
            .map(|(k,v)| (k.clone(),v.clone())).collect();
        merged.extend(local.iter().filter(|(_,v)| !v.is_empty()).cloned());
        merged
    }
}
fn encode(fields: &Fields) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for (key,value) in fields {
        // tar-rs's PAX view is line based. Refuse ambiguous values rather than
        // emitting an extension its consumer would interpret differently.
        if value.contains('\n') { return Err(unsupported("含换行的属性值暂不支持")); }
        let body = format!(" {key}={value}\n");
        let mut len = body.len()+1;
        loop { let next = body.len()+len.to_string().len(); if next == len { break; } len = next; }
        bytes.extend_from_slice(format!("{len}{body}").as_bytes());
        if bytes.len() > MAX_LOCAL { return Err(unsupported("条目元数据超过支持范围")); }
    }
    Ok(bytes)
}

/// Normalise g/x into one effective x before each real entry. File bytes are
/// streamed unchanged; control records always retain their own physical size.
/// GNU long names are canonicalised after physical validation; long links and
/// old sparse extension blocks retain their original layout.
pub(crate) struct Reader<R> {
    input: R, state: State, local: Option<Fields>, pending: Cursor<Vec<u8>>,
    remaining: u64, sparse_extensions: bool, ended: bool,
    long_name: Option<Vec<u8>>,
}
impl<R: Read> Reader<R> {
    pub(crate) fn new(input: R) -> Self {
        Self { input, state: State::default(), local: None, pending: Cursor::new(Vec::new()), remaining: 0, sparse_extensions: false, ended: false, long_name: None }
    }
    fn prepare(&mut self) -> io::Result<bool> {
        let mut controls = 0;
        loop {
            let mut block = [0u8;512];
            let n = self.input.read(&mut block[..1])?;
            if n == 0 {
                if self.local.is_some() || self.long_name.is_some() { return Err(invalid("名称元数据后缺少对应条目")); }
                return Ok(false);
            }
            self.input.read_exact(&mut block[1..])?;
            if block.iter().all(|b| *b == 0) {
                if self.local.is_some() || self.long_name.is_some() { return Err(invalid("名称元数据后缺少对应条目")); }
                self.ended = true;
                self.pending = Cursor::new(block.to_vec());
                return Ok(true);
            }
            let mut header = tar::Header::from_byte_slice(&block).clone();
            let checksum: u32 = block.iter().enumerate().map(|(i,b)| if (148..156).contains(&i) {32} else {*b as u32}).sum();
            if header.cksum()? != checksum { return Err(invalid("文件头校验失败")); }
            let kind = header.entry_type();
            let physical = header.entry_size()?;
            if kind.is_pax_global_extensions() || kind.is_pax_local_extensions() {
                controls += 1;
                if controls > 1024 || physical > if kind.is_pax_global_extensions() {MAX_METADATA as u64} else {MAX_LOCAL as u64} { return Err(unsupported("元数据超过支持范围")); }
                let mut bytes = vec![0; physical as usize];
                self.input.read_exact(&mut bytes)?;
                let mut padding = [0;512];
                let pad = ((512-physical%512)%512) as usize;
                self.input.read_exact(&mut padding[..pad])?;
                if padding[..pad].iter().any(|b| *b != 0) { return Err(invalid("元数据填充无效")); }
                let fields = parse(&bytes)?;
                if kind.is_pax_global_extensions() { self.state.update(fields)?; }
                else {
                    if self.local.is_some() { return Err(invalid("局部元数据重复")); }
                    self.local = Some(fields);
                }
                continue;
            }
            if kind.is_gnu_longname() {
                if self.long_name.is_some() { return Err(invalid("GNU 长名称记录重复")); }
                if physical > MAX_METADATA as u64 { return Err(unsupported("GNU 长名称超过支持范围")); }
                let mut bytes = vec![0; padded(physical)? as usize];
                self.input.read_exact(&mut bytes)?;
                let payload = &bytes[..physical as usize];
                let end = payload.iter().position(|b| *b == 0).ok_or_else(|| invalid("GNU 长名称缺少结束标记"))?;
                if end == 0 || bytes[end..].iter().any(|b| *b != 0) { return Err(invalid("GNU 长名称或填充无效")); }
                self.long_name = Some(payload[..end].to_vec());
                // Some producers include zero padding in L's declared payload.
                // Consume that physical record, then emit one NUL terminator so
                // tar-rs and prefix recovery expose the same actual filename.
                header.set_size((end+1) as u64); header.set_cksum();
                let mut record = header.as_bytes().to_vec();
                record.extend_from_slice(&payload[..end+1]);
                record.resize(512+padded((end+1) as u64)? as usize,0);
                self.pending = Cursor::new(record);
                return Ok(true);
            }
            if kind.is_gnu_longlink() {
                self.remaining = padded(physical)?;
                self.pending = Cursor::new(block.to_vec());
                return Ok(true);
            }
            let local = self.local.take().unwrap_or_default();
            if let Some(name) = self.long_name.take() { check_mixed_path(&name, &local)?; }
            let fields = self.state.effective(&local);
            let stored = size(&fields)?.unwrap_or(physical);
            if stored > super::extraction_transaction::MAX_EXTRACTED_BYTES { return Err(unsupported("条目大小超过支持范围")); }
            let mut prefix = Vec::new();
            if !fields.is_empty() {
                let bytes = encode(&fields)?;
                let mut ext = tar::Header::new_ustar();
                ext.set_path("PaxHeaders/entry")?; ext.set_entry_type(tar::EntryType::XHeader);
                ext.set_size(bytes.len() as u64); ext.set_mode(0o644); ext.set_cksum();
                prefix.extend_from_slice(ext.as_bytes()); prefix.extend_from_slice(&bytes);
                prefix.resize(prefix.len()+((512-bytes.len()%512)%512),0);
                // Keep header-only consumers in step with ordinary whole-second
                // PAX timestamps; LogicalEntry retains signed/fractional times.
                if let Some((_,v)) = fields.iter().find(|(k,_)| k == "mtime") {
                    let t: f64 = v.parse().map_err(|_| invalid("mtime 无效"))?;
                    if t.is_finite() && t >= 0.0 && t <= 1e12 { header.set_mtime(t.floor() as u64); header.set_cksum(); }
                }
            }
            prefix.extend_from_slice(header.as_bytes());
            self.sparse_extensions = header.as_gnu().map_or(false, |g| kind.is_gnu_sparse() && g.is_extended());
            self.remaining = padded(stored)?;
            self.pending = Cursor::new(prefix);
            return Ok(true);
        }
    }
}
fn padded(size: u64) -> io::Result<u64> { size.checked_add(511).map(|v| v & !511).ok_or_else(|| invalid("大小溢出")) }
impl<R: Read> Read for Reader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() { return Ok(0); }
        loop {
            let n = self.pending.read(buffer)?;
            if n != 0 { return Ok(n); }
            if self.ended { return self.input.read(buffer); }
            if self.sparse_extensions {
                let mut ext = tar::GnuExtSparseHeader::new();
                self.input.read_exact(ext.as_mut_bytes())?;
                self.sparse_extensions = ext.is_extended();
                self.pending = Cursor::new(ext.as_bytes().to_vec());
                continue;
            }
            if self.remaining != 0 {
                let count = buffer.len().min(self.remaining.min(usize::MAX as u64) as usize);
                let n = self.input.read(&mut buffer[..count])?;
                if n == 0 { return Err(invalid("条目内容或填充被截断")); }
                self.remaining -= n as u64;
                return Ok(n);
            }
            if !self.prepare()? { return Ok(0); }
        }
    }
}

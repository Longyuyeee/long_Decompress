//! ALZ native framing and source identity, shared with the isolated decoder.
use anyhow::{ensure, Context, Result};
use std::{collections::BTreeMap, fs::File, io::{Read, Seek, SeekFrom}, path::{Path, PathBuf}};

#[derive(Debug, Clone)]
pub struct Group {
    pub first: PathBuf,
    pub parts: Vec<PathBuf>,
    pub missing: Vec<PathBuf>,
    pub total: usize,
    pub bytes: u64,
    pub virtual_bytes: u64,
    pub stem: String,
}

fn index(extension: &str) -> Option<usize> {
    let e = extension.as_bytes();
    if extension.eq_ignore_ascii_case("alz") { return Some(0); }
    if e.len() == 3 && e[0].is_ascii_alphabetic() && e[1..].iter().all(u8::is_ascii_digit) {
        return Some(1 + usize::from(e[0].to_ascii_lowercase() - b'a') * 100
            + usize::from(e[1] - b'0') * 10 + usize::from(e[2] - b'0'));
    }
    None
}

pub fn is_candidate(path: &Path) -> bool {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let mut header = [0; 4];
    File::open(path).and_then(|mut f| f.read_exact(&mut header)).is_ok() && &header == b"ALZ\x01"
        || index(ext).is_some_and(|i| i > 0) && path.with_extension("alz").is_file()
}

fn member_name(stem: &str, number: usize) -> String {
    if number == 0 { format!("{stem}.alz") }
    else { format!("{stem}.{}{:02}", (b'a' + ((number - 1) / 100) as u8) as char, (number - 1) % 100) }
}

pub fn inspect(path: &Path) -> Result<Option<Group>> {
    if !is_candidate(path) { return Ok(None); }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).context("ALZ 文件名称无效")?.to_string();
    let mut members = BTreeMap::new();
    let renamed_single = path.extension().and_then(|s| s.to_str()).and_then(index).is_none();
    if renamed_single { members.insert(0, path.to_path_buf()); }
    for entry in std::fs::read_dir(parent)? {
        if renamed_single { break; }
        let entry = entry?;
        let p = entry.path();
        if !p.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case(&stem)) { continue; }
        let Some(number) = p.extension().and_then(|s| s.to_str()).and_then(|s| index(&s.to_ascii_lowercase())) else { continue; };
        ensure!(number < 1000, "[archive-inspection:unsupported] ALZ 原生卷超过本版支持的 1000 卷范围");
        ensure!(entry.file_type()?.is_file(), "[archive-inspection:damaged] ALZ 卷不是普通文件：{}", p.display());
        ensure!(members.insert(number, p).is_none(), "[archive-inspection:damaged] ALZ 卷号重复，请检查同目录文件");
    }
    let highest = *members.keys().next_back().context("没有找到 ALZ 原卷")?;
    let mut missing = (0..=highest).filter(|i| !members.contains_key(i))
        .map(|i| parent.join(member_name(&stem, i))).collect::<Vec<_>>();
    let mut bytes = 0u64;
    let mut virtual_bytes = 0u64;
    let mut version = None;
    let mut needs_next = false;
    for (&number, p) in &members {
        let mut file = File::open(p).with_context(|| format!("[alz-io] 无法打开 ALZ 卷：{}", p.display()))?;
        let size = file.metadata()?.len();
        ensure!(size >= 24, "[archive-inspection:damaged] ALZ 卷头或卷尾不完整：{}", p.display());
        let mut head = [0u8; 8]; file.read_exact(&mut head)?;
        ensure!(&head[..4] == b"ALZ\x01", "[archive-inspection:damaged] ALZ 卷头损坏或混入其他文件：{}", p.display());
        ensure!(u16::from_le_bytes([head[6], head[7]]) as usize == number,
            "[archive-inspection:damaged] ALZ 卷内编号与文件名不符：{}", p.display());
        let v = [head[4], head[5]];
        if let Some(old) = version { ensure!(old == v, "[archive-inspection:damaged] ALZ 卷版本不一致，可能混入其他卷组"); }
        version = Some(v);
        file.seek(SeekFrom::End(-16))?;
        let mut tail = [0u8; 16]; file.read_exact(&mut tail)?;
        ensure!(&tail[..4] == b"CLZ\x01" && matches!(&tail[12..], b"CLZ\x02" | b"CLZ\x03"),
            "[archive-inspection:damaged] ALZ 卷尾损坏或下载不完整：{}", p.display());
        let more = &tail[12..] == b"CLZ\x03";
        ensure!(number == highest || more, "[archive-inspection:damaged] ALZ 已结束的卷后还有同名数据卷，可能混卷：{}", p.display());
        if number == highest { needs_next = more; }
        bytes = bytes.checked_add(size).context("ALZ 卷组大小溢出")?;
        virtual_bytes = virtual_bytes.checked_add(size - if number == 0 { 0 } else { 8 } - if number < highest { 16 } else { 0 }).context("ALZ 大小溢出")?;
    }
    if needs_next { missing.push(parent.join(member_name(&stem, highest + 1))); }
    let first = members.get(&0).cloned().unwrap_or_else(|| parent.join(member_name(&stem, 0)));
    Ok(Some(Group { first, parts: members.into_values().collect(), missing,
        total: highest + 1 + usize::from(needs_next), bytes, virtual_bytes, stem }))
}

pub fn complete(path: &Path) -> Result<Group> {
    let group = inspect(path)?.context("[archive-inspection:damaged] ALZ 文件头无效或不完整，不是可读取的 ALZ 原卷")?;
    ensure!(group.missing.is_empty(), "[archive-inspection:incomplete] 缺少 ALZ 分卷：{}；请补齐原卷后重试，不必反复尝试密码",
        group.missing.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy()).collect::<Vec<_>>().join("、"));
    Ok(group)
}

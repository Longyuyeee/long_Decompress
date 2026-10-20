//! Export completed observations without reopening or hashing source files.
use super::file_integrity_service::ChecksumAlgorithm;
use serde::Deserialize;
use std::{collections::HashSet, io::Write, path::{Component, Path}};

#[derive(Debug, Deserialize)]
pub struct ChecksumExportEntry {
    pub path: String,
    pub checksum: String,
    pub status: String,
}

pub fn export(path: &Path, results: &[ChecksumExportEntry], algorithm: ChecksumAlgorithm) -> Result<(), String> {
    let (extension, length) = match algorithm {
        ChecksumAlgorithm::CRC32 => ("sfv", 8),
        ChecksumAlgorithm::MD5 => ("md5", 32),
        ChecksumAlgorithm::SHA256 => ("sha256", 64),
    };
    if !path.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case(extension)) {
        return Err(format!("请将清单保存为 .{extension} 文件，以便正确核对。"));
    }
    let parent = path.parent().ok_or("请选择清单保存目录")?;
    let base = parent.canonicalize().map_err(|e| format!("无法访问清单保存目录：{e}"))?;
    let destination = base.join(path.file_name().ok_or("请填写清单文件名")?);
    let mut lines = String::new();
    let mut seen = HashSet::new();
    for entry in results.iter().filter(|entry| entry.status == "success") {
        if entry.checksum.len() != length || !entry.checksum.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("摘要不完整，请重新计算此文件：{}", entry.path));
        }
        let source = Path::new(&entry.path);
        if !source.is_absolute() { return Err(format!("来源不是完整路径，请重新选择文件：{}", entry.path)); }
        // Canonicalize directories only. A completed observation can be exported even
        // if its file was subsequently removed; verification will then report missing.
        let directory = source.parent().ok_or("来源缺少目录")?.canonicalize()
            .map_err(|e| format!("无法访问来源目录，请恢复目录后重试：{}（{e}）", entry.path))?;
        let source = directory.join(source.file_name().ok_or("来源缺少文件名")?);
        let base_components: Vec<_> = base.components().collect();
        let source_components: Vec<_> = source.components().collect();
        let equal = |a: Component<'_>, b: Component<'_>| {
            if cfg!(windows) { a.as_os_str().to_string_lossy().eq_ignore_ascii_case(&b.as_os_str().to_string_lossy()) }
            else { a == b }
        };
        if source_components.len() <= base_components.len() || !base_components.iter().zip(&source_components).all(|(&a, &b)| equal(a, b)) {
            return Err(format!("文件不在清单保存目录内：{}。请把清单保存到所有来源的共同上级目录；跨磁盘时请先把文件放到同一磁盘，或分开生成清单。", entry.path));
        }
        if equal_path(&source, &destination) { return Err("清单不能覆盖参与计算的来源文件，请更换清单文件名。".into()); }
        let relative: std::path::PathBuf = source_components[base_components.len()..].iter().collect();
        let name = relative.to_str().ok_or("文件名不能表示为 UTF-8，请重命名后重试")?.replace('\\', "/");
        if name.trim() != name || name.chars().any(char::is_control) || (extension == "sfv" && name.starts_with([';', '#'])) {
            return Err(format!("此文件名无法写入可核对的 {extension} 清单，请重命名或将清单保存在上级目录：{name}"));
        }
        let key = if cfg!(windows) { name.to_lowercase() } else { name.clone() };
        if !seen.insert(key) { continue; }
        if extension == "sfv" { lines.push_str(&format!("{name} {}\n", entry.checksum)); }
        else { lines.push_str(&format!("{} *{name}\n", entry.checksum)); }
    }
    if lines.is_empty() { return Err("没有完整成功的摘要可导出，请先完成计算。".into()); }
    // Validate every row before touching the existing destination. A failed write
    // leaves the original manifest intact, and no partial manifest is published.
    let mut temporary = tempfile::NamedTempFile::new_in(&base).map_err(|e| format!("无法创建清单：{e}"))?;
    temporary.write_all(lines.as_bytes()).map_err(|e| format!("清单写入失败：{e}"))?;
    temporary.as_file().sync_all().map_err(|e| format!("清单保存失败：{e}"))?;
    temporary.persist(&destination).map_err(|e| format!("无法保存清单，请检查文件占用或更换位置：{}", e.error))?;
    Ok(())
}

fn equal_path(a: &Path, b: &Path) -> bool {
    if cfg!(windows) { a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy()) } else { a == b }
}

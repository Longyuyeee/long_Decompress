//! Metadata candidates and scoped retrieval; publication uses the original extraction transaction.
use super::{
    extraction_transaction as publication, task_template::metadata_is_link_or_reparse_point,
    workflow_triggers::display_path,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
const MAX_ENTRIES: usize = 50_000;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub path: String,
    pub kind: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub encrypted: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    pub directory: bool,
    pub size: u64,
    pub modified: Option<String>,
    pub selectable: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub encrypted: bool,
    #[serde(skip)]
    pub fingerprint: String,
    #[serde(skip)]
    pub raw: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub source: SourceRef,
    pub name: String,
    pub entries: Vec<Entry>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub source: SourceRef,
    pub paths: Vec<String>,
    pub keep_structure: bool,
    pub conflict_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip_password_encoding: Option<super::zip_password::ZipPasswordEncoding>,
}
impl Selection {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            matches!(self.source.kind.as_str(), "directory" | "zip")
                && !self.source.path.is_empty(),
            "对比来源无效，请返回资料对比重新选择"
        );
        ensure!(
            self.source.revision.len() == 64
                && self.source.revision.bytes().all(|b| b.is_ascii_hexdigit()),
            "来源版本缺失，请重新读取对比"
        );
        ensure!(
            !self.paths.is_empty()
                && self.paths.len() <= 2000
                && serde_json::to_string(&self.paths)?.len() <= 60000,
            "每批请选择最多 2000 项，并缩小过长路径范围"
        );
        ensure!(
            matches!(self.conflict_policy.as_str(), "rename" | "skip"),
            "请选择保留两份或跳过同名文件"
        );
        for path in &self.paths {
            relative(path)?;
        }
        Ok(())
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub entries: usize,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub path: String,
}
fn check(flag: &Arc<AtomicBool>) -> Result<()> {
    super::task_control::wait_if_paused(flag);
    ensure!(!flag.load(Ordering::SeqCst), "资料对比或取回已停止");
    Ok(())
}
fn relative(value: &str) -> Result<PathBuf> {
    ensure!(
        !value.is_empty()
            && !value.contains('\\')
            && !value.contains(':')
            && !value.starts_with('/'),
        "来源中含无法定位的路径：{value}"
    );
    let path = PathBuf::from(value);
    ensure!(
        path.components().all(|c| matches!(c, Component::Normal(_)))
            && !value
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == ".."),
        "来源路径无法唯一定位：{value}"
    );
    Ok(path)
}
fn metadata(path: &Path) -> Result<fs::Metadata> {
    let meta = fs::symlink_metadata(path)
        .with_context(|| format!("来源缺失或无法读取：{}", path.display()))?;
    ensure!(
        !metadata_is_link_or_reparse_point(&meta) && (meta.is_file() || meta.is_dir()),
        "来源包含无法读取的链接或特殊条目：{}",
        path.display()
    );
    Ok(meta)
}
pub(super) fn identity(meta: &fs::Metadata) -> Result<String> {
    Ok(format!(
        "{}:{}",
        meta.len(),
        meta.modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}
fn finish(path: &Path, kind: &str, map: BTreeMap<String, Entry>, base: &str) -> Result<Source> {
    let mut hash = blake3::Hasher::new();
    hash.update(base.as_bytes());
    let mut names = BTreeSet::new();
    for (path, entry) in &map {
        ensure!(
            names.insert(path.to_lowercase()),
            "来源存在仅大小写不同的同名路径，请使用原浏览器处理：{path}"
        );
        hash.update(path.as_bytes());
        hash.update(&[0]);
        hash.update(entry.fingerprint.as_bytes());
        hash.update(&[0]);
    }
    let entries = map
        .values()
        .map(|entry| {
            let mut e = entry.clone();
            let prefix = e.path.clone() + "/";
            e.selectable = !e.directory
                || !map
                    .range(prefix.clone()..)
                    .next()
                    .is_some_and(|(p, _)| p.starts_with(&prefix));
            e
        })
        .collect();
    Ok(Source {
        source: SourceRef {
            path: display_path(path),
            kind: kind.into(),
            revision: hash.finalize().to_hex().to_string(),
            encrypted: map.values().any(|entry| entry.encrypted),
        },
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        entries,
    })
}
fn insert(map: &mut BTreeMap<String, Entry>, entry: Entry) -> Result<()> {
    let parts: Vec<_> = entry.path.split('/').collect();
    for count in 1..parts.len() {
        let parent = parts[..count].join("/");
        if let Some(old) = map.get(&parent) {
            ensure!(old.directory, "来源的文件与目录路径冲突：{parent}");
        } else {
            map.insert(
                parent.clone(),
                Entry {
                    path: parent,
                    directory: true,
                    size: 0,
                    modified: None,
                    selectable: false,
                    encrypted: false,
                    fingerprint: "directory".into(),
                    raw: String::new(),
                },
            );
        }
    }
    if let Some(old) = map.get(&entry.path) {
        ensure!(
            old.directory && entry.directory,
            "来源有重复路径，无法唯一对比：{}",
            entry.path
        );
    }
    map.insert(entry.path.clone(), entry);
    ensure!(
        map.len() <= MAX_ENTRIES,
        "单侧超过 5 万条，请选择更小的文件夹或 ZIP 后再对比"
    );
    Ok(())
}
pub fn read(
    path: &Path,
    flag: Arc<AtomicBool>,
    mut progress: impl FnMut(Progress),
) -> Result<Source> {
    check(&flag)?;
    metadata(path)?;
    let path = fs::canonicalize(path)?;
    if path.is_dir() {
        let first = read_folder(&path, &flag, &mut progress)?;
        check(&flag)?;
        let second = read_folder(&path, &flag, &mut |_| {})?;
        ensure!(
            first.source.revision == second.source.revision,
            "读取期间来源有变化，请重新读取这一侧"
        );
        Ok(first)
    } else {
        let path = super::split_archive_detector::SplitArchiveDetector::complete_source(&path)?;
        read_zip(&fs::canonicalize(path)?, &flag, &mut progress)
    }
}
fn read_folder(
    root: &Path,
    flag: &Arc<AtomicBool>,
    progress: &mut impl FnMut(Progress),
) -> Result<Source> {
    let mut map = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(current) = pending.pop() {
        check(flag)?;
        for item in fs::read_dir(&current)
            .with_context(|| format!("无法完整读取目录：{}", current.display()))?
        {
            check(flag)?;
            let path = item?.path();
            let meta = metadata(&path)?;
            let name = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            relative(&name)?;
            let modified = if meta.is_file() {
                Some(
                    chrono::DateTime::<chrono::Local>::from(meta.modified()?)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string(),
                )
            } else {
                None
            };
            insert(
                &mut map,
                Entry {
                    path: name.clone(),
                    directory: meta.is_dir(),
                    size: if meta.is_file() { meta.len() } else { 0 },
                    modified,
                    selectable: false,
                    encrypted: false,
                    fingerprint: if meta.is_dir() {
                        "directory".into()
                    } else {
                        identity(&meta)?
                    },
                    raw: String::new(),
                },
            )?;
            if meta.is_dir() {
                pending.push(path);
            }
            if map.len() % 500 == 0 {
                progress(Progress {
                    phase: "reading".into(),
                    entries: map.len(),
                    processed_bytes: 0,
                    total_bytes: 0,
                    path: name,
                });
            }
        }
    }
    finish(root, "directory", map, "")
}
fn read_zip(
    path: &Path,
    flag: &Arc<AtomicBool>,
    progress: &mut impl FnMut(Progress),
) -> Result<Source> {
    let before = source_identity(path)?;
    let mut file = super::numeric_volume_reader::ZipInputReader::open(path)?;
    let count = super::zip_edit::declared_count_reader(&mut file)
        .context("请使用完整可读取的 ZIP 来源；缺卷请补齐后重新读取")?;
    ensure!(
        count <= MAX_ENTRIES as u64,
        "单侧超过 5 万条，请缩小 ZIP 范围"
    );
    let mut archive = zip_aes::ZipArchive::new(file)?;
    ensure!(
        count == archive.len() as u64,
        "ZIP 有重复目录条目，无法完整唯一对比，请使用原浏览器"
    );
    let mut map = BTreeMap::new();
    let mut explicit = BTreeSet::new();
    for index in 0..archive.len() {
        check(flag)?;
        let entry = archive.by_index_raw(index)?;
        ensure!(
            matches!(
                entry.compression(),
                zip_aes::CompressionMethod::Stored | zip_aes::CompressionMethod::Deflated | zip_aes::CompressionMethod::Bzip2 | zip_aes::CompressionMethod::Lzma | zip_aes::CompressionMethod::Zstd
            ),
            "资料对比支持 Store/Deflate/Bzip2/LZMA/Zstandard ZIP；条目 {} 使用 {:?}，请使用原浏览器浏览/提取", entry.name(), entry.compression()
        );
        ensure!(
            entry
                .unix_mode()
                .is_none_or(|mode| mode & 0o170000 != 0o120000),
            "ZIP 包含链接条目，请使用原浏览器处理"
        );
        let raw = entry.name().to_string();
        let name = raw.replace('\\', "/").trim_end_matches('/').to_string();
        relative(&name)?;
        ensure!(explicit.insert(name.clone()), "ZIP 存在重复条目：{name}");
        let modified = entry.last_modified().map(|t| {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                t.year(),
                t.month(),
                t.day(),
                t.hour(),
                t.minute(),
                t.second()
            )
        });
        let fingerprint = if entry.is_dir() {
            "directory".into()
        } else {
            format!("{}:{modified:?}:{}", entry.size(), entry.crc32())
        };
        insert(
            &mut map,
            Entry {
                path: name.clone(),
                directory: entry.is_dir(),
                size: entry.size(),
                modified,
                selectable: false,
                encrypted: entry.encrypted(),
                fingerprint,
                raw,
            },
        )?;
        if index % 500 == 0 {
            progress(Progress {
                phase: "reading".into(),
                entries: map.len(),
                processed_bytes: 0,
                total_bytes: 0,
                path: name,
            });
        }
    }
    ensure!(
        before == source_identity(path)?,
        "读取期间 ZIP 有变化，请重新读取这一侧"
    );
    finish(path, "zip", map, &before)
}
pub(super) fn source_identity(path: &Path) -> Result<String> {
    if let Some(group) =
        super::split_archive_detector::SplitArchiveDetector::detect_split_archive(path)?
    {
        ensure!(
            group.is_complete,
            "缺少分卷：{}；请补齐后重新读取",
            group.missing_part_names()
        );
        let parts = group
            .parts
            .iter()
            .map(|part| Ok((display_path(part), identity(&metadata(part)?)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(serde_json::to_string(&parts)?)
    } else {
        identity(&metadata(path)?)
    }
}
pub(super) fn open_zip(
    source: &Source,
) -> Result<zip_aes::ZipArchive<super::numeric_volume_reader::ZipInputReader>> {
    Ok(zip_aes::ZipArchive::new(
        super::numeric_volume_reader::ZipInputReader::open(Path::new(&source.source.path))?,
    )?)
}
pub(super) fn open_entry<'a>(
    archive: &'a mut zip_aes::ZipArchive<super::numeric_volume_reader::ZipInputReader>,
    raw: &str,
    password: Option<&str>,
    encoding: Option<super::zip_password::ZipPasswordEncoding>,
) -> Result<zip_aes::read::ZipFile<'a, super::numeric_volume_reader::ZipInputReader>> {
    let index = archive
        .index_for_name(raw)
        .context("所选文件不在当前 ZIP 中")?;
    let encrypted = archive.by_index_raw(index)?.encrypted();
    if encrypted {
        ensure!(
            password.is_some_and(|p| !p.is_empty()),
            "需要本次 ZIP 密码，请在来源密码设置中填写"
        );
    }
    super::zip_password::open_entry_with_encoding(archive, index, password.unwrap_or(""), encoding)
        .map_err(|e| {
            anyhow::anyhow!(e).context("ZIP 内容无法读取，请核对密码、密码编码和原卷完整性")
        })
}
pub fn validate_password(
    request: &Selection,
    source: &Source,
    password: Option<&str>,
) -> Result<()> {
    if source.source.kind != "zip" {
        return Ok(());
    }
    let mut archive = open_zip(source)?;
    for entry in source
        .entries
        .iter()
        .filter(|entry| entry.encrypted && request.paths.contains(&entry.path))
    {
        let mut reader = open_entry(
            &mut archive,
            &entry.raw,
            password,
            request.zip_password_encoding,
        )?;
        reader
            .read(&mut [0u8; 1])
            .context("无法读取所选 ZIP 内容，请核对密码和编码")?;
    }
    Ok(())
}
fn output_path(value: &Path) -> Result<PathBuf> {
    ensure!(value.is_absolute(), "请选择完整取回位置");
    publication::ensure_no_link_ancestors(value)?;
    let mut missing = Vec::new();
    let mut parent = value;
    while !parent.exists() {
        missing.push(parent.file_name().context("输出目录无效")?.to_os_string());
        parent = parent.parent().context("输出目录无效")?;
    }
    let mut result = fs::canonicalize(parent)?;
    for part in missing.into_iter().rev() {
        ensure!(part != "." && part != "..", "输出目录无效");
        result.push(part);
    }
    ensure!(!result.exists() || result.is_dir(), "取回位置不是文件夹");
    Ok(result)
}
pub fn validate_output(sources: &[SourceRef], output: &Path) -> Result<PathBuf> {
    let output = output_path(output)?;
    for source in sources {
        if source.kind == "directory" {
            let root =
                fs::canonicalize(&source.path).unwrap_or_else(|_| PathBuf::from(&source.path));
            ensure!(
                {
                    let a = super::workflow_triggers::key(&output)
                        .trim_end_matches('/')
                        .to_string();
                    let b = super::workflow_triggers::key(&root)
                        .trim_end_matches('/')
                        .to_string();
                    a != b && !a.starts_with(&(b.clone() + "/")) && !b.starts_with(&(a + "/"))
                },
                "取回位置不能与任意一侧的来源文件夹互相包含，请选择独立目录"
            );
        }
    }
    Ok(output)
}
pub fn validate(request: &Selection, output: &Path, flag: Arc<AtomicBool>) -> Result<Source> {
    request.validate()?;
    validate_output(std::slice::from_ref(&request.source), output)?;
    let source = read(Path::new(&request.source.path), flag, |_| {})?;
    ensure!(
        source.source.kind == request.source.kind
            && source.source.revision == request.source.revision,
        "来源已变化，请返回资料对比重新读取并勾选；不会改为整包取回"
    );
    for path in &request.paths {
        ensure!(
            source
                .entries
                .iter()
                .any(|e| &e.path == path && e.selectable),
            "所选位置不在当前可取回范围：{path}；非空文件夹请勾选其子文件"
        );
    }
    Ok(source)
}
fn next_name(root: &Path, path: &Path, policy: &str) -> Option<PathBuf> {
    let target = root.join(path);
    if !target.exists() {
        return Some(target);
    }
    if policy == "skip" {
        return None;
    }
    let parent = target.parent()?;
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|n| parent.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
}
pub fn retrieve(
    request: &Selection,
    output: &Path,
    flag: Arc<AtomicBool>,
    progress: impl FnMut(Progress),
) -> Result<(String, u64)> {
    retrieve_with_password(request, output, None, flag, progress)
}
pub fn retrieve_with_password(
    request: &Selection,
    output: &Path,
    password: Option<&str>,
    flag: Arc<AtomicBool>,
    mut progress: impl FnMut(Progress),
) -> Result<(String, u64)> {
    let source = validate(request, output, flag.clone())?;
    let output = validate_output(std::slice::from_ref(&request.source), output)?;
    let wanted: BTreeSet<_> = request.paths.iter().collect();
    let selected: Vec<_> = source
        .entries
        .iter()
        .filter(|e| wanted.contains(&e.path))
        .collect();
    let total = selected.iter().try_fold(0u64, |n, e| {
        n.checked_add(e.size).context("所选大小超出可处理范围")
    })?;
    publication::validate_disk_capacity(output.parent().unwrap_or(&output), total)?;
    let mut staging = publication::ExtractionStaging::create_for(&output)?;
    let mut zip = if source.source.kind == "zip" {
        Some(open_zip(&source)?)
    } else {
        None
    };
    let mut processed = 0;
    let mut gate = super::progress_gate::ProgressGate::new();
    for (index, entry) in selected.iter().enumerate() {
        check(&flag)?;
        let relative = relative(&entry.path)?;
        let name = if request.keep_structure {
            relative.clone()
        } else {
            PathBuf::from(relative.file_name().context("路径无效")?)
        };
        let name = if entry.directory && output.join(&name).is_file() {
            if request.conflict_policy == "skip" {
                continue;
            }
            let stem = name.file_stem().unwrap_or_default().to_string_lossy();
            let ext = name
                .extension()
                .map(|v| format!(".{}", v.to_string_lossy()))
                .unwrap_or_default();
            let parent = name.parent().unwrap_or(Path::new(""));
            (2..)
                .map(|n| parent.join(format!("{stem} ({n}){ext}")))
                .find(|p| !output.join(p).exists() && !staging.path().join(p).exists())
                .unwrap()
        } else {
            name
        };
        let Some(target) = next_name(staging.path(), &name, &request.conflict_policy) else {
            continue;
        };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        if entry.directory {
            fs::create_dir(&target)?;
            continue;
        }
        let mut input: Box<dyn Read> = if let Some(archive) = zip.as_mut() {
            Box::new(open_entry(
                archive,
                &entry.raw,
                password,
                request.zip_password_encoding,
            )?)
        } else {
            Box::new(fs::File::open(
                Path::new(&source.source.path).join(relative),
            )?)
        };
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        let mut bytes = 0;
        let mut buffer = [0u8; 65536];
        loop {
            check(&flag)?;
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            out.write_all(&buffer[..count])?;
            processed += count as u64;
            bytes += count as u64;
            if gate.allow(false) {
                progress(Progress {
                    phase: "copying".into(),
                    entries: index,
                    processed_bytes: processed,
                    total_bytes: total,
                    path: entry.path.clone(),
                });
            }
        }
        ensure!(
            bytes == entry.size,
            "取回期间文件大小有变化，请重新读取来源"
        );
        out.flush()?;
        out.sync_all()?;
    }
    check(&flag)?;
    validate(request, &output, flag.clone())?;
    check(&flag)?;
    progress(Progress {
        phase: "publishing".into(),
        entries: selected.len(),
        processed_bytes: processed,
        total_bytes: total,
        path: String::new(),
    });
    let options = crate::models::compression::DecompressOptions {
        preserve_paths: true,
        conflict_policy: request.conflict_policy.clone(),
        ..Default::default()
    };
    let bytes = match publication::commit_staged_extraction(
        &source.source.path,
        staging.path(),
        &output,
        &options,
        |_| {},
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            staging.retain_after_incomplete_rollback(&error);
            return Err(error);
        }
    };
    let _ = staging.cleanup_after_commit();
    Ok((display_path(&output), bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flag() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let output = dir.path().join("out");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir_all(source.join("empty")).unwrap();
        fs::write(source.join("nested/合同.txt"), b"selected-data").unwrap();
        fs::write(source.join("unselected.txt"), b"leave-original").unwrap();
        fs::create_dir(&output).unwrap();
        (dir, source, output)
    }
    fn zip(path: &Path) {
        let mut z = zip_aes::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, bytes) in [
            ("nested/合同.txt", "selected-data"),
            ("other/合同.txt", "second-data"),
            ("unselected.txt", "leave-original"),
        ] {
            z.start_file(
                name,
                zip_aes::write::SimpleFileOptions::default()
                    .compression_method(zip_aes::CompressionMethod::Deflated),
            )
            .unwrap();
            z.write_all(bytes.as_bytes()).unwrap();
        }
        z.add_directory("empty/", zip_aes::write::SimpleFileOptions::default())
            .unwrap();
        z.finish().unwrap();
    }
    fn selection(source: &Path, paths: &[&str]) -> Selection {
        Selection {
            source: read(source, flag(), |_| {}).unwrap().source,
            paths: paths.iter().map(|s| s.to_string()).collect(),
            keep_structure: true,
            conflict_policy: "rename".into(),
            zip_password_encoding: None,
        }
    }
    #[test]
    fn comparison1554_folder_zip_inventory_keeps_implicit_directories_and_empty_selection() {
        let (dir, source, _out) = fixture();
        let folder = read(&source, flag(), |_| {}).unwrap();
        let path = dir.path().join("sample.zip");
        zip(&path);
        let archive = read(&path, flag(), |_| {}).unwrap();
        for entries in [folder.entries, archive.entries] {
            assert!(
                entries
                    .iter()
                    .find(|e| e.path == "empty")
                    .unwrap()
                    .selectable
            );
            assert!(
                !entries
                    .iter()
                    .find(|e| e.path == "nested")
                    .unwrap()
                    .selectable
            );
            assert!(entries
                .iter()
                .any(|e| e.path == "nested/合同.txt" && e.size == 13));
        }
    }
    #[test]
    fn comparison1554_selected_folder_copy_keeps_originals_and_excludes_unselected() {
        let (_dir, source, out) = fixture();
        let request = selection(&source, &["nested/合同.txt", "empty"]);
        retrieve(&request, &out, flag(), |_| {}).unwrap();
        assert_eq!(
            fs::read(out.join("nested/合同.txt")).unwrap(),
            b"selected-data"
        );
        assert!(out.join("empty").is_dir());
        assert!(!out.join("unselected.txt").exists());
        assert_eq!(
            fs::read(source.join("unselected.txt")).unwrap(),
            b"leave-original"
        );
    }
    #[test]
    fn comparison1554_zip_selected_flatten_rename_and_skip_are_real_bytes() {
        let (dir, _source, out) = fixture();
        let path = dir.path().join("sample.zip");
        zip(&path);
        let mut request = selection(&path, &["nested/合同.txt", "other/合同.txt"]);
        request.keep_structure = false;
        fs::write(out.join("合同.txt"), b"existing").unwrap();
        retrieve(&request, &out, flag(), |_| {}).unwrap();
        assert_eq!(fs::read(out.join("合同.txt")).unwrap(), b"existing");
        let contents = fs::read_dir(&out)
            .unwrap()
            .map(|e| fs::read(e.unwrap().path()).unwrap())
            .collect::<Vec<_>>();
        assert!(
            contents.contains(&b"selected-data".to_vec())
                && contents.contains(&b"second-data".to_vec())
        );
        request.conflict_policy = "skip".into();
        let before = fs::read_dir(&out).unwrap().count();
        retrieve(&request, &out, flag(), |_| {}).unwrap();
        assert_eq!(fs::read_dir(&out).unwrap().count(), before);
        assert!(!out.join("unselected.txt").exists());
    }
    #[test]
    fn comparison1554_changed_or_missing_source_never_publishes() {
        let (_dir, source, out) = fixture();
        let request = selection(&source, &["nested/合同.txt"]);
        fs::write(source.join("unselected.txt"), b"new metadata").unwrap();
        assert!(retrieve(&request, &out, flag(), |_| {}).is_err());
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
        fs::remove_file(source.join("nested/合同.txt")).unwrap();
        assert!(validate(&request, &out, flag()).is_err());
    }
    #[test]
    fn comparison1554_cancelled_read_and_payload_leave_no_published_result() {
        let (_dir, source, out) = fixture();
        let cancelled = flag();
        cancelled.store(true, Ordering::SeqCst);
        assert!(read(&source, cancelled, |_| {}).is_err());
        fs::write(source.join("big.bin"), vec![42u8; 4 * 1024 * 1024]).unwrap();
        let request = selection(&source, &["big.bin"]);
        let cancelled = flag();
        let cancel = cancelled.clone();
        let result = retrieve(&request, &out, cancelled, |p| {
            if p.phase == "copying" {
                cancel.store(true, Ordering::SeqCst);
            }
        });
        assert!(result.is_err());
        assert_eq!(fs::read_dir(out).unwrap().count(), 0);
        assert_eq!(
            fs::metadata(source.join("big.bin")).unwrap().len(),
            4 * 1024 * 1024
        );
    }
    #[test]
    fn comparison1554_protected_duplicate_or_multivolume_zip_reports_scope() {
        let (dir, _source, _out) = fixture();
        let path = dir.path().join("protected.zip");
        let mut z = zip_aes::ZipWriter::new(fs::File::create(&path).unwrap());
        z.start_file(
            "secret.txt",
            zip_aes::write::SimpleFileOptions::default()
                .with_aes_encryption(zip_aes::AesMode::Aes256, "secret"),
        )
        .unwrap();
        z.write_all(b"secret").unwrap();
        z.finish().unwrap();
        assert!(read(&path,flag(),|_|{}).unwrap().source.encrypted);
        let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        for _ in 0..2 {
            z.start_file("same.txt", zip::write::FileOptions::default())
                .unwrap();
            z.write_all(b"same").unwrap();
        }
        z.finish().unwrap();
        assert!(read(&path, flag(), |_| {}).is_err());
        zip(&path);
        let mut bytes = fs::read(&path).unwrap();
        let end = bytes.len() - 22;
        bytes[end + 4] = 1;
        fs::write(&path, bytes).unwrap();
        assert!(read(&path, flag(), |_| {}).is_err());
    }
    #[test]
    fn comparison1554_output_cannot_feed_back_into_either_side() {
        let (dir, source, out) = fixture();
        let reference = read(&source, flag(), |_| {}).unwrap().source;
        assert!(validate_output(std::slice::from_ref(&reference), &source.join("out")).is_err());
        assert!(validate_output(std::slice::from_ref(&reference), dir.path()).is_err());
        assert!(validate_output(std::slice::from_ref(&reference), &out).is_ok());
        let missing = SourceRef {
            path: dir.path().join("missing").to_string_lossy().into(),
            ..reference
        };
        assert!(validate_output(&[missing], &out).is_ok());
    }
    #[test]
    fn comparison1554_empty_directory_against_existing_file_respects_policy() {
        let (_dir, source, out) = fixture();
        let mut request = selection(&source, &["empty"]);
        fs::write(out.join("empty"), b"keep file").unwrap();
        retrieve(&request, &out, flag(), |_| {}).unwrap();
        assert!(out.join("empty (2)").is_dir());
        assert_eq!(fs::read(out.join("empty")).unwrap(), b"keep file");
        request.conflict_policy = "skip".into();
        retrieve(&request, &out, flag(), |_| {}).unwrap();
        assert_eq!(fs::read_dir(out).unwrap().count(), 2);
    }
}

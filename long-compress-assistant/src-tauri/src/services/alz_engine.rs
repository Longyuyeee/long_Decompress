//! ALZ metadata/content share one isolated decoder and the same native members.
use anyhow::{Context, Result};
use std::{path::{Path, PathBuf}, sync::{Arc, atomic::AtomicBool}, time::Duration};
use crate::models::compression::{ArchiveBrowseResult, DecompressOptions};
use super::zip_password::ZipPasswordEncoding;

fn source_encoding(encoding: Option<ZipPasswordEncoding>) -> Result<String> {
    if encoding != Some(ZipPasswordEncoding::WindowsLocal) { return Ok(encoding.map(|e| e.as_str()).unwrap_or("cp949").into()); }
    #[cfg(windows)]
    let page = unsafe { windows_sys::Win32::Globalization::GetACP() };
    #[cfg(not(windows))]
    let page = 0;
    Ok(match page { 65001 => "utf8", 949 => "cp949", 936 => "cp936", 950 => "cp950", 932 => "cp932",
        _ => anyhow::bail!("[alz-encoding] 当前 Windows 代码页 {page} 不在 ALZ 本版验证范围，请明确选择原包编码") }.into())
}

pub fn command_path() -> Option<PathBuf> {
    let mut roots = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")];
    if let Ok(exe) = std::env::current_exe() { if let Some(parent) = exe.parent() {
        roots.insert(0, parent.join("resources")); roots.insert(0, parent.to_path_buf());
    } }
    roots.into_iter().map(|p| p.join("alz-engine/long-alz.exe")).find(|p| p.is_file())
}

async fn run(operation: &str, path: &Path, password: Option<&str>, output: Option<&Path>, selected: &[String], cancelled: Arc<AtomicBool>, encoding: Option<ZipPasswordEncoding>, preserve_timestamps: bool) -> Result<serde_json::Value> {
    let group = super::alz_volumes::complete(path)?;
    let tool = command_path().context("[archive-inspection:engine-unavailable] 缺少随附 ALZ 读取引擎，请修复安装")?;
    let mut command = crate::utils::process::async_command(tool);
    command.arg(operation).arg(&group.first);
    let input = serde_json::to_vec(&serde_json::json!({"password":password,"output":output,"selected":selected,"encoding":source_encoding(encoding)?,"preserve_timestamps":preserve_timestamps}))?;
    let result = crate::utils::process::cancellable_archive_output_with_stdin(&mut command, &cancelled,
        Duration::from_secs(24*60*60), Some(input)).await.context("ALZ 引擎读取取消或运行失败")?;
    let response: serde_json::Value = serde_json::from_slice(&result.stdout).context("ALZ 引擎没有返回有效结果")?;
    if let Some(error) = response.get("error").and_then(|v| v.as_str()) { anyhow::bail!("{}", error); }
    anyhow::ensure!(result.status.success(), "ALZ 引擎异常退出：{}", String::from_utf8_lossy(&result.stderr));
    response.get("result").cloned().context("ALZ 缺少引擎结果")
}

pub async fn browse(path: &Path, cancelled: Arc<AtomicBool>) -> Result<ArchiveBrowseResult> {
    browse_with_encoding(path, None, cancelled).await
}

pub async fn browse_with_encoding(path: &Path, encoding: Option<ZipPasswordEncoding>, cancelled: Arc<AtomicBool>) -> Result<ArchiveBrowseResult> {
    let directory: ArchiveBrowseResult = serde_json::from_value(run("list", path, None, None, &[], cancelled, encoding, false).await?)?;
    for entry in &directory.entries { super::archive_entry_open::normalize_safe_entry_path(&entry.path)?; }
    super::extraction_transaction::validate_resource_limits(path, directory.entries.len(), directory.total_uncompressed_size)?;
    Ok(directory)
}

fn selection(directory: &ArchiveBrowseResult, options: &DecompressOptions) -> Result<Vec<String>> {
    let filter = super::extraction_transaction::compile_file_filter(options.file_filter.as_deref());
    let names = directory.entries.iter().filter(|e|
        super::extraction_transaction::matches_selected_entries(Path::new(&e.path), &options.selected_entries)
            && super::extraction_transaction::matches_compiled_file_filter(Path::new(&e.path), &filter))
        .map(|e| e.path.clone()).collect::<Vec<_>>();
    anyhow::ensure!(!names.is_empty() || !super::zip_selection::has_selection(options),
        "所选范围内没有可解压的 ALZ 条目，请重新选择文件或修改筛选条件");
    Ok(names)
}

pub async fn plan(path: &Path, options: &DecompressOptions, cancelled: Arc<AtomicBool>) -> Result<ArchiveBrowseResult> {
    let directory = browse_with_encoding(path, options.zip_password_encoding, cancelled.clone()).await?;
    let selected = selection(&directory, options)?;
    // Empty archive is distinct from an empty requested selection (rejected above).
    serde_json::from_value(run("plan", path, None, None, &selected, cancelled, options.zip_password_encoding, false).await?).context("ALZ 所选内容预检结果无效")
}

pub async fn requires_password(path: &Path, options: &DecompressOptions, cancelled: Arc<AtomicBool>) -> Result<bool> {
    Ok(plan(path, options, cancelled).await?.encrypted)
}

pub async fn verify(path: &Path, password: Option<&str>, options: &DecompressOptions, cancelled: Arc<AtomicBool>) -> Result<()> {
    let directory = plan(path, options, cancelled.clone()).await?;
    let selected = directory.entries.iter().map(|e| e.path.clone()).collect::<Vec<_>>();
    run("verify", path, password, None, &selected, cancelled, options.zip_password_encoding, false).await?;
    Ok(())
}

pub async fn try_password(path: &Path, password: &str, options: &DecompressOptions, cancelled: Arc<AtomicBool>) -> Result<bool> {
    if !requires_password(path, options, cancelled.clone()).await? { return Ok(false); }
    match verify(path, Some(password), options, cancelled).await {
        Ok(()) => Ok(true),
        Err(e) if e.to_string().contains("[alz-password]") || e.to_string().contains("[archive-inspection:ambiguous-data]") => Ok(false),
        Err(e) => Err(e),
    }
}

pub async fn extract(path: &Path, output: &Path, password: Option<&str>, options: &DecompressOptions, cancelled: Arc<AtomicBool>) -> Result<()> {
    let directory = plan(path, options, cancelled.clone()).await?;
    let selected = directory.entries.iter().map(|e| e.path.clone()).collect::<Vec<_>>();
    run("extract", path, password, Some(output), &selected, cancelled, options.zip_password_encoding, options.preserve_timestamps).await?;
    Ok(())
}

/// Read one complete bounded entry in memory. No extraction directory is created,
/// and unrelated entries are listed but never decoded or password-tested.
async fn preview_entry(path: &Path, entry: &str, password: Option<&str>, encoding: Option<ZipPasswordEncoding>, cancelled: Arc<AtomicBool>, image: bool) -> Result<(Vec<u8>, u64)> {
    let max_output: u64 = if image { 8 * 1024 * 1024 } else { 1024 * 1024 };
    const MAX_INPUT: u64 = 64 * 1024 * 1024;
    anyhow::ensure!(!cancelled.load(std::sync::atomic::Ordering::Acquire), "ALZ 文件预览已取消");
    let entry = super::archive_entry_open::normalize_safe_entry_path(entry)?;
    let listing = browse_with_encoding(path, encoding, cancelled.clone()).await?;
    let selected = listing.entries.iter().find(|e| e.path == entry && !e.is_dir).context("所选 ALZ 文件已不存在，请刷新目录")?;
    anyhow::ensure!(selected.size <= max_output, "{}", if image { "ALZ 图片超过 8 MiB 预览范围，请先提取后打开" } else { "ALZ 文件预览支持 1 MiB 以内的完整文件；此文件较大，请先提取后打开" });
    anyhow::ensure!(selected.compressed_size.unwrap_or(MAX_INPUT+1) <= MAX_INPUT, "所选 ALZ 文件压缩数据超过 64 MiB 预览范围，请先提取后打开");
    let group = super::alz_volumes::complete(path)?;
    let tool = command_path().context("缺少随附 ALZ 读取引擎，请修复安装")?;
    let mut command = crate::utils::process::async_command(tool);
    command.arg(if image { "preview-image" } else { "preview-text" }).arg(&group.first);
    let input = serde_json::to_vec(&serde_json::json!({"password":password,"selected":[entry],"encoding":source_encoding(encoding)?}))?;
    let result = crate::utils::process::cancellable_output_with_stdin(&mut command, &cancelled, Duration::from_secs(30), Some(input)).await.context("ALZ 文件预览已取消或读取未完成")?;
    let response: serde_json::Value = serde_json::from_slice(&result.stdout).context("ALZ 文件预览没有返回有效结果")?;
    if let Some(error) = response.get("error").and_then(|v| v.as_str()) { anyhow::bail!("{error}"); }
    anyhow::ensure!(result.status.success(), "ALZ 文件预览未完成，请先提取后打开");
    let value = response.get("result").context("ALZ 文件预览结果缺失")?;
    let bytes: Vec<u8> = serde_json::from_value(value["bytes"].clone())?;
    let size = value["totalSize"].as_u64().context("ALZ 文件大小缺失")?;
    anyhow::ensure!(size == selected.size && bytes.len() as u64 == size && size <= max_output, "ALZ 文件预览大小不一致，请刷新后重试");
    anyhow::ensure!(!cancelled.load(std::sync::atomic::Ordering::Acquire), "ALZ 文件预览已取消");
    Ok((bytes, size))
}

pub async fn preview_text_entry(path: &Path, entry: &str, password: Option<&str>, encoding: Option<ZipPasswordEncoding>, cancelled: Arc<AtomicBool>) -> Result<(Vec<u8>, u64)> {
    preview_entry(path, entry, password, encoding, cancelled, false).await
}

pub async fn preview_image_entry(path: &Path, entry: &str, password: Option<&str>, encoding: Option<ZipPasswordEncoding>, cancelled: Arc<AtomicBool>) -> Result<Vec<u8>> {
    Ok(preview_entry(path, entry, password, encoding, cancelled, true).await?.0)
}

/// Bound the selected image before password-vault trials can read it.
pub async fn plan_image_preview(path: &Path, entry: &str, encoding: Option<ZipPasswordEncoding>, cancelled: Arc<AtomicBool>) -> Result<()> {
    let entry = super::archive_entry_open::normalize_safe_entry_path(entry)?;
    let options = DecompressOptions { selected_entries: vec![entry], zip_password_encoding: encoding, ..Default::default() };
    let metadata = plan(path, &options, cancelled).await?;
    anyhow::ensure!(metadata.entries.len() == 1 && !metadata.entries[0].is_dir, "请选择一张 ALZ 图片预览");
    let entry = &metadata.entries[0];
    anyhow::ensure!(entry.size <= 8*1024*1024, "ALZ 图片超过 8 MiB 预览范围，请先提取后打开");
    anyhow::ensure!(entry.compressed_size.unwrap_or(u64::MAX) <= 64*1024*1024, "所选 ALZ 图片压缩数据超过 64 MiB 预览范围，请先提取后打开");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/alz-152") }
    fn flag() -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
    fn stream_root() -> PathBuf { root().parent().unwrap().join("alz-1512") }
    #[tokio::test]
    async fn alz_bzip2_termination_causes_survive_password_and_selected_entry_routes() {
        let fixtures = root().parent().unwrap().join("alz-bzip2-165");
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(fixtures.join("manifest.json")).unwrap()).unwrap();
        let options = DecompressOptions::default();
        for case in manifest["cases"].as_array().unwrap() {
            let entry = if case["controlledNativeFraming"] == true {
                fixtures.join(case["members"][1]["name"].as_str().unwrap())
            } else { fixtures.join(case["archive"].as_str().unwrap()) };
            let password = case["password"].as_str();
            assert_eq!(browse(&entry, flag()).await.unwrap().total_files, 2);
            let error = verify(&entry, password, &options, flag()).await.unwrap_err().to_string();
            assert!(error.contains("[archive-inspection:damaged]") && error.contains("不必反复尝试密码"), "{error}");
            assert!(error.contains(if case["fault"] == "trailing" { "5 字节异常数据" } else { "缺少结束标记" }), "{error}");
            if let Some(password) = password {
                assert!(try_password(&entry, password, &options, flag()).await.unwrap_err().to_string().contains("[archive-inspection:damaged]"));
                assert!(!try_password(&entry, "wrong password", &options, flag()).await.unwrap());
            }
            // The unrelated intact file remains available through existing exact
            // selection. Do not introduce a new recovery mode or demand its key.
            let selected = DecompressOptions { selected_entries: vec!["b.txt".into()], ..Default::default() };
            assert!(!requires_password(&entry, &selected, flag()).await.unwrap());
            verify(&entry, None, &selected, flag()).await.unwrap();
            let out = tempfile::tempdir().unwrap();
            extract(&entry, out.path(), None, &selected, flag()).await.unwrap();
            assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 1);
            assert_eq!(std::fs::read(out.path().join("b.txt")).unwrap(), b"lorem ipsum dolor sit amet ".repeat(80));
        }
        let uncertain = fixtures.join(manifest["uncertain"]["name"].as_str().unwrap());
        let password = manifest["uncertain"]["password"].as_str().unwrap();
        let error = verify(&uncertain, Some(password), &options, flag()).await.unwrap_err().to_string();
        assert!(error.contains("[archive-inspection:ambiguous-data]") && !error.contains("[archive-inspection:damaged]"), "{error}");
        assert!(!try_password(&uncertain, password, &options, flag()).await.unwrap());
    }
    #[tokio::test]
    async fn alz_original_deflate_stream_matrix_and_selected_payloads() {
        let m:serde_json::Value=serde_json::from_slice(&std::fs::read(stream_root().join("manifest.json")).unwrap()).unwrap();
        assert_eq!(m["cases"].as_array().unwrap().len(),8);
        for c in m["cases"].as_array().unwrap() {
            let source=stream_root().join(c["archive"].as_str().unwrap());let password=c["password"].as_str();
            let options=DecompressOptions{zip_password_encoding:Some(ZipPasswordEncoding::Cp936),..Default::default()};
            verify(&source,password,&options,flag()).await.unwrap();let out=tempfile::tempdir().unwrap();extract(&source,out.path(),password,&options,flag()).await.unwrap();
            for e in c["expected"].as_array().unwrap(){let b=std::fs::read(out.path().join(e["path"].as_str().unwrap())).unwrap();assert_eq!(hex::encode(sha2::Sha256::digest(&b)),e["sha256"].as_str().unwrap());}
            assert!(out.path().join("空目录").is_dir());let selected=DecompressOptions{selected_entries:vec!["文档/99说明.txt".into()],..options};let out=tempfile::tempdir().unwrap();extract(&source,out.path(),password,&selected,flag()).await.unwrap();assert!(out.path().join("文档/99说明.txt").is_file());assert!(!out.path().join("01payload.bin").exists());
        }
    }
    #[tokio::test]
    async fn alz_deflate_trailing_data_is_damage_after_full_crc_not_a_password_retry() {
        let m:serde_json::Value=serde_json::from_slice(&std::fs::read(stream_root().join("manifest.json")).unwrap()).unwrap();assert_eq!(m["faults"].as_array().unwrap().len(),8);
        for c in m["faults"].as_array().unwrap(){let source=stream_root().join(c["archive"].as_str().unwrap());let options=DecompressOptions{zip_password_encoding:Some(ZipPasswordEncoding::Cp936),..Default::default()};let password=c["password"].as_str();
            let error=verify(&source,password,&options,flag()).await.unwrap_err().to_string();assert!(error.contains("[archive-inspection:damaged]")&&error.contains("尾部")&&error.contains("CRC"),"{error}");
            if let Some(password)=password {let error=try_password(&source,password,&options,flag()).await.unwrap_err().to_string();assert!(error.contains("[archive-inspection:damaged]"),"{error}");assert!(!try_password(&source,"wrong",&options,flag()).await.unwrap());}
        }
    }
    #[tokio::test]
    async fn alz_trailing_ciphertext_without_matching_crc_stays_uncertain() {
        let m:serde_json::Value=serde_json::from_slice(&std::fs::read(stream_root().join("manifest.json")).unwrap()).unwrap();let c=m["faults"].as_array().unwrap().iter().find(|c|c["name"]=="m2-key-single-trailing").unwrap();
        let mut bytes=std::fs::read(stream_root().join(c["archive"].as_str().unwrap())).unwrap();bytes[23]^=1;let dir=tempfile::tempdir().unwrap();let source=dir.path().join("crc-uncertain.alz");std::fs::write(&source,bytes).unwrap();let opts=DecompressOptions{zip_password_encoding:Some(ZipPasswordEncoding::Cp936),..Default::default()};
        let error=verify(&source,c["password"].as_str(),&opts,flag()).await.unwrap_err().to_string();assert!(error.contains("[archive-inspection:ambiguous-data]"),"{error}");assert!(!try_password(&source,c["password"].as_str().unwrap(),&opts,flag()).await.unwrap());
    }
    fn copied_split(dir: &Path) -> Vec<PathBuf> {
        ["split_store.alz", "split_store.a00", "split_store.a01"].into_iter().map(|n| {
            let p = dir.join(n); std::fs::copy(root().join(n), &p).unwrap(); p
        }).collect()
    }
    #[tokio::test]
    async fn alz_genuine_matrix_matches_independent_payloads() {
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap();
        for case in manifest["cases"].as_array().unwrap() {
            let path = root().join(case["archive"].as_str().unwrap());
            let password = case["password"].as_str();
            let directory = browse(&path, flag()).await.unwrap();
            assert_eq!(directory.format, "ALZ");
            assert_eq!(directory.total_files, case["expected"].as_array().unwrap().len());
            verify(&path, password, &Default::default(), flag()).await.unwrap();
            let output = tempfile::tempdir().unwrap();
            extract(&path, output.path(), password, &Default::default(), flag()).await.unwrap();
            for e in case["expected"].as_array().unwrap() {
                let bytes = std::fs::read(output.path().join(e["path"].as_str().unwrap())).unwrap();
                assert_eq!(bytes.len() as u64, e["bytes"].as_u64().unwrap());
                assert_eq!(hex::encode(sha2::Sha256::digest(&bytes)), e["sha256"].as_str().unwrap());
            }
            if case["name"] == "dir_tree" { assert!(output.path().join("work/tree/empty_folder").is_dir()); }
            assert_eq!(requires_password(&path, &Default::default(), flag()).await.unwrap(), password.is_some());
        }
    }
    #[tokio::test]
    async fn alz_password_is_full_content_not_short_header_or_plain_success() {
        let source = root().join("zip20.alz");
        assert!(!try_password(&source, "wrong", &Default::default(), flag()).await.unwrap());
        assert!(try_password(&source, "test1234", &Default::default(), flag()).await.unwrap());
        let plain = root().join("store.alz");
        assert!(!try_password(&plain, "test1234", &Default::default(), flag()).await.unwrap());
        let error = verify(&source, Some("🔑"), &Default::default(), flag()).await.unwrap_err().to_string();
        assert!(error.contains("不能无损表示"), "{error}");
        let selected = DecompressOptions { selected_entries: vec!["empty.txt".into()], ..Default::default() };
        assert!(!requires_password(&source, &selected, flag()).await.unwrap());
        let out = tempfile::tempdir().unwrap();
        extract(&source, out.path(), None, &selected, flag()).await.unwrap();
        assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 1);
        assert_eq!(std::fs::read(out.path().join("empty.txt")).unwrap(), Vec::<u8>::new());
    }
    #[tokio::test]
    async fn alz_any_original_member_has_one_group_and_same_directory() {
        let original = browse(&root().join("split_store.alz"), flag()).await.unwrap();
        for name in ["split_store.alz", "split_store.a00", "split_store.a01"] {
            let p = root().join(name);
            let group = super::super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&p).unwrap().unwrap();
            assert!(group.is_complete); assert_eq!(group.total_parts, 3);
            assert_eq!(group.first_part, root().join("split_store.alz"));
            assert_eq!(browse(&p, flag()).await.unwrap(), original);
            verify(&p, None, &Default::default(), flag()).await.unwrap();
        }
    }
    #[tokio::test]
    async fn alz_missing_first_middle_last_and_mixed_number_outrank_password() {
        for missing_index in 0..3 {
            let dir = tempfile::tempdir().unwrap(); let parts = copied_split(dir.path());
            std::fs::remove_file(&parts[missing_index]).unwrap();
            let source = &parts[if missing_index == 0 { 1 } else { 0 }];
            let error = try_password(source, "whatever", &Default::default(), flag()).await.unwrap_err().to_string();
            assert!(error.contains("[archive-inspection:incomplete]"), "{error}");
            assert!(error.contains(parts[missing_index].file_name().unwrap().to_str().unwrap()), "{error}");
        }
        let dir = tempfile::tempdir().unwrap(); let parts = copied_split(dir.path());
        let mut bytes = std::fs::read(&parts[1]).unwrap(); bytes[6] = 17; std::fs::write(&parts[1], bytes).unwrap();
        let error = browse(&parts[2], flag()).await.unwrap_err().to_string();
        assert!(error.contains("编号") && error.contains("[archive-inspection:damaged]"), "{error}");
    }
    #[tokio::test]
    async fn alz_method_damage_and_cancellation_have_explicit_causes() {
        let dir = tempfile::tempdir().unwrap();
        let unknown = dir.path().join("unknown.alz");
        let mut bytes = std::fs::read(root().join("enc_deflate.alz")).unwrap(); bytes[21] = 250;
        std::fs::write(&unknown, bytes).unwrap();
        let error = try_password(&unknown, "wrong", &Default::default(), flag()).await.unwrap_err().to_string();
        assert!(error.contains("方法 250") && error.contains("[archive-inspection:unsupported]"), "{error}");
        let damaged = dir.path().join("bad.alz"); let mut bytes = std::fs::read(root().join("store.alz")).unwrap();
        let pattern = (0..32u8).collect::<Vec<_>>(); let offset = bytes.windows(pattern.len()).position(|b| b == pattern).unwrap();
        bytes[offset + 10] ^= 0x55; std::fs::write(&damaged, bytes).unwrap();
        let error = verify(&damaged, Some("unused"), &Default::default(), flag()).await.unwrap_err().to_string();
        assert!(error.contains("[archive-inspection:damaged]") && error.contains("CRC"), "{error}");
        let cancelled = flag(); cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
        let error = browse(&root().join("store.alz"), cancelled).await.unwrap_err();
        assert_eq!(error.downcast_ref::<std::io::Error>().unwrap().kind(), std::io::ErrorKind::Interrupted);
    }
    #[tokio::test]
    async fn alz_original_selection_and_filter_intersect_before_decode() {
        let source = root().join("store.alz");
        let options = DecompressOptions { selected_entries: vec!["subdir".into()], file_filter: Some("*.txt".into()), ..Default::default() };
        let selected_plan = plan(&source, &options, flag()).await.unwrap(); assert_eq!(selected_plan.total_files, 2);
        let output = tempfile::tempdir().unwrap(); extract(&source, output.path(), None, &options, flag()).await.unwrap();
        assert!(output.path().join("subdir/nested/deep.txt").is_file()); assert!(!output.path().join("hello.txt").exists());
        let missing = DecompressOptions { selected_entries: vec!["missing".into()], ..Default::default() };
        assert!(plan(&source, &missing, flag()).await.unwrap_err().to_string().contains("没有可解压"));
    }
    #[tokio::test]
    async fn alz_official_cp936_password_originals_match_independent_payloads() {
        let base = root().join("original-alzip");
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(base.join("cp936-provenance.json")).unwrap()).unwrap();
        let options = DecompressOptions { zip_password_encoding: Some(ZipPasswordEncoding::Cp936), ..Default::default() };
        for case in manifest["cases"].as_array().unwrap() {
            let path = base.join(case["archive"].as_str().unwrap());
            let password = case["password"].as_str().unwrap();
            let list = browse_with_encoding(&path, options.zip_password_encoding, flag()).await.unwrap();
            assert!(list.entries.iter().any(|e| e.path == "说明 文件.txt"));
            assert!(try_password(&path, password, &options, flag()).await.unwrap());
            assert!(!try_password(&path, "wrong", &options, flag()).await.unwrap());
            let output = tempfile::tempdir().unwrap();
            extract(&path, output.path(), Some(password), &options, flag()).await.unwrap();
            assert!(output.path().join("空目录").is_dir());
            for e in manifest["sourceFiles"].as_array().unwrap() {
                let bytes = std::fs::read(output.path().join(e["path"].as_str().unwrap())).unwrap();
                assert_eq!(hex::encode(sha2::Sha256::digest(&bytes)), e["sha256"].as_str().unwrap());
            }
            let report = super::super::archive_diagnostics::diagnose_archive_with_encoding(&path, Some(password), options.zip_password_encoding, flag()).await.unwrap();
            assert_eq!(report.status, "healthy"); assert!(report.integrity_tested); assert!(!report.can_repair);
            if case["split"] == true {
                for index in 0..4 {
                    let member = path.with_extension(format!("a0{index}"));
                    assert_eq!(browse_with_encoding(&member, options.zip_password_encoding, flag()).await.unwrap(), list);
                }
            }
        }
        // Original producer lost Korean names on this CP936 machine: never
        // silently rename its literal '?' names to manufacture a successful test.
        let lost = base.join("native-key-volumes.alz");
        assert!(browse_with_encoding(&lost, options.zip_password_encoding, flag()).await.unwrap_err().to_string().contains("路径"));
    }
    #[tokio::test]
    async fn alz_diagnostic_structure_and_methods_outrank_password_requests() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("method.alz");
        let mut data = std::fs::read(root().join("enc_deflate.alz")).unwrap(); data[21] = 250;
        std::fs::write(&path, data).unwrap();
        let report = super::super::archive_diagnostics::diagnose_archive(&path, None, flag()).await.unwrap();
        assert_eq!(report.status, "unsupported_feature"); assert!(!report.integrity_tested);
        let parts = copied_split(dir.path());
        let mut data = std::fs::read(&parts[1]).unwrap(); data[6] = 17; std::fs::write(&parts[1], data).unwrap();
        let report = super::super::archive_diagnostics::diagnose_archive(&parts[0], None, flag()).await.unwrap();
        assert_eq!(report.status, "damaged");
        let truncated = dir.path().join("truncated.alz"); std::fs::write(&truncated, b"ALZ").unwrap();
        let report = super::super::archive_diagnostics::diagnose_archive(&truncated, None, flag()).await.unwrap();
        assert_eq!(report.actual_format, "ALZ"); assert_ne!(report.status, "password_required");
        let alias = dir.path().join("unknown.bin"); std::fs::copy(root().join("normal.alz"), &alias).unwrap();
        assert_eq!(browse(&alias, flag()).await.unwrap().format, "ALZ");
        std::fs::copy(root().join("normal.alz"), dir.path().join("unrelated.alz")).unwrap();
        let unrelated = dir.path().join("unrelated.zip");
        let file = std::fs::File::create(&unrelated).unwrap();
        zip::ZipWriter::new(file).finish().unwrap();
        assert_eq!(super::super::archive_browser::detect_format(&unrelated).unwrap(), super::super::archive_format::ArchiveFormat::Zip);
    }
}

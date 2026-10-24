use super::byte_progress::{source_total_bytes, ByteProgress};
use super::CompressionRuntime;
use crate::models::compression::{CompressionOptions, TaskLogSeverity};
use crate::services::compression_entries;
use crate::services::compression_service::CompressionError;
use crate::services::progress_gate::ProgressGate;
use crate::services::split_compression::SplitCompressionService;
use anyhow::Result;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tauri::Window;

fn copy_cancellable<R: CompressionRuntime, Rd: Read, Wr: Write>(
    runtime: &R,
    reader: &mut Rd,
    writer: &mut Wr,
    buffer: &mut [u8],
    mut on_read: impl FnMut(u64),
) -> Result<()> {
    loop {
        runtime.check_cancellation()?;
        let read = reader.read(buffer)?;
        if read == 0 {
            return Ok(());
        }
        writer.write_all(&buffer[..read])?;
        on_read(read as u64);
    }
}

#[cfg(test)]
pub(crate) fn create_encrypted_zip<R: CompressionRuntime>(
    runtime: &R,
    password: &str,
    password_encoding: crate::services::zip_password::ZipPasswordEncoding,
    encryption: crate::services::zip_password::ZipEncryption,
    level: u32,
    sources: &[String],
    output: &str,
    preserve_paths: bool,
    _legacy_fast_pack: bool,
    progress_context: Option<(&Window, &str)>,
) -> Result<u64> {
    create_zip(runtime, Some(password), password_encoding, encryption, level, sources, output,
        preserve_paths, crate::services::zip_method::ZipMethod::Deflate, false, progress_context)
}

fn create_zip<R: CompressionRuntime>(
    runtime: &R,
    password: Option<&str>,
    password_encoding: crate::services::zip_password::ZipPasswordEncoding,
    encryption: crate::services::zip_password::ZipEncryption,
    level: u32,
    sources: &[String],
    output: &str,
    preserve_paths: bool,
    method: crate::services::zip_method::ZipMethod,
    zipx: bool,
    progress_context: Option<(&Window, &str)>,
) -> Result<u64> {
    // Resolve before creating output so unrepresentable passwords leave no file.
    let password_bytes = password.map(|password| crate::services::zip_password::encode(password, password_encoding)).transpose()?;
    let file = File::create(output)?;
    let mut writer = zip_aes::ZipWriter::new(file);
    let options = crate::services::zip_method::file_options(zipx, method, level);
    let options = if let Some(password_bytes) = &password_bytes {
        match encryption {
            crate::services::zip_password::ZipEncryption::Aes256 => options.with_aes_encryption_bytes(zip_aes::AesMode::Aes256, password_bytes),
            crate::services::zip_password::ZipEncryption::Zipcrypto => {
                use zip_aes::unstable::write::FileOptionsExt;
                options.with_deprecated_encryption(password_bytes)?
            }
        }
    } else { options };
    let entries = compression_entries::collect(sources, preserve_paths, true)?;
    let total_bytes = source_total_bytes(&entries)?;
    let mut progress = ByteProgress::new(total_bytes);
    let mut progress_gate = ProgressGate::new();
    // PPMd / LZMA are CPU intensive: keep pause/cancel and byte progress responsive.
    let buffer_size = if !zipx && matches!(method, crate::services::zip_method::ZipMethod::Ppmd | crate::services::zip_method::ZipMethod::Lzma) {
        runtime.copy_buffer_size().min(64 * 1024)
    } else { runtime.copy_buffer_size() };
    let mut copy_buffer = vec![0u8; buffer_size];

    for entry in entries {
        runtime.check_cancellation()?;
        let archive_name = entry.archive_name.replace('\\', "/");
        if entry.is_dir {
            writer.add_directory(archive_name, options)?;
        } else {
            let mut source = File::open(entry.path)?;
            let size = source.metadata()?.len();
            // The current XZ encoder emits an invalid index for an untouched empty
            // stream. Empty ZIP members need no compression; retain encryption.
            let entry_options = if zipx && size == 0 {
                options.compression_method(zip_aes::CompressionMethod::Stored).compression_level(None)
            } else { options };
            writer.start_file(archive_name, entry_options.large_file(size >= u32::MAX as u64))?;
            let current_name = entry.archive_name.clone();
            copy_cancellable(
                runtime,
                &mut source,
                &mut writer,
                &mut copy_buffer,
                |read| {
                    if let Some((ratio, processed, total)) = progress.record(read, false) {
                        if !progress_gate.allow(ratio >= 1.0) {
                            return;
                        }
                        if let Some((window, task_id)) = progress_context {
                            runtime.emit_progress(
                                window,
                                task_id,
                                ratio,
                                Some(current_name.clone()),
                                processed,
                                total,
                            );
                        }
                    }
                },
            )?;
            if let Some((ratio, processed, total)) = progress.record(0, true) {
                if let Some((window, task_id)) =
                    progress_context.filter(|_| progress_gate.allow(ratio >= 1.0))
                {
                    runtime.emit_progress(
                        window,
                        task_id,
                        ratio,
                        Some(entry.archive_name.clone()),
                        processed,
                        total,
                    );
                }
            }
        }
    }
    writer.finish()?;
    Ok(total_bytes)
}

pub(crate) fn compress<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    if ![".zip", ".zipx"].iter().any(|extension| output.to_ascii_lowercase().ends_with(extension)) {
        return Err(CompressionError::CompressionFailed(
            "ZIP 压缩输出路径需要以 .zip 或 .zipx 结尾".to_string(),
        )
        .into());
    }

    if let Some(parent) = Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }

    if let Some(split_size) = options.split_size.filter(|size| *size > 0) {
        runtime.check_cancellation()?;
        // Reuse the real ZIP writer for directories, path layout and encryption.
        // A temporary complete archive costs disk space, not archive-sized RAM.
        struct TemporaryZip(std::path::PathBuf);
        impl Drop for TemporaryZip {
            fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
        }
        let staged = crate::services::output_publish_transaction::staged_output_path(
            Path::new(output), "split-source")?;
        let reservation = std::fs::OpenOptions::new().write(true).create_new(true).open(&staged)?;
        let temporary = TemporaryZip(staged);
        drop(reservation);
        let mut unsplit = options.clone();
        unsplit.split_size = None;
        compress(runtime, window, task_id, sources, &temporary.0.to_string_lossy(), unsplit)?;
        runtime.check_cancellation()?;
        let mut split_progress_gate = ProgressGate::new();
        let on_progress = |processed, total| {
                if processed == 0 || split_progress_gate.allow(processed >= total) {
                    if let Some(window) = window {
                        runtime.emit_split_progress(window, task_id, processed, total);
                    }
                }
            };
        let result = if options.zip_split_mode == crate::services::standard_zip_writer::ZipSplitMode::Standard {
            crate::services::standard_zip_writer::split(&temporary.0, Path::new(output), split_size, || runtime.check_cancellation(), on_progress)?
        } else {
            SplitCompressionService::split_archive_file_with_progress(&temporary.0, Path::new(output), split_size, || runtime.check_cancellation(), on_progress)?
        };
        if let Some(window) = window {
            runtime.emit_log(
                window,
                task_id,
                &format!("分卷压缩完成：{} 个分卷", result.part_count),
                TaskLogSeverity::Success,
            );
        }
        return Ok(());
    }

    let zipx = output.to_ascii_lowercase().ends_with(".zipx");
    let method_name = if zipx { "XZ" } else if options.zip_method == crate::services::zip_method::ZipMethod::Lzma { "LZMA" } else if options.zip_method == crate::services::zip_method::ZipMethod::Ppmd { "PPMd" } else if options.zip_method == crate::services::zip_method::ZipMethod::Bzip2 { "BZip2" } else if options.zip_method == crate::services::zip_method::ZipMethod::Zstd { "Zstandard" }
        else if options.zip_method == crate::services::zip_method::ZipMethod::Store || options.level == 0 { "Store" } else { "Deflate" };
    if let Some(window) = window {
        runtime.emit_log(window, task_id, &format!("正在创建 {} · {}{}…",
            if zipx { "ZIPX" } else { "ZIP" }, method_name,
            if options.password.as_deref().is_some_and(|password| !password.is_empty()) { " · 密码保护" } else { "" }), TaskLogSeverity::Info);
    }
    let total_bytes = create_zip(runtime, options.password.as_deref().filter(|password| !password.is_empty()),
        options.zip_password_encoding, options.zip_encryption, options.level,
        sources, output, options.preserve_paths.unwrap_or(true), options.zip_method, zipx,
        window.map(|window| (window, task_id)))?;
    if let Some(window) = window {
        if total_bytes == 0 { runtime.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0); }
        runtime.emit_log(window, task_id, &format!("{} · {} 创建完成", if zipx { "ZIPX" } else { "ZIP" }, method_name), TaskLogSeverity::Success);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes and hashes genuine >4 GiB content; run explicitly for v1.5.15"]
    fn zip64_1515_real_written_password_volume_matrix() {
        use sha2::{Digest, Sha256};
        use crate::services::{standard_zip_writer::ZipSplitMode, zip_password::ZipEncryption};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/zip64-1515");
        let source = root.join("source");
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("source.json")).unwrap()).unwrap();
        assert_eq!(manifest["physicalWrite"], true);
        assert!(manifest["bytesActuallyWritten"].as_u64().unwrap() > u32::MAX as u64);
        let files = manifest["files"].as_array().unwrap();
        let mut sources = files.iter().map(|f| source.join(f["name"].as_str().unwrap()).to_string_lossy().into_owned()).collect::<Vec<_>>();
        sources.push(source.join("空目录").to_string_lossy().into_owned());
        let checkpoint = root.join("native-progress.json");
        let mut cases: Vec<serde_json::Value> = if checkpoint.exists() {
            serde_json::from_slice(&std::fs::read(&checkpoint).unwrap()).unwrap()
        } else { Vec::new() };
        for (mode, encryption) in [("plain", None), ("aes", Some(ZipEncryption::Aes256)), ("zipcrypto", Some(ZipEncryption::Zipcrypto))] {
            let password = encryption.map(|_| "ZIP64-test-password + !");
            let output = root.join(format!("product-{mode}.zip"));
            let reused = output.exists();
            if !reused {
                eprintln!("[zip64-1515] create {mode}, genuinely written {} bytes", manifest["bytesActuallyWritten"]);
                compress(&TestRuntime::default(), None, "zip64-1515", &sources, &output.to_string_lossy(),
                    CompressionOptions { format: Some("zip".into()), level: if encryption.is_none() { 0 } else { 1 },
                        password: password.map(str::to_owned), zip_encryption: encryption.unwrap_or_default(),
                        preserve_paths: Some(true), ..Default::default() }).unwrap();
            }
            for layout in [None, Some(ZipSplitMode::Numeric), Some(ZipSplitMode::Standard)] {
                let length = output.metadata().unwrap().len();
                let layout_name = match layout { None => "single", Some(ZipSplitMode::Numeric) => "numeric", _ => "standard" };
                if cases.iter().any(|case| case["mode"] == mode && case["layout"] == layout_name) {
                    eprintln!("[zip64-1515] reuse completed {mode}/{layout_name} with unchanged runtime source");
                    continue;
                }
                let destination = root.join(format!("{mode}-{layout_name}"));
                std::fs::create_dir_all(&destination).unwrap();
                let split_output = destination.join("大文件分卷.zip");
                let parts = match layout {
                    None => vec![output.clone()],
                    Some(ZipSplitMode::Numeric) => crate::services::split_compression::SplitCompressionService::split_archive_file_with_progress(
                        &output, &split_output, (length / 2).max(65536), || Ok(()), |_, _| {}).unwrap().part_files,
                    Some(ZipSplitMode::Standard) => crate::services::standard_zip_writer::split(&output, &split_output,
                        (length / 2).clamp(65536, u32::MAX as u64 - 65536), || Ok(()), |_, _| {}).unwrap().part_files,
                };
                let input = if layout == Some(ZipSplitMode::Standard) { parts.last().unwrap() } else { &parts[0] };
                eprintln!("[zip64-1515] read all content {mode}/{layout_name}, {} parts", parts.len());
                let reader = crate::services::numeric_volume_reader::ZipInputReader::open(input).unwrap();
                let mut archive = zip_aes::ZipArchive::new(reader).unwrap();
                let mut checked = Vec::new();
                let mut tail_offset = None;
                for file in files {
                    let name = file["name"].as_str().unwrap();
                    let mut entry = if let Some(password) = password {
                        crate::services::zip_password::open_named_entry(&mut archive, name, password).unwrap()
                    } else { archive.by_name(name).unwrap() };
                    assert_eq!(entry.size(), file["bytes"].as_u64().unwrap());
                    if name == "99-末尾说明.txt" { tail_offset = Some(entry.header_start()); }
                    let mut hash = Sha256::new();
                    let mut buffer = vec![0; 1024 * 1024];
                    let mut bytes = 0u64;
                    loop { let n = entry.read(&mut buffer).unwrap(); if n == 0 { break; } hash.update(&buffer[..n]); bytes += n as u64; }
                    let digest = format!("{:x}", hash.finalize());
                    assert_eq!(digest, file["sha256"].as_str().unwrap(), "{mode}/{layout_name}/{name}");
                    assert_eq!(bytes, entry.size());
                    checked.push(serde_json::json!({"name":name,"bytes":bytes,"sha256":digest}));
                }
                assert!(archive.by_name("空目录/").unwrap().is_dir());
                let tail_offset = tail_offset.expect("Following entry was fully checked above, including its password");
                if mode == "plain" { assert!(tail_offset > u32::MAX as u64); }
                cases.push(serde_json::json!({"mode":mode,"layout":layout_name,"baseReused":reused,
                    "source":input,"parts":parts,"tailVirtualOffset":tail_offset,"files":checked}));
                std::fs::write(root.join("native-progress.json"), serde_json::to_vec_pretty(&cases).unwrap()).unwrap();
            }
        }
        assert_eq!(cases.len(), 9);
        std::fs::write(root.join("native-results.json"), serde_json::to_vec_pretty(&cases).unwrap()).unwrap();
    }

    #[test]
    fn zip_output_1414_methods_passwords_and_both_volume_types_round_trip() {
        use crate::services::{zip_method::ZipMethod, zip_password::ZipEncryption, standard_zip_writer::ZipSplitMode};
        let mut seed = 1414u32;
        let payload: Vec<u8> = (0..260_000).map(|_| { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; seed as u8 }).collect();
        for format in ["zip", "zipx"] {
            for password in [None, Some(ZipEncryption::Aes256), Some(ZipEncryption::Zipcrypto)] {
                for split in [None, Some(ZipSplitMode::Numeric), Some(ZipSplitMode::Standard)] {
                    let root = tempfile::tempdir().unwrap();
                    let source = root.path().join("中文资料");
                    std::fs::create_dir_all(source.join("空目录")).unwrap();
                    std::fs::write(source.join("内容.bin"), &payload).unwrap();
                    std::fs::write(source.join("空文件.txt"), []).unwrap();
                    let output = root.path().join(format!("结果.{format}"));
                    let options = CompressionOptions { format: Some(format.into()), level: 6,
                        zip_method: ZipMethod::Zstd, password: password.map(|_| "测试 + password!".into()),
                        zip_encryption: password.unwrap_or_default(),
                        zip_split_mode: split.unwrap_or_default(), split_size: split.map(|_| 65_536),
                        preserve_paths: Some(true), ..Default::default() };
                    compress(&TestRuntime::default(), None, "1414", &[source.to_string_lossy().into()], &output.to_string_lossy(), options).unwrap();
                    let input = if split.is_some() { crate::services::archive_volumes::first_volume_path(&output) } else { output.clone() };
                    let reader = crate::services::numeric_volume_reader::ZipInputReader::open(&input).unwrap();
                    let mut archive = zip_aes::ZipArchive::new(reader).unwrap();
                    let mut content = Vec::new();
                    let mut empty_dir = false;
                    for index in 0..archive.len() {
                        let mut file = if password.is_some() { archive.by_index_decrypt(index, "测试 + password!".as_bytes()).unwrap_or_else(|error| panic!("{format} {password:?} {split:?} index {index}: {error}")) }
                            else { archive.by_index(index).unwrap_or_else(|error| panic!("{format} {password:?} {split:?} index {index}: {error}")) };
                        if file.name().ends_with("空目录/") { empty_dir = true; }
                        let is_payload = file.name().ends_with("内容.bin");
                        let name = file.name().to_string();
                        if !file.is_dir() {
                            let mut restored = Vec::new(); file.read_to_end(&mut restored).unwrap_or_else(|error| panic!("{format} {password:?} {split:?} {name}: {error}"));
                            if is_payload { content = restored; } else { assert!(restored.is_empty()); }
                        }
                        if is_payload {
                            assert_eq!(file.compression(), if format == "zipx" { zip_aes::CompressionMethod::Xz } else { zip_aes::CompressionMethod::Zstd });
                        }
                    }
                    assert_eq!(content, payload, "{format} {password:?} {split:?}");
                    assert!(empty_dir);
                    if split.is_some() {
                        let parts = crate::services::archive_volumes::complete_volume_paths(&output).unwrap();
                        assert!(parts.len() > 1);
                        for part in &parts {
                            let group = crate::services::split_archive_detector::SplitArchiveDetector::detect_split_archive(part).unwrap().unwrap();
                            assert!(group.is_complete); assert_eq!(group.parts.len(), parts.len());
                        }
                    }
                }
            }
        }
        let legacy: CompressionOptions = serde_json::from_value(serde_json::json!({"level":0})).unwrap();
        assert_eq!(legacy.zip_method, ZipMethod::Deflate);
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.txt"); std::fs::write(&source, b"legacy").unwrap();
        let output = root.path().join("legacy.zip");
        compress(&TestRuntime::default(), None, "legacy", &[source.to_string_lossy().into()], &output.to_string_lossy(), legacy).unwrap();
        let mut archive = zip_aes::ZipArchive::new(File::open(output).unwrap()).unwrap();
        assert_eq!(archive.by_index(0).unwrap().compression(), zip_aes::CompressionMethod::Stored);
        assert!(serde_json::from_value::<CompressionOptions>(serde_json::json!({"level":6,"zip_method":"unknown"})).is_err());
    }

    #[test]
    fn zipcrypto_is_explicit_and_roundtrips_with_both_password_encodings() {
        use crate::services::zip_password::{ZipEncryption, ZipPasswordEncoding, encode};
        for mode in [ZipPasswordEncoding::Utf8, ZipPasswordEncoding::WindowsLocal] {
            for encryption in [ZipEncryption::Aes256, ZipEncryption::Zipcrypto] {
                let password = "密码 + !测试";
                let Ok(bytes) = encode(password, mode) else { continue };
                let root = tempfile::tempdir().unwrap();
                let source = root.path().join("内容.txt");
                std::fs::write(&source, b"payload content").unwrap();
                let output = root.path().join("out.zip");
                create_encrypted_zip(&TestRuntime::default(), password, mode, encryption, 6,
                    &[source.to_string_lossy().into_owned()], &output.to_string_lossy(), true, false, None).unwrap();
                let mut zip = zip_aes::ZipArchive::new(File::open(&output).unwrap()).unwrap();
                let wrong_failed = match zip.by_index_decrypt(0, b"wrong-password") {
                    Err(_) => true,
                    Ok(mut entry) => entry.read_to_end(&mut Vec::new()).is_err(),
                };
                assert!(wrong_failed);
                let mut entry = zip.by_index_decrypt(0, &bytes).unwrap();
                let mut restored = Vec::new();
                entry.read_to_end(&mut restored).unwrap();
                assert_eq!(restored, b"payload content");
                let data = std::fs::read(output).unwrap();
                assert_eq!(u16::from_le_bytes([data[8], data[9]]), if encryption == ZipEncryption::Aes256 { 99 } else { 8 });
            }
        }
        let legacy: CompressionOptions = serde_json::from_value(serde_json::json!({"level":6,"password":"x","split_size":null})).unwrap();
        assert_eq!(legacy.zip_encryption, ZipEncryption::Aes256);
        assert!(serde_json::from_value::<CompressionOptions>(serde_json::json!({"level":6,"zip_encryption":"unknown"})).is_err());
    }
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[derive(Default)]
    struct TestRuntime {
        cancelled: Arc<AtomicBool>,
        cancel_after: std::cell::Cell<Option<usize>>,
    }

    #[test]
    fn native_split_creation_preserves_directories_passwords_and_cleans_temporary_zip() {
        use crate::services::zip_password::{encode, ZipPasswordEncoding};
        for mode in [ZipPasswordEncoding::Utf8, ZipPasswordEncoding::WindowsLocal] {
            for password in [None, Some("密码 + !测试")] {
                if password.is_some_and(|value| encode(value, mode).is_err()) { continue; }
                let temp = tempfile::tempdir().unwrap();
                let folder = temp.path().join("资料");
                std::fs::create_dir_all(folder.join("空目录")).unwrap();
                std::fs::write(folder.join("内容.txt"), b"folder payload").unwrap();
                let second = temp.path().join("第二份.txt");
                std::fs::write(&second, b"second payload").unwrap();
                let final_output = temp.path().join("bundle.zip");
                let output = crate::services::output_publish_transaction::staged_output_path(&final_output, "compress").unwrap();
                compress(&TestRuntime::default(), None, "split-test",
                    &[folder.to_string_lossy().into_owned(), second.to_string_lossy().into_owned()],
                    &output.to_string_lossy(), CompressionOptions {
                        password: password.map(str::to_owned), zip_password_encoding: mode,
                        split_size: Some(37), preserve_paths: Some(true), ..Default::default()
                    }).unwrap();
                let staged_first = std::path::PathBuf::from(format!("{}.001", output.display()));
                crate::services::compression_verification::verify_numeric_zip(&staged_first, password, || false).unwrap();
                assert!(!temp.path().join("bundle.zip.001").exists());
                crate::services::output_publish_transaction::publish_verified_volumes(&output, &final_output, || false).unwrap();
                assert!(!staged_first.exists());
                let first = temp.path().join("bundle.zip.001");
                crate::services::compression_verification::verify_numeric_zip(&first, password, || false).unwrap();
                let reader = crate::services::numeric_volume_reader::NumericVolumeReader::open(&first).unwrap();
                let mut archive = zip_aes::ZipArchive::new(reader).unwrap();
                for (name, expected) in [("资料/内容.txt", b"folder payload".as_slice()), ("第二份.txt", b"second payload".as_slice())] {
                    let mut entry = match password {
                        Some(value) => crate::services::zip_password::open_named_entry(&mut archive, name, value).unwrap(),
                        None => archive.by_name(name).unwrap(),
                    };
                    let mut bytes = Vec::new(); entry.read_to_end(&mut bytes).unwrap();
                    assert_eq!(bytes, expected);
                }
                assert!(archive.index_for_name("资料/空目录/").is_some());
                if password.is_none() {
                    // Exercise the independent engine used by the app for numeric split extraction.
                    let engine = crate::utils::archive_tools::find_7z_command().expect("7-Zip required for split interoperability");
                    let extracted = temp.path().join("independent-output");
                    let result = crate::utils::process::command(engine)
                        .arg("x").arg("-y").arg("-p-")
                        .arg(format!("-o{}", extracted.display())).arg(&first)
                        .output().unwrap();
                    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
                    assert!(extracted.join("资料/空目录").is_dir());
                    assert_eq!(std::fs::read(extracted.join("资料/内容.txt")).unwrap(), b"folder payload");
                    assert_eq!(std::fs::read(extracted.join("第二份.txt")).unwrap(), b"second payload");
                }
                assert!(!output.exists());
                assert!(std::fs::read_dir(temp.path()).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().starts_with(".split-source-")));
            }
        }
    }

    #[test]
    fn native_split_collision_preserves_existing_volume_and_removes_temporary_archive() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        std::fs::write(&source, b"source payload").unwrap();
        let output = temp.path().join("bundle.zip");
        let first = temp.path().join("bundle.zip.001");
        std::fs::write(&first, b"existing user data").unwrap();
        let result = compress(&TestRuntime::default(), None, "split-test",
            &[source.to_string_lossy().into_owned()], &output.to_string_lossy(),
            CompressionOptions { split_size: Some(37), ..Default::default() });
        assert!(result.is_err());
        assert_eq!(std::fs::read(first).unwrap(), b"existing user data");
        assert_eq!(std::fs::read(source).unwrap(), b"source payload");
        assert!(std::fs::read_dir(temp.path()).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().starts_with(".split-source-")));
    }

    #[test]
    fn native_split_cancellation_leaves_no_temporary_or_partial_output() {
        for checks in [0, 3] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("source.bin");
            std::fs::write(&source, vec![42; 1024 * 1024]).unwrap();
            let runtime = TestRuntime::default();
            runtime.cancel_after.set(Some(checks));
            let result = compress(&runtime, None, "cancel-split",
                &[source.to_string_lossy().into_owned()], &temp.path().join("result.zip").to_string_lossy(),
                CompressionOptions { split_size: Some(10), ..Default::default() });
            assert!(matches!(result.unwrap_err().downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
            let remaining: Vec<_> = std::fs::read_dir(temp.path()).unwrap().map(|entry| entry.unwrap().path()).collect();
            assert_eq!(remaining, vec![source]);
        }
    }

    #[test]
    fn requested_password_encoding_flows_through_real_creation_and_verification() {
        use crate::services::zip_password::{encode, ZipPasswordEncoding};
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("资料");
        std::fs::create_dir_all(folder.join("空目录")).unwrap();
        std::fs::write(folder.join("内容.txt"), b"real creation payload").unwrap();
        let password = "密码 + !测试";
        for encoding in [ZipPasswordEncoding::Utf8, ZipPasswordEncoding::WindowsLocal] {
            if encode(password, encoding).is_err() { continue; }
            let output = temp.path().join(format!("{encoding:?}.zip"));
            compress(&TestRuntime::default(), None, "encoding-test",
                &[folder.to_string_lossy().into_owned()], &output.to_string_lossy(),
                CompressionOptions { password: Some(password.into()), zip_password_encoding: encoding,
                    preserve_paths: Some(true), level: 6, ..Default::default() }).unwrap();
            assert!(crate::services::compression_verification::verify_native(
                crate::services::compression_format::CompressionRoute::Zip,
                &output, Some(password), || false).unwrap());
            assert!(crate::services::universal_engine::UniversalCliEngine::try_zip_password(&output, password).unwrap());
            assert!(!crate::services::universal_engine::UniversalCliEngine::try_zip_password(&output, "wrong").unwrap());
            if encoding == ZipPasswordEncoding::WindowsLocal {
                let engine = crate::utils::archive_tools::find_7z_command().expect("7-Zip interoperability engine");
                let destination = temp.path().join("independent");
                // Only this synthetic test password goes to argv, never a user's password.
                let result = crate::utils::process::command(engine).arg("x").arg("-y")
                    .arg(format!("-p{password}")).arg(format!("-o{}", destination.display()))
                    .arg(&output).output().unwrap();
                assert!(result.status.success(), "independent extraction failed");
                assert_eq!(std::fs::read(destination.join("资料/内容.txt")).unwrap(), b"real creation payload");
                assert!(destination.join("资料/空目录").is_dir());
            }
        }
        let old: CompressionOptions = serde_json::from_value(serde_json::json!({"level":6})).unwrap();
        assert_eq!(old.zip_password_encoding, ZipPasswordEncoding::Utf8);
        assert!(serde_json::from_value::<CompressionOptions>(serde_json::json!({"level":6,"zip_password_encoding":"unknown"})).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn unrepresentable_local_password_leaves_no_archive() {
        use crate::services::zip_password::{encode, ZipPasswordEncoding};
        let password = "🗝️";
        if encode(password, ZipPasswordEncoding::WindowsLocal).is_ok() { return; }
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        std::fs::write(&source, b"keep").unwrap();
        let output = temp.path().join("bad.zip");
        let result = compress(&TestRuntime::default(), None, "encoding-test",
            &[source.to_string_lossy().into_owned()], &output.to_string_lossy(),
            CompressionOptions { password: Some(password.into()),
                zip_password_encoding: ZipPasswordEncoding::WindowsLocal, ..Default::default() });
        assert!(result.is_err());
        assert!(!output.exists());
        assert_eq!(std::fs::read(source).unwrap(), b"keep");
    }

    #[test]
    fn retired_fast_pack_is_ignored_and_zip_payloads_remain_intact() {
        let legacy: CompressionOptions =
            serde_json::from_value(serde_json::json!({ "level": 6, "fast_pack": true })).unwrap();
        assert!(!legacy.fast_pack);
        assert!(serde_json::to_value(&legacy).unwrap().get("fast_pack").is_none());
        let temp = tempfile::tempdir().unwrap();
        let mut state = 0x12345678u32;
        let random: Vec<u8> = (0..1024 * 1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        let fixtures = [
            ("random.txt", random),
            ("text.jpg", vec![b'A'; 1024 * 1024]),
            ("small.bin", vec![1, 2, 3, 4]),
        ];
        let sources: Vec<String> = fixtures
            .iter()
            .map(|(name, bytes)| {
                let path = temp.path().join(name);
                std::fs::write(&path, bytes).unwrap();
                path.to_string_lossy().into_owned()
            })
            .collect();
        for encrypted in [false, true] {
            for fast_pack in [false, true] {
                let output = temp.path().join(format!("{encrypted}-{fast_pack}.zip"));
                compress(
                    &TestRuntime::default(),
                    None,
                    "fast-pack",
                    &sources,
                    output.to_str().unwrap(),
                    CompressionOptions {
                        fast_pack,
                        level: 6,
                        preserve_paths: Some(false),
                        password: encrypted.then(|| "test-password".into()),
                        ..Default::default()
                    },
                )
                .unwrap();
                let mut archive = zip_aes::ZipArchive::new(File::open(&output).unwrap()).unwrap();
                assert_eq!(archive.len(), fixtures.len());
                for (name, expected) in &fixtures {
                    let mut entry = if encrypted {
                        archive.by_name_decrypt(name, b"test-password").unwrap()
                    } else {
                        archive.by_name(name).unwrap()
                    };
                    let expected_method = zip_aes::CompressionMethod::Deflated;
                    assert_eq!(entry.compression(), expected_method);
                    let mut actual = Vec::new();
                    entry.read_to_end(&mut actual).unwrap();
                    assert_eq!(&actual, expected);
                }
                if encrypted {
                    assert!(archive
                        .by_name_decrypt("random.txt", b"wrong-password")
                        .is_err());
                }
            }
        }
    }

    impl CompressionRuntime for TestRuntime {
        fn check_cancellation(&self) -> Result<()> {
            if let Some(remaining) = self.cancel_after.get() {
                if remaining == 0 { self.cancelled.store(true, Ordering::SeqCst); }
                else { self.cancel_after.set(Some(remaining - 1)); }
            }
            if self.cancelled.load(Ordering::Relaxed) {
                Err(CompressionError::Cancelled.into())
            } else {
                Ok(())
            }
        }

        fn cancellation_flag(&self) -> Arc<AtomicBool> {
            self.cancelled.clone()
        }

        fn copy_buffer_size(&self) -> usize {
            256 * 1024
        }

        fn emit_log(
            &self,
            _window: &Window,
            _task_id: &str,
            _message: &str,
            _severity: TaskLogSeverity,
        ) {
        }

        fn emit_progress(
            &self,
            _window: &Window,
            _task_id: &str,
            _progress: f32,
            _current_file: Option<String>,
            _processed_bytes: u64,
            _total_bytes: u64,
        ) {
        }
    }

    #[test]
    fn plain_zip_round_trip_preserves_payload() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("payload.zip");
        std::fs::write(&source, b"native zip payload").expect("write source");

        compress(
            &TestRuntime::default(),
            None,
            "zip-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect("create zip");

        let mut archive =
            zip::ZipArchive::new(File::open(archive).expect("open zip")).expect("read zip");
        let mut payload = String::new();
        archive
            .by_name("payload.txt")
            .expect("payload entry")
            .read_to_string(&mut payload)
            .expect("read payload");
        assert_eq!(payload, "native zip payload");
    }

    #[test]
    fn cancellation_stops_before_creating_entries() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("cancelled.zip");
        std::fs::write(&source, b"cancelled payload").expect("write source");
        let runtime = TestRuntime::default();
        runtime.cancelled.store(true, Ordering::Relaxed);

        let error = compress(
            &runtime,
            None,
            "cancel-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect_err("cancelled zip must fail");
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
    }

    #[test]
    fn source_total_bytes_ignores_directories_and_counts_payloads() {
        let temp = tempfile::tempdir().expect("temp dir");
        let folder = temp.path().join("payloads");
        std::fs::create_dir_all(&folder).expect("create folder");
        std::fs::write(folder.join("alpha.bin"), vec![1u8; 8192]).expect("alpha");
        std::fs::write(folder.join("beta.bin"), vec![2u8; 4096]).expect("beta");
        let entries =
            compression_entries::collect(&[folder.to_string_lossy().to_string()], true, true)
                .expect("collect entries");

        assert_eq!(source_total_bytes(&entries).expect("total bytes"), 12_288);
    }
}

#[cfg(test)]
#[path = "zip_bzip2_1534_tests.rs"]
mod bzip2_tests;

#[cfg(test)]
#[path = "zip_ppmd_1535_tests.rs"]
mod ppmd_tests;

#[cfg(test)]
#[path = "zip_lzma_1538_tests.rs"]
mod lzma_tests;

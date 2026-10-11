use long_compress_assistant::services::split_compression::SplitCompressionService;
use long_compress_assistant::models::compression::CompressionOptions;
use tempfile::tempdir;
use std::fs::{self, File};
use std::io::Write;
use anyhow::Result;

#[test]
fn numeric_split_preserves_encrypted_zip_directories_and_multiple_files() {
    check_encrypted_split_interoperability("密码 + !测试", "密码 + !测试".as_bytes());
}

#[test]
fn numeric_split_ascii_password_interoperability() {
    check_encrypted_split_interoperability("password + !test", b"password + !test");
}

#[test]
#[cfg(windows)]
fn windows_local_zip_password_interoperability() {
    let password = "密码 + !测试";
    use long_compress_assistant::services::zip_password::{encode, ZipPasswordEncoding};
    let encoded = match encode(password, ZipPasswordEncoding::WindowsLocal) {
        Ok(bytes) => bytes,
        Err(error) => {
            // A machine whose ACP cannot represent Chinese must reject, not substitute.
            assert!(error.to_string().contains("unavailable") || error.to_string().contains("replacement"));
            return;
        }
    };
    check_encrypted_split_interoperability(password, &encoded);
}

fn check_encrypted_split_interoperability(password: &str, password_bytes: &[u8]) {
    use std::io::Read;
    let temp = tempdir().unwrap();
    let archive = temp.path().join("private.zip");
    let mut zip = zip_aes::ZipWriter::new(File::create(&archive).unwrap());
    let options = zip_aes::write::SimpleFileOptions::default()
        .compression_method(zip_aes::CompressionMethod::Deflated)
        .with_aes_encryption_bytes(zip_aes::AesMode::Aes256, password_bytes);
    zip.add_directory("资料/空目录/", options).unwrap();
    for (name, payload) in [("资料/说明.txt", "中文内容"), ("第二份.txt", "another file")] {
        zip.start_file(name, options).unwrap();
        zip.write_all(payload.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
    let result = SplitCompressionService::split_archive_file(&archive, &temp.path().join("分享.zip"), 17, || Ok(())).unwrap();
    assert!(result.part_count > 1);
    let joined: Vec<u8> = result.part_files.iter().flat_map(|p| {
        let bytes = fs::read(p).unwrap();
        assert!(bytes.len() <= 17);
        bytes
    }).collect();
    assert_eq!(joined, fs::read(&archive).unwrap());
    assert_eq!(result.total_size, joined.len() as u64);
    let mut reader = zip_aes::ZipArchive::new(std::io::Cursor::new(joined)).unwrap();
    assert!(reader.by_name_decrypt("资料/空目录/", password_bytes).unwrap().is_dir());
    assert!(reader.by_name_decrypt("资料/说明.txt", b"wrong").is_err());
    for (name, expected) in [("资料/说明.txt", "中文内容"), ("第二份.txt", "another file")] {
        let mut entry = reader.by_name_decrypt(name, password_bytes).unwrap();
        let mut actual = String::new();
        entry.read_to_string(&mut actual).unwrap();
        assert_eq!(actual, expected);
    }
    // Synthetic fixture password only; production creation does not use CLI passwords.
    let engine = long_compress_assistant::utils::archive_tools::find_7z_command().expect("7-Zip required for interoperability");
    let direct = long_compress_assistant::utils::process::command(&engine)
        .arg("t").arg(format!("-p{password}")).arg(&archive).output().unwrap();
    // Windows 7-Zip converts CLI passwords to ACP bytes. The explicit local
    // mode is the interoperability contract; UTF-8 remains a separate dialect.
    // Assert that boundary instead of requiring a CP936 process to use UTF-8.
    #[cfg(windows)]
    let independent_expected = long_compress_assistant::services::zip_password::encode(
        password, long_compress_assistant::services::zip_password::ZipPasswordEncoding::WindowsLocal,
    ).is_ok_and(|bytes| bytes == password_bytes);
    #[cfg(not(windows))]
    let independent_expected = true;
    assert_eq!(direct.status.success(), independent_expected, "single archive encoding contract");
    let extracted = temp.path().join("independent");
    let result = long_compress_assistant::utils::process::command(engine)
        .arg("x").arg("-y").arg(format!("-p{password}"))
        .arg(format!("-o{}", extracted.display())).arg(&result.part_files[0])
        .output().unwrap();
    assert_eq!(result.status.success(), independent_expected, "{}", String::from_utf8_lossy(&result.stderr));
    if !independent_expected {
        assert!(String::from_utf8_lossy(&result.stderr).contains("Wrong password"));
        return; // Native full-payload assertions above are still mandatory.
    }
    assert!(extracted.join("资料/空目录").is_dir());
    assert_eq!(fs::read_to_string(extracted.join("资料/说明.txt")).unwrap(), "中文内容");
    assert_eq!(fs::read_to_string(extracted.join("第二份.txt")).unwrap(), "another file");
}

#[test]
fn numeric_split_rolls_back_on_cancellation_and_preserves_late_collisions() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.zip");
    fs::write(&source, vec![42u8; 200]).unwrap();
    let output = temp.path().join("result.zip");
    let mut checks = 0;
    let result = SplitCompressionService::split_archive_file(&source, &output, 50, || {
        checks += 1;
        if checks == 3 { anyhow::bail!("cancelled"); }
        Ok(())
    });
    assert!(result.is_err());
    assert!(!temp.path().join("result.zip.001").exists());
    let collision = temp.path().join("result.zip.002");
    let mut checks = 0;
    let result = SplitCompressionService::split_archive_file(&source, &output, 50, || {
        checks += 1;
        if checks == 3 { fs::write(&collision, b"someone else's output")?; }
        Ok(())
    });
    assert!(result.is_err());
    assert!(!temp.path().join("result.zip.001").exists());
    assert_eq!(fs::read(&collision).unwrap(), b"someone else's output");
    assert_eq!(fs::read(&source).unwrap(), vec![42u8; 200]);
}

#[test]
fn numeric_split_handles_exact_boundaries_and_more_than_999_parts() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.zip");
    fs::write(&source, vec![7; 1001]).unwrap();
    let output = temp.path().join("many.zip");
    assert!(SplitCompressionService::split_archive_file(&source, &output, 0, || Ok(())).is_err());
    let result = SplitCompressionService::split_archive_file(&source, &output, 1, || Ok(())).unwrap();
    assert_eq!(result.part_count, 1001);
    assert_eq!(result.part_files[999], temp.path().join("many.zip.1000"));
    assert!(!temp.path().join("many.zip.1002").exists());
}

#[test]
fn numeric_split_rejects_empty_sources_and_existing_volume_sets() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("empty.zip");
    fs::write(&source, b"").unwrap();
    let output = temp.path().join("output.zip");
    assert!(SplitCompressionService::split_archive_file(&source, &output, 10, || Ok(())).is_err());
    assert!(!temp.path().join("output.zip.001").exists());
    fs::write(&source, b"archive").unwrap();
    let existing = temp.path().join("output.zip.003");
    fs::write(&existing, b"keep me").unwrap();
    assert!(SplitCompressionService::split_archive_file(&source, &output, 10, || Ok(())).is_err());
    assert_eq!(fs::read(existing).unwrap(), b"keep me");
    assert!(!temp.path().join("output.zip.001").exists());
}

#[tokio::test]
async fn unsupported_split_combinations_fail_without_creating_output() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.txt");
    fs::write(&source, b"payload").unwrap();
    for (input, password, expected) in [
        (source.clone(), Some("密码测试".to_string()), "Encrypted split ZIP"),
        (temp.path().to_path_buf(), None, "regular files only"),
    ] {
        let output = temp.path().join("unsupported.zip");
        let error = SplitCompressionService::new().compress_to_split_zips(
            &[input.to_string_lossy().into_owned()], &output,
            CompressionOptions { split_size: Some(10), password, ..Default::default() },
        ).await.unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert!(!output.exists());
        assert!(!temp.path().join("unsupported.zip.001").exists());
        assert_eq!(fs::read(&source).unwrap(), b"payload");
    }
}

#[tokio::test]
async fn test_split_compression_basic() {
    let temp_dir = tempdir().unwrap();
    let test_file1 = temp_dir.path().join("test1.txt");
    let test_file2 = temp_dir.path().join("test2.txt");

    let mut file1 = File::create(&test_file1).unwrap();
    file1.write_all(b"This is test file 1 content").unwrap();
    let mut file2 = File::create(&test_file2).unwrap();
    file2.write_all(b"This is test file 2 content with more data").unwrap();
    drop(file1);
    drop(file2);

    let output_zip = temp_dir.path().join("split_test.zip");
    let service = SplitCompressionService::new();
    let files = vec![
        test_file1.to_string_lossy().to_string(),
        test_file2.to_string_lossy().to_string(),
    ];
    let options = CompressionOptions {
        split_size: Some(50),
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    assert!(result.is_ok(), "分卷压缩失败: {:?}", result.err());
    let result = result.unwrap();
    assert!(result.part_count >= 1, "至少应该有一个分卷");
    assert_eq!(result.part_files.len(), result.part_count, "分卷文件数量不匹配");

    for part_file in &result.part_files {
        assert!(part_file.exists(), "分卷文件不存在: {:?}", part_file);
        let metadata = fs::metadata(part_file).unwrap();
        assert!(metadata.len() > 0, "分卷文件为空: {:?}", part_file);
    }
}

#[tokio::test]
async fn test_split_compression_no_split() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"Small test file").unwrap();
    drop(file);

    let output_zip = temp_dir.path().join("no_split.zip");
    let service = SplitCompressionService::new();
    let files = vec![test_file.to_string_lossy().to_string()];
    let options = CompressionOptions {
        split_size: None,
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    assert!(result.is_ok(), "不分卷压缩失败: {:?}", result.err());
    let result = result.unwrap();
    assert_eq!(result.part_count, 1, "应该只有一个分卷");
    assert_eq!(result.part_files.len(), 1, "应该只有一个分卷文件");
    assert!(result.part_files[0].extension().unwrap_or_default() == "zip", "应该是.zip文件");
}

#[tokio::test]
async fn test_split_compression_zero_split_size() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"Test file").unwrap();
    drop(file);

    let output_zip = temp_dir.path().join("zero_split.zip");
    let service = SplitCompressionService::new();
    let files = vec![test_file.to_string_lossy().to_string()];
    let options = CompressionOptions {
        split_size: Some(0),
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    assert!(result.is_ok(), "分卷大小为0时压缩失败: {:?}", result.err());
    let result = result.unwrap();
    assert_eq!(result.part_count, 1, "分卷大小为0时应该只有一个分卷");
}

#[tokio::test]
async fn test_split_compression_large_file() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("large.txt");
    let mut file = File::create(&test_file).unwrap();
    let data = vec![b'X'; 1024];
    file.write_all(&data).unwrap();
    drop(file);

    let output_zip = temp_dir.path().join("large_split.zip");
    let service = SplitCompressionService::new();
    let files = vec![test_file.to_string_lossy().to_string()];
    let options = CompressionOptions {
        split_size: Some(500),
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    if let Ok(result) = result {
        assert!(result.part_count >= 1, "至少应该有一个分卷");
    }
}

#[tokio::test]
async fn test_split_compression_nonexistent_file() {
    let temp_dir = tempdir().unwrap();
    let output_zip = temp_dir.path().join("error_test.zip");
    let service = SplitCompressionService::new();
    let files = vec!["nonexistent_file.txt".to_string()];
    let options = CompressionOptions {
        split_size: Some(100),
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    assert!(result.is_err(), "不存在的文件应该失败");
}

#[tokio::test]
async fn standard_split_zip_roundtrips_from_first_volume() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("payload.txt");
    fs::write(&source, b"standard split payload").unwrap();
    let output = temp.path().join("standard.zip");
    let service = SplitCompressionService::new();
    let result = service
        .compress_to_split_zips(
            &[source.to_string_lossy().to_string()],
            &output,
            CompressionOptions {
                split_size: Some(10),
                ..Default::default()
            },
        )
        .await
        .expect("create standard split ZIP");

    assert!(result.part_files[0].to_string_lossy().ends_with(".zip.001"));
    let extracted = temp.path().join("extracted");
    let engine = long_compress_assistant::utils::archive_tools::find_7z_command()
        .expect("bundled archive engine");
    let status = long_compress_assistant::utils::process::command(engine)
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", extracted.display()))
        .arg(&result.part_files[0])
        .status()
        .expect("extract standard split ZIP");
    assert!(status.success());
    assert_eq!(fs::read(extracted.join("payload.txt")).unwrap(), b"standard split payload");
}

#[tokio::test]
async fn preexisting_split_volumes_are_never_deleted_on_validation_failure() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("payload.txt");
    fs::write(&source, b"payload").unwrap();
    let output = temp.path().join("existing.zip");
    let existing = temp.path().join("existing.zip.001");
    fs::write(&existing, b"user data").unwrap();
    let result = SplitCompressionService::new()
        .compress_to_split_zips(
            &[source.to_string_lossy().to_string()],
            &output,
            CompressionOptions {
                split_size: Some(10),
                ..Default::default()
            },
        )
        .await;
    assert!(result.is_err());
    assert_eq!(fs::read(existing).unwrap(), b"user data");
}

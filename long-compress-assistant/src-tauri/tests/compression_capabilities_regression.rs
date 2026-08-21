use std::fs;

use long_compress_assistant::models::compression::CompressionOptions;
use long_compress_assistant::services::archive_engine::ArchiveEngine;
use long_compress_assistant::services::compression_service::CompressionService;
use long_compress_assistant::services::universal_engine::UniversalCliEngine;
use long_compress_assistant::utils::archive_tools::find_7z_command;
use tempfile::tempdir;

fn compression_options(format: Option<&str>) -> CompressionOptions {
    CompressionOptions {
        format: format.map(str::to_string),
        level: 6,
        password: None,
        split_size: None,
        create_solid_archive: false,
        preserve_paths: Some(true),
        delete_after: false,
        verify_after: true,
        allow_insecure_password_cli: false,
    }
}

#[test]
fn validates_single_file_stream_formats_need_one_regular_file() {
    let temp = tempdir().expect("temp dir");
    let first_file = temp.path().join("first.txt");
    let second_file = temp.path().join("second.txt");
    fs::write(&first_file, b"first").expect("first fixture");
    fs::write(&second_file, b"second").expect("second fixture");

    let first_source = first_file.to_string_lossy().to_string();
    let second_source = second_file.to_string_lossy().to_string();
    let output = temp
        .path()
        .join("first.txt.gz")
        .to_string_lossy()
        .to_string();

    let valid_format = CompressionService::validate_compression_request(
        std::slice::from_ref(&first_source),
        &output,
        &compression_options(Some("gz")),
    )
    .expect("single file gzip is supported");
    assert_eq!(valid_format, "gz");

    let multi_file_error = CompressionService::validate_compression_request(
        &[first_source.clone(), second_source],
        &output,
        &compression_options(Some("gz")),
    );
    assert!(multi_file_error.is_err());

    let directory_source = temp.path().to_string_lossy().to_string();
    let directory_error = CompressionService::validate_compression_request(
        &[directory_source],
        &output,
        &compression_options(Some("xz")),
    );
    assert!(directory_error.is_err());
}

#[test]
fn validates_password_support_per_format() {
    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("source.txt");
    fs::write(&source_file, b"source").expect("source fixture");

    let source = source_file.to_string_lossy().to_string();
    let output = temp
        .path()
        .join("archive.zip")
        .to_string_lossy()
        .to_string();

    let mut zip_options = compression_options(Some("zip"));
    zip_options.password = Some("secret".to_string());
    // ZIP 现在支持密码（通过 7z CLI）
    assert_eq!(
        CompressionService::validate_compression_request(
            std::slice::from_ref(&source),
            &output,
            &zip_options
        ).expect("ZIP 密码压缩应被支持"),
        "zip"
    );

    let mut seven_zip_options = compression_options(Some("7z"));
    seven_zip_options.password = Some("secret".to_string());
    assert_eq!(
        CompressionService::validate_compression_request(
            std::slice::from_ref(&source),
            &output,
            &seven_zip_options
        )
        .expect("7z password is supported"),
        "7z"
    );

    let mut rar_options = compression_options(Some("rar"));
    rar_options.password = Some("secret".to_string());
    assert_eq!(
        CompressionService::validate_compression_request(&[source], &output, &rar_options)
            .expect("RAR password is supported by the user-installed encoder"),
        "rar"
    );
}

#[test]
fn validates_split_archive_support() {
    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("source.txt");
    fs::write(&source_file, b"source").expect("source fixture");

    let source = source_file.to_string_lossy().to_string();
    let output = temp
        .path()
        .join("archive.zip")
        .to_string_lossy()
        .to_string();

    let mut options = compression_options(Some("zip"));
    options.split_size = Some(1024);

    // 分卷压缩现在被支持
    assert!(
        CompressionService::validate_compression_request(&[source], &output, &options).is_ok()
    );
}

#[test]
fn infers_zip_from_output_path_when_format_missing() {
    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("source.txt");
    fs::write(&source_file, b"source").expect("source fixture");

    let source = source_file.to_string_lossy().to_string();
    let output = temp
        .path()
        .join("archive.zip")
        .to_string_lossy()
        .to_string();

    let format = CompressionService::validate_compression_request(
        &[source],
        &output,
        &compression_options(None),
    )
    .expect("zip output extension should infer zip format");

    assert_eq!(format, "zip");
}

#[test]
fn non_native_password_formats_require_an_explicit_7z_output() {
    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("source.txt");
    fs::write(&source_file, b"source").expect("source fixture");
    let source = source_file.to_string_lossy().to_string();

    let mut options = compression_options(Some("tar.gz"));
    options.password = Some("secret".to_string());

    let wrong_output = temp.path().join("archive.tar.gz").to_string_lossy().to_string();
    assert!(CompressionService::validate_compression_request(std::slice::from_ref(&source), &wrong_output, &options).is_err());

    let encrypted_output = temp.path().join("archive.7z").to_string_lossy().to_string();
    assert_eq!(
        CompressionService::validate_compression_request(&[source], &encrypted_output, &options)
            .expect("7z fallback should be explicit"),
        "7z"
    );
}

#[test]
fn infers_common_compression_formats_from_output_path() {
    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("source.txt");
    fs::write(&source_file, b"source").expect("source fixture");
    let source = source_file.to_string_lossy().to_string();

    for (file_name, expected) in [
        ("archive.7z", "7z"),
        ("archive.tar.gz", "tar.gz"),
        ("archive.tgz", "tar.gz"),
        ("archive.tar.bz2", "tar.bz2"),
        ("archive.tbz2", "tar.bz2"),
        ("archive.tar.xz", "tar.xz"),
        ("archive.txz", "tar.xz"),
        ("archive.tar.zst", "tar.zst"),
        ("archive.tzst", "tar.zst"),
        ("archive.zstd", "zst"),
        ("archive.lzma", "lzma"),
    ] {
        let output = temp.path().join(file_name).to_string_lossy().to_string();
        let format = CompressionService::validate_compression_request(
            std::slice::from_ref(&source),
            &output,
            &compression_options(None),
        )
        .unwrap_or_else(|err| panic!("{} should infer as {}: {}", file_name, expected, err));
        assert_eq!(format, expected);
    }
}

#[test]
fn exposes_backend_compression_capability_matrix() {
    let capabilities = CompressionService::compression_format_capabilities();

    assert!(capabilities.iter().any(|capability| capability.format == "tar.zst"));
    assert!(capabilities.iter().any(|capability| capability.format == "lzma" && capability.requires_7za));
    assert!(capabilities.iter().any(|capability| capability.format == "rar" && capability.requires_winrar));
    assert!(capabilities.iter().any(|capability| capability.format == "zip" && capability.supports_split));
    assert!(capabilities.iter().any(|capability| capability.format == "7z" && !capability.supports_split));
    assert!(capabilities.iter().any(|capability| capability.format == "wim" && capability.requires_7za));
}

#[test]
fn every_declared_alias_resolves_through_the_public_service_facade() {
    for capability in CompressionService::compression_format_capabilities() {
        for extension in capability.extensions {
            let resolved = CompressionService::find_compression_format_capability(extension)
                .unwrap_or_else(|| panic!("{} alias should resolve", extension));
            assert_eq!(
                resolved.format,
                capability.format,
                "{} should resolve to {}",
                extension,
                capability.format
            );
            assert_eq!(
                CompressionService::infer_compression_format(
                    "ignored.output",
                    Some(extension)
                ),
                capability.format
            );
        }
    }
}

#[test]
fn every_backend_compressible_format_can_be_explicitly_validated() {
    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("source.txt");
    fs::write(&source_file, b"source").expect("source fixture");
    let source = source_file.to_string_lossy().to_string();

    for capability in CompressionService::compression_format_capabilities() {
        if !capability.can_compress {
            continue;
        }
        let format = CompressionService::validate_compression_request(
            std::slice::from_ref(&source),
            &temp.path().join(format!("archive.{}", capability.extensions[0])).to_string_lossy(),
            &compression_options(Some(capability.format)),
        )
        .unwrap_or_else(|err| panic!("{} should validate: {}", capability.format, err));

        assert_eq!(format, capability.format);
    }
}

#[test]
fn universal_engine_uses_auto_rename_when_overwrite_is_disabled() {
    assert_eq!(UniversalCliEngine::overwrite_mode_arg(false), "-aou");
    assert_eq!(UniversalCliEngine::overwrite_mode_arg(true), "-aoa");
}

#[test]
fn source_cleanup_only_targets_files_after_distinct_output_exists() {
    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("source.txt");
    let second_file = temp.path().join("second.txt");
    let output_file = temp.path().join("archive.zip");
    fs::write(&source_file, b"source").expect("source fixture");
    fs::write(&second_file, b"second").expect("second fixture");
    fs::write(&output_file, b"archive").expect("output fixture");

    let removable = CompressionService::removable_compressed_sources(
        &[
            source_file.to_string_lossy().to_string(),
            temp.path().to_string_lossy().to_string(),
            output_file.to_string_lossy().to_string(),
            second_file.to_string_lossy().to_string(),
        ],
        &output_file.to_string_lossy(),
    )
    .expect("cleanup candidates");

    assert_eq!(removable.len(), 2);
    assert!(removable.contains(&source_file.canonicalize().expect("source canonical")));
    assert!(removable.contains(&second_file.canonicalize().expect("second canonical")));
}

#[test]
fn source_cleanup_waits_for_existing_output() {
    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("source.txt");
    let missing_output = temp.path().join("archive.zip");
    fs::write(&source_file, b"source").expect("source fixture");

    let removable = CompressionService::removable_compressed_sources(
        &[source_file.to_string_lossy().to_string()],
        &missing_output.to_string_lossy(),
    )
    .expect("cleanup candidates");

    assert!(removable.is_empty());
}

#[tokio::test]
async fn universal_engine_detects_password_zip_created_by_bundled_7za() {
    let Some(seven_zip) = find_7z_command() else {
        eprintln!("Skipping password ZIP test because 7za is unavailable");
        return;
    };

    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("secret.txt");
    let archive = temp.path().join("secret.zip");
    fs::write(&source_file, b"top secret").expect("secret fixture");

    let output = std::process::Command::new(seven_zip)
        .arg("a")
        .arg("-tzip")
        .arg("-popen-sesame")
        .arg("-y")
        .arg(&archive)
        .arg(&source_file)
        .output()
        .expect("7za should run");

    assert!(output.status.success(), "7za failed: {}", String::from_utf8_lossy(&output.stderr));

    let engine = UniversalCliEngine::new();
    assert!(engine.requires_password(&archive).await.expect("password detection"));
    assert!(engine.try_password(&archive, "open-sesame").await.expect("correct password"));
    assert!(!engine.try_password(&archive, "wrong-password").await.expect("wrong password"));
}

#[tokio::test]
async fn universal_engine_reads_split_zip_encryption_metadata_without_full_test() {
    let Some(seven_zip) = find_7z_command() else {
        eprintln!("Skipping split ZIP metadata test because 7-Zip is unavailable");
        return;
    };

    let temp = tempdir().expect("temp dir");
    let source_file = temp.path().join("split-source.bin");
    let archive = temp.path().join("split.zip");
    let payload: Vec<u8> = (0..512 * 1024)
        .map(|index| ((index * 17 + index / 127) % 256) as u8)
        .collect();
    fs::write(&source_file, payload).expect("split source fixture");

    let output = std::process::Command::new(seven_zip)
        .arg("a")
        .arg("-tzip")
        .arg("-v128k")
        .arg("-y")
        .arg(&archive)
        .arg(&source_file)
        .output()
        .expect("7-Zip should create split ZIP fixture");
    assert!(
        output.status.success(),
        "7-Zip split fixture failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let first_volume = temp.path().join("split.zip.001");
    assert!(first_volume.exists(), "first split volume should exist");
    let engine = UniversalCliEngine::new();
    assert!(!engine
        .requires_password(&first_volume)
        .await
        .expect("split ZIP encryption metadata"));
}

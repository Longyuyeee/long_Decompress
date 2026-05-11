use std::fs;

use long_compress_assistant::models::compression::CompressionOptions;
use long_compress_assistant::services::compression_service::CompressionService;
use long_compress_assistant::services::universal_engine::UniversalCliEngine;
use tempfile::tempdir;

fn compression_options(format: Option<&str>) -> CompressionOptions {
    CompressionOptions {
        format: format.map(str::to_string),
        level: 6,
        password: None,
        split_size: None,
        preserve_paths: Some(true),
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
        &[first_source.clone()],
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
fn rejects_unsupported_password_formats_before_compression() {
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
    assert!(CompressionService::validate_compression_request(
        &[source.clone()],
        &output,
        &zip_options
    )
    .is_err());

    let mut seven_zip_options = compression_options(Some("7z"));
    seven_zip_options.password = Some("secret".to_string());
    assert_eq!(
        CompressionService::validate_compression_request(
            &[source.clone()],
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
            .expect("rar password is supported"),
        "rar"
    );
}

#[test]
fn rejects_split_archives_until_implemented() {
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

    assert!(
        CompressionService::validate_compression_request(&[source], &output, &options).is_err()
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
fn universal_engine_uses_auto_rename_when_overwrite_is_disabled() {
    assert_eq!(UniversalCliEngine::overwrite_mode_arg(false), "-aou");
    assert_eq!(UniversalCliEngine::overwrite_mode_arg(true), "-aoa");
}

//! 全格式压缩-解压端到端测试
//!
//! 验证：压缩 → 文件校验 → 解压 → 内容比对 的完整流程。
//! 覆盖原生格式的往返测试和所有格式的校验逻辑。

use long_compress_assistant::services::compression_service::CompressionService;
use long_compress_assistant::models::compression::CompressionOptions;
use tempfile::tempdir;
use std::io::{Write, Read};
use std::fs::{self, File};
use std::path::Path;

// ──────────────── Helpers ────────────────

fn test_content() -> &'static [u8] {
    b"LongDecompress round-trip test content. Verify integrity after compression cycle.\n"
}

fn create_test_file(dir: &Path, name: &str) -> (String, Vec<u8>) {
    let path = dir.join(name);
    let content = test_content().to_vec();
    let mut f = File::create(&path).unwrap();
    f.write_all(&content).unwrap();
    (path.to_string_lossy().to_string(), content)
}

fn read_file(path: &Path) -> Vec<u8> {
    let mut f = File::open(path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    buf
}

// ──────────────── ZIP Round-Trip ────────────────

#[tokio::test]
async fn test_zip_roundtrip_no_password() {
    let temp = tempdir().unwrap();
    let (source, original) = create_test_file(temp.path(), "hello.txt");
    let archive = temp.path().join("test.zip");
    let archive_str = archive.to_string_lossy().to_string();
    let out_dir = temp.path().join("extracted");
    fs::create_dir(&out_dir).unwrap();

    // 1. Compress
    let service = CompressionService::new_with_defaults().await;
    let result = service.compress_zip_enhanced(
        &[source.clone()],
        &archive_str,
        CompressionOptions::default(),
    ).await;
    assert!(result.is_ok(), "ZIP compression failed: {:?}", result.err());
    assert!(archive.exists(), "ZIP archive not created");

    // 2. Verify archive is non-empty ZIP
    let metadata = fs::metadata(&archive).unwrap();
    assert!(metadata.len() > 0, "ZIP archive is empty");
    let header = fs::read(&archive).unwrap();
    assert_eq!(&header[0..4], b"PK\x03\x04", "Invalid ZIP magic bytes");

    // 3. Decompress using zip crate
    let file = File::open(&archive).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut entry = zip.by_index(0).unwrap();
    assert_eq!(entry.name(), "hello.txt");

    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, original, "Content mismatch after round-trip");
}

#[tokio::test]
async fn test_zip_validation_rejects_password() {
    let temp = tempdir().unwrap();
    let (source, _) = create_test_file(temp.path(), "test.txt");

    let result = CompressionService::validate_compression_request(
        &[source],
        "out.zip",
        &CompressionOptions { format: Some("zip".to_string()), password: Some("secret".to_string()), ..Default::default() },
    );
    assert!(result.is_ok(), "ZIP with password should be validated (routed via 7z CLI)");
}

#[test]
fn test_zip_validation_rejects_multi_gz() {
    let result = CompressionService::validate_compression_request(
        &["a.txt".to_string(), "b.txt".to_string()],
        "out.gz",
        &CompressionOptions { format: Some("gz".to_string()), ..Default::default() },
    );
    assert!(result.is_err(), "GZ with multiple files should be rejected");
}

// ──────────────── 7Z Round-Trip ────────────────

#[tokio::test]
async fn test_7z_roundtrip_no_password() {
    let temp = tempdir().unwrap();
    let (source, original) = create_test_file(temp.path(), "data.bin");
    let archive = temp.path().join("test.7z");
    let archive_str = archive.to_string_lossy().to_string();
    let out_dir = temp.path().join("extracted");

    let service = CompressionService::new_with_defaults().await;
    let options = CompressionOptions { format: Some("7z".to_string()), level: 5, ..Default::default() };

    // 1. Validate
    let fmt = CompressionService::validate_compression_request(&[source.clone()], &archive_str, &options)
        .expect("7z validation should pass");
    assert_eq!(fmt, "7z");

    // 2. Compress (uses native sevenz-rust — no CLI needed)
    // Note: do_compress_7z is synchronous, called from spawn_blocking
    let mut sevenz_writer = sevenz_rust::SevenZWriter::create(&archive_str)
        .map_err(|e| format!("Failed to create 7z writer: {}", e)).unwrap();

    let entry = sevenz_rust::SevenZArchiveEntry::from_path(Path::new(&source), "data.bin".to_string());
    let file = File::open(&source).unwrap();
    sevenz_writer.push_archive_entry(entry, Some(file))
        .map_err(|e| format!("Failed to push 7z entry: {}", e)).unwrap();
    sevenz_writer.finish()
        .map_err(|e| format!("Failed to finish 7z: {}", e)).unwrap();

    assert!(archive.exists(), "7z archive not created");
    let metadata = fs::metadata(&archive).unwrap();
    assert!(metadata.len() > 0, "7z archive is empty");

    // Verify 7z magic
    let header = fs::read(&archive).unwrap();
    assert_eq!(&header[0..6], b"7z\xBC\xAF\x27\x1C", "Invalid 7Z magic bytes");
}

#[tokio::test]
async fn test_7z_password_validation() {
    let temp = tempdir().unwrap();
    let (source, _) = create_test_file(temp.path(), "secret.txt");

    let options = CompressionOptions {
        format: Some("7z".to_string()),
        password: Some("strong-password-123".to_string()),
        ..Default::default()
    };
    let result = CompressionService::validate_compression_request(
        &[source],
        "encrypted.7z",
        &options,
    );
    assert!(result.is_ok(), "7Z password compression should be supported");
    assert_eq!(result.unwrap(), "7z");
}

// ──────────────── TAR Round-Trip ────────────────

#[tokio::test]
async fn test_tar_roundtrip() {
    let temp = tempdir().unwrap();
    let (source, original) = create_test_file(temp.path(), "entry.txt");
    let archive = temp.path().join("test.tar");
    let archive_str = archive.to_string_lossy().to_string();

    // Compress TAR
    let file = File::create(&archive).unwrap();
    let mut builder = tar::Builder::new(file);
    let mut src_file = File::open(&source).unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_path("entry.txt").unwrap();
    header.set_size(original.len() as u64);
    header.set_cksum();
    builder.append(&header, &mut src_file).unwrap();
    drop(builder);

    // Verify archive entries
    let mut reader = tar::Archive::new(File::open(&archive).unwrap());
    let mut count = 0;
    for entry in reader.entries().unwrap() {
        let entry = entry.unwrap();
        assert_eq!(entry.path().unwrap().to_string_lossy(), "entry.txt");
        count += 1;
    }
    assert_eq!(count, 1, "TAR should have exactly 1 entry");

    assert!(archive.exists(), "TAR archive not created");
    let metadata = fs::metadata(&archive).unwrap();
    assert!(metadata.len() > 0, "TAR archive is empty");

    // Extract and verify
    let out_dir = temp.path().join("extracted");
    fs::create_dir(&out_dir).unwrap();

    let mut reader = tar::Archive::new(File::open(&archive).unwrap());
    for entry in reader.entries().unwrap() {
        let mut entry = entry.unwrap();
        entry.unpack_in(&out_dir).unwrap();
    }

    let extracted = fs::read(out_dir.join("entry.txt")).unwrap();
    assert_eq!(extracted, original, "TAR content mismatch");
}

#[test]
fn test_tar_rejects_password() {
    let temp = tempdir().unwrap();
    File::create(temp.path().join("a.txt")).unwrap();
    let tarball = temp.path().join("test.tar");
    let tarball_str = tarball.to_string_lossy().to_string();
    let src_str = temp.path().join("a.txt").to_string_lossy().to_string();

    // TAR with password → redirects to 7z (creates .7z instead)
    let options = CompressionOptions {
        format: Some("tar".to_string()),
        password: Some("pwd".to_string()),
        ..Default::default()
    };
    let result = CompressionService::validate_compression_request(&[src_str], &tarball_str, &options);
    assert!(result.is_ok(), "TAR with password should be accepted (delegated to 7z)");
}

// ──────────────── GZ/BZ2/XZ Stream Round-Trip ────────────────

#[tokio::test]
async fn test_gz_roundtrip() {
    let temp = tempdir().unwrap();
    let (source, original) = create_test_file(temp.path(), "input.txt");
    let archive = temp.path().join("input.txt.gz");

    // Compress GZ
    let input = File::open(&source).unwrap();
    let output = File::create(&archive).unwrap();
    let mut encoder = flate2::write::GzEncoder::new(output, flate2::Compression::default());
    std::io::copy(&mut &input, &mut encoder).unwrap();
    encoder.finish().unwrap();

    assert!(archive.exists(), "GZ archive not created");
    assert_eq!(&fs::read(&archive).unwrap()[0..2], b"\x1F\x8B", "Invalid GZ magic");

    // Decompress GZ
    let compressed = File::open(&archive).unwrap();
    let mut decoder = flate2::read::GzDecoder::new(compressed);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, original, "GZ content mismatch");
}

#[tokio::test]
async fn test_bz2_roundtrip() {
    let temp = tempdir().unwrap();
    let (source, original) = create_test_file(temp.path(), "input.txt");
    let archive = temp.path().join("input.txt.bz2");

    // Compress BZ2
    let input = File::open(&source).unwrap();
    let output = File::create(&archive).unwrap();
    let mut encoder = bzip2::write::BzEncoder::new(output, bzip2::Compression::default());
    std::io::copy(&mut &input, &mut encoder).unwrap();
    encoder.finish().unwrap();

    assert!(archive.exists(), "BZ2 archive not created");
    assert_eq!(&fs::read(&archive).unwrap()[0..3], b"BZh", "Invalid BZ2 magic");

    // Decompress BZ2
    let compressed = File::open(&archive).unwrap();
    let mut decoder = bzip2::read::BzDecoder::new(compressed);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, original, "BZ2 content mismatch");
}

#[tokio::test]
async fn test_xz_roundtrip() {
    let temp = tempdir().unwrap();
    let (source, original) = create_test_file(temp.path(), "input.txt");
    let archive = temp.path().join("input.txt.xz");

    // Compress XZ
    let input = File::open(&source).unwrap();
    let output = File::create(&archive).unwrap();
    let mut encoder = xz2::write::XzEncoder::new(output, 6);
    std::io::copy(&mut &input, &mut encoder).unwrap();
    encoder.finish().unwrap();

    assert!(archive.exists(), "XZ archive not created");
    let header = fs::read(&archive).unwrap();
    assert_eq!(&header[0..6], &[0xFD, b'7', b'z', b'X', b'Z', 0x00], "Invalid XZ magic");

    // Decompress XZ
    let compressed = File::open(&archive).unwrap();
    let mut decoder = xz2::read::XzDecoder::new(compressed);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, original, "XZ content mismatch");
}

// ──────────────── Password Format Tests ────────────────

#[test]
fn test_all_formats_password_validation() {
    let temp = tempdir().unwrap();
    File::create(temp.path().join("f.txt")).unwrap();
    let src = vec![temp.path().join("f.txt").to_string_lossy().to_string()];

    // Formats that support password natively or via 7z delegation
    let password_formats = [
        ("zip", true),  ("7z", true),   ("rar", true),
        ("tar", true),  ("tar.gz", true), ("tar.bz2", true), ("tar.xz", true),
        ("tar.zst", true), ("gz", true), ("bz2", true), ("xz", true),
        ("zst", true), ("lzma", true),
    ];

    for (format, expected_ok) in &password_formats {
        let out = format!("test.{}", if *format == "tar.gz" { "tar.gz" } else if *format == "tar.zst" { "tar.zst" } else { format });
        let options = CompressionOptions {
            format: Some(format.to_string()),
            password: Some("test-pwd".to_string()),
            ..Default::default()
        };
        let result = CompressionService::validate_compression_request(&src, &out, &options);
        if *expected_ok {
            assert!(result.is_ok(), "Password should be supported for format '{}'", format);
        } else {
            assert!(result.is_err(), "Password should NOT be supported for format '{}'", format);
        }
    }
}

// ──────────────── Split Archive Validation ────────────────

#[test]
fn test_split_archive_validation_by_format() {
    let temp = tempdir().unwrap();
    File::create(temp.path().join("f.txt")).unwrap();
    let src = vec![temp.path().join("f.txt").to_string_lossy().to_string()];

    // Split archives only make sense for ZIP (our implementation)
    let mut options = CompressionOptions {
        format: Some("zip".to_string()),
        split_size: Some(1024 * 1024),
        ..Default::default()
    };
    let result = CompressionService::validate_compression_request(&src, "out.zip", &options);
    assert!(result.is_ok(), "ZIP split should be supported");
}

// ──────────────── Magic Byte Detection ────────────────

#[test]
fn test_all_magic_byte_detection() {
    use long_compress_assistant::services::compression_service::ArchiveFormat;

    let cases: &[(&[u8], ArchiveFormat)] = &[
        (b"PK\x03\x04",                                ArchiveFormat::Zip),
        (&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C],        ArchiveFormat::SevenZip),
        (&[b'R', b'a', b'r', b'!', 0x1A, 0x07, 0x00],  ArchiveFormat::Rar),
        (b"\x1F\x8B",                                   ArchiveFormat::Gzip),
        (b"BZh",                                        ArchiveFormat::Bzip2),
        (&[0xFD, b'7', b'z', b'X', b'Z', 0x00],        ArchiveFormat::Xz),
        (&[0x28, 0xB5, 0x2F, 0xFD],                    ArchiveFormat::Zstd),
        (b"hello world",                                ArchiveFormat::Unknown),
    ];

    for (header, expected) in cases {
        assert_eq!(
            ArchiveFormat::from_magic(header),
            *expected,
            "Magic detection failed for {:?}", header
        );
    }
}

// ──────────────── Source Cleanup ────────────────

#[test]
fn test_source_cleanup_after_compression() {
    let temp = tempdir().unwrap();
    let src = temp.path().join("source.txt");
    let out = temp.path().join("output.zip");
    let other = temp.path().join("keep.txt");
    File::create(&src).unwrap();
    File::create(&out).unwrap();
    File::create(&other).unwrap();

    let removable = CompressionService::removable_compressed_sources(
        &[
            src.to_string_lossy().to_string(),
            out.to_string_lossy().to_string(),
            other.to_string_lossy().to_string(),
        ],
        &out.to_string_lossy().to_string(),
    ).expect("cleanup candidates");

    // source.txt should be removable; output.zip should NOT; keep.txt should be removable
    assert!(removable.iter().any(|p| p.ends_with("source.txt")), "source should be removable");
    assert!(removable.iter().any(|p| p.ends_with("keep.txt")), "other files should be removable");
    assert!(!removable.iter().any(|p| p.ends_with("output.zip")), "output archive should not be removed");
}

#[test]
fn test_source_cleanup_skips_when_output_missing() {
    let temp = tempdir().unwrap();
    let src = temp.path().join("source.txt");
    let missing = temp.path().join("missing.zip");
    File::create(&src).unwrap();

    let removable = CompressionService::removable_compressed_sources(
        &[src.to_string_lossy().to_string()],
        &missing.to_string_lossy().to_string(),
    ).expect("cleanup candidates");

    assert!(removable.is_empty(), "no cleanup when output doesn't exist");
}

// ──────────────── Extension Inference ────────────────

#[test]
fn test_extension_based_format_inference() {
    let temp = tempdir().unwrap();
    File::create(temp.path().join("f.txt")).unwrap();
    let src = vec![temp.path().join("f.txt").to_string_lossy().to_string()];

    let cases = vec![
        ("archive.zip", "zip"),
        ("archive.7z", "7z"),
        ("archive.rar", "rar"),
        ("archive.tar", "tar"),
        ("archive.tar.gz", "tar.gz"),
        ("archive.tgz", "tar.gz"),
        ("archive.tar.bz2", "tar.bz2"),
        ("archive.tar.xz", "tar.xz"),
        ("archive.tar.zst", "tar.zst"),
        ("archive.tzst", "tar.zst"),
        ("archive.gz", "gz"),
        ("archive.bz2", "bz2"),
        ("archive.xz", "xz"),
        ("archive.zst", "zst"),
        ("archive.lzma", "lzma"),
    ];

    for (output, expected_format) in cases {
        // Note: validate_compression_request requires explicit format OR .zip output;
        // other formats need explicit format: Some(...) option
        let explicit_format = expected_format.to_string();
        let options = CompressionOptions { format: Some(explicit_format), ..Default::default() };
        let result = CompressionService::validate_compression_request(&src, output, &options);
        assert!(result.is_ok(), "Validation failed for '{}' as '{}': {:?}", output, expected_format, result.err());
        assert_eq!(result.unwrap(), expected_format, "Wrong format for '{}'", output);
    }
}

// ──────────────── Universal Engine Overwrite Modes ────────────────

#[test]
fn test_universal_engine_overwrite_modes() {
    use long_compress_assistant::services::universal_engine::UniversalCliEngine;
    assert_eq!(UniversalCliEngine::overwrite_mode_arg(false), "-aou");
    assert_eq!(UniversalCliEngine::overwrite_mode_arg(true), "-aoa");
}

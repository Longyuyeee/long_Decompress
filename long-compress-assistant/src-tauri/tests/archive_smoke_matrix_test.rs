use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use long_compress_assistant::services::universal_engine::UniversalCliEngine;
use long_compress_assistant::utils::archive_tools::find_7z_command;
use tempfile::tempdir;

fn write_fixture(root: &Path) -> PathBuf {
    let source = root.join("payload.txt");
    fs::write(&source, b"archive smoke matrix\n").expect("fixture");
    source
}

fn create_zip_like(path: &Path, entry_name: &str, content: &[u8]) {
    let file = File::create(path).expect("zip-like file");
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(entry_name, options).expect("zip entry");
    zip.write_all(content).expect("zip content");
    zip.finish().expect("zip finish");
}

fn create_tar(path: &Path, source: &Path) {
    let file = File::create(path).expect("tar file");
    let mut builder = tar::Builder::new(file);
    builder.append_path_with_name(source, "payload.txt").expect("tar entry");
    builder.finish().expect("tar finish");
}

fn create_tar_gz(path: &Path, source: &Path) {
    let file = File::create(path).expect("tar.gz file");
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    builder.append_path_with_name(source, "payload.txt").expect("tar.gz entry");
    let encoder = builder.into_inner().expect("tar.gz inner");
    encoder.finish().expect("tar.gz finish");
}

fn create_tar_bz2(path: &Path, source: &Path) {
    let file = File::create(path).expect("tar.bz2 file");
    let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    builder.append_path_with_name(source, "payload.txt").expect("tar.bz2 entry");
    let encoder = builder.into_inner().expect("tar.bz2 inner");
    encoder.finish().expect("tar.bz2 finish");
}

fn create_tar_xz(path: &Path, source: &Path) {
    let file = File::create(path).expect("tar.xz file");
    let encoder = xz2::write::XzEncoder::new(file, 6);
    let mut builder = tar::Builder::new(encoder);
    builder.append_path_with_name(source, "payload.txt").expect("tar.xz entry");
    let encoder = builder.into_inner().expect("tar.xz inner");
    encoder.finish().expect("tar.xz finish");
}

fn create_lzma_with_7za(path: &Path, source: &Path) -> bool {
    let Some(seven_zip) = find_7z_command() else {
        return false;
    };

    let output = std::process::Command::new(seven_zip)
        .arg("a")
        .arg("-tlzma")
        .arg("-y")
        .arg(path)
        .arg(source)
        .output()
        .expect("run 7za lzma");

    output.status.success()
}

#[tokio::test]
async fn universal_engine_lists_and_tests_common_archive_matrix() {
    let temp = tempdir().expect("temp dir");
    let source = write_fixture(temp.path());

    let archives = [
        ("zip", temp.path().join("sample.zip")),
        ("jar", temp.path().join("sample.jar")),
        ("apk", temp.path().join("sample.apk")),
        ("tar", temp.path().join("sample.tar")),
        ("tar.gz", temp.path().join("sample.tar.gz")),
        ("tar.bz2", temp.path().join("sample.tar.bz2")),
        ("tar.xz", temp.path().join("sample.tar.xz")),
    ];

    for (label, path) in &archives {
        match *label {
            "zip" | "jar" | "apk" => create_zip_like(path, "payload.txt", b"archive smoke matrix\n"),
            "tar" => create_tar(path, &source),
            "tar.gz" => create_tar_gz(path, &source),
            "tar.bz2" => create_tar_bz2(path, &source),
            "tar.xz" => create_tar_xz(path, &source),
            _ => unreachable!(),
        }
    }

    let lzma_path = temp.path().join("sample.lzma");
    let mut matrix: Vec<(&str, PathBuf)> = archives
        .iter()
        .map(|(label, path)| (*label, path.clone()))
        .collect();
    if create_lzma_with_7za(&lzma_path, &source) {
        matrix.push(("lzma", lzma_path));
    }

    for (label, path) in matrix {
        UniversalCliEngine::test_integrity(&path, None)
            .await
            .unwrap_or_else(|err| panic!("{} integrity failed: {}", label, err));

        if matches!(label, "zip" | "jar" | "apk" | "tar" | "lzma") {
            let entries = UniversalCliEngine::list_contents(&path, None)
                .await
                .unwrap_or_else(|err| panic!("{} listing failed: {}", label, err));

            assert!(
                entries.iter().any(|entry| entry.ends_with("payload.txt") || entry.contains("sample")),
                "{} listing did not expose expected content: {:?}",
                label,
                entries
            );
        }
    }
}

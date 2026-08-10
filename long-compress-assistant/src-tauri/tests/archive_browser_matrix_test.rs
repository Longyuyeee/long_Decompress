use long_compress_assistant::services::archive_browser::browse_archive;
use long_compress_assistant::utils::archive_tools::find_7z_command;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn create_zip(path: &Path) {
    let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
    writer.add_directory("非常长的目录名称/第二层目录/", zip::write::FileOptions::default()).unwrap();
    writer.start_file("非常长的目录名称/第二层目录/说明文档.txt", zip::write::FileOptions::default()).unwrap();
    writer.write_all(b"zip browser matrix").unwrap();
    writer.finish().unwrap();
}

fn create_tar_variants(root: &Path) -> Vec<PathBuf> {
    let payload = root.join("payload.txt");
    std::fs::write(&payload, b"tar browser matrix").unwrap();
    let tar_path = root.join("sample.tar");
    let mut tar = tar::Builder::new(File::create(&tar_path).unwrap());
    tar.append_path_with_name(&payload, "nested/payload.txt").unwrap();
    tar.finish().unwrap();

    let gz_path = root.join("sample.tar.gz");
    let encoder = flate2::write::GzEncoder::new(File::create(&gz_path).unwrap(), flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    tar.append_path_with_name(&payload, "nested/payload.txt").unwrap();
    tar.into_inner().unwrap().finish().unwrap();

    let bz2_path = root.join("sample.tar.bz2");
    let encoder = bzip2::write::BzEncoder::new(File::create(&bz2_path).unwrap(), bzip2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    tar.append_path_with_name(&payload, "nested/payload.txt").unwrap();
    tar.into_inner().unwrap().finish().unwrap();

    let xz_path = root.join("sample.tar.xz");
    let encoder = xz2::write::XzEncoder::new(File::create(&xz_path).unwrap(), 6);
    let mut tar = tar::Builder::new(encoder);
    tar.append_path_with_name(&payload, "nested/payload.txt").unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    vec![tar_path, gz_path, bz2_path, xz_path]
}

fn create_7z(root: &Path, password: Option<&str>) -> PathBuf {
    let engine = find_7z_command().expect("bundled 7-Zip engine");
    let payload = root.join(password.map(|_| "secret.txt").unwrap_or("plain.txt"));
    std::fs::write(&payload, b"seven zip browser matrix").unwrap();
    let archive = root.join(password.map(|_| "secret.7z").unwrap_or("plain.7z"));
    let mut command = std::process::Command::new(engine);
    command.arg("a").arg("-t7z").arg("-y");
    if let Some(password) = password {
        command.arg("-mhe=on").arg(format!("-p{password}"));
    }
    let output = command.arg(&archive).arg(&payload).output().unwrap();
    assert!(output.status.success(), "7Z fixture creation failed: {}", String::from_utf8_lossy(&output.stderr));
    archive
}

#[tokio::test]
async fn browses_real_zip_7z_password_and_tar_matrix() {
    let temp = tempfile::tempdir().unwrap();
    let zip_path = temp.path().join("long-path.zip");
    create_zip(&zip_path);
    let zip = browse_archive(&zip_path, None).await.unwrap();
    assert_eq!(zip.total_files, 1);
    assert!(zip.entries.iter().any(|entry| entry.path.ends_with("说明文档.txt")));

    for path in create_tar_variants(temp.path()) {
        let listed = browse_archive(&path, None).await.unwrap();
        assert!(listed.entries.iter().any(|entry| entry.path.ends_with("payload.txt")), "missing TAR payload in {}", path.display());
    }

    let plain = create_7z(temp.path(), None);
    let listed = browse_archive(&plain, None).await.unwrap();
    assert_eq!(listed.total_files, 1);
    assert!(listed.entries.iter().any(|entry| entry.name == "plain.txt"));

    let encrypted = create_7z(temp.path(), Some("browser-password"));
    assert!(browse_archive(&encrypted, Some("wrong-password")).await.is_err());
    let listed = browse_archive(&encrypted, Some("browser-password")).await.unwrap();
    assert!(listed.encrypted);
    assert!(listed.entries.iter().any(|entry| entry.name == "secret.txt"));
}

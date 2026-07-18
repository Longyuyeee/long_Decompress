use long_compress_assistant::utils::archive_tools::{
    detect_archive_engine_capabilities, find_7z_command,
};
use std::fs;
use std::path::Path;
use std::process::Command;

fn run(engine: &str, cwd: &Path, args: &[String]) {
    let output = Command::new(engine)
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("archive engine should start");
    assert!(
        output.status.success(),
        "engine command failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn write_ar_sample(path: &Path, name: &str, content: &[u8]) {
    let mut bytes = b"!<arch>\n".to_vec();
    let header = format!(
        "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
        format!("{name}/"),
        "0",
        "0",
        "0",
        "100644",
        content.len()
    );
    assert_eq!(header.len(), 60);
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(content);
    if content.len() % 2 != 0 {
        bytes.push(b'\n');
    }
    fs::write(path, bytes).expect("AR sample should be written");
}

#[test]
fn bundled_engine_reports_full_dynamic_capabilities() {
    let capabilities = detect_archive_engine_capabilities();
    assert!(capabilities.available, "{}", capabilities.message);
    assert!(capabilities.full_engine, "{}", capabilities.message);
    assert!(capabilities
        .version
        .as_deref()
        .is_some_and(|version| version >= "26.02"));
    for required in ["APFS", "Ar", "Ext", "QCOW", "VDI", "VMDK", "wim"] {
        assert!(
            capabilities
                .formats
                .iter()
                .any(|format| format.name == required),
            "missing dynamic capability {required}",
        );
    }
    assert!(capabilities
        .formats
        .iter()
        .any(|format| format.name == "wim" && format.can_create));
}

#[test]
fn creates_and_extracts_real_archive_sample_matrix() {
    let engine = find_7z_command().expect("bundled full 7-Zip engine should exist");
    let temp = tempfile::tempdir().expect("temp directory should be created");
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(
        source_dir.join("payload.txt"),
        b"Long Decompress real archive matrix\n",
    )
    .unwrap();

    for (format, extension) in [("7z", "7z"), ("zip", "zip"), ("tar", "tar"), ("wim", "wim")] {
        let archive = temp.path().join(format!("sample.{extension}"));
        run(
            &engine,
            &source_dir,
            &[
                "a".into(),
                format!("-t{format}"),
                "-y".into(),
                archive.to_string_lossy().to_string(),
                "payload.txt".into(),
            ],
        );
        let output = temp.path().join(format!("extract-{format}"));
        run(
            &engine,
            temp.path(),
            &[
                "x".into(),
                "-y".into(),
                archive.to_string_lossy().to_string(),
                format!("-o{}", output.display()),
            ],
        );
        assert_eq!(
            fs::read(output.join("payload.txt")).unwrap(),
            b"Long Decompress real archive matrix\n"
        );
    }

    let encrypted = temp.path().join("encrypted.7z");
    run(
        &engine,
        &source_dir,
        &[
            "a".into(),
            "-t7z".into(),
            "-pMatrixPassword!".into(),
            "-mhe=on".into(),
            encrypted.to_string_lossy().to_string(),
            "payload.txt".into(),
        ],
    );
    run(
        &engine,
        temp.path(),
        &[
            "t".into(),
            "-pMatrixPassword!".into(),
            encrypted.to_string_lossy().to_string(),
        ],
    );

    let ar_archive = temp.path().join("sample.ar");
    write_ar_sample(&ar_archive, "payload.txt", b"real ar sample\n");
    let ar_output = temp.path().join("extract-ar");
    run(
        &engine,
        temp.path(),
        &[
            "x".into(),
            "-y".into(),
            ar_archive.to_string_lossy().to_string(),
            format!("-o{}", ar_output.display()),
        ],
    );
    assert_eq!(
        fs::read(ar_output.join("payload.txt")).unwrap(),
        b"real ar sample\n"
    );
}

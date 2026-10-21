//! Real method-98 streams across editing, comparison and reference materialization.
use super::{
    comparison_package as package, source_comparison as comparison,
    source_comparison_content as content, zip_edit as edit, zip_password::ZipPasswordEncoding,
};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use zip_aes::unstable::write::FileOptionsExt;
use zip_aes::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};
const TEXT: &[u8] = b"actual ppmd content, not a changed method marker";
fn flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
fn observer() -> edit::Observer {
    Arc::new(|_| {})
}
fn fixture(path: &Path, encryption: edit::Encryption, password: &[u8]) {
    let mut z = ZipWriter::new(File::create(path).unwrap());
    z.set_comment("archive comment").unwrap();
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Ppmd)
        .compression_level(Some(1))
        .last_modified_time(zip_aes::DateTime::from_date_and_time(2020, 2, 3, 4, 5, 6).unwrap())
        .unix_permissions(0o100644)
        .into_full_options()
        .with_file_comment("entry comment");
    let options = match encryption {
        edit::Encryption::None => options,
        edit::Encryption::Aes256 => {
            options.with_aes_encryption_bytes(zip_aes::AesMode::Aes256, password)
        }
        edit::Encryption::Zipcrypto => options.with_deprecated_encryption(password).unwrap(),
        _ => unreachable!(),
    };
    z.start_file("nested/合同.txt", options.clone()).unwrap();
    z.write_all(TEXT).unwrap();
    z.start_file("unselected.txt", options.clone()).unwrap();
    z.write_all(b"not requested").unwrap();
    z.start_file("empty.txt", options).unwrap();
    z.add_directory(
        "empty/",
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )
    .unwrap();
    z.finish().unwrap();
}
fn rename() -> Vec<edit::OriginalEdit> {
    vec![edit::OriginalEdit {
        source_path: "nested/合同.txt".into(),
        target_path: Some("delivery/合同.txt".into()),
    }]
}
fn bytes(
    path: &Path,
    name: &str,
    password: &str,
    encoding: Option<ZipPasswordEncoding>,
) -> Vec<u8> {
    let mut z = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let index = z.index_for_name(name).unwrap();
    let mut out = Vec::new();
    super::zip_password::open_entry_with_encoding(&mut z, index, password, encoding)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}
fn request(
    left: &Path,
    right: &Path,
    password: &str,
    encoding: Option<ZipPasswordEncoding>,
) -> content::Request {
    content::Request {
        left: comparison::read(left, flag(), |_| {}).unwrap().source,
        right: comparison::read(right, flag(), |_| {}).unwrap().source,
        paths: vec!["nested/合同.txt".into()],
        left_credentials: content::Credentials::default(),
        right_credentials: content::Credentials {
            password: password.into(),
            encoding,
        },
    }
}
fn folder(path: &Path) {
    fs::create_dir_all(path.join("nested")).unwrap();
    fs::write(path.join("nested/合同.txt"), TEXT).unwrap();
}
#[test]
fn ppmd1562_plain_edit_preserves_compressed_bytes_method_comments_and_time() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source.zip");
    fixture(&source, edit::Encryption::None, b"");
    let before = fs::read(&source).unwrap();
    let addition = t.path().join("new.txt");
    fs::write(&addition, b"added bytes").unwrap();
    let changes = edit::prepare(&[addition.to_string_lossy().into_owned()], "")
        .unwrap()
        .into_iter()
        .map(|input| edit::Change {
            input,
            action: "add".into(),
            replaces_path: None,
        })
        .collect::<Vec<_>>();
    let out = t.path().join("edited.zip");
    edit::save_with_edits(
        &source,
        &edit::inspect(&source).unwrap().revision,
        &changes,
        &rename(),
        &out,
        "deflate",
        6,
        flag(),
        observer(),
    )
    .unwrap();
    assert_eq!(bytes(&out, "delivery/合同.txt", "", None), TEXT);
    assert_eq!(fs::read(&source).unwrap(), before);
    let mut old = ZipArchive::new(File::open(source).unwrap()).unwrap();
    let mut new = ZipArchive::new(File::open(out).unwrap()).unwrap();
    assert_eq!(new.comment(), old.comment());
    let old = old.by_name("nested/合同.txt").unwrap();
    let new_entry = new.by_name("delivery/合同.txt").unwrap();
    assert_eq!(new_entry.compression(), CompressionMethod::Ppmd);
    assert_eq!(new_entry.comment(), old.comment());
    assert_eq!(new_entry.last_modified(), old.last_modified());
    assert_eq!(new_entry.unix_mode(), old.unix_mode());
    drop(new_entry);
    drop(old);
    let mut a = Vec::new();
    old_raw(
        &mut ZipArchive::new(File::open(t.path().join("source.zip")).unwrap()).unwrap(),
        "nested/合同.txt",
        &mut a,
    );
    let mut b = Vec::new();
    old_raw(&mut new, "delivery/合同.txt", &mut b);
    assert_eq!(a, b);
    assert_eq!(
        new.by_name("new.txt").unwrap().compression(),
        CompressionMethod::Deflated
    );
}
fn old_raw(z: &mut ZipArchive<File>, name: &str, bytes: &mut Vec<u8>) {
    let i = z.index_for_name(name).unwrap();
    z.by_index_raw(i).unwrap().read_to_end(bytes).unwrap();
}
#[test]
fn ppmd1562_encrypted_edit_keeps_method_and_rekeys_with_explicit_encoding() {
    for scheme in [edit::Encryption::Aes256, edit::Encryption::Zipcrypto] {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("protected.zip");
        let secret = super::zip_password::encode("原包密码", ZipPasswordEncoding::Cp936).unwrap();
        fixture(&source, scheme, &secret);
        let before = fs::read(&source).unwrap();
        let catalog = edit::inspect(&source).unwrap();
        for (label, mode, password) in [
            ("keep", edit::OutputMode::Keep, "原包密码"),
            ("plain", edit::OutputMode::None, ""),
            ("rekey", edit::OutputMode::Aes256, "new-password"),
        ] {
            let out = t.path().join(format!("{label}.zip"));
            let credentials = edit::Credentials {
                source_password: "原包密码".into(),
                source_encoding: Some(ZipPasswordEncoding::Cp936),
                output_mode: mode,
                output_password: "new-password".into(),
                ..Default::default()
            };
            edit::save_encrypted(
                &source,
                &catalog.revision,
                &[],
                &rename(),
                &out,
                "store",
                0,
                &credentials,
                flag(),
                observer(),
            )
            .unwrap();
            assert_eq!(
                bytes(
                    &out,
                    "delivery/合同.txt",
                    password,
                    Some(ZipPasswordEncoding::Utf8)
                ),
                TEXT
            );
            assert!(bytes(&out, "empty.txt", password, Some(ZipPasswordEncoding::Utf8)).is_empty());
            let mut z = ZipArchive::new(File::open(&out).unwrap()).unwrap();
            assert_eq!(
                z.by_index_raw(z.index_for_name("delivery/合同.txt").unwrap())
                    .unwrap()
                    .compression(),
                CompressionMethod::Ppmd
            );
            assert_eq!(
                edit::inspect(&out).unwrap().encrypted,
                mode != edit::OutputMode::None
            );
        }
        assert_eq!(fs::read(source).unwrap(), before);
    }
}
#[test]
fn ppmd1562_password_failure_preserves_original_edits_and_existing_output() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("protected.zip");
    fixture(&source, edit::Encryption::Aes256, b"secret");
    let before = fs::read(&source).unwrap();
    let catalog = edit::inspect(&source).unwrap();
    let out = t.path().join("output.zip");
    for password in ["", "wrong"] {
        assert!(edit::save_encrypted(
            &source,
            &catalog.revision,
            &[],
            &rename(),
            &out,
            "deflate",
            6,
            &edit::Credentials {
                source_password: password.into(),
                ..Default::default()
            },
            flag(),
            observer()
        )
        .is_err());
        assert!(!out.exists());
    }
    fs::write(&out, b"existing").unwrap();
    assert!(edit::save_encrypted(
        &source,
        &catalog.revision,
        &[],
        &rename(),
        &out,
        "deflate",
        6,
        &edit::Credentials {
            source_password: "secret".into(),
            ..Default::default()
        },
        flag(),
        observer()
    )
    .is_err());
    assert_eq!(fs::read(out).unwrap(), b"existing");
    assert_eq!(fs::read(source).unwrap(), before);
}
#[test]
fn ppmd1562_folder_and_zip_content_comparison_reads_real_plain_and_password_bytes() {
    for scheme in [
        edit::Encryption::None,
        edit::Encryption::Aes256,
        edit::Encryption::Zipcrypto,
    ] {
        let t = tempfile::tempdir().unwrap();
        let left = t.path().join("folder");
        folder(&left);
        let right = t.path().join("source.zip");
        let secret = super::zip_password::encode("原包密码", ZipPasswordEncoding::Cp936).unwrap();
        fixture(&right, scheme, &secret);
        let req = request(&left, &right, "原包密码", Some(ZipPasswordEncoding::Cp936));
        let result = content::run(&req, flag(), |_| {}, |_| {}).unwrap();
        assert!(matches!(
            result.entries[0].kind.as_str(),
            "same" | "metadata"
        ));
        if scheme != edit::Encryption::None {
            let mut bad = req;
            bad.right_credentials.password = "wrong".into();
            assert_eq!(
                content::run(&bad, flag(), |_| {}, |_| {}).unwrap().entries[0].kind,
                "unreadable"
            );
        }
    }
}
#[test]
fn ppmd1562_numeric_password_material_uses_exact_range_and_detects_missing_or_changed_members() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source.zip");
    fixture(&source, edit::Encryption::Aes256, b"secret");
    let raw = fs::read(&source).unwrap();
    fs::remove_file(&source).unwrap();
    let mut parts = Vec::new();
    for (i, chunk) in raw.chunks(80).enumerate() {
        let p = t.path().join(format!("source.zip.{:03}", i + 1));
        fs::write(&p, chunk).unwrap();
        parts.push(p);
    }
    let source = comparison::read(&parts[1], flag(), |_| {}).unwrap().source;
    let prepared = package::prepare(
        &[package::Part {
            source: source.clone(),
            paths: Some(vec!["nested/合同.txt".into()]),
            encoding: None,
        }],
        &t.path().join("seeds"),
        flag(),
    )
    .unwrap();
    let changes = prepared
        .inputs
        .iter()
        .cloned()
        .map(|input| edit::Change {
            input,
            action: "add".into(),
            replaces_path: None,
        })
        .collect::<Vec<_>>();
    let seed = Path::new(&prepared.source_path);
    let revision = edit::inspect(seed).unwrap().revision;
    let out = t.path().join("new.zip");
    package::save(
        seed,
        &revision,
        &changes,
        &[],
        &out,
        "deflate",
        6,
        &edit::Credentials::default(),
        &[package::Credential {
            path: source.path.clone(),
            password: "secret".into(),
            encoding: None,
        }],
        flag(),
        observer(),
    )
    .unwrap();
    assert_eq!(bytes(&out, "nested/合同.txt", "", None), TEXT);
    let mut z = ZipArchive::new(File::open(out).unwrap()).unwrap();
    assert!(z.by_name("unselected.txt").is_err());
    assert!(edit::inspect(&parts[1]).is_err());
    let old = fs::read(&parts[1]).unwrap();
    fs::write(&parts[1], b"changed member").unwrap();
    assert!(!package::check_inputs(&prepared.inputs)
        .values()
        .next()
        .unwrap()
        .is_ok());
    fs::write(&parts[1], old).unwrap();
    fs::remove_file(&parts[1]).unwrap();
    assert!(comparison::read(&parts[0], flag(), |_| {}).is_err());
}
fn damage(path: &Path, kind: &str) {
    let mut raw = fs::read(path).unwrap();
    let central = raw.windows(4).position(|p| p == b"PK\x01\x02").unwrap();
    match kind {
        "size" => {
            let n = u32::from_le_bytes(raw[central + 24..central + 28].try_into().unwrap());
            raw[central + 24..central + 28].copy_from_slice(&(n + 1).to_le_bytes());
        }
        "crc" => {
            raw[central + 16] ^= 1;
        }
        "truncated-stream" => {
            raw[central + 20..central + 24].copy_from_slice(&8u32.to_le_bytes());
        }
        _ => unreachable!(),
    };
    fs::write(path, raw).unwrap();
}
#[test]
fn ppmd1562_bad_crc_size_and_truncated_stream_never_publish_or_become_content_evidence() {
    for kind in ["crc", "size", "truncated-stream"] {
        let t = tempfile::tempdir().unwrap();
        let left = t.path().join("folder");
        folder(&left);
        let source = t.path().join("bad.zip");
        fixture(&source, edit::Encryption::None, b"");
        damage(&source, kind);
        let before = fs::read(&source).unwrap();
        let out = t.path().join("new.zip");
        assert!(
            edit::save_with_edits(
                &source,
                &edit::inspect(&source).unwrap().revision,
                &[],
                &rename(),
                &out,
                "deflate",
                6,
                flag(),
                observer()
            )
            .is_err(),
            "{kind}"
        );
        assert!(!out.exists());
        let mut req = request(&left, &source, "", None);
        // Match recorded sizes to actually read both sides even for the forged central size.
        if kind == "size" {
            fs::write(left.join("nested/合同.txt"), [TEXT, b"x"].concat()).unwrap();
            req = request(&left, &source, "", None);
        }
        assert_eq!(
            content::run(&req, flag(), |_| {}, |_| {}).unwrap().entries[0].kind,
            "unreadable",
            "{kind}"
        );
        assert_eq!(fs::read(source).unwrap(), before);
    }
}

fn single(path: &Path) {
    let mut writer = ZipWriter::new(File::create(path).unwrap());
    writer
        .start_file(
            "nested/合同.txt",
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Ppmd)
                .compression_level(Some(1)),
        )
        .unwrap();
    writer.write_all(TEXT).unwrap();
    writer.finish().unwrap();
}
fn data_start(raw: &[u8]) -> usize {
    30 + u16::from_le_bytes(raw[26..28].try_into().unwrap()) as usize
        + u16::from_le_bytes(raw[28..30].try_into().unwrap()) as usize
}
#[test]
fn ppmd1562_independent_restart_cutoff_keep_real_parameters_and_extract_after_raw_copy() {
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/archive-engine/7z.exe");
    let payload = b"Real independently encoded PPMd ZIP repeated text. ".repeat(400);
    for (level, restoration) in [(5, 0), (9, 1)] {
        let t = tempfile::tempdir().unwrap();
        let nested = t.path().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("合同.txt"), &payload).unwrap();
        let source = t.path().join("source.zip");
        let result = crate::utils::process::command(&engine)
            .current_dir(t.path())
            .args(["a", "-tzip", "-mm=PPMd"])
            .arg(format!("-mx={level}"))
            .arg(&source)
            .arg("nested")
            .output()
            .unwrap();
        assert!(result.status.success());
        let mut old = ZipArchive::new(File::open(&source).unwrap()).unwrap();
        let entry = old.by_name("nested/合同.txt").unwrap();
        assert_eq!(entry.compression(), CompressionMethod::Ppmd);
        drop(entry);
        let mut compressed = Vec::new();
        old_raw(&mut old, "nested/合同.txt", &mut compressed);
        let parameters = u16::from_le_bytes(compressed[..2].try_into().unwrap());
        assert_eq!(parameters >> 12, restoration);
        assert_eq!(bytes(&source, "nested/合同.txt", "", None), payload);
        let out = t.path().join("edited.zip");
        edit::save_with_edits(
            &source,
            &edit::inspect(&source).unwrap().revision,
            &[],
            &rename(),
            &out,
            "deflate",
            6,
            flag(),
            observer(),
        )
        .unwrap();
        let mut new = ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let mut kept = Vec::new();
        old_raw(&mut new, "delivery/合同.txt", &mut kept);
        assert_eq!(kept, compressed);
        let decoded = crate::utils::process::command(&engine)
            .args(["x", "-so"])
            .arg(out)
            .arg("delivery/合同.txt")
            .output()
            .unwrap();
        assert!(decoded.status.success());
        assert_eq!(decoded.stdout, payload);
    }
}
#[test]
fn ppmd1562_late_truncation_unsupported_restore_trailing_bytes_and_forged_size_are_rejected() {
    for kind in [
        "late-tail",
        "order",
        "freeze",
        "unknown-restore",
        "trailing",
        "matching-prefix",
    ] {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("bad.zip");
        single(&source);
        let mut raw = fs::read(&source).unwrap();
        let start = data_start(&raw);
        let end = raw.windows(4).rposition(|s| s == b"PK\x05\x06").unwrap();
        let mut central = u32::from_le_bytes(raw[end + 16..end + 20].try_into().unwrap()) as usize;
        match kind {
            "late-tail" => {
                raw.remove(central - 1);
                central -= 1;
            }
            "order" => raw[start] &= 0xf0,
            "freeze" => raw[start + 1] = (raw[start + 1] & 0x0f) | 0x20,
            "unknown-restore" => raw[start + 1] |= 0xf0,
            "trailing" => {
                raw.insert(central, 0);
                central += 1;
            }
            "matching-prefix" => {
                for (at, size, crc) in [(0, 22, 14), (central, 24, 16)] {
                    raw[at + size..at + size + 4].copy_from_slice(&1u32.to_le_bytes());
                    raw[at + crc..at + crc + 4]
                        .copy_from_slice(&crc32fast::hash(&TEXT[..1]).to_le_bytes());
                }
            }
            _ => unreachable!(),
        }
        let end = raw.windows(4).rposition(|s| s == b"PK\x05\x06").unwrap();
        raw[end + 16..end + 20].copy_from_slice(&(central as u32).to_le_bytes());
        for (at, size) in [(0, 18), (central, 20)] {
            raw[at + size..at + size + 4]
                .copy_from_slice(&((central - start) as u32).to_le_bytes());
        }
        fs::write(&source, &raw).unwrap();
        let mut archive = ZipArchive::new(File::open(&source).unwrap()).unwrap();
        assert!(
            archive
                .by_name("nested/合同.txt")
                .unwrap()
                .read_to_end(&mut Vec::new())
                .is_err(),
            "{kind}"
        );
        let out = t.path().join("not-published.zip");
        let error = edit::save_with_edits(
            &source,
            &edit::inspect(&source).unwrap().revision,
            &[],
            &rename(),
            &out,
            "store",
            0,
            flag(),
            observer(),
        )
        .unwrap_err();
        if ["order", "freeze", "unknown-restore"].contains(&kind) {
            assert!(format!("{error:#}").contains("参数暂不支持"));
        }
        assert!(!out.exists());
        assert_eq!(fs::read(source).unwrap(), raw);
    }
}
#[test]
fn ppmd1562_stream_empty_read_does_not_initialize_and_full_end_is_checked() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("stream.zip");
    single(&source);
    let mut cursor = std::io::Cursor::new(fs::read(source).unwrap());
    let mut file = zip_aes::read::read_zipfile_from_stream(&mut cursor)
        .unwrap()
        .unwrap();
    assert_eq!(file.read(&mut []).unwrap(), 0);
    let mut content = Vec::new();
    file.read_to_end(&mut content).unwrap();
    assert_eq!(content, TEXT);
    assert_eq!(file.read(&mut [0; 1]).unwrap(), 0);
}

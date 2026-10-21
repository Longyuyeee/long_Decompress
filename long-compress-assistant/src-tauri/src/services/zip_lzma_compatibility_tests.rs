//! Real method-14 streams across editing, comparison and reference materialization.
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
const TEXT: &[u8] = b"actual lzma content, not a changed method marker";
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
        .compression_method(CompressionMethod::Lzma)
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
fn lzma1560_plain_edit_preserves_compressed_bytes_method_comments_and_time() {
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
    assert_eq!(new_entry.compression(), CompressionMethod::Lzma);
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
fn lzma1560_encrypted_edit_keeps_method_and_rekeys_with_explicit_encoding() {
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
                CompressionMethod::Lzma
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
fn lzma1560_password_failure_preserves_original_edits_and_existing_output() {
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
fn lzma1560_folder_and_zip_content_comparison_reads_real_plain_and_password_bytes() {
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
fn lzma1560_numeric_password_material_uses_exact_range_and_detects_missing_or_changed_members() {
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
fn lzma1560_bad_crc_size_and_truncated_stream_never_publish_or_become_content_evidence() {
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

// Wrap an independently encoded raw LZMA1 stream in a real method-14 ZIP.
// In particular use_end_marker=false is not a truncated EOS stream.
fn framed(path: &Path, eos: bool) {
    let options = lzma_rust2::LzmaOptions::with_preset(1);
    let mut encoder = lzma_rust2::LzmaWriter::new_no_header(Vec::new(), &options, eos).unwrap();
    encoder.write_all(TEXT).unwrap();
    let stream = encoder.finish().unwrap();
    let mut payload = vec![9, 4, 5, 0, options.get_props()];
    payload.extend(options.dict_size.to_le_bytes());
    payload.extend(stream);
    let mut writer = ZipWriter::new(File::create(path).unwrap());
    writer
        .start_file(
            "nested/合同.txt",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(TEXT).unwrap();
    writer.finish().unwrap();
    let mut raw = fs::read(path).unwrap();
    let name = u16::from_le_bytes(raw[26..28].try_into().unwrap()) as usize;
    let extra = u16::from_le_bytes(raw[28..30].try_into().unwrap()) as usize;
    let start = 30 + name + extra;
    raw.splice(start..start + TEXT.len(), payload.iter().copied());
    let end = raw.len() - 22;
    let central = start + payload.len();
    raw[end + 16..end + 20].copy_from_slice(&(central as u32).to_le_bytes());
    for (at, flags, method, size, version) in [(0, 6, 8, 18, 4), (central, 8, 10, 20, 6)] {
        let original_flags =
            u16::from_le_bytes(raw[at + flags..at + flags + 2].try_into().unwrap());
        raw[at + flags..at + flags + 2]
            .copy_from_slice(&((original_flags & !2) | if eos { 2 } else { 0 }).to_le_bytes());
        raw[at + method..at + method + 2].copy_from_slice(&14u16.to_le_bytes());
        raw[at + size..at + size + 4].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        raw[at + version..at + version + 2].copy_from_slice(&63u16.to_le_bytes());
    }
    fs::write(path, raw).unwrap();
}
fn data_start(raw: &[u8]) -> usize {
    30 + u16::from_le_bytes(raw[26..28].try_into().unwrap()) as usize
        + u16::from_le_bytes(raw[28..30].try_into().unwrap()) as usize
}
#[test]
fn lzma1560_valid_eos_and_sized_no_eos_survive_raw_copy_and_independent_extraction() {
    for eos in [true, false] {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("source.zip");
        framed(&source, eos);
        assert_eq!(bytes(&source, "nested/合同.txt", "", None), TEXT);
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
        let mut old = ZipArchive::new(File::open(&source).unwrap()).unwrap();
        let mut new = ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        old_raw(&mut old, "nested/合同.txt", &mut a);
        old_raw(&mut new, "delivery/合同.txt", &mut b);
        assert_eq!(a, b);
        let raw = fs::read(&out).unwrap();
        assert_eq!(
            u16::from_le_bytes(raw[6..8].try_into().unwrap()) & 2,
            if eos { 2 } else { 0 }
        );
        let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/archive-engine/7z.exe");
        let dir = t.path().join("independent");
        let result = crate::utils::process::command(engine)
            .args(["x", "-y"])
            .arg(&out)
            .arg(format!("-o{}", dir.display()))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(fs::read(dir.join("delivery/合同.txt")).unwrap(), TEXT);
    }
}
#[test]
fn lzma1560_eos_truncation_matching_prefix_and_invalid_properties_are_not_complete() {
    for kind in [
        "missing-eos",
        "matching-prefix",
        "properties-length",
        "properties-byte",
        "dictionary",
    ] {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("damaged.zip");
        framed(&source, true);
        let mut raw = fs::read(&source).unwrap();
        let start = data_start(&raw);
        let central =
            u32::from_le_bytes(raw[raw.len() - 6..raw.len() - 2].try_into().unwrap()) as usize;
        match kind {
            "missing-eos" => {
                let packed = u32::from_le_bytes(raw[18..22].try_into().unwrap()) - 5;
                raw.drain(central - 5..central);
                let cd = central - 5;
                let end = raw.len() - 22;
                raw[18..22].copy_from_slice(&packed.to_le_bytes());
                raw[cd + 20..cd + 24].copy_from_slice(&packed.to_le_bytes());
                raw[end + 16..end + 20].copy_from_slice(&(cd as u32).to_le_bytes());
            }
            "matching-prefix" => {
                for (at, size, crc) in [(0, 22, 14), (central, 24, 16)] {
                    raw[at + size..at + size + 4].copy_from_slice(&1u32.to_le_bytes());
                    raw[at + crc..at + crc + 4]
                        .copy_from_slice(&crc32fast::hash(&TEXT[..1]).to_le_bytes());
                }
            }
            "properties-length" => raw[start + 2..start + 4].copy_from_slice(&4u16.to_le_bytes()),
            "properties-byte" => raw[start + 4] = 255,
            "dictionary" => raw[start + 5..start + 9].copy_from_slice(&u32::MAX.to_le_bytes()),
            _ => unreachable!(),
        }
        fs::write(&source, &raw).unwrap();
        let mut archive = ZipArchive::new(File::open(&source).unwrap()).unwrap();
        let mut bytes = Vec::new();
        assert!(
            archive
                .by_name("nested/合同.txt")
                .unwrap()
                .read_to_end(&mut bytes)
                .is_err(),
            "{kind}"
        );
        let out = t.path().join("not-published.zip");
        assert!(
            edit::save_with_edits(
                &source,
                &edit::inspect(&source).unwrap().revision,
                &[],
                &rename(),
                &out,
                "store",
                0,
                flag(),
                observer()
            )
            .is_err(),
            "{kind}"
        );
        assert!(!out.exists());
        assert_eq!(fs::read(source).unwrap(), raw);
    }
}
#[test]
fn lzma1560_stream_reader_obeys_eos_and_sized_stream_flags() {
    for eos in [true, false] {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("stream.zip");
        framed(&source, eos);
        let mut cursor = std::io::Cursor::new(fs::read(source).unwrap());
        let mut file = zip_aes::read::read_zipfile_from_stream(&mut cursor)
            .unwrap()
            .unwrap();
        let mut decoded = Vec::new();
        file.read_to_end(&mut decoded).unwrap();
        assert_eq!(decoded, TEXT);
    }
}

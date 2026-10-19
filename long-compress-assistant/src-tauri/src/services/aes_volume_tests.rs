use super::{aes_stream_v2::{AesStreamKind, AesStreamV2}, compression_format, aes_wrapper::AesWrapper};
use crate::models::compression::CompressionOptions;
use std::{fs, path::{Path, PathBuf}};

fn fixtures() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/aes-1518") }
fn split(source: &Path, root: &Path, size: usize) -> Vec<PathBuf> {
    let bytes = fs::read(source).unwrap();
    bytes.chunks(size).enumerate().map(|(i, bytes)| {
        let part=root.join(format!("{}.{:03}",source.file_name().unwrap().to_str().unwrap(),i+1));
        fs::write(&part,bytes).unwrap(); part
    }).collect()
}

#[test]
fn aes1518_existing_creation_reaches_numeric_split_for_all_nine_formats() {
    let temp=tempfile::tempdir().unwrap(); let source=temp.path().join("source.txt");
    fs::write(&source,b"real source").unwrap();
    for format in ["tar.aes","tar.gz.aes","tar.bz2.aes","tar.xz.aes","tar.zst.aes","gz.aes","bz2.aes","xz.aes","zst.aes"] {
        let options=CompressionOptions {format:Some(format.into()),password:Some("Long 分卷-1518".into()),split_size:Some(8191),..Default::default()};
        let output=temp.path().join(format!("new.{format}"));
        let sources=vec![source.to_string_lossy().into_owned()];
        assert_eq!(compression_format::validate_compression_request(&sources,&output.to_string_lossy(),&options).unwrap(),format);
    }
}

#[test]
fn aes1518_independent_original_reads_header_across_tiny_volumes() {
    let temp=tempfile::tempdir().unwrap();
    let parts=split(&fixtures().join("v2-说明.txt.gz.aes"),temp.path(),37);
    let output=temp.path().join("decoded.gz");
    AesStreamV2::decrypt_file(&parts[0],&output,"Long 分卷-1518",AesStreamKind::Generic).unwrap();
    assert_eq!(fs::read(output).unwrap(),fs::read(fixtures().join("../multistream-1517/说明.txt.gz")).unwrap());
}

#[test]
fn aes1518_independent_original_single_numeric_member_control() {
    let temp=tempfile::tempdir().unwrap();
    let parts=split(&fixtures().join("v2-说明.txt.gz.aes"),temp.path(),8191);
    let output=temp.path().join("decoded.gz");
    AesWrapper::decrypt_file(&parts[parts.len()/2],&output,"Long 分卷-1518").unwrap();
    assert_eq!(fs::read(output).unwrap(),fs::read(fixtures().join("../multistream-1517/说明.txt.gz")).unwrap());
}

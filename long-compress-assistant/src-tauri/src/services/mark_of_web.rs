use std::io;
use std::path::Path;

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::io::Read;
#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
use std::path::PathBuf;

#[cfg(any(windows, test))]
const MAX_ZONE_IDENTIFIER_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct MarkOfWeb {
    contents: Vec<u8>,
    zone_id: u8,
}

impl MarkOfWeb {
    pub(crate) fn zone_id(&self) -> u8 {
        self.zone_id
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PropagationStatus {
    Applied(usize),
    Unsupported,
}

fn parse_internet_zone(contents: &[u8]) -> Option<u8> {
    let text = String::from_utf8_lossy(contents);
    text.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        if !key.trim().eq_ignore_ascii_case("ZoneId") {
            return None;
        }
        match value.trim().parse::<u8>().ok()? {
            zone @ (3 | 4) => Some(zone),
            _ => None,
        }
    })
}

#[cfg(windows)]
fn stream_path(path: &Path) -> PathBuf {
    let mut encoded: Vec<u16> = path.as_os_str().encode_wide().collect();
    encoded.extend(":Zone.Identifier".encode_utf16());
    PathBuf::from(OsString::from_wide(&encoded))
}

#[cfg(windows)]
pub(crate) fn read_from(path: &Path) -> io::Result<Option<MarkOfWeb>> {
    let stream = stream_path(path);
    let mut file = match std::fs::File::open(stream) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) if ads_is_unsupported(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    if file.metadata()?.len() > MAX_ZONE_IDENTIFIER_BYTES {
        return Ok(None);
    }

    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(parse_internet_zone(&contents).map(|zone_id| MarkOfWeb { contents, zone_id }))
}

#[cfg(not(windows))]
pub(crate) fn read_from(_path: &Path) -> io::Result<Option<MarkOfWeb>> {
    Ok(None)
}

#[cfg(windows)]
fn ads_is_unsupported(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(1 | 50 | 123))
        || error.kind() == io::ErrorKind::Unsupported
}

#[cfg(windows)]
fn remove_streams(paths: &[PathBuf]) -> io::Result<()> {
    let mut first_error = None;
    for path in paths {
        if let Err(error) = std::fs::remove_file(stream_path(path)) {
            if error.kind() != io::ErrorKind::NotFound && first_error.is_none() {
                first_error = Some(error);
            }
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(windows)]
pub(crate) fn propagate_to_tree(
    root: &Path,
    mark: &MarkOfWeb,
    mut is_cancelled: impl FnMut() -> bool,
) -> io::Result<PropagationStatus> {
    let mut marked_files = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(io::Error::other)?;
        if !entry.file_type().is_file() {
            continue;
        }
        if is_cancelled() {
            remove_streams(&marked_files)?;
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Mark-of-the-Web propagation was cancelled",
            ));
        }

        if let Err(error) = std::fs::write(stream_path(entry.path()), &mark.contents) {
            let cleanup_result = remove_streams(&marked_files);
            if let Err(cleanup_error) = cleanup_result {
                return Err(io::Error::other(format!(
                    "failed to propagate Mark-of-the-Web: {error}; cleanup was incomplete: {cleanup_error}"
                )));
            }
            if ads_is_unsupported(&error) {
                return Ok(PropagationStatus::Unsupported);
            }
            return Err(error);
        }
        marked_files.push(entry.path().to_path_buf());
    }
    Ok(PropagationStatus::Applied(marked_files.len()))
}

#[cfg(not(windows))]
pub(crate) fn propagate_to_tree(
    _root: &Path,
    _mark: &MarkOfWeb,
    _is_cancelled: impl FnMut() -> bool,
) -> io::Result<PropagationStatus> {
    Ok(PropagationStatus::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_only_accepts_internet_and_restricted_zones() {
        assert_eq!(parse_internet_zone(b"[ZoneTransfer]\r\nZoneId=3\r\n"), Some(3));
        assert_eq!(parse_internet_zone(b"[ZoneTransfer]\nzoneid = 4\n"), Some(4));
        assert_eq!(parse_internet_zone(b"[ZoneTransfer]\r\nZoneId=2\r\n"), None);
        assert_eq!(parse_internet_zone(b"ZoneId=invalid"), None);
    }

    #[cfg(windows)]
    #[test]
    fn reads_propagates_and_preserves_mark_across_rename() {
        let temp = tempfile::tempdir().expect("temp dir");
        let archive = temp.path().join("download.zip");
        std::fs::write(&archive, b"archive").expect("archive fixture");
        let zone = b"[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://example.test/file.zip\r\n";
        std::fs::write(stream_path(&archive), zone).expect("source zone stream");

        let mark = read_from(&archive).expect("read source mark").expect("source is marked");
        assert_eq!(mark.zone_id(), 3);

        let staging = temp.path().join("staging");
        std::fs::create_dir_all(staging.join("nested")).expect("staging tree");
        let staged = staging.join("nested/file.txt");
        std::fs::write(&staged, b"payload").expect("staged file");
        assert_eq!(
            propagate_to_tree(&staging, &mark, || false).expect("propagate mark"),
            PropagationStatus::Applied(1)
        );

        let destination = temp.path().join("committed.txt");
        std::fs::rename(&staged, &destination).expect("commit staged file");
        assert_eq!(std::fs::read(stream_path(&destination)).expect("committed zone stream"), zone);
    }

    #[cfg(windows)]
    #[test]
    fn ignores_missing_malformed_and_oversized_source_streams() {
        let temp = tempfile::tempdir().expect("temp dir");
        let archive = temp.path().join("download.zip");
        std::fs::write(&archive, b"archive").expect("archive fixture");
        assert!(read_from(&archive).expect("missing stream").is_none());

        std::fs::write(stream_path(&archive), b"[ZoneTransfer]\r\nZoneId=2\r\n")
            .expect("local zone stream");
        assert!(read_from(&archive).expect("local stream").is_none());

        std::fs::write(
            stream_path(&archive),
            vec![b'x'; MAX_ZONE_IDENTIFIER_BYTES as usize + 1],
        )
        .expect("oversized zone stream");
        assert!(read_from(&archive).expect("oversized stream").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn cancellation_removes_streams_already_written() {
        let temp = tempfile::tempdir().expect("temp dir");
        let archive = temp.path().join("download.zip");
        std::fs::write(&archive, b"archive").expect("archive fixture");
        std::fs::write(
            stream_path(&archive),
            b"[ZoneTransfer]\r\nZoneId=3\r\n",
        )
        .expect("source zone stream");
        let mark = read_from(&archive).unwrap().unwrap();
        let staging = temp.path().join("staging");
        std::fs::create_dir_all(&staging).expect("staging tree");
        let first = staging.join("a.txt");
        let second = staging.join("b.txt");
        std::fs::write(&first, b"first").expect("first file");
        std::fs::write(&second, b"second").expect("second file");
        let mut checks = 0;

        let error = propagate_to_tree(&staging, &mark, || {
            checks += 1;
            checks > 1
        })
        .expect_err("second file should observe cancellation");

        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(!stream_path(&first).exists());
        assert!(!stream_path(&second).exists());
    }
}

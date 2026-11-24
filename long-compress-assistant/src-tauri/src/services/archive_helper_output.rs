use crate::models::compression::{ArchiveBrowseResult, ArchiveEntryInfo};
use serde::{de, de::DeserializeOwned, Deserialize, Deserializer};
use std::{fmt, marker::PhantomData, process::Output};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HelperResponse<T: DeserializeOwned> {
    #[serde(deserialize_with = "deserialize_object")]
    result: T,
}

pub(super) fn decode_response<T: DeserializeOwned>(output: &Output) -> Option<T> {
    if !output.status.success() {
        return None;
    }
    let mut deserializer = serde_json::Deserializer::from_slice(&output.stdout);
    let response: HelperResponse<T> = deserialize_object(&mut deserializer).ok()?;
    deserializer.end().ok()?;
    Some(response.result)
}

fn deserialize_object<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Object<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Object<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON object")
        }

        fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_map(Object(PhantomData))
}

// The helpers emit a fixed directory shape. Unexpected fields fall back to the
// Value-based path so version differences and malformed replies retain their handling.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Directory {
    format: String,
    #[serde(deserialize_with = "decode_entries")]
    entries: Vec<ArchiveEntryInfo>,
    total_files: usize,
    total_directories: usize,
    total_uncompressed_size: u64,
    total_compressed_size: u64,
    encrypted: bool,
    #[serde(rename = "solid")]
    _solid: Option<bool>,
}

pub(super) fn decode_directory(output: &Output) -> Option<ArchiveBrowseResult> {
    let value: Directory = decode_response(output)?;
    Some(ArchiveBrowseResult {
        zpaq_history: None,
        format: value.format,
        entries: value.entries,
        total_files: value.total_files,
        total_directories: value.total_directories,
        total_uncompressed_size: value.total_uncompressed_size,
        total_compressed_size: value.total_compressed_size,
        encrypted: value.encrypted,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DirectoryEntry {
    path: String,
    name: String,
    size: u64,
    size_known: Option<bool>,
    compressed_size: Option<u64>,
    modified: Option<String>,
    crc: Option<String>,
    encrypted: bool,
    is_dir: bool,
    #[serde(rename = "methods")]
    _methods: Option<Vec<String>>,
    #[serde(rename = "encryption")]
    _encryption: Option<String>,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct DirectoryEntryObject(#[serde(deserialize_with = "deserialize_object")] DirectoryEntry);

fn decode_entries<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ArchiveEntryInfo>, D::Error> {
    struct Entries;

    impl<'de> de::Visitor<'de> for Entries {
        type Value = Vec<ArchiveEntryInfo>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an array of archive entries")
        }

        fn visit_seq<A: de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let mut entries = Vec::new();
            while let Some(DirectoryEntryObject(entry)) = sequence.next_element()? {
                entries.push(ArchiveEntryInfo {
                    link_target: None,
                    path: entry.path,
                    name: entry.name,
                    size: entry.size,
                    size_known: entry.size_known,
                    compressed_size: entry.compressed_size,
                    modified: entry.modified,
                    crc: entry.crc,
                    encrypted: entry.encrypted,
                    is_dir: entry.is_dir,
                });
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_seq(Entries)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{os::windows::process::ExitStatusExt, process::ExitStatus};

    fn output(bytes: Vec<u8>, status: u32) -> Output {
        Output {
            status: ExitStatus::from_raw(status),
            stdout: bytes,
            stderr: Vec::new(),
        }
    }

    #[test]
    fn directory_fields_match_the_existing_model() {
        let mut value = serde_json::json!({"result": {
            "format": "EGG", "entries": [{"path": "资料/说明.txt", "name": "说明.txt",
                "size": 3, "sizeKnown": true, "compressedSize": null, "modified": null,
                "crc": null, "encrypted": true, "isDir": false, "methods": ["LZMA"],
                "encryption": "AES256"}],
            "totalFiles": 1, "totalDirectories": 0, "totalUncompressedSize": 3,
            "totalCompressedSize": 9, "encrypted": true, "solid": true
        }});
        for format in ["EGG", "ALZ"] {
            if format == "ALZ" {
                value["result"]["format"] = serde_json::json!("ALZ");
                value["result"].as_object_mut().unwrap().remove("solid");
                let entry = value["result"]["entries"][0].as_object_mut().unwrap();
                entry.remove("methods");
                entry.remove("encryption");
                entry.insert("compressedSize".to_owned(), serde_json::json!(9));
                entry.insert("crc".to_owned(), serde_json::json!("0123ABCD"));
            }
            let expected: ArchiveBrowseResult =
                serde_json::from_value(value["result"].clone()).unwrap();
            let bytes = serde_json::to_vec(&value).unwrap();
            assert_eq!(decode_directory(&output(bytes.clone(), 0)), Some(expected));
            assert_eq!(decode_directory(&output(bytes, 2)), None);
        }
    }

    #[test]
    fn unexpected_or_invalid_directory_frames_use_existing_handling() {
        const DATA: &str = r#"{"format":"ALZ","entries":ENTRIES,"totalFiles":1,"totalDirectories":0,
            "totalUncompressedSize":0,"totalCompressedSize":0,"encrypted":false}"#;
        for entries in [
            r#"[["path","filename",0,true,0,null,null,false,false]]"#,
            r#"[{"path":"x","name":"x","size":0,"size":1,"encrypted":false,"isDir":false}]"#,
            r#"[{"path":"x","name":"x","size":0,"encrypted":false,"isDir":false,"linkTarget":"y"}]"#,
            r#"[{"path":"x","name":"x","size":0,"encrypted":false,"isDir":false,"methods":["\uD800"]}]"#,
        ] {
            let bytes = format!(r#"{{"result":{}}}"#, DATA.replace("ENTRIES", entries));
            assert_eq!(decode_directory(&output(bytes.into_bytes(), 0)), None);
        }
        let data = DATA.replace("ENTRIES", "[]");
        let both = format!(r#"{{"error":"helper error","result":{data}}}"#);
        assert_eq!(decode_directory(&output(both.into_bytes(), 0)), None);
    }
}

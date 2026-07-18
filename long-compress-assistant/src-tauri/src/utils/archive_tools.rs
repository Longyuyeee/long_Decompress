use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEngineFormatCapability {
    pub name: String,
    pub extensions: Vec<String>,
    pub can_create: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEngineCapabilities {
    pub available: bool,
    pub command: Option<String>,
    pub version: Option<String>,
    pub full_engine: bool,
    pub formats: Vec<ArchiveEngineFormatCapability>,
    pub message: String,
}

const KNOWN_ENGINE_FORMATS: &[(&str, &[&str])] = &[
    ("7z", &["7z"]),
    ("APFS", &["apfs"]),
    ("APM", &["apm"]),
    ("Ar", &["ar", "a", "deb", "udeb", "lib"]),
    ("Arj", &["arj"]),
    ("Cab", &["cab"]),
    ("Chm", &["chm", "chi", "chq", "chw"]),
    ("Compound", &["msi", "msp", "msm", "doc", "xls", "ppt"]),
    ("Cpio", &["cpio"]),
    ("CramFS", &["cramfs"]),
    ("Dmg", &["dmg"]),
    ("Ext", &["ext", "ext2", "ext3", "ext4"]),
    ("FAT", &["fat"]),
    ("GPT", &["gpt"]),
    ("HFS", &["hfs", "hfsx"]),
    ("IHex", &["ihex"]),
    ("Iso", &["iso"]),
    ("Lzh", &["lzh", "lha"]),
    ("MBR", &["mbr"]),
    ("NTFS", &["ntfs"]),
    ("Nsis", &["nsis"]),
    ("QCOW", &["qcow", "qcow2", "qcow2c"]),
    ("Rar", &["rar", "r00"]),
    ("Rar5", &["rar", "r00"]),
    ("Rpm", &["rpm"]),
    ("SquashFS", &["squashfs", "sfs"]),
    ("UEFIc", &["scap"]),
    ("UEFIf", &["uefif"]),
    ("Udf", &["udf"]),
    ("VDI", &["vdi"]),
    ("VHD", &["vhd"]),
    ("VHDX", &["vhdx", "avhdx"]),
    ("VMDK", &["vmdk"]),
    ("Xar", &["xar", "pkg", "xip"]),
    ("Z", &["z", "taz"]),
    ("bzip2", &["bz2", "bzip2", "tbz", "tbz2"]),
    ("gzip", &["gz", "gzip", "tgz", "tpz"]),
    ("lzma", &["lzma"]),
    ("tar", &["tar", "ova"]),
    ("wim", &["wim", "swm", "esd", "ppkg"]),
    ("xz", &["xz", "txz"]),
    ("zip", &["zip", "zipx", "jar", "epub", "apk", "appx"]),
];

fn command_exists(command: &str) -> bool {
    crate::utils::process::command(command)
        .arg("i")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn candidate_exists(path: &Path) -> bool {
    path.is_file()
}

fn bundled_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.extend([
                exe_dir.join("archive-engine").join("7z.exe"),
                exe_dir
                    .join("resources")
                    .join("archive-engine")
                    .join("7z.exe"),
                exe_dir.join("resources").join("7z.exe"),
                exe_dir.join("7z.exe"),
                exe_dir.join("7za.exe"),
                exe_dir.join("resources").join("7za.exe"),
                exe_dir
                    .join("_up_")
                    .join("src-tauri")
                    .join("resources")
                    .join("archive-engine")
                    .join("7z.exe"),
                exe_dir
                    .join("_up_")
                    .join("node_modules")
                    .join("7zip-bin")
                    .join("win")
                    .join("x64")
                    .join("7za.exe"),
            ]);
        }
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest_dir);
        candidates.extend([
            root.join("resources").join("archive-engine").join("7z.exe"),
            root.join("bin").join("7za.exe"),
            root.join("..")
                .join("node_modules")
                .join("7zip-bin")
                .join("win")
                .join("x64")
                .join("7za.exe"),
        ]);
    }
    candidates
}

pub fn find_7z_command() -> Option<String> {
    if let Some(candidate) = bundled_candidates()
        .into_iter()
        .find(|path| candidate_exists(path))
    {
        return Some(candidate.to_string_lossy().to_string());
    }
    for command in ["7z", "7zz", "7za"] {
        if command_exists(command) {
            return Some(command.to_string());
        }
    }
    #[cfg(target_os = "windows")]
    for candidate in [
        PathBuf::from(r"C:\Program Files\7-Zip\7z.exe"),
        PathBuf::from(r"C:\Program Files (x86)\7-Zip\7z.exe"),
    ] {
        if candidate_exists(&candidate) {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

fn parse_engine_version(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.trim_start().starts_with("7-Zip"))
        .and_then(|line| {
            line.split_whitespace()
                .find(|token| {
                    token.chars().next().is_some_and(|ch| ch.is_ascii_digit())
                        && token.contains('.')
                })
                .map(|token| {
                    token
                        .trim_matches(|ch: char| !ch.is_ascii_digit() && ch != '.')
                        .to_string()
                })
        })
}

pub fn parse_archive_engine_formats(output: &str) -> Vec<ArchiveEngineFormatCapability> {
    let formats_block = output.split("Formats:").nth(1).unwrap_or(output);
    let formats_block = formats_block
        .split("Codecs:")
        .next()
        .unwrap_or(formats_block);
    let mut formats = Vec::new();
    for (name, extensions) in KNOWN_ENGINE_FORMATS {
        let mut can_create = false;
        let found = formats_block.lines().any(|line| {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let Some(name_index) = tokens.iter().position(|token| token == name) else {
                return false;
            };
            can_create = tokens[..name_index].iter().any(|token| token.contains('C'));
            true
        });
        if found {
            formats.push(ArchiveEngineFormatCapability {
                name: (*name).to_string(),
                extensions: extensions
                    .iter()
                    .map(|extension| (*extension).to_string())
                    .collect(),
                can_create,
            });
        }
    }
    formats
}

pub fn detect_archive_engine_capabilities() -> ArchiveEngineCapabilities {
    let Some(command) = find_7z_command() else {
        return ArchiveEngineCapabilities {
            available: false,
            command: None,
            version: None,
            full_engine: false,
            formats: Vec::new(),
            message: missing_7z_message(),
        };
    };
    let Ok(result) = crate::utils::process::command(&command).arg("i").output() else {
        return ArchiveEngineCapabilities {
            available: false,
            command: Some(command),
            version: None,
            full_engine: false,
            formats: Vec::new(),
            message: "Unable to inspect the archive engine.".to_string(),
        };
    };
    let stdout = String::from_utf8_lossy(&result.stdout);
    let formats = parse_archive_engine_formats(&stdout);
    let full_engine = ["APFS", "QCOW", "VDI", "VMDK", "wim"]
        .iter()
        .all(|required| formats.iter().any(|format| format.name == *required));
    ArchiveEngineCapabilities {
        available: result.status.success(),
        command: Some(command),
        version: parse_engine_version(&stdout),
        full_engine,
        message: if full_engine {
            "Full 7-Zip archive engine is ready."
        } else {
            "A limited 7-Zip engine was detected; some formats are unavailable."
        }
        .to_string(),
        formats,
    }
}

pub fn archive_engine_can_create(format: &str) -> bool {
    detect_archive_engine_capabilities()
        .formats
        .iter()
        .any(|capability| {
            capability.can_create
                && (capability.name.eq_ignore_ascii_case(format)
                    || capability
                        .extensions
                        .iter()
                        .any(|extension| extension.eq_ignore_ascii_case(format)))
        })
}

pub fn missing_7z_message() -> String {
    "The bundled full 7-Zip archive engine is unavailable. Reinstall the application or configure an external 7z.exe.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    const FULL_SAMPLE: &str = r#"7-Zip 26.02 (x64)
Formats:
 0 C...F..........c.a.m+.. w...0  7z       7z
 0  ......................  APFS     apfs img
 0  ......................  QCOW     qcow qcow2
 0  ......................  VDI      vdi
 0  ......................  VMDK     vmdk
 0 C.SN.......LH..c.a.m+.. w...0  wim      wim swm esd
Codecs:"#;

    #[test]
    fn parses_full_engine_formats_and_create_flags() {
        let formats = parse_archive_engine_formats(FULL_SAMPLE);
        assert!(formats
            .iter()
            .any(|format| format.name == "APFS" && !format.can_create));
        assert!(formats
            .iter()
            .any(|format| format.name == "wim" && format.can_create));
        assert!(formats
            .iter()
            .any(|format| format.name == "7z" && format.can_create));
    }

    #[test]
    fn parses_engine_version() {
        assert_eq!(parse_engine_version(FULL_SAMPLE).as_deref(), Some("26.02"));
        assert_eq!(
            parse_engine_version("7-Zip (a) 21.07 (x64)").as_deref(),
            Some("21.07")
        );
    }
}

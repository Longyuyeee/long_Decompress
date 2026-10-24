use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionTask {
    pub id: String,
    pub source_files: Vec<String>,
    pub output_path: String,
    pub format: CompressionFormat,
    pub options: CompressionOptions,
    pub status: CompressionStatus,
    pub progress: f32,
    pub total_size: u64,
    pub processed_size: u64,
    pub password: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl CompressionTask {
    pub fn new(
        source_files: Vec<String>,
        output_path: String,
        format: CompressionFormat,
        options: CompressionOptions,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_files,
            output_path,
            format,
            options,
            status: CompressionStatus::Pending,
            progress: 0.0,
            total_size: 0,
            processed_size: 0,
            password: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
        }
    }
}

impl Default for CompressionTask {
    fn default() -> Self {
        Self::new(Vec::new(), String::new(), CompressionFormat::Zip, CompressionOptions::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionFormat {
    Zpaq,
    Zip,
    Zipx,
    SevenZip,
    Rar,
    Tar,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    Lzma,
    Lzip,
    TarLzip,
    TarGz,
    TarBzip2,
    TarXz,
    TarZstd,
}

impl CompressionFormat {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "zpaq" => Self::Zpaq,
            "zipx" => Self::Zipx,
            "7z" => Self::SevenZip,
            "rar" => Self::Rar,
            "tar" => Self::Tar,
            "gz" | "gzip" => Self::Gzip,
            "bz2" | "bzip2" => Self::Bzip2,
            "xz" => Self::Xz,
            "tgz" | "tar.gz" => Self::TarGz,
            "tbz2" | "tar.bz2" => Self::TarBzip2,
            "txz" | "tar.xz" => Self::TarXz,
            "zst" | "zstd" => Self::Zstd,
            "tzst" | "tar.zst" => Self::TarZstd,
            "lzma" => Self::Lzma,
            "lz" => Self::Lzip,
            "tar.lz" | "tlz" => Self::TarLzip,
            _ => Self::Zip,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Zpaq => "zpaq",
            Self::Zip => "zip",
            Self::Zipx => "zipx",
            Self::SevenZip => "7z",
            Self::Rar => "rar",
            Self::Tar => "tar",
            Self::Gzip => "gz",
            Self::Bzip2 => "bz2",
            Self::Xz => "xz",
            Self::TarGz => "tar.gz",
            Self::TarBzip2 => "tar.bz2",
            Self::TarXz => "tar.xz",
            Self::Zstd => "zst",
            Self::TarZstd => "tar.zst",
            Self::Lzma => "lzma",
            Self::Lzip => "lz",
            Self::TarLzip => "tar.lz",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionOptions {
    #[serde(default)]
    pub wim_method: crate::services::wim_writer::WimMethod,
    #[serde(default)]
    pub zip_method: crate::services::zip_method::ZipMethod,
    #[serde(default)]
    pub seven_zip_method: crate::services::seven_zip_method::SevenZipMethod,
    #[serde(default)]
    pub seven_zip_filter: crate::services::seven_zip_method::SevenZipFilter,
    #[serde(default = "crate::services::seven_zip_method::default_delta_distance")]
    pub seven_zip_delta_distance: u32,
    // Retired in 1.4.0: accept old payloads, but never deserialize or emit this option.
    #[serde(skip)]
    pub fast_pack: bool,
    #[serde(default)]
    pub zip_password_encoding: crate::services::zip_password::ZipPasswordEncoding,
    #[serde(default)]
    pub zip_encryption: crate::services::zip_password::ZipEncryption,
    #[serde(default)]
    pub zip_split_mode: crate::services::standard_zip_writer::ZipSplitMode,
    #[serde(default)]
    pub format: Option<String>,
    pub level: u32,
    pub password: Option<String>,
    pub split_size: Option<u64>,
    #[serde(default)]
    pub create_solid_archive: bool,
    /// Explicit opt-in; legacy requests continue to encrypt contents only.
    #[serde(default)]
    pub encrypt_filenames: bool,
    #[serde(default)]
    pub rar_recovery_percent: u32,
    #[serde(default)]
    pub rar_recovery_volumes: u32,
    #[serde(default)]
    pub preserve_paths: Option<bool>,
    #[serde(default)]
    pub delete_after: bool,
    #[serde(default = "default_true")]
    pub verify_after: bool,
    /// RAR creation currently uses argv for full Unicode password compatibility.
    /// The UI must approve this explicitly; repair uses a separate OEM stdin channel.
    #[serde(default)]
    pub allow_insecure_password_cli: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompressOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zpaq_retrieval: Option<crate::services::zpaq_engine::retrieval::Selection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reader_selection: Option<crate::services::image_reader::Selection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zpaq_snapshot: Option<ZpaqSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zpaq_catalog_id: Option<String>,
    /// ZIP: None retains automatic password compatibility, filenames unchanged.
    /// ALZ: None means CP949; an explicit source codepage reads names and key.
      /// EGG: None means UTF-8 password; names use their own format marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip_password_encoding: Option<crate::services::zip_password::ZipPasswordEncoding>,
    #[serde(default)]
    pub parallel_zip: bool,
    pub preserve_paths: bool,
    pub overwrite_existing: bool,
    pub delete_after: bool,
    pub preserve_timestamps: bool,
    pub skip_corrupted: bool,
    pub extract_only_newer: bool,
    pub create_subdirectory: bool,
    #[serde(default = "default_true")]
    pub preserve_mark_of_web: bool,
    pub file_filter: Option<String>,
    /// Exact archive-relative file paths selected in the archive browser.
    /// An empty list keeps the historical "extract everything" behavior.
    #[serde(default)]
    pub selected_entries: Vec<String>,
    #[serde(default = "default_conflict_policy")]
    pub conflict_policy: String,
    #[serde(default)]
    pub enable_bruteforce: bool,
    #[serde(default)]
    pub bruteforce_wordlists: Vec<String>,
}

fn default_conflict_policy() -> String {
    "rename".to_string()
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            wim_method: Default::default(),
            zip_method: Default::default(),
            seven_zip_method: Default::default(),
            seven_zip_filter: Default::default(),
            seven_zip_delta_distance: 1,
            fast_pack: false,
            zip_password_encoding: Default::default(),
            zip_encryption: Default::default(),
            zip_split_mode: Default::default(),
            format: None,
            level: 0,
            password: None,
            split_size: None,
            create_solid_archive: false,
            encrypt_filenames: false,
            rar_recovery_percent: 0,
            rar_recovery_volumes: 0,
            preserve_paths: None,
            delete_after: false,
            verify_after: true,
            allow_insecure_password_cli: false,
        }
    }
}

fn default_true() -> bool {
    true
}

impl Default for DecompressOptions {
    fn default() -> Self {
        Self {
            zpaq_retrieval: None,
            reader_selection: None,
            library_source_id: None,
            library_revision: None,
            zpaq_snapshot: None,
            zpaq_catalog_id: None,
            zip_password_encoding: None,
            parallel_zip: false,
            preserve_paths: false,
            overwrite_existing: false,
            delete_after: false,
            preserve_timestamps: false,
            skip_corrupted: false,
            extract_only_newer: false,
            create_subdirectory: false,
            preserve_mark_of_web: true,
            file_filter: None,
            selected_entries: Vec::new(),
            conflict_policy: default_conflict_policy(),
            enable_bruteforce: false,
            bruteforce_wordlists: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntryInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_known: Option<bool>,
    pub compressed_size: Option<u64>,
    pub modified: Option<String>,
    pub crc: Option<String>,
    pub encrypted: bool,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveBrowseResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zpaq_history: Option<ZpaqHistory>,
    pub format: String,
    pub entries: Vec<ArchiveEntryInfo>,
    pub total_files: usize,
    pub total_directories: usize,
    pub total_uncompressed_size: u64,
    pub total_compressed_size: u64,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZpaqSnapshot { pub version: u32, pub date: i64 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZpaqSnapshotInfo {
    #[serde(flatten)] pub snapshot: ZpaqSnapshot,
    pub updates: u64, pub deletes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZpaqHistory {
    pub selected: ZpaqSnapshot, pub snapshots: Vec<ZpaqSnapshotInfo>,
    #[serde(default, rename="interruptedTail", skip_serializing_if="Option::is_none")]
    pub interrupted_tail: Option<ZpaqInterruptedTail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZpaqInterruptedTail { pub offset: u64, pub date: i64 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionStatus {
    Pending,
    Preparing,
    Running,
    Compressing,
    Extracting,
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

impl CompressionStatus {
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Running | Self::Compressing | Self::Extracting | Self::Finalizing
        )
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLog {
    pub task_id: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub severity: TaskLogSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskLogSeverity {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConflict {
    pub task_id: String,
    pub file_name: String,
    pub source_path: String,
    pub dest_path: String,
    pub source_size: u64,
    pub dest_size: u64,
    pub source_modified: u64,
    pub dest_modified: u64,
}

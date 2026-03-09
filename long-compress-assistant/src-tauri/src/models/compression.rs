use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    Zip,
    SevenZip,
    Tar,
    Gzip,
}

impl CompressionFormat {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "7z" => Self::SevenZip,
            "tar" => Self::Tar,
            "gz" | "tgz" => Self::Gzip,
            _ => Self::Zip,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::SevenZip => "7z",
            Self::Tar => "tar",
            Self::Gzip => "gz",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionOptions {
    pub level: u32,
    pub password: Option<String>,
    pub split_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecompressOptions {
    pub preserve_paths: bool,
    pub overwrite_existing: bool,
    pub delete_after: bool,
    pub preserve_timestamps: bool,
    pub skip_corrupted: bool,
    pub extract_only_newer: bool,
    pub create_subdirectory: bool,
    pub file_filter: Option<String>,
}

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

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
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub total_size: u64,
    pub processed_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionOptions {
    pub password: Option<String>,
    pub compression_level: u32,
    pub split_size: Option<u64>, // 分卷大小（字节）
    pub preserve_paths: bool,
    pub exclude_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    pub create_subdirectories: bool,
    pub overwrite_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionFormat {
    Zip,
    Tar,
    Gz,
    TarGz,
    Bz2,
    TarBz2,
    Xz,
    TarXz,
    SevenZip,
    Rar,
}

impl CompressionFormat {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_lowercase().as_str() {
            "zip" => Some(Self::Zip),
            "tar" => Some(Self::Tar),
            "gz" => Some(Self::Gz),
            "tgz" | "tar.gz" => Some(Self::TarGz),
            "bz2" => Some(Self::Bz2),
            "tbz2" | "tar.bz2" => Some(Self::TarBz2),
            "xz" => Some(Self::Xz),
            "txz" | "tar.xz" => Some(Self::TarXz),
            "7z" => Some(Self::SevenZip),
            "rar" => Some(Self::Rar),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::Gz => "gz",
            Self::TarGz => "tar.gz",
            Self::Bz2 => "bz2",
            Self::TarBz2 => "tar.bz2",
            Self::Xz => "xz",
            Self::TarXz => "tar.xz",
            Self::SevenZip => "7z",
            Self::Rar => "rar",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::Tar => "TAR",
            Self::Gz => "GZIP",
            Self::TarGz => "TAR.GZ",
            Self::Bz2 => "BZIP2",
            Self::TarBz2 => "TAR.BZ2",
            Self::Xz => "XZ",
            Self::TarXz => "TAR.XZ",
            Self::SevenZip => "7-Zip",
            Self::Rar => "RAR",
        }
    }

    pub fn supports_password(&self) -> bool {
        matches!(self, Self::Zip | Self::SevenZip | Self::Rar)
    }

    pub fn supports_compression_level(&self) -> bool {
        matches!(self, Self::Zip | Self::Gz | Self::Bz2 | Self::Xz | Self::SevenZip)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionStatus {
    Pending,
    Preparing,
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
            Self::Preparing | Self::Compressing | Self::Extracting | Self::Finalizing
        )
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionHistory {
    pub id: String,
    pub task_id: String,
    pub operation_type: OperationType,
    pub source_paths: Vec<String>,
    pub output_path: String,
    pub format: CompressionFormat,
    pub size_before: u64,
    pub size_after: u64,
    pub compression_ratio: f32,
    pub duration_seconds: f32,
    pub created_at: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationType {
    Compress,
    Extract,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStatistics {
    pub total_operations: u32,
    pub successful_operations: u32,
    pub failed_operations: u32,
    pub total_compressed_size: u64,
    pub total_extracted_size: u64,
    pub average_compression_ratio: f32,
    pub most_used_format: CompressionFormat,
    pub last_operation_time: Option<DateTime<Utc>>,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            password: None,
            compression_level: 6,
            split_size: None,
            preserve_paths: true,
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            create_subdirectories: true,
            overwrite_existing: false,
        }
    }
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
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            total_size: 0,
            processed_size: 0,
        }
    }

    pub fn update_progress(&mut self, processed: u64, total: u64) {
        self.processed_size = processed;
        self.total_size = total;

        if total > 0 {
            self.progress = (processed as f32 / total as f32) * 100.0;
        }
    }

    pub fn start(&mut self) {
        self.status = CompressionStatus::Preparing;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self, success: bool, error_message: Option<String>) {
        if success {
            self.status = CompressionStatus::Completed;
            self.progress = 100.0;
        } else {
            self.status = CompressionStatus::Failed;
            self.error_message = error_message;
        }
        self.completed_at = Some(Utc::now());
    }

    pub fn cancel(&mut self) {
        self.status = CompressionStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }
}
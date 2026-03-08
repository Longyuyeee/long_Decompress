use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub absolute_path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub permissions: FilePermissions,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryInfo {
    pub path: String,
    pub total_files: u64,
    pub total_dirs: u64,
    pub total_size: u64,
    pub depth: u32,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub id: String,
    pub operation_type: FileOperationType,
    pub source_paths: Vec<String>,
    pub destination_path: Option<String>,
    pub status: FileOperationStatus,
    pub progress: f32,
    pub total_size: u64,
    pub processed_size: u64,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileOperationType {
    Copy,
    Move,
    Delete,
    Rename,
    CreateDirectory,
    CreateFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileOperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCriteria {
    pub name_pattern: Option<String>,
    pub content_pattern: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub file_types: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub modified_after: Option<SystemTime>,
    pub modified_before: Option<SystemTime>,
    pub recursive: bool,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_entry: FileEntry,
    pub relevance_score: f32,
    pub match_positions: Vec<MatchPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPosition {
    pub line: u32,
    pub column: u32,
    pub length: u32,
    pub matched_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePreview {
    pub path: String,
    pub content: String,
    pub truncated: bool,
    pub line_count: u32,
    pub encoding: String,
    pub language: Option<String>,
}

impl FileEntry {
    pub fn from_path(path: &Path) -> Result<Self, std::io::Error> {
        let metadata = std::fs::metadata(path)?;

        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_string());

        let is_hidden = name.starts_with('.') || {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                let attributes = metadata.file_attributes();
                attributes & 0x2 != 0 // FILE_ATTRIBUTE_HIDDEN
            }
            #[cfg(not(windows))]
            {
                name.starts_with('.')
            }
        };

        let permissions = FilePermissions {
            readable: true, // 简化处理，实际需要检查权限
            writable: metadata.permissions().readonly(),
            executable: false, // 简化处理
        };

        Ok(Self {
            name,
            path: path.to_string_lossy().to_string(),
            absolute_path: path.canonicalize()
                .unwrap_or_else(|_| path.to_path_buf())
                .to_string_lossy()
                .to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            is_file: metadata.is_file(),
            is_symlink: metadata.file_type().is_symlink(),
            is_hidden,
            permissions,
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            accessed: metadata.accessed().ok(),
            extension,
            mime_type: None,
            icon: None,
        })
    }

    pub fn get_icon(&self) -> &'static str {
        if self.is_dir {
            return "📁";
        }

        match self.extension.as_deref() {
            Some("txt") | Some("md") | Some("json") | Some("yaml") | Some("yml") => "📄",
            Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("bmp") | Some("svg") => "🖼️",
            Some("mp3") | Some("wav") | Some("flac") | Some("ogg") => "🎵",
            Some("mp4") | Some("avi") | Some("mkv") | Some("mov") => "🎬",
            Some("zip") | Some("rar") | Some("7z") | Some("tar") | Some("gz") => "📦",
            Some("pdf") => "📕",
            Some("doc") | Some("docx") => "📘",
            Some("xls") | Some("xlsx") => "📗",
            Some("ppt") | Some("pptx") => "📙",
            Some("exe") | Some("msi") => "⚙️",
            Some("html") | Some("htm") => "🌐",
            Some("css") => "🎨",
            Some("js") | Some("ts") => "📜",
            Some("rs") => "🦀",
            Some("py") => "🐍",
            Some("java") => "☕",
            Some("cpp") | Some("c") | Some("h") | Some("hpp") => "🔧",
            _ => "📄",
        }
    }

    pub fn format_size(&self) -> String {
        if self.is_dir {
            return "文件夹".to_string();
        }

        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

        if self.size == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let size_f64 = self.size as f64;
        let exponent = (size_f64.log10() / base.log10()).floor() as i32;
        let unit_index = exponent.min(5).max(0) as usize;

        let formatted_size = size_f64 / base.powi(exponent);

        format!("{:.2} {}", formatted_size, UNITS[unit_index])
    }
}

impl FileOperation {
    pub fn new(
        operation_type: FileOperationType,
        source_paths: Vec<String>,
        destination_path: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            operation_type,
            source_paths,
            destination_path,
            status: FileOperationStatus::Pending,
            progress: 0.0,
            total_size: 0,
            processed_size: 0,
            started_at: None,
            completed_at: None,
            error_message: None,
        }
    }

    pub fn start(&mut self) {
        self.status = FileOperationStatus::InProgress;
        self.started_at = Some(SystemTime::now());
    }

    pub fn update_progress(&mut self, processed: u64, total: u64) {
        self.processed_size = processed;
        self.total_size = total;

        if total > 0 {
            self.progress = (processed as f32 / total as f32) * 100.0;
        }
    }

    pub fn complete(&mut self, success: bool, error_message: Option<String>) {
        if success {
            self.status = FileOperationStatus::Completed;
            self.progress = 100.0;
        } else {
            self.status = FileOperationStatus::Failed;
            self.error_message = error_message;
        }
        self.completed_at = Some(SystemTime::now());
    }

    pub fn cancel(&mut self) {
        self.status = FileOperationStatus::Cancelled;
        self.completed_at = Some(SystemTime::now());
    }
}
use crate::utils::error::AppError;
use std::path::{Path, PathBuf};
use std::fs;
use tokio::fs as tokio_fs;

/// 获取文件扩展名（不带点）
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// 获取文件名（不带扩展名）
pub fn get_file_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|s| s.to_string())
}

/// 获取文件名（带扩展名）
pub fn get_file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

/// 检查文件是否为压缩文件
pub fn is_compressed_file(path: &Path) -> bool {
    if let Some(extension) = get_file_extension(path) {
        matches!(
            extension.as_str(),
            "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "tbz2" | "xz" | "txz" |
            "tar.gz" | "tar.bz2" | "tar.xz" | "z" | "lz" | "lzma" | "lzo" | "rz" | "sz"
        )
    } else {
        false
    }
}

/// 检查文件是否为图片
pub fn is_image_file(path: &Path) -> bool {
    if let Some(extension) = get_file_extension(path) {
        matches!(
            extension.as_str(),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "svg" | "ico"
        )
    } else {
        false
    }
}

/// 检查文件是否为文档
pub fn is_document_file(path: &Path) -> bool {
    if let Some(extension) = get_file_extension(path) {
        matches!(
            extension.as_str(),
            "txt" | "md" | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" |
            "odt" | "ods" | "odp" | "rtf" | "csv" | "json" | "xml" | "yaml" | "yml"
        )
    } else {
        false
    }
}

/// 检查文件是否为媒体文件
pub fn is_media_file(path: &Path) -> bool {
    if let Some(extension) = get_file_extension(path) {
        matches!(
            extension.as_str(),
            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" |
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v"
        )
    } else {
        false
    }
}

/// 检查文件是否为可执行文件
pub fn is_executable_file(path: &Path) -> bool {
    if let Some(extension) = get_file_extension(path) {
        matches!(
            extension.as_str(),
            "exe" | "msi" | "bat" | "cmd" | "sh" | "bash" | "app" | "dmg" | "pkg"
        )
    } else {
        false
    }
}

/// 获取文件MIME类型
pub fn get_mime_type(path: &Path) -> &'static str {
    if let Some(extension) = get_file_extension(path) {
        match extension.as_str() {
            // 图片
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "tiff" | "tif" => "image/tiff",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",

            // 文档
            "txt" => "text/plain",
            "md" => "text/markdown",
            "pdf" => "application/pdf",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "json" => "application/json",
            "xml" => "application/xml",
            "csv" => "text/csv",

            // 压缩文件
            "zip" => "application/zip",
            "rar" => "application/vnd.rar",
            "7z" => "application/x-7z-compressed",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            "bz2" => "application/x-bzip2",
            "xz" => "application/x-xz",

            // 媒体文件
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "flac" => "audio/flac",
            "ogg" => "audio/ogg",
            "mp4" => "video/mp4",
            "avi" => "video/x-msvideo",
            "mkv" => "video/x-matroska",
            "mov" => "video/quicktime",

            // 可执行文件
            "exe" => "application/x-msdownload",
            "msi" => "application/x-msi",
            "sh" => "application/x-sh",

            _ => "application/octet-stream",
        }
    } else {
        "application/octet-stream"
    }
}

/// 获取文件图标（基于扩展名）
pub fn get_file_icon(path: &Path) -> &'static str {
    if path.is_dir() {
        return "📁";
    }

    if let Some(extension) = get_file_extension(path) {
        match extension.as_str() {
            // 图片
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" => "🖼️",
            "svg" => "📐",

            // 文档
            "txt" | "md" => "📄",
            "pdf" => "📕",
            "doc" | "docx" => "📘",
            "xls" | "xlsx" => "📗",
            "ppt" | "pptx" => "📙",
            "json" | "xml" | "yaml" | "yml" => "📋",
            "csv" => "📊",

            // 压缩文件
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "📦",

            // 媒体文件
            "mp3" | "wav" | "flac" | "ogg" => "🎵",
            "mp4" | "avi" | "mkv" | "mov" => "🎬",

            // 可执行文件
            "exe" | "msi" => "⚙️",
            "sh" | "bat" | "cmd" => "📟",

            // 代码文件
            "rs" => "🦀",
            "py" => "🐍",
            "js" | "ts" => "📜",
            "java" => "☕",
            "cpp" | "c" | "h" | "hpp" => "🔧",
            "html" | "htm" => "🌐",
            "css" => "🎨",
            "php" => "🐘",

            // 配置文件
            "ini" | "cfg" | "conf" => "⚙️",
            "toml" => "⚙️",

            _ => "📄",
        }
    } else {
        "📄"
    }
}

/// 安全地创建目录
pub async fn create_directory_safe(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        if !path.is_dir() {
            return Err(AppError::file(format!(
                "路径已存在但不是目录: {}",
                path.display()
            )));
        }
        return Ok(());
    }

    tokio_fs::create_dir_all(path)
        .await
        .map_err(|e| AppError::file(format!("创建目录失败: {}: {}", path.display(), e)))
}

/// 安全地删除文件或目录
pub async fn delete_path_safe(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        tokio_fs::remove_dir_all(path)
            .await
            .map_err(|e| AppError::file(format!("删除目录失败: {}: {}", path.display(), e)))
    } else {
        tokio_fs::remove_file(path)
            .await
            .map_err(|e| AppError::file(format!("删除文件失败: {}: {}", path.display(), e)))
    }
}

/// 安全地重命名文件或目录
pub async fn rename_path_safe(old_path: &Path, new_path: &Path) -> Result<(), AppError> {
    if !old_path.exists() {
        return Err(AppError::file(format!(
            "源路径不存在: {}",
            old_path.display()
        )));
    }

    if new_path.exists() {
        return Err(AppError::file(format!(
            "目标路径已存在: {}",
            new_path.display()
        )));
    }

    // 确保目标目录存在
    if let Some(parent) = new_path.parent() {
        create_directory_safe(parent).await?;
    }

    tokio_fs::rename(old_path, new_path)
        .await
        .map_err(|e| AppError::file(format!(
            "重命名失败: {} -> {}: {}",
            old_path.display(),
            new_path.display(),
            e
        )))
}

/// 安全地复制文件
pub async fn copy_file_safe(src: &Path, dst: &Path) -> Result<(), AppError> {
    if !src.exists() {
        return Err(AppError::file(format!(
            "源文件不存在: {}",
            src.display()
        )));
    }

    if !src.is_file() {
        return Err(AppError::file(format!(
            "源路径不是文件: {}",
            src.display()
        )));
    }

    if dst.exists() {
        return Err(AppError::file(format!(
            "目标文件已存在: {}",
            dst.display()
        )));
    }

    // 确保目标目录存在
    if let Some(parent) = dst.parent() {
        create_directory_safe(parent).await?;
    }

    tokio_fs::copy(src, dst)
        .await
        .map_err(|e| AppError::file(format!(
            "复制文件失败: {} -> {}: {}",
            src.display(),
            dst.display(),
            e
        )))
        .map(|_| ())
}

/// 获取目录大小（递归）
pub async fn get_directory_size(path: &Path) -> Result<u64, AppError> {
    if !path.exists() {
        return Err(AppError::file(format!(
            "目录不存在: {}",
            path.display()
        )));
    }

    if !path.is_dir() {
        return Err(AppError::file(format!(
            "路径不是目录: {}",
            path.display()
        )));
    }

    let mut total_size = 0;
    let mut dirs_to_process = vec![path.to_path_buf()];

    while let Some(current_dir) = dirs_to_process.pop() {
        let mut entries = tokio_fs::read_dir(&current_dir)
            .await
            .map_err(|e| AppError::file(format!("读取目录失败: {}: {}", current_dir.display(), e)))?;

        while let Some(entry) = entries.next_entry()
            .await
            .map_err(|e| AppError::file(format!("读取目录条目失败: {}: {}", current_dir.display(), e)))?
        {
            let metadata = entry.metadata()
                .await
                .map_err(|e| AppError::file(format!("获取文件元数据失败: {}: {}", current_dir.display(), e)))?;

            if metadata.is_dir() {
                dirs_to_process.push(entry.path());
            } else {
                total_size += metadata.len();
            }
        }
    }

    Ok(total_size)
}

/// 获取文件数量（递归）
pub async fn get_file_count(path: &Path) -> Result<u64, AppError> {
    if !path.exists() {
        return Err(AppError::file(format!(
            "目录不存在: {}",
            path.display()
        )));
    }

    if !path.is_dir() {
        return Err(AppError::file(format!(
            "路径不是目录: {}",
            path.display()
        )));
    }

    let mut file_count = 0;
    let mut dirs_to_process = vec![path.to_path_buf()];

    while let Some(current_dir) = dirs_to_process.pop() {
        let mut entries = tokio_fs::read_dir(&current_dir)
            .await
            .map_err(|e| AppError::file(format!("读取目录失败: {}: {}", current_dir.display(), e)))?;

        while let Some(entry) = entries.next_entry()
            .await
            .map_err(|e| AppError::file(format!("读取目录条目失败: {}: {}", current_dir.display(), e)))?
        {
            let metadata = entry.metadata()
                .await
                .map_err(|e| AppError::file(format!("获取文件元数据失败: {}: {}", current_dir.display(), e)))?;

            if metadata.is_dir() {
                dirs_to_process.push(entry.path());
            } else {
                file_count += 1;
            }
        }
    }

    Ok(file_count)
}

/// 获取目录数量（递归）
pub async fn get_directory_count(path: &Path) -> Result<u64, AppError> {
    if !path.exists() {
        return Err(AppError::file(format!(
            "目录不存在: {}",
            path.display()
        )));
    }

    if !path.is_dir() {
        return Err(AppError::file(format!(
            "路径不是目录: {}",
            path.display()
        )));
    }

    let mut dir_count = 0;
    let mut dirs_to_process = vec![path.to_path_buf()];

    while let Some(current_dir) = dirs_to_process.pop() {
        dir_count += 1; // 当前目录

        let mut entries = tokio_fs::read_dir(&current_dir)
            .await
            .map_err(|e| AppError::file(format!("读取目录失败: {}: {}", current_dir.display(), e)))?;

        while let Some(entry) = entries.next_entry()
            .await
            .map_err(|e| AppError::file(format!("读取目录条目失败: {}: {}", current_dir.display(), e)))?
        {
            let metadata = entry.metadata()
                .await
                .map_err(|e| AppError::file(format!("获取文件元数据失败: {}: {}", current_dir.display(), e)))?;

            if metadata.is_dir() {
                dirs_to_process.push(entry.path());
            }
        }
    }

    Ok(dir_count)
}

/// 检查文件是否可读
pub fn is_file_readable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

/// 检查文件是否可写
pub fn is_file_writable(path: &Path) -> bool {
    if !path.exists() {
        // 检查父目录是否可写
        return path.parent()
            .map(|parent| is_directory_writable(parent))
            .unwrap_or(false);
    }

    fs::metadata(path)
        .map(|metadata| {
            let permissions = metadata.permissions();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                permissions.mode() & 0o200 != 0
            }
            #[cfg(windows)]
            {
                !permissions.readonly()
            }
            #[cfg(not(any(unix, windows)))]
            {
                true // 未知平台，假设可写
            }
        })
        .unwrap_or(false)
}

/// 检查目录是否可写
pub fn is_directory_writable(path: &Path) -> bool {
    if !path.exists() {
        // 递归检查父目录
        return path.parent()
            .map(|parent| is_directory_writable(parent))
            .unwrap_or(false);
    }

    // 尝试创建临时文件来测试写入权限
    let temp_file = path.join(".write_test.tmp");

    match fs::File::create(&temp_file) {
        Ok(_) => {
            let _ = fs::remove_file(&temp_file);
            true
        }
        Err(_) => false,
    }
}

/// 获取规范化的路径（解析符号链接等）
pub fn get_canonical_path(path: &Path) -> Result<PathBuf, AppError> {
    path.canonicalize()
        .map_err(|e| AppError::file(format!("获取规范路径失败: {}: {}", path.display(), e)))
}

/// 获取相对路径
pub fn get_relative_path(path: &Path, base: &Path) -> Result<PathBuf, AppError> {
    pathdiff::diff_paths(path, base)
        .ok_or_else(|| AppError::file(format!(
            "无法计算相对路径: {} 相对于 {}",
            path.display(),
            base.display()
        )))
}

/// 清理文件名（移除非法字符）
pub fn sanitize_filename(filename: &str) -> String {
    let illegal_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];
    filename
        .chars()
        .filter(|c| !illegal_chars.contains(c))
        .collect()
}

/// 生成唯一文件名（避免冲突）
pub fn generate_unique_filename(directory: &Path, base_name: &str, extension: &str) -> PathBuf {
    let sanitized_name = sanitize_filename(base_name);
    let mut counter = 1;
    let mut candidate = directory.join(format!("{}.{}", sanitized_name, extension));

    while candidate.exists() {
        candidate = directory.join(format!("{} ({}).{}", sanitized_name, counter, extension));
        counter += 1;
    }

    candidate
}
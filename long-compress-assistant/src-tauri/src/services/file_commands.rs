use crate::services::file_service::{
    FileService, FileServiceConfig, FileInfo, FileFilter, HashAlgorithm,
    BatchOperationResult, FileComparisonResult, FilePreview, FilePermission,
    FileServiceError,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 文件服务状态
pub struct FileServiceState {
    service: Arc<Mutex<FileService>>,
}

impl FileServiceState {
    pub fn new(config: FileServiceConfig) -> Self {
        Self {
            service: Arc::new(Mutex::new(FileService::new(config))),
        }
    }

    pub fn default() -> Self {
        Self {
            service: Arc::new(Mutex::new(FileService::default())),
        }
    }

    pub async fn get_service(&self) -> tokio::sync::MutexGuard<'_, FileService> {
        self.service.lock().await
    }
}

/// 文件列表请求
#[derive(Debug, Deserialize)]
pub struct ListFilesRequest {
    pub dir_path: String,
    pub recursive: Option<bool>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub filter: Option<FileFilter>,
}

/// 文件列表响应
#[derive(Debug, Serialize)]
pub struct ListFilesResponse {
    pub files: Vec<FileInfo>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// 文件哈希请求
#[derive(Debug, Deserialize)]
pub struct FileHashRequest {
    pub file_path: String,
    pub algorithm: HashAlgorithm,
}

/// 文件哈希响应
#[derive(Debug, Serialize)]
pub struct FileHashResponse {
    pub file_path: String,
    pub algorithm: String,
    pub hash: String,
    pub success: bool,
    pub error: Option<String>,
}

/// 文件搜索请求
#[derive(Debug, Deserialize)]
pub struct SearchFilesRequest {
    pub root_dir: String,
    pub search_query: String,
    pub recursive: Option<bool>,
}

/// 批量操作请求
#[derive(Debug, Deserialize)]
pub struct BatchCopyRequest {
    pub source_files: Vec<String>,
    pub destination_dir: String,
}

/// 文件比较请求
#[derive(Debug, Deserialize)]
pub struct CompareFilesRequest {
    pub file1: String,
    pub file2: String,
}

/// 文件预览请求
#[derive(Debug, Deserialize)]
pub struct FilePreviewRequest {
    pub file_path: String,
}

/// 权限检查请求
#[derive(Debug, Deserialize)]
pub struct CheckPermissionRequest {
    pub path: String,
    pub permission: FilePermission,
}

/// 列出文件
#[tauri::command]
pub async fn list_files(
    state: State<'_, FileServiceState>,
    request: ListFilesRequest,
) -> Result<ListFilesResponse, String> {
    let service = state.get_service().await;

    let recursive = request.recursive.unwrap_or(false);
    let page = request.page.unwrap_or(0);
    let page_size = request.page_size.unwrap_or(50);

    if let Some(filter) = request.filter {
        // 使用过滤列表
        let files = service.list_files_filtered(&request.dir_path, recursive, filter)
            .await
            .map_err(|e| format!("列出文件失败: {}", e))?;

        let total = files.len();
        let start = page * page_size;
        let end = std::cmp::min(start + page_size, total);
        let total_pages = (total + page_size - 1) / page_size;

        let paged_files = if start < total {
            files[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok(ListFilesResponse {
            files: paged_files,
            total,
            page,
            page_size,
            total_pages,
        })
    } else if request.page.is_some() {
        // 使用分页列表
        let (files, total) = service.list_files_paged(&request.dir_path, recursive, page, page_size)
            .await
            .map_err(|e| format!("列出文件失败: {}", e))?;

        let total_pages = (total + page_size - 1) / page_size;

        Ok(ListFilesResponse {
            files,
            total,
            page,
            page_size,
            total_pages,
        })
    } else {
        // 普通列表
        let files = service.list_files(&request.dir_path, recursive)
            .await
            .map_err(|e| format!("列出文件失败: {}", e))?;

        let total = files.len();
        let total_pages = 1;

        Ok(ListFilesResponse {
            files,
            total,
            page: 0,
            page_size: total,
            total_pages,
        })
    }
}

/// 获取文件信息
#[tauri::command]
pub async fn get_file_info(
    state: State<'_, FileServiceState>,
    file_path: String,
) -> Result<FileInfo, String> {
    let service = state.get_service().await;

    service.get_file_info(&file_path)
        .await
        .map_err(|e| format!("获取文件信息失败: {}", e))
}

/// 计算文件哈希
#[tauri::command]
pub async fn calculate_file_hash(
    state: State<'_, FileServiceState>,
    request: FileHashRequest,
) -> Result<FileHashResponse, String> {
    let service = state.get_service().await;

    match service.calculate_file_hash(&request.file_path, request.algorithm).await {
        Ok(hash) => Ok(FileHashResponse {
            file_path: request.file_path,
            algorithm: format!("{:?}", request.algorithm),
            hash,
            success: true,
            error: None,
        }),
        Err(e) => Ok(FileHashResponse {
            file_path: request.file_path,
            algorithm: format!("{:?}", request.algorithm),
            hash: String::new(),
            success: false,
            error: Some(format!("{}", e)),
        }),
    }
}

/// 搜索文件
#[tauri::command]
pub async fn search_files(
    state: State<'_, FileServiceState>,
    request: SearchFilesRequest,
) -> Result<Vec<FileInfo>, String> {
    let service = state.get_service().await;

    let recursive = request.recursive.unwrap_or(false);

    service.search_files(&request.root_dir, &request.search_query, recursive)
        .await
        .map_err(|e| format!("搜索文件失败: {}", e))
}

/// 批量复制文件
#[tauri::command]
pub async fn batch_copy_files(
    state: State<'_, FileServiceState>,
    request: BatchCopyRequest,
) -> Result<BatchOperationResult, String> {
    let service = state.get_service().await;

    service.batch_copy_files(&request.source_files, &request.destination_dir)
        .await
        .map_err(|e| format!("批量复制文件失败: {}", e))
}

/// 复制文件
#[tauri::command]
pub async fn copy_file(
    state: State<'_, FileServiceState>,
    src: String,
    dst: String,
) -> Result<bool, String> {
    let service = state.get_service().await;

    service.copy_file(&src, &dst)
        .await
        .map_err(|e| format!("复制文件失败: {}", e))?;

    Ok(true)
}

/// 移动文件
#[tauri::command]
pub async fn move_file(
    state: State<'_, FileServiceState>,
    src: String,
    dst: String,
) -> Result<bool, String> {
    let service = state.get_service().await;

    service.move_file(&src, &dst)
        .await
        .map_err(|e| format!("移动文件失败: {}", e))?;

    Ok(true)
}

/// 删除文件或目录
#[tauri::command]
pub async fn delete_path(
    state: State<'_, FileServiceState>,
    path: String,
    force: Option<bool>,
) -> Result<bool, String> {
    let service = state.get_service().await;

    let force = force.unwrap_or(false);

    if force {
        service.delete_path_force(&path)
            .await
            .map_err(|e| format!("强制删除失败: {}", e))?;
    } else {
        service.delete_path(&path)
            .await
            .map_err(|e| format!("删除失败: {}", e))?;
    }

    Ok(true)
}

/// 创建目录
#[tauri::command]
pub async fn create_directory(
    state: State<'_, FileServiceState>,
    dir_path: String,
    recursive: Option<bool>,
) -> Result<bool, String> {
    let service = state.get_service().await;

    let recursive = recursive.unwrap_or(false);

    service.create_directory(&dir_path, recursive)
        .await
        .map_err(|e| format!("创建目录失败: {}", e))?;

    Ok(true)
}

/// 重命名文件或目录
#[tauri::command]
pub async fn rename_path(
    state: State<'_, FileServiceState>,
    old_path: String,
    new_path: String,
) -> Result<bool, String> {
    let service = state.get_service().await;

    service.rename_path(&old_path, &new_path)
        .await
        .map_err(|e| format!("重命名失败: {}", e))?;

    Ok(true)
}

/// 检查文件权限
#[tauri::command]
pub async fn check_permission(
    state: State<'_, FileServiceState>,
    request: CheckPermissionRequest,
) -> Result<bool, String> {
    let service = state.get_service().await;

    service.check_permissions(&request.path, request.permission)
        .await
        .map_err(|e| format!("检查权限失败: {}", e))
}

/// 比较两个文件
#[tauri::command]
pub async fn compare_files(
    state: State<'_, FileServiceState>,
    request: CompareFilesRequest,
) -> Result<FileComparisonResult, String> {
    let service = state.get_service().await;

    service.compare_files(&request.file1, &request.file2)
        .await
        .map_err(|e| format!("比较文件失败: {}", e))
}

/// 获取文件预览
#[tauri::command]
pub async fn get_file_preview(
    state: State<'_, FileServiceState>,
    request: FilePreviewRequest,
) -> Result<FilePreview, String> {
    let service = state.get_service().await;

    service.get_file_preview(&request.file_path)
        .await
        .map_err(|e| format!("获取文件预览失败: {}", e))
}

/// 获取目录大小
#[tauri::command]
pub async fn get_directory_size(
    state: State<'_, FileServiceState>,
    dir_path: String,
) -> Result<u64, String> {
    let service = state.get_service().await;

    service.get_directory_size(&dir_path)
        .await
        .map_err(|e| format!("获取目录大小失败: {}", e))
}

/// 检查文件是否为压缩文件
#[tauri::command]
pub async fn is_compressed_file(file_path: String) -> Result<bool, String> {
    Ok(crate::services::file_service::FileService::is_compressed_file(&file_path))
}

/// 格式化文件大小
#[tauri::command]
pub async fn format_file_size(size: u64) -> Result<String, String> {
    Ok(crate::services::file_service::FileService::format_file_size(size))
}

/// 获取文件服务配置
#[tauri::command]
pub async fn get_file_service_config(
    state: State<'_, FileServiceState>,
) -> Result<FileServiceConfig, String> {
    let service = state.get_service().await;
    Ok(service.config().clone())
}

/// 更新文件服务配置
#[tauri::command]
pub async fn update_file_service_config(
    state: State<'_, FileServiceState>,
    config: FileServiceConfig,
) -> Result<bool, String> {
    let mut service = state.get_service().await;
    service.update_config(config);
    Ok(true)
}

/// 检查文件是否存在
#[tauri::command]
pub async fn check_file_exists(file_path: String) -> Result<bool, String> {
    use std::path::Path;
    Ok(Path::new(&file_path).exists())
}

/// 检测文件类型
#[tauri::command]
pub async fn detect_file_type(file_path: String) -> Result<String, String> {
    use std::path::Path;
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err("文件不存在".to_string());
    }

    if path.is_dir() {
        Ok("directory".to_string())
    } else if path.is_file() {
        // 根据扩展名判断文件类型
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "txt" | "md" | "json" | "yaml" | "yml" => Ok("text".to_string()),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => Ok("image".to_string()),
            "mp3" | "wav" | "flac" | "ogg" => Ok("audio".to_string()),
            "mp4" | "avi" | "mkv" | "mov" => Ok("video".to_string()),
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => Ok("archive".to_string()),
            "pdf" => Ok("pdf".to_string()),
            "doc" | "docx" => Ok("document".to_string()),
            "xls" | "xlsx" => Ok("spreadsheet".to_string()),
            "ppt" | "pptx" => Ok("presentation".to_string()),
            "exe" | "msi" => Ok("executable".to_string()),
            _ => Ok("unknown".to_string()),
        }
    } else {
        Ok("unknown".to_string())
    }
}
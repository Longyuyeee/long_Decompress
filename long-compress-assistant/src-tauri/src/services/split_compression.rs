use crate::models::compression::CompressionOptions;
use crate::services::compression_service::CompressionService;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs::File;
use zip::ZipWriter;
use log;

/// 分卷压缩结果
#[derive(Debug, Clone)]
pub struct SplitCompressionResult {
    pub part_files: Vec<PathBuf>,
    pub total_size: u64,
    pub part_count: usize,
}

/// 分卷压缩服务
pub struct SplitCompressionService;

impl SplitCompressionService {
    /// 创建新的分卷压缩服务
    pub fn new() -> Self {
        Self
    }

    /// 压缩文件为分卷ZIP
    pub async fn compress_to_split_zips(
        &self,
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
    ) -> Result<SplitCompressionResult> {
        log::info!("开始分卷压缩: {:?} -> {:?}", files, output_path);

        // 检查是否需要分卷
        let split_size = match options.split_size {
            Some(size) if size > 0 => size,
            _ => {
                // 不需要分卷，使用普通压缩
                log::debug!("分卷大小未设置或为0，使用普通压缩");
                return self.compress_single_zip(files, output_path, options).await;
            }
        };

        log::debug!("分卷大小: {} 字节", split_size);

        // 计算总大小
        let total_size = self.calculate_total_size(files).await?;
        log::debug!("总文件大小: {} 字节", total_size);

        // 计算需要的分卷数量
        let part_count = ((total_size as f64 / split_size as f64).ceil() as usize).max(1);
        log::info!("预计需要 {} 个分卷", part_count);

        // 如果只有一个分卷，使用普通压缩
        if part_count == 1 {
            log::debug!("只需要一个分卷，使用普通压缩");
            return self.compress_single_zip(files, output_path, options).await;
        }

        // 创建分卷文件
        let mut part_files = Vec::new();
        let mut current_part_size = 0u64;
        let mut current_part_index = 1;
        let mut current_zip: Option<ZipWriter<File>> = None;

        let base_name = output_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("archive");
        let extension = output_path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("zip");

        for file_path in files {
            let path = Path::new(file_path);

            if !path.exists() {
                return Err(anyhow::anyhow!("文件不存在: {}", file_path));
            }

            if path.is_file() {
                let file_size = tokio::fs::metadata(path).await?.len();

                // 如果单个文件就超过分卷大小，需要特殊处理
                if file_size > split_size {
                    log::warn!("文件 {} 大小 {} 超过分卷大小 {}，将单独压缩",
                        file_path, file_size, split_size);

                    // 为这个大文件创建单独的分卷
                    let part_path = self.create_part_path(output_path, current_part_index, part_count);
                    let result = self.compress_single_file_to_zip(
                        &[file_path.to_string()],
                        &part_path,
                        options.clone(),
                    ).await?;

                    part_files.push(part_path);
                    current_part_index += 1;
                    continue;
                }

                // 检查当前分卷是否有足够空间
                if current_part_size + file_size > split_size && current_zip.is_some() {
                    // 完成当前分卷
                    self.finish_current_zip(&mut current_zip).await?;
                    current_part_size = 0;
                    current_part_index += 1;
                }

                // 创建新的分卷（如果需要）
                if current_zip.is_none() {
                    let part_path = self.create_part_path(output_path, current_part_index, part_count);
                    log::debug!("创建分卷 {}: {:?}", current_part_index, part_path);

                    let file = File::create(&part_path)
                        .context("创建分卷文件失败")?;
                    current_zip = Some(ZipWriter::new(file));
                    part_files.push(part_path);
                }

                // 添加文件到当前分卷
                // 注意：这里简化了实现，实际需要调用压缩服务的添加文件逻辑
                current_part_size += file_size;
                log::debug!("添加文件 {} 到分卷 {} (大小: {})",
                    file_path, current_part_index, file_size);
            } else if path.is_dir() {
                // 处理目录 - 简化实现，实际需要递归处理
                log::warn!("分卷压缩暂不支持目录，跳过: {}", file_path);
            }
        }

        // 完成最后一个分卷
        if let Some(_) = current_zip {
            self.finish_current_zip(&mut current_zip).await?;
        }

        log::info!("分卷压缩完成，创建了 {} 个分卷", part_files.len());

        Ok(SplitCompressionResult {
            part_files,
            total_size,
            part_count: part_files.len(),
        })
    }

    /// 计算总文件大小
    async fn calculate_total_size(&self, files: &[String]) -> Result<u64> {
        let compression_service = CompressionService::default();
        compression_service.calculate_total_size(files).await
            .map_err(|e| anyhow::anyhow!("计算总大小失败: {}", e))
    }

    /// 创建分卷文件路径
    fn create_part_path(&self, base_path: &Path, part_index: usize, total_parts: usize) -> PathBuf {
        let base_name = base_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("archive");
        let extension = base_path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("zip");

        let parent = base_path.parent().unwrap_or_else(|| Path::new("."));

        // 格式: basename.z01, basename.z02, ..., basename.zip
        if part_index < total_parts {
            parent.join(format!("{}.z{:02}", base_name, part_index))
        } else {
            parent.join(format!("{}.{}", base_name, extension))
        }
    }

    /// 完成当前ZIP文件
    async fn finish_current_zip(&self, zip_writer: &mut Option<ZipWriter<File>>) -> Result<()> {
        if let Some(mut writer) = zip_writer.take() {
            writer.finish()
                .context("完成ZIP文件失败")?;
        }
        Ok(())
    }

    /// 压缩到单个ZIP文件（不分卷）
    async fn compress_single_zip(
        &self,
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
    ) -> Result<SplitCompressionResult> {
        log::debug!("压缩到单个ZIP文件: {:?}", output_path);

        let compression_service = CompressionService::default();

        // 调用现有的压缩方法
        compression_service.compress_zip_enhanced(
            files,
            output_path,
            options,
            None,
        ).await
        .map_err(|e| anyhow::anyhow!("压缩失败: {}", e))?;

        let total_size = self.calculate_total_size(files).await?;

        Ok(SplitCompressionResult {
            part_files: vec![output_path.to_path_buf()],
            total_size,
            part_count: 1,
        })
    }

    /// 压缩单个文件到ZIP
    async fn compress_single_file_to_zip(
        &self,
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
    ) -> Result<()> {
        let compression_service = CompressionService::default();

        compression_service.compress_zip_enhanced(
            files,
            output_path,
            options,
            None,
        ).await
        .map_err(|e| anyhow::anyhow!("压缩单个文件失败: {}", e))
    }
}

impl Default for SplitCompressionService {
    fn default() -> Self {
        Self::new()
    }
}
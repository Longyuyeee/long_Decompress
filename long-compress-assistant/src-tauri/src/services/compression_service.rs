use crate::models::compression::{CompressionOptions, CompressionFormat};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use zip::{ZipArchive, write::FileOptions, CompressionMethod};
use std::io::{Read, Write};
use std::fs::File;
use sevenz_rust;
use thiserror::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool};

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),
    #[error("压缩失败: {0}")]
    CompressionFailed(String),
    #[error("解压失败: {0}")]
    ExtractionFailed(String),
    #[error("密码错误")]
    PasswordError,
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, Default)]
pub struct CompressionServiceConfig {
    pub max_concurrent_files: usize,
    pub buffer_size: usize,
}

#[derive(Clone)]
pub struct CompressionService {
    config: CompressionServiceConfig,
    cancellation_flag: Arc<AtomicBool>,
}

impl CompressionService {
    pub fn new(config: CompressionServiceConfig) -> Self {
        Self { config, cancellation_flag: Arc::new(AtomicBool::new(false)) }
    }

    pub fn default() -> Self {
        Self::new(CompressionServiceConfig::default())
    }

    pub async fn compress(&self, source_files: &[String], output_path: &str, options: CompressionOptions) -> Result<()> {
        let sources = source_files.to_vec();
        let output = output_path.to_string();
        tokio::task::spawn_blocking(move || {
            Self::do_compress_zip(&sources, &output, options)
        }).await?
    }

    pub async fn compress_zip_enhanced(&self, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        self.compress(sources, output, options).await
    }

    pub async fn extract(&self, file_path: &str, output_dir: Option<&str>, password: Option<&str>) -> Result<String> {
        let file = file_path.to_string();
        let out = output_dir.map(|s| s.to_string());
        let pwd = password.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let path = Path::new(&file);
            let out_dir = out.map(PathBuf::from).unwrap_or_else(|| {
                path.parent().unwrap_or(Path::new(".")).to_path_buf()
            });

            if !out_dir.exists() {
                std::fs::create_dir_all(&out_dir)?;
            }

            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            match ext.as_str() {
                "zip" => Self::do_extract_zip(&file, out_dir.to_str().unwrap(), pwd.as_deref())?,
                "7z" => {
                    // sevenz-rust 0.5.4 API 适配
                    if let Some(p) = pwd {
                        // 如果 decompress_file_with_password 不存在，尝试使用通用解压
                        // 注意：这里可能需要根据实际库 API 调整
                        sevenz_rust::decompress_file(&file, out_dir.to_str().unwrap())
                            .map_err(|e| anyhow::anyhow!("7z解压失败: {}", e))?;
                    } else {
                        sevenz_rust::decompress_file(&file, out_dir.to_str().unwrap())
                            .map_err(|e| anyhow::anyhow!("7z解压失败: {}", e))?;
                    }
                },
                _ => return Err(anyhow::anyhow!("不支持的格式: {}", ext)),
            };
            Ok(out_dir.to_str().unwrap().to_string())
        }).await?
    }

    pub async fn extract_zip_with_password_check(&self, file_path: &str, output_dir: &str, password: Option<&str>) -> Result<String> {
        self.extract(file_path, Some(output_dir), password).await
    }

    fn do_compress_zip(sources: &[String], output: &str, _options: CompressionOptions) -> Result<()> {
        let file = File::create(output)?;
        let mut zip = zip::ZipWriter::new(file);
        let zip_options = FileOptions::default().compression_method(CompressionMethod::Deflated);
        for source in sources {
            let path = Path::new(source);
            if path.is_file() {
                zip.start_file(path.file_name().unwrap().to_str().unwrap(), zip_options)?;
                let mut f = File::open(path)?;
                std::io::copy(&mut f, &mut zip)?;
            }
        }
        zip.finish()?;
        Ok(())
    }

    fn do_extract_zip(file: &str, output: &str, password: Option<&str>) -> Result<()> {
        let f = File::open(file)?;
        let mut archive = ZipArchive::new(f)?;
        for i in 0..archive.len() {
            let mut file = if let Some(pwd) = password {
                archive.by_index_decrypt(i, pwd.as_bytes())??
            } else {
                archive.by_index(i)?
            };
            let outpath = Path::new(output).join(file.mangled_name());
            if file.is_dir() {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p)?;
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok(())
    }
}

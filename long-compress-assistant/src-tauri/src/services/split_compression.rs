use crate::models::compression::CompressionOptions;
use crate::services::compression_service::{CompressionError, CompressionService};
use crate::utils::archive_tools::{find_7z_command, missing_7z_message};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone)]
pub struct SplitCompressionResult {
    pub part_files: Vec<PathBuf>,
    pub total_size: u64,
    pub part_count: usize,
}

pub struct SplitCompressionService;

impl SplitCompressionService {
    pub fn new() -> Self {
        Self
    }

    pub async fn compress_to_split_zips(
        &self,
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
    ) -> Result<SplitCompressionResult> {
        self.compress_to_split_zips_cancellable(
            files,
            output_path,
            options,
            Arc::new(AtomicBool::new(false)),
        )
        .await
    }

    pub async fn compress_to_split_zips_cancellable(
        &self,
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
        cancellation: Arc<AtomicBool>,
    ) -> Result<SplitCompressionResult> {
        if options.password.as_deref().is_some_and(|value| !value.is_empty()) {
            return Err(anyhow::anyhow!(
                "Encrypted split ZIP creation is not supported safely"
            ));
        }
        if files.is_empty() {
            return Err(anyhow::anyhow!("At least one source file is required"));
        }

        let mut total_size = 0u64;
        for source in files {
            let path = Path::new(source);
            if !path.is_file() {
                return Err(anyhow::anyhow!(
                    "Split ZIP creation supports regular files only: {}",
                    path.display()
                ));
            }
            total_size = total_size.checked_add(path.metadata()?.len()).ok_or_else(|| {
                anyhow::anyhow!("Source size overflowed the supported range")
            })?;
        }

        let Some(split_size) = options.split_size.filter(|size| *size > 0) else {
            let service = CompressionService::new_with_defaults().await;
            service
                .compress_zip_enhanced(files, output_path.to_string_lossy().as_ref(), options)
                .await?;
            return Ok(SplitCompressionResult {
                part_files: vec![output_path.to_path_buf()],
                total_size,
                part_count: 1,
            });
        };

        let engine = find_7z_command().ok_or_else(|| anyhow::anyhow!(missing_7z_message()))?;
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let existing_parts = Self::discover_parts(output_path)?;
        if output_path.exists() || !existing_parts.is_empty() {
            return Err(anyhow::anyhow!(
                "Split archive output already exists: {}",
                output_path.display()
            ));
        }

        let mut command = crate::utils::process::async_command(engine);
        command
            .arg("a")
            .arg("-tzip")
            .arg("-y")
            .arg(format!("-mx{}", options.level.clamp(1, 9)))
            .arg(format!("-v{}b", split_size))
            .arg(output_path);
        for source in files {
            command.arg(source);
        }
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        command.kill_on_drop(true);
        let mut child = command.spawn().context("Unable to start the standard split ZIP engine")?;
        let mut stdout = child.stdout.take().context("Unable to capture split-engine output")?;
        let mut stderr = child.stderr.take().context("Unable to capture split-engine errors")?;
        let stdout_reader = tokio::spawn(async move {
            let mut bytes = Vec::new();
            let _ = stdout.read_to_end(&mut bytes).await;
            bytes
        });
        let stderr_reader = tokio::spawn(async move {
            let mut bytes = Vec::new();
            let _ = stderr.read_to_end(&mut bytes).await;
            bytes
        });

        let mut suspended = false;
        let status = loop {
            if let Some(process_id) = child.id() {
                if let Err(error) = crate::services::task_control::sync_child_pause(
                    &cancellation,
                    process_id,
                    &mut suspended,
                ) {
                    let _ = child.kill().await;
                    Self::cleanup_parts(output_path);
                    return Err(anyhow::anyhow!(error));
                }
            }
            tokio::select! {
                result = child.wait() => break result?,
                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                    if cancellation.load(Ordering::SeqCst) {
                        let _ = child.kill().await;
                        Self::cleanup_parts(output_path);
                        return Err(CompressionError::Cancelled.into());
                    }
                }
            }
        };
        let stdout = stdout_reader.await.unwrap_or_default();
        let stderr = stderr_reader.await.unwrap_or_default();
        if !status.success() {
            Self::cleanup_parts(output_path);
            let detail = if stderr.is_empty() {
                String::from_utf8_lossy(&stdout).trim().to_string()
            } else {
                String::from_utf8_lossy(&stderr).trim().to_string()
            };
            return Err(anyhow::anyhow!(
                "Standard split ZIP creation failed with status {}: {}",
                status,
                detail
            ));
        }

        let part_files = Self::discover_parts(output_path)?;
        if part_files.is_empty() {
            Self::cleanup_parts(output_path);
            return Err(anyhow::anyhow!("Split engine completed without producing volume files"));
        }
        Ok(SplitCompressionResult {
            part_count: part_files.len(),
            part_files,
            total_size,
        })
    }

    fn discover_parts(output_path: &Path) -> Result<Vec<PathBuf>> {
        let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
        let base = output_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("Split archive output name is invalid"))?;
        let prefix = format!("{}.", base);
        let mut parts = Vec::new();
        if !parent.exists() {
            return Ok(parts);
        }
        for entry in std::fs::read_dir(parent)? {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Some(suffix) = name.strip_prefix(&prefix) else {
                continue;
            };
            if suffix.len() >= 3 && suffix.chars().all(|value| value.is_ascii_digit()) {
                parts.push(path);
            }
        }
        parts.sort();
        Ok(parts)
    }

    pub(crate) fn cleanup_parts(output_path: &Path) {
        if let Ok(parts) = Self::discover_parts(output_path) {
            for part in parts {
                let _ = std::fs::remove_file(part);
            }
        }
        let _ = std::fs::remove_file(output_path);
    }
}

impl Default for SplitCompressionService {
    fn default() -> Self {
        Self::new()
    }
}

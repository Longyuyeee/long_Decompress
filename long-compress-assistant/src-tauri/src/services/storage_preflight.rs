use std::path::{Path, PathBuf};

use serde::Serialize;
use sysinfo::{DiskKind, Disks};

use crate::services::archive_browser;

pub const DISK_SAFETY_RESERVE: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageTarget {
    pub probe_path: String,
    pub mount_point: Option<String>,
    pub file_system: Option<String>,
    pub location: String,
    pub medium: String,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePreflightReport {
    pub operation: String,
    pub output_path: String,
    pub probe_path: String,
    pub mount_point: Option<String>,
    pub file_system: Option<String>,
    pub location: String,
    pub medium: String,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub estimated_output_bytes: Option<u64>,
    pub required_bytes: Option<u64>,
    pub reserve_bytes: u64,
    pub estimate_source: String,
    pub estimate_reliable: bool,
    pub status: String,
    pub can_start: bool,
    pub summary: String,
    pub warnings: Vec<String>,
}

fn absolute_probe_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn is_unc_path(path: &Path) -> bool {
    let value = path.to_string_lossy().replace('/', "\\");
    value.starts_with("\\\\")
}

fn medium_label(kind: DiskKind) -> String {
    match kind {
        DiskKind::SSD => "ssd",
        DiskKind::HDD => "hdd",
        DiskKind::Unknown(_) => "unknown",
    }
    .to_string()
}

#[cfg(windows)]
fn nearest_existing_path(path: &Path) -> PathBuf {
    let mut candidate = path.to_path_buf();
    while !candidate.exists() && candidate.pop() {}
    candidate
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetDiskFreeSpaceExW(
        directory_name: *const u16,
        free_bytes_available: *mut u64,
        total_bytes: *mut u64,
        total_free_bytes: *mut u64,
    ) -> i32;
    fn GetVolumePathNameW(
        file_name: *const u16,
        volume_path_name: *mut u16,
        buffer_length: u32,
    ) -> i32;
    fn GetVolumeInformationW(
        root_path_name: *const u16,
        volume_name: *mut u16,
        volume_name_size: u32,
        volume_serial_number: *mut u32,
        maximum_component_length: *mut u32,
        file_system_flags: *mut u32,
        file_system_name: *mut u16,
        file_system_name_size: u32,
    ) -> i32;
    fn GetDriveTypeW(root_path_name: *const u16) -> u32;
}

#[cfg(windows)]
fn probe_windows_storage(requested: &Path) -> Option<StorageTarget> {
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_REMOTE: u32 = 4;

    let existing = nearest_existing_path(requested);
    if existing.as_os_str().is_empty() {
        return None;
    }
    let existing_wide = wide_path(&existing);
    let mut available = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    // SAFETY: all pointers target live writable/read-only buffers for the call.
    if unsafe {
        GetDiskFreeSpaceExW(
            existing_wide.as_ptr(),
            &mut available,
            &mut total,
            &mut total_free,
        )
    } == 0
    {
        return None;
    }

    let mut volume_buffer = vec![0u16; 512];
    let has_volume = unsafe {
        GetVolumePathNameW(
            existing_wide.as_ptr(),
            volume_buffer.as_mut_ptr(),
            volume_buffer.len() as u32,
        )
    } != 0;
    let volume_length = volume_buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(0);
    let volume_path = has_volume.then(|| String::from_utf16_lossy(&volume_buffer[..volume_length]));

    let mut file_system_buffer = vec![0u16; 64];
    let file_system = volume_path.as_ref().and_then(|volume| {
        let volume_wide = wide_path(Path::new(volume));
        let ok = unsafe {
            GetVolumeInformationW(
                volume_wide.as_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                file_system_buffer.as_mut_ptr(),
                file_system_buffer.len() as u32,
            )
        } != 0;
        if !ok {
            return None;
        }
        let length = file_system_buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(0);
        Some(String::from_utf16_lossy(&file_system_buffer[..length]))
    });
    let drive_type = volume_path.as_ref().map(|volume| {
        let volume_wide = wide_path(Path::new(volume));
        unsafe { GetDriveTypeW(volume_wide.as_ptr()) }
    });
    let location = match drive_type {
        Some(DRIVE_REMOVABLE) => "removable",
        Some(DRIVE_REMOTE) => "network",
        _ => "local",
    };

    Some(StorageTarget {
        probe_path: requested.to_string_lossy().to_string(),
        mount_point: volume_path,
        file_system,
        location: location.to_string(),
        medium: "unknown".to_string(),
        total_bytes: Some(total),
        available_bytes: Some(available),
    })
}

#[cfg(not(windows))]
fn probe_windows_storage(_: &Path) -> Option<StorageTarget> {
    None
}

pub fn probe_storage(path: &Path) -> StorageTarget {
    let requested = absolute_probe_path(path);
    let native_target = probe_windows_storage(&requested);
    let disks = Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .filter(|disk| requested.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count());

    if let (Some(native), Some(disk)) = (&native_target, disk) {
        let native_depth = native
            .mount_point
            .as_deref()
            .map(Path::new)
            .map(|mount| mount.components().count())
            .unwrap_or_default();
        if native_depth > disk.mount_point().components().count() {
            return native.clone();
        }
    }

    if let Some(disk) = disk {
        let location = if disk.is_removable() {
            "removable"
        } else {
            "local"
        };
        return StorageTarget {
            probe_path: requested.to_string_lossy().to_string(),
            mount_point: Some(disk.mount_point().to_string_lossy().to_string()),
            file_system: Some(disk.file_system().to_string_lossy().to_string()),
            location: location.to_string(),
            medium: medium_label(disk.kind()),
            total_bytes: Some(disk.total_space()),
            available_bytes: Some(disk.available_space()),
        };
    }

    if let Some(target) = native_target {
        return target;
    }

    StorageTarget {
        probe_path: requested.to_string_lossy().to_string(),
        mount_point: None,
        file_system: None,
        location: if is_unc_path(&requested) {
            "network"
        } else {
            "unknown"
        }
        .to_string(),
        medium: "unknown".to_string(),
        total_bytes: None,
        available_bytes: None,
    }
}

pub fn available_disk_space(path: &Path) -> Option<u64> {
    probe_storage(path).available_bytes
}

fn evaluate_preflight(
    operation: &str,
    output_path: &str,
    target: StorageTarget,
    estimated_output_bytes: Option<u64>,
    estimate_source: &str,
    estimate_reliable: bool,
    mut warnings: Vec<String>,
) -> ResourcePreflightReport {
    let required_bytes =
        estimated_output_bytes.map(|estimate| estimate.saturating_add(DISK_SAFETY_RESERVE));

    match target.location.as_str() {
        "network" => warnings.push(
            "目标位于网络位置；吞吐和断线风险取决于网络，任务仍使用事务式临时输出。".to_string(),
        ),
        "removable" => warnings.push(
            "目标位于可移动设备；开始前请确认设备不会被拔出，且文件系统支持目标文件大小。"
                .to_string(),
        ),
        _ => {}
    }
    if target.medium == "hdd" {
        warnings.push("目标为机械硬盘；大量小文件任务可能受随机写入性能限制。".to_string());
    }

    let (status, can_start, summary) = match target.available_bytes {
        Some(available) if available < DISK_SAFETY_RESERVE => (
            "blocked",
            false,
            "目标盘可用空间低于 128 MiB 安全预留，任务未启动。".to_string(),
        ),
        Some(available) if required_bytes.is_some_and(|required| available < required) => (
            if estimate_reliable {
                "blocked"
            } else {
                "warning"
            },
            !estimate_reliable,
            if estimate_reliable {
                "目标盘空间不足以容纳预计输出和安全预留，任务未启动。".to_string()
            } else {
                warnings.push(
                    "非可靠估算显示可用空间可能不足；任务允许启动，并继续依赖运行时保护。"
                        .to_string(),
                );
                "预计输出可能超过当前可用空间，请留意任务日志。".to_string()
            },
        ),
        Some(_) if estimated_output_bytes.is_some() => (
            "ready",
            true,
            "目标盘空间满足当前估算；运行时仍会持续执行事务与容量检查。".to_string(),
        ),
        Some(_) => {
            warnings.push(
                "无法在启动前取得可靠输出体积；仅验证安全预留，运行时会继续检查。".to_string(),
            );
            (
                "warning",
                true,
                "已验证目标盘安全预留，但输出体积仍需在运行时确认。".to_string(),
            )
        }
        None => {
            warnings.push(
                "无法读取目标位置的容量；允许启动，但运行时容量检查仍可能阻止写入。".to_string(),
            );
            (
                "warning",
                true,
                "目标容量未知；任务将依赖运行时事务与容量保护。".to_string(),
            )
        }
    };

    let status = if status == "ready" && !warnings.is_empty() {
        "warning"
    } else {
        status
    };

    ResourcePreflightReport {
        operation: operation.to_string(),
        output_path: output_path.to_string(),
        probe_path: target.probe_path,
        mount_point: target.mount_point,
        file_system: target.file_system,
        location: target.location,
        medium: target.medium,
        total_bytes: target.total_bytes,
        available_bytes: target.available_bytes,
        estimated_output_bytes,
        required_bytes,
        reserve_bytes: DISK_SAFETY_RESERVE,
        estimate_source: estimate_source.to_string(),
        estimate_reliable,
        status: status.to_string(),
        can_start,
        summary,
        warnings,
    }
}

pub async fn preflight_operation_resources(
    operation: &str,
    output_path: &str,
    source_paths: &[String],
    password: Option<&str>,
    estimated_output_bytes: Option<u64>,
    estimate_reliable: bool,
) -> anyhow::Result<ResourcePreflightReport> {
    if !matches!(operation, "compression" | "decompression") {
        anyhow::bail!("Unsupported resource preflight operation: {operation}");
    }
    if output_path.trim().is_empty() {
        anyhow::bail!("Resource preflight requires an output path");
    }

    let mut warnings = Vec::new();
    let (estimate, estimate_source, reliable) =
        if operation == "decompression" && estimated_output_bytes.is_none() {
            match source_paths.first() {
                Some(source) => {
                    match archive_browser::browse_archive(Path::new(source), password).await {
                        Ok(metadata) if metadata.total_uncompressed_size > 0 => (
                            Some(metadata.total_uncompressed_size),
                            "archive_metadata",
                            true,
                        ),
                        Ok(_) => (None, "unknown", false),
                        Err(error) => {
                            warnings.push(format!(
                                "启动前无法读取归档展开体积：{error}；不会记录或回传密码。"
                            ));
                            (None, "unknown", false)
                        }
                    }
                }
                None => (None, "unknown", false),
            }
        } else if estimated_output_bytes.is_some() {
            (
                estimated_output_bytes,
                "provided_estimate",
                estimate_reliable,
            )
        } else {
            (None, "unknown", false)
        };

    let target = probe_storage(Path::new(output_path));
    Ok(evaluate_preflight(
        operation,
        output_path,
        target,
        estimate,
        estimate_source,
        reliable,
        warnings,
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn target(available_bytes: Option<u64>, location: &str, medium: &str) -> StorageTarget {
        StorageTarget {
            probe_path: "X:\\output".to_string(),
            mount_point: Some("X:\\".to_string()),
            file_system: Some("NTFS".to_string()),
            location: location.to_string(),
            medium: medium.to_string(),
            total_bytes: Some(4 * 1024 * 1024 * 1024),
            available_bytes,
        }
    }

    #[test]
    fn blocks_when_estimate_and_reserve_do_not_fit() {
        let report = evaluate_preflight(
            "compression",
            "X:\\archive.zip",
            target(Some(200 * 1024 * 1024), "local", "ssd"),
            Some(100 * 1024 * 1024),
            "provided_estimate",
            true,
            Vec::new(),
        );
        assert_eq!(report.status, "blocked");
        assert!(!report.can_start);
        assert_eq!(report.required_bytes, Some(228 * 1024 * 1024));
    }

    #[test]
    fn unknown_capacity_warns_but_keeps_runtime_fallback() {
        let report = evaluate_preflight(
            "decompression",
            "\\\\server\\share\\output",
            target(None, "network", "unknown"),
            None,
            "unknown",
            false,
            Vec::new(),
        );
        assert_eq!(report.status, "warning");
        assert!(report.can_start);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("网络")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("容量")));
    }

    #[test]
    fn mechanical_disk_is_visible_without_changing_scheduling() {
        let report = evaluate_preflight(
            "compression",
            "X:\\archive.zip",
            target(Some(2 * 1024 * 1024 * 1024), "local", "hdd"),
            Some(256 * 1024 * 1024),
            "provided_estimate",
            true,
            Vec::new(),
        );
        assert_eq!(report.status, "warning");
        assert!(report.can_start);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("机械硬盘")));
    }

    #[test]
    fn unreliable_compression_estimate_warns_instead_of_false_blocking() {
        let report = evaluate_preflight(
            "compression",
            "X:\\archive.zip",
            target(Some(200 * 1024 * 1024), "local", "ssd"),
            Some(100 * 1024 * 1024),
            "provided_estimate",
            false,
            Vec::new(),
        );
        assert_eq!(report.status, "warning");
        assert!(report.can_start);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("非可靠估算")));
    }

    #[test]
    fn current_temporary_directory_resolves_to_a_real_volume() {
        let directory = tempfile::tempdir().unwrap();
        let target = probe_storage(directory.path());
        assert!(target.available_bytes.is_some());
        assert!(target.mount_point.is_some());
    }

    #[cfg(windows)]
    #[test]
    fn native_windows_probe_reads_capacity_for_a_missing_descendant() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("not-created").join("output");
        let target = probe_windows_storage(&missing).expect("Windows volume probe");
        assert!(target.total_bytes.is_some_and(|value| value > 0));
        assert!(target.available_bytes.is_some());
        assert!(target.mount_point.is_some());
        assert_ne!(target.location, "unknown");
    }

    #[cfg(windows)]
    #[test]
    fn configured_windows_volume_resolves_through_public_probe() {
        let Ok(path) = std::env::var("LONG_STORAGE_PREFLIGHT_TEST_PATH") else {
            return;
        };
        let target = probe_storage(Path::new(&path));
        assert!(target.total_bytes.is_some_and(|value| value > 0));
        assert!(target.available_bytes.is_some_and(|value| value > 0));
        assert!(target.mount_point.is_some());
        assert_ne!(target.location, "unknown");
    }

    #[tokio::test]
    async fn decompression_uses_real_archive_metadata_before_starting() {
        let directory = tempfile::tempdir().unwrap();
        let archive_path = directory.path().join("preflight.zip");
        let file = std::fs::File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("payload.txt", zip::write::FileOptions::default())
            .unwrap();
        writer.write_all(b"real payload").unwrap();
        writer.finish().unwrap();

        let report = preflight_operation_resources(
            "decompression",
            directory.path().to_str().unwrap(),
            &[archive_path.to_string_lossy().to_string()],
            None,
            None,
            false,
        )
        .await
        .unwrap();

        assert_eq!(report.estimated_output_bytes, Some(12));
        assert_eq!(report.estimate_source, "archive_metadata");
        assert!(report.estimate_reliable);
        assert!(report.can_start);
    }
}

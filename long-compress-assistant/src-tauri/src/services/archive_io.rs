//! Keep filesystem failures distinct from data/password failures at IPC boundaries.
use std::io;

pub(crate) fn kind(error: &anyhow::Error) -> Option<(&'static str, &'static str)> {
    use super::compression_service::CompressionError;
    if error.chain().any(|cause| matches!(cause.downcast_ref::<CompressionError>(), Some(CompressionError::DiskFull))) {
        return Some(("disk-full", "磁盘空间不足，请释放空间或选择空间充足的输出目录后创建新任务"));
    }
    if error.chain().any(|cause| matches!(cause.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled))) { return None; }
    let cause = error.chain().find_map(|cause| {
        cause.downcast_ref::<io::Error>()
            .or_else(|| match cause.downcast_ref::<zip_aes::result::ZipError>() { Some(zip_aes::result::ZipError::Io(io)) => Some(io), _ => None })
            .or_else(|| match cause.downcast_ref::<sevenz_rust::Error>() { Some(sevenz_rust::Error::Io(io, _)) => Some(io), _ => None })
    })?;
    // Sharing/lock violations are not the same as account permissions.
    #[cfg(windows)]
    if matches!(cause.raw_os_error(), Some(32 | 33)) {
        return Some(("target-busy", "文件正在被其他程序占用，请按原始路径关闭占用后创建新任务"));
    }
    match cause.kind() {
        io::ErrorKind::StorageFull => Some(("disk-full", "磁盘空间不足，请释放空间或选择空间充足的输出目录后创建新任务")),
        io::ErrorKind::PermissionDenied => Some(("permission", "文件访问受阻，请按原始路径确认权限后创建新任务")),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory | io::ErrorKind::IsADirectory |
        io::ErrorKind::AlreadyExists | io::ErrorKind::ReadOnlyFilesystem |
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut |
        io::ErrorKind::BrokenPipe | io::ErrorKind::WriteZero =>
            Some(("io", "文件读写未完成，请按原始路径检查来源、输出目录和设备连接，处理后创建新任务")),
        // Decoder-generated EOF/invalid data is not proof of a filesystem fault.
        io::ErrorKind::Other if cause.raw_os_error().is_some() => Some(("io", "设备读写失败，请按原始原因恢复设备或访问条件后创建新任务")),
        _ => None,
    }
}

pub(crate) fn message(error: &anyhow::Error) -> String {
    let original = format!("{error:#}");
    // These markers confirm a source or transaction boundary; retain priority.
    if original.contains("[archive-source:") || original.contains("[archive-output:")
        || original.contains("[archive-io:") || original.contains("[archive-inspection:") {
        return original;
    }
    if ["[egg-io]", "[alz-io]", "[zpaq-io]"].iter().any(|tag| original.contains(tag)) {
        return format!("[archive-io:io] 文件读写未完成，请按原始原因检查空间、设备和访问条件，处理后创建新任务。原始原因：{original}");
    }
    match kind(error) {
        Some((code, summary)) => format!("[archive-io:{code}] {summary}。原始原因：{original}"),
        None => original,
    }
}

pub(crate) fn seven_zip_error(error: impl std::error::Error + Send + Sync + 'static) -> anyhow::Error {
    // Keep the owned typed error, including OS codes, across native writers.
    anyhow::Error::new(error).context("7Z 文件读写或编码未完成")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn io1516_typed_native_failures_precede_password_text_without_inventing_corruption() {
        for k in [io::ErrorKind::StorageFull, io::ErrorKind::PermissionDenied, io::ErrorKind::NotFound] {
            let e = seven_zip_error(sevenz_rust::Error::io_msg(io::Error::new(k, "filesystem detail"), "wrong password is not the cause"));
            let text = message(&e);
            assert!(text.contains("[archive-io:"), "{text}");
            assert!(text.contains("filesystem detail"));
        }
        for k in [io::ErrorKind::UnexpectedEof, io::ErrorKind::InvalidData] {
            assert_eq!(kind(&io::Error::from(k).into()), None);
        }
        let e = anyhow::Error::new(io::Error::from(io::ErrorKind::StorageFull))
            .context("[archive-output:Publishing:rollback-incomplete] recovery retained");
        assert!(message(&e).starts_with("[archive-output:Publishing:rollback-incomplete]"));
    }

    #[cfg(windows)]
    #[test]
    fn io1516_real_windows_sharing_violation_is_preserved() {
        use std::os::windows::fs::OpenOptionsExt;
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("占用的目标.zip");
        std::fs::write(&file, b"existing output").unwrap();
        let _lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(&file).unwrap();
        let error: anyhow::Error = std::fs::File::create(&file).unwrap_err().into();
        assert_eq!(kind(&error).unwrap().0, "target-busy");
        assert!(message(&error).contains("os error 32"));
    }
}

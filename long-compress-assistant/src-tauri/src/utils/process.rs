use std::ffi::OsStr;

/// Build a child process without creating a visible console window on Windows.
pub fn command(program: impl AsRef<OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Tokio equivalent of [`command`].
pub fn async_command(program: impl AsRef<OsStr>) -> tokio::process::Command {
    let command = command(program);
    tokio::process::Command::from(command)
}

/// Wait for child exit before returning cancellation so callers can remove
/// files held open by ffprobe on Windows, not merely request kill-on-drop.
pub async fn cancellable_output(
    command: &mut tokio::process::Command,
    cancelled: &std::sync::atomic::AtomicBool,
    timeout: std::time::Duration,
) -> std::io::Result<std::process::Output> {
    use std::{io, process::Stdio, sync::atomic::Ordering};
    use tokio::io::AsyncReadExt;
    if cancelled.load(Ordering::Acquire) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "VIDEO_COMPRESSION_CANCELLED",
        ));
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).await.map(|_| bytes)
    });
    let stderr_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).await.map(|_| bytes)
    });
    let cancellation = async {
        while !cancelled.load(Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    };
    let status = tokio::select! {
        status = child.wait() => status,
        _ = cancellation => {
            let result = child.kill().await;
            result.and(Err(io::Error::new(io::ErrorKind::Interrupted, "VIDEO_COMPRESSION_CANCELLED")))
        },
        _ = tokio::time::sleep(timeout) => {
            let result = child.kill().await;
            result.and(Err(io::Error::new(io::ErrorKind::TimedOut, "video probe timed out")))
        },
    };
    let stdout = stdout_reader.await.map_err(io::Error::other)??;
    let stderr = stderr_reader.await.map_err(io::Error::other)??;
    Ok(std::process::Output {
        status: status?,
        stdout,
        stderr,
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Duration,
    };

    #[tokio::test]
    async fn cancellation_releases_child_file_handle_before_returning() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("held.tmp");
        let ready = directory.path().join("ready");
        std::fs::write(&source, b"temporary encoding output").unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let child_flag = cancelled.clone();
        let mut command = async_command("powershell.exe");
        command.env("LONG_PROCESS_TEST_FILE", &source).env("LONG_PROCESS_TEST_READY", &ready)
            .args(["-NoProfile", "-NonInteractive", "-Command", "$handle=[IO.File]::Open($env:LONG_PROCESS_TEST_FILE,'Open','Read','None'); [IO.File]::WriteAllText($env:LONG_PROCESS_TEST_READY,'ready'); Start-Sleep -Seconds 30"]);
        let running = tokio::spawn(async move {
            cancellable_output(&mut command, &child_flag, Duration::from_secs(10)).await
        });
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        while !ready.exists() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let opened = ready.exists();
        cancelled.store(true, Ordering::Release);
        let error = running.await.unwrap().unwrap_err();
        assert!(
            opened,
            "child must actually hold the file before cancellation"
        );
        assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
        std::fs::remove_file(&source)
            .expect("cancel must return only after the child releases its file");
    }

    #[tokio::test]
    async fn cancellation_before_launch_creates_no_child() {
        let mut command = async_command("nonexistent-video-probe-test.exe");
        let error =
            cancellable_output(&mut command, &AtomicBool::new(true), Duration::from_secs(1))
                .await
                .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
    }
}

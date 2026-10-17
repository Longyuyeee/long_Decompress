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
    cancellable_output_with_stdin(command, cancelled, timeout, None).await
}

/// RAR's interactive password prompt reads redirected stdin in the ANSI charset.
/// Close the pipe after one line so an unexpected second prompt cannot hang.
pub async fn cancellable_output_with_stdin(
    command: &mut tokio::process::Command,
    cancelled: &std::sync::atomic::AtomicBool,
    timeout: std::time::Duration,
    input: Option<Vec<u8>>,
) -> std::io::Result<std::process::Output> {
    output_with_stdin_mode(command, cancelled, timeout, input, 0).await
}

/// RAR4 rc opens each surviving volume with a separate password prompt.
/// Reply once per actual prompt, with a caller-supplied finite limit. Writing
/// repeated lines at once does not work: RAR consumes a whole pipe read as one
/// password. No password is added to arguments or the environment.
pub async fn cancellable_output_with_rar_prompts(
    command: &mut tokio::process::Command,
    cancelled: &std::sync::atomic::AtomicBool,
    timeout: std::time::Duration,
    input: Option<Vec<u8>>,
    max_prompts: usize,
) -> std::io::Result<std::process::Output> {
    output_with_stdin_mode(command, cancelled, timeout, input, max_prompts.clamp(1, 512)).await
}

async fn output_with_stdin_mode(
    command: &mut tokio::process::Command,
    cancelled: &std::sync::atomic::AtomicBool,
    timeout: std::time::Duration,
    mut input: Option<Vec<u8>>,
    max_prompts: usize,
) -> std::io::Result<std::process::Output> {
    use std::{io, process::Stdio, sync::atomic::Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    if cancelled.load(Ordering::Acquire) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "VIDEO_COMPRESSION_CANCELLED",
        ));
    }
    if input.is_some() { command.stdin(Stdio::piped()); }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut prompt_stdin = if max_prompts > 0 { child.stdin.take() } else { None };
    if max_prompts == 0 { if let Some(mut bytes) = input.take() {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let result = stdin.write_all(&bytes).await;
        bytes.fill(0);
        // Some operations repair encrypted bytes without asking a password.
        // BrokenPipe then simply means the child has already finished.
        if let Err(error) = result {
            if error.kind() != io::ErrorKind::BrokenPipe { let _ = child.kill().await; return Err(error); }
        }
    } }
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).await.map(|_| bytes)
    });
    let stderr_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let mut prompt_input = input.unwrap_or_default();
        let mut seen = 0;
        let mut scan_from: usize = 0;
        let prompt = b"Enter password (will not be echoed)";
        let result = async {
            let mut buffer = [0; 8192];
            loop {
                let count = stderr.read(&mut buffer).await?;
                if count == 0 { break; }
                bytes.extend_from_slice(&buffer[..count]);
                if max_prompts == 0 { continue; }
                while let Some(offset) = bytes[scan_from..].windows(prompt.len()).position(|w| w == prompt) {
                    scan_from += offset + prompt.len();
                    if seen < max_prompts {
                        if let Some(stdin) = prompt_stdin.as_mut() {
                            if let Err(error) = stdin.write_all(&prompt_input).await {
                                if error.kind() != io::ErrorKind::BrokenPipe { return Err(error); }
                            }
                        }
                        seen += 1;
                    }
                    if seen >= max_prompts { prompt_stdin.take(); }
                }
                scan_from = scan_from.max(bytes.len().saturating_sub(prompt.len() - 1));
            }
            Ok(bytes)
        }.await;
        prompt_input.fill(0);
        result
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

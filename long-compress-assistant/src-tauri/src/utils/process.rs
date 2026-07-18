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

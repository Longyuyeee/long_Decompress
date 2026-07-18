use std::path::{Path, PathBuf};

fn command_exists(command: &str) -> bool {
    crate::utils::process::command(command)
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn candidate_exists(path: &Path) -> bool {
    path.is_file()
}

pub fn find_7z_command() -> Option<String> {
    for command in ["7z", "7za", "7zz"] {
        if command_exists(command) {
            return Some(command.to_string());
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.extend([
                exe_dir.join("7za.exe"),
                exe_dir.join("resources").join("7za.exe"),
                exe_dir.join("resources").join("bin").join("7za.exe"),
                exe_dir.join("..").join("resources").join("7za.exe"),
                exe_dir.join("..").join("resources").join("bin").join("7za.exe"),
                exe_dir.join("_up_").join("node_modules").join("7zip-bin").join("win").join("x64").join("7za.exe"),
            ]);
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest_dir);
        candidates.extend([
            root.join("bin").join("7za.exe"),
            root.join("..").join("node_modules").join("7zip-bin").join("win").join("x64").join("7za.exe"),
        ]);
    }

    #[cfg(target_os = "windows")]
    candidates.extend([
        PathBuf::from(r"C:\Program Files\7-Zip\7z.exe"),
        PathBuf::from(r"C:\Program Files (x86)\7-Zip\7z.exe"),
    ]);

    candidates
        .into_iter()
        .find(|candidate| candidate_exists(candidate))
        .map(|candidate| candidate.to_string_lossy().to_string())
}

pub fn missing_7z_message() -> String {
    "7-Zip command line tool is not available. The installer should bundle 7za.exe; otherwise install 7-Zip and retry.".to_string()
}

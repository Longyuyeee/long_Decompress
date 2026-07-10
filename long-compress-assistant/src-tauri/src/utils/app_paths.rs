use std::path::PathBuf;

const APP_DATA_FOLDER: &str = "LongDecompress";

pub fn app_data_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        let mut path = std::env::current_dir().unwrap_or_default();
        if path.ends_with("src-tauri") {
            path.pop();
        }
        return path.join(".password_book_data");
    }

    dirs::data_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join(APP_DATA_FOLDER)
}

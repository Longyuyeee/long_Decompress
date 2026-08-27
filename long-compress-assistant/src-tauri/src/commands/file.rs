use crate::services::file_service::{FileInfo, FileService};
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use tauri::{command, Manager};

const IMAGE_PREVIEW_LIMIT_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePreviewAuthorization {
    pub format: String,
    pub bytes: u64,
}

fn validate_image_preview_source(path: &Path) -> Result<ImagePreviewAuthorization, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Image preview source is not readable: {error}"))?;
    if !metadata.is_file() {
        return Err("Image preview source is not a file.".to_string());
    }
    if metadata.len() == 0 || metadata.len() > IMAGE_PREVIEW_LIMIT_BYTES {
        return Err(format!(
            "Image preview source must be between 1 byte and {} MiB.",
            IMAGE_PREVIEW_LIMIT_BYTES / 1024 / 1024
        ));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut header = [0_u8; 12];
    let read = File::open(path)
        .and_then(|mut file| file.read(&mut header))
        .map_err(|error| format!("Failed to inspect image preview source: {error}"))?;
    let format = if read >= 8 && header[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        "png"
    } else if read >= 3 && header[..3] == [0xFF, 0xD8, 0xFF] {
        "jpeg"
    } else if read >= 12 && &header[..4] == b"RIFF" && &header[8..12] == b"WEBP" {
        "webp"
    } else {
        return Err("File header is not a supported JPEG, PNG or WebP image.".to_string());
    };
    let extension_matches = match format {
        "jpeg" => matches!(extension.as_str(), "jpg" | "jpeg"),
        expected => extension == expected,
    };
    if !extension_matches {
        return Err(format!(
            "Image file extension .{extension} does not match detected {format} content."
        ));
    }
    Ok(ImagePreviewAuthorization {
        format: format.to_string(),
        bytes: metadata.len(),
    })
}

#[command]
pub async fn authorize_image_preview(
    app: tauri::AppHandle,
    path: String,
) -> Result<ImagePreviewAuthorization, String> {
    let source = Path::new(&path);
    let authorization = validate_image_preview_source(source)?;
    app.asset_protocol_scope()
        .allow_file(source)
        .map_err(|error| format!("Failed to authorize the selected image preview: {error}"))?;
    Ok(authorization)
}

#[command]
pub async fn list_files(path: String) -> Result<Vec<FileInfo>, String> {
    let service = FileService::new(crate::services::file_service::FileServiceConfig::default());
    service
        .list_files(&path, false)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_file_info(path: String) -> Result<FileInfo, String> {
    let service = FileService::new(crate::services::file_service::FileServiceConfig::default());
    service
        .get_file_info(&path)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    let source = Path::new(&path);
    validate_text_file_extension(source)?;

    let metadata =
        std::fs::metadata(source).map_err(|err| format!("File is not readable: {}", err))?;
    if !metadata.is_file() {
        return Err("Selected path is not a file.".to_string());
    }
    if metadata.len() > 10 * 1024 * 1024 {
        return Err("Selected text file is larger than 10 MB.".to_string());
    }

    std::fs::read_to_string(source).map_err(|err| format!("Failed to read text file: {}", err))
}

#[command]
pub async fn write_text_file(path: String, content: String) -> Result<(), String> {
    let target = Path::new(&path);
    validate_text_file_extension(target)?;

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create output directory: {}", err))?;
    }

    std::fs::write(target, content).map_err(|err| format!("Failed to write text file: {}", err))
}

#[derive(Debug, Serialize)]
pub struct WordlistValidationResult {
    pub path: String,
    pub valid: bool,
    pub valid_password_count: usize,
    pub error: Option<String>,
}

#[command]
pub async fn validate_wordlists(
    paths: Vec<String>,
) -> Result<Vec<WordlistValidationResult>, String> {
    let mut results = Vec::with_capacity(paths.len());

    for path in paths {
        results.push(validate_wordlist_path(path));
    }

    Ok(results)
}

fn validate_wordlist_path(path: String) -> WordlistValidationResult {
    let source = Path::new(&path);

    if source
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| !ext.eq_ignore_ascii_case("txt"))
        .unwrap_or(true)
    {
        return invalid_wordlist(path, "Only .txt wordlists are supported.");
    }

    let metadata = match std::fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(err) => return invalid_wordlist(path, &format!("File is not readable: {}", err)),
    };

    if !metadata.is_file() {
        return invalid_wordlist(path, "Selected path is not a file.");
    }

    if metadata.len() == 0 {
        return invalid_wordlist(path, "Wordlist is empty.");
    }

    let file = match File::open(source) {
        Ok(file) => file,
        Err(err) => return invalid_wordlist(path, &format!("File is not readable: {}", err)),
    };

    let mut valid_password_count = 0usize;
    for line in BufReader::new(file).lines() {
        match line {
            Ok(value) => {
                if !value.trim().trim_end_matches('\u{feff}').is_empty() {
                    valid_password_count += 1;
                }
            }
            Err(err) => {
                return invalid_wordlist(path, &format!("Failed to read wordlist: {}", err))
            }
        }
    }

    if valid_password_count == 0 {
        return invalid_wordlist(path, "Wordlist has no usable password lines.");
    }

    WordlistValidationResult {
        path,
        valid: true,
        valid_password_count,
        error: None,
    }
}

fn invalid_wordlist(path: String, error: &str) -> WordlistValidationResult {
    WordlistValidationResult {
        path,
        valid: false,
        valid_password_count: 0,
        error: Some(error.to_string()),
    }
}

fn validate_text_file_extension(path: &Path) -> Result<(), String> {
    let allowed = ["json", "txt", "md"];
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .ok_or_else(|| "Only .json, .txt and .md files are supported.".to_string())?;

    if allowed.iter().any(|allowed_ext| *allowed_ext == extension) {
        Ok(())
    } else {
        Err("Only .json, .txt and .md files are supported.".to_string())
    }
}

#[cfg(test)]
mod image_preview_tests {
    use super::validate_image_preview_source;
    use std::fs;

    #[test]
    fn preview_authorization_requires_supported_magic_and_matching_extension() {
        let root = std::env::temp_dir().join(format!(
            "long-image-preview-auth-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();

        let png = root.join("real.png");
        fs::write(
            &png,
            [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0],
        )
        .unwrap();
        let result = validate_image_preview_source(&png).unwrap();
        assert_eq!(result.format, "png");
        assert_eq!(result.bytes, 12);

        let disguised = root.join("disguised.jpg");
        fs::write(
            &disguised,
            [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0],
        )
        .unwrap();
        assert!(validate_image_preview_source(&disguised)
            .unwrap_err()
            .contains("does not match"));

        let gif = root.join("animated.gif");
        fs::write(&gif, b"GIF89a-not-authorized").unwrap();
        assert!(validate_image_preview_source(&gif)
            .unwrap_err()
            .contains("not a supported"));

        fs::remove_dir_all(root).unwrap();
    }
}

use crate::services::password_generator_service::{CharsetOptions, PasswordGeneratorService};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordCharsetOptions {
    pub lowercase: bool,
    pub uppercase: bool,
    pub numbers: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}

fn password_length(strength: &str) -> Result<usize, String> {
    match strength {
        "weak" => Ok(10),
        "medium" => Ok(14),
        "strong" => Ok(18),
        "very_strong" => Ok(28),
        _ => Err(format!("Unsupported password strength: {strength}")),
    }
}

#[tauri::command]
pub fn generate_password(
    strength: String,
    options: PasswordCharsetOptions,
) -> Result<String, String> {
    if !options.lowercase && !options.uppercase && !options.numbers && !options.symbols {
        return Err("Select at least one character set.".to_string());
    }
    Ok(PasswordGeneratorService::generate_custom(
        password_length(&strength)?,
        CharsetOptions {
            lowercase: options.lowercase,
            uppercase: options.uppercase,
            numbers: options.numbers,
            symbols: options.symbols,
            exclude_ambiguous: options.exclude_ambiguous,
        },
    ))
}

#[tauri::command]
pub fn generate_memorable_password(word_count: usize) -> Result<String, String> {
    if !(2..=8).contains(&word_count) {
        return Err("Word count must be between 2 and 8.".to_string());
    }
    Ok(PasswordGeneratorService::generate_memorable(
        word_count, "-",
    ))
}

#[tauri::command]
pub fn generate_pin(length: usize) -> Result<String, String> {
    if !(4..=12).contains(&length) {
        return Err("PIN length must be between 4 and 12.".to_string());
    }
    Ok(PasswordGeneratorService::generate_pin(length))
}

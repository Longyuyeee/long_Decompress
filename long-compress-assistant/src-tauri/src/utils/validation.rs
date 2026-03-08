use crate::utils::error::AppError;

/// 验证文件名是否有效
pub fn validate_filename(filename: &str) -> Result<(), AppError> {
    if filename.is_empty() {
        return Err(AppError::validation("文件名不能为空"));
    }

    // 检查非法字符
    let illegal_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    if filename.chars().any(|c| illegal_chars.contains(&c)) {
        return Err(AppError::validation(format!(
            "文件名包含非法字符: {}",
            filename
        )));
    }

    // 检查保留名称
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let upper_name = filename.to_uppercase();
    if reserved_names.contains(&upper_name.as_str()) {
        return Err(AppError::validation(format!(
            "文件名是保留名称: {}",
            filename
        )));
    }

    // 检查长度
    if filename.len() > 255 {
        return Err(AppError::validation("文件名过长"));
    }

    Ok(())
}

/// 验证文件路径是否有效
pub fn validate_file_path(path: &str) -> Result<(), AppError> {
    if path.is_empty() {
        return Err(AppError::validation("文件路径不能为空"));
    }

    // 检查路径长度
    if path.len() > 4096 {
        return Err(AppError::validation("文件路径过长"));
    }

    // 检查空字符
    if path.contains('\0') {
        return Err(AppError::validation("文件路径包含空字符"));
    }

    Ok(())
}

/// 验证目录路径是否有效
pub fn validate_directory_path(path: &str) -> Result<(), AppError> {
    validate_file_path(path)?;

    // 额外的目录验证
    if path.ends_with('.') {
        return Err(AppError::validation("目录路径不能以点结尾"));
    }

    Ok(())
}

/// 验证密码强度
pub fn validate_password(password: &str) -> Result<(), AppError> {
    if password.is_empty() {
        return Err(AppError::validation("密码不能为空"));
    }

    if password.len() < 8 {
        return Err(AppError::validation("密码长度至少8个字符"));
    }

    // 检查字符类型
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_digit = password.chars().any(|c| c.is_digit(10));
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    let mut missing = Vec::new();
    if !has_lowercase {
        missing.push("小写字母");
    }
    if !has_uppercase {
        missing.push("大写字母");
    }
    if !has_digit {
        missing.push("数字");
    }
    if !has_special {
        missing.push("特殊字符");
    }

    if missing.len() > 2 {
        return Err(AppError::validation(format!(
            "密码需要包含更多字符类型，缺少: {}",
            missing.join(", ")
        )));
    }

    // 检查常见弱密码
    let weak_passwords = [
        "password", "123456", "qwerty", "admin", "welcome",
        "password123", "12345678", "123456789", "1234567890",
    ];

    let password_lower = password.to_lowercase();
    if weak_passwords.iter().any(|&p| password_lower.contains(p)) {
        return Err(AppError::validation("密码太弱，请使用更强的密码"));
    }

    Ok(())
}

/// 验证电子邮件地址
pub fn validate_email(email: &str) -> Result<(), AppError> {
    if email.is_empty() {
        return Err(AppError::validation("电子邮件地址不能为空"));
    }

    // 简单的电子邮件验证
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(AppError::validation("无效的电子邮件地址格式"));
    }

    let local_part = parts[0];
    let domain_part = parts[1];

    if local_part.is_empty() {
        return Err(AppError::validation("电子邮件本地部分不能为空"));
    }

    if domain_part.is_empty() {
        return Err(AppError::validation("电子邮件域名部分不能为空"));
    }

    // 检查域名是否包含点
    if !domain_part.contains('.') {
        return Err(AppError::validation("电子邮件域名无效"));
    }

    // 检查长度
    if email.len() > 254 {
        return Err(AppError::validation("电子邮件地址过长"));
    }

    Ok(())
}

/// 验证URL
pub fn validate_url(url: &str) -> Result<(), AppError> {
    if url.is_empty() {
        return Err(AppError::validation("URL不能为空"));
    }

    // 简单的URL验证
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::validation("URL必须以http://或https://开头"));
    }

    // 检查长度
    if url.len() > 2048 {
        return Err(AppError::validation("URL过长"));
    }

    Ok(())
}

/// 验证数字范围
pub fn validate_number_range(
    value: i64,
    min: i64,
    max: i64,
    field_name: &str,
) -> Result<(), AppError> {
    if value < min {
        return Err(AppError::validation(format!(
            "{}不能小于{}",
            field_name, min
        )));
    }

    if value > max {
        return Err(AppError::validation(format!(
            "{}不能大于{}",
            field_name, max
        )));
    }

    Ok(())
}

/// 验证字符串长度
pub fn validate_string_length(
    value: &str,
    min: usize,
    max: usize,
    field_name: &str,
) -> Result<(), AppError> {
    if value.len() < min {
        return Err(AppError::validation(format!(
            "{}长度不能小于{}个字符",
            field_name, min
        )));
    }

    if value.len() > max {
        return Err(AppError::validation(format!(
            "{}长度不能大于{}个字符",
            field_name, max
        )));
    }

    Ok(())
}

/// 验证文件大小
pub fn validate_file_size(size: u64, max_size: u64) -> Result<(), AppError> {
    if size > max_size {
        return Err(AppError::validation(format!(
            "文件大小不能超过{}",
            format_file_size(max_size)
        )));
    }

    Ok(())
}

/// 格式化文件大小
fn format_file_size(size: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

    if size == 0 {
        return "0 B".to_string();
    }

    let base = 1024_f64;
    let size_f64 = size as f64;
    let exponent = (size_f64.log10() / base.log10()).floor() as i32;
    let unit_index = exponent.min(5).max(0) as usize;

    let formatted = size_f64 / base.powi(exponent);

    format!("{:.2} {}", formatted, UNITS[unit_index])
}

/// 验证压缩级别
pub fn validate_compression_level(level: u32) -> Result<(), AppError> {
    validate_number_range(level as i64, 0, 9, "压缩级别")
}

/// 验证文件扩展名
pub fn validate_file_extension(filename: &str, allowed_extensions: &[&str]) -> Result<(), AppError> {
    if let Some(dot_idx) = filename.rfind('.') {
        let extension = &filename[dot_idx + 1..].to_lowercase();

        if allowed_extensions.iter().any(|&ext| ext == extension) {
            Ok(())
        } else {
            Err(AppError::validation(format!(
                "不支持的文件扩展名: {}，支持的扩展名: {}",
                extension,
                allowed_extensions.join(", ")
            )))
        }
    } else {
        Err(AppError::validation("文件没有扩展名"))
    }
}

/// 批量验证
pub struct Validator {
    errors: Vec<String>,
}

impl Validator {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn validate<F>(&mut self, validation: F, field_name: &str)
    where
        F: FnOnce() -> Result<(), AppError>,
    {
        match validation() {
            Ok(_) => {}
            Err(err) => {
                self.errors.push(format!("{}: {}", field_name, err));
            }
        }
    }

    pub fn check(&self) -> Result<(), AppError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::validation(self.errors.join("; ")))
        }
    }
}
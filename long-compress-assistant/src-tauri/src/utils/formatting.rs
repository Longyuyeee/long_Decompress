use chrono::{DateTime, Local, Utc};
use std::time::{Duration, SystemTime};

/// 格式化文件大小
pub fn format_file_size(size: u64) -> String {
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

/// 格式化持续时间
pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();

    if seconds < 60 {
        format!("{}秒", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let remaining_seconds = seconds % 60;
        format!("{}分{}秒", minutes, remaining_seconds)
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{}小时{}分", hours, minutes)
    } else {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        format!("{}天{}小时", days, hours)
    }
}

/// 格式化日期时间（本地时间）
pub fn format_datetime_local(dt: DateTime<Utc>) -> String {
    let local_dt = dt.with_timezone(&Local);
    local_dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 格式化日期时间（UTC时间）
pub fn format_datetime_utc(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// 格式化相对时间
pub fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < 60 {
        "刚刚".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}分钟前", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}小时前", duration.num_hours())
    } else if duration.num_days() < 30 {
        format!("{}天前", duration.num_days())
    } else if duration.num_days() < 365 {
        format!("{}个月前", duration.num_days() / 30)
    } else {
        format!("{}年前", duration.num_days() / 365)
    }
}

/// 格式化百分比
pub fn format_percentage(value: f64, total: f64) -> String {
    if total == 0.0 {
        return "0.00%".to_string();
    }

    let percentage = (value / total) * 100.0;
    format!("{:.2}%", percentage)
}

/// 格式化进度条
pub fn format_progress_bar(progress: f64, width: usize) -> String {
    let filled = (progress * width as f64 / 100.0).round() as usize;
    let empty = width.saturating_sub(filled);

    let bar = "█".repeat(filled) + &"░".repeat(empty);
    format!("[{:3.0}%] {}", progress, bar)
}

/// 格式化数字（添加千位分隔符）
pub fn format_number_with_commas(number: i64) -> String {
    let mut result = String::new();
    let number_str = number.abs().to_string();
    let len = number_str.len();

    for (i, c) in number_str.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    if number < 0 {
        format!("-{}", result)
    } else {
        result
    }
}

/// 格式化字节速率
pub fn format_byte_rate(bytes_per_second: u64) -> String {
    if bytes_per_second == 0 {
        return "0 B/s".to_string();
    }

    const UNITS: [&str; 6] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s", "PB/s"];

    let base = 1024_f64;
    let rate_f64 = bytes_per_second as f64;
    let exponent = (rate_f64.log10() / base.log10()).floor() as i32;
    let unit_index = exponent.min(5).max(0) as usize;

    let formatted = rate_f64 / base.powi(exponent);

    format!("{:.2} {}", formatted, UNITS[unit_index])
}

/// 格式化系统时间
pub fn format_system_time(time: SystemTime) -> String {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => {
            let dt = DateTime::from_timestamp(duration.as_secs() as i64, 0)
                .unwrap_or_default();
            format_datetime_local(dt)
        }
        Err(_) => "无效时间".to_string(),
    }
}

/// 格式化压缩比率
pub fn format_compression_ratio(original_size: u64, compressed_size: u64) -> String {
    if original_size == 0 {
        return "N/A".to_string();
    }

    let ratio = compressed_size as f64 / original_size as f64;
    let percentage = (1.0 - ratio) * 100.0;

    if percentage > 0.0 {
        format!("{:.1}% 节省", percentage)
    } else if percentage < 0.0 {
        format!("{:.1}% 增加", -percentage)
    } else {
        "无变化".to_string()
    }
}

/// 格式化文件权限（Unix风格）
pub fn format_unix_permissions(mode: u32) -> String {
    let mut result = String::with_capacity(9);

    // 所有者权限
    result.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o100 != 0 { 'x' } else { '-' });

    // 组权限
    result.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o010 != 0 { 'x' } else { '-' });

    // 其他用户权限
    result.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o001 != 0 { 'x' } else { '-' });

    result
}

/// 截断字符串并添加省略号
pub fn truncate_string(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else if max_length <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &text[..max_length - 3])
    }
}

/// 格式化十六进制数据
pub fn format_hex_data(data: &[u8], bytes_per_line: usize) -> String {
    let mut result = String::new();

    for (i, chunk) in data.chunks(bytes_per_line).enumerate() {
        if i > 0 {
            result.push('\n');
        }

        // 十六进制部分
        for (j, byte) in chunk.iter().enumerate() {
            if j > 0 {
                result.push(' ');
            }
            result.push_str(&format!("{:02X}", byte));
        }

        // 填充对齐
        if chunk.len() < bytes_per_line {
            let padding = (bytes_per_line - chunk.len()) * 3;
            result.push_str(&" ".repeat(padding));
        }

        result.push_str("  ");

        // ASCII部分
        for byte in chunk.iter() {
            let c = *byte as char;
            if c.is_ascii_graphic() || c == ' ' {
                result.push(c);
            } else {
                result.push('.');
            }
        }
    }

    result
}

/// 格式化颜色代码（RGB到十六进制）
pub fn format_color_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// 格式化版本号
pub fn format_version(major: u32, minor: u32, patch: u32) -> String {
    format!("{}.{}.{}", major, minor, patch)
}

/// 格式化版本号（带构建号）
pub fn format_version_with_build(major: u32, minor: u32, patch: u32, build: u32) -> String {
    format!("{}.{}.{}.{}", major, minor, patch, build)
}
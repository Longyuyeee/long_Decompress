//! LZOP names have a charset family, not necessarily a concrete code page.
use super::super::zip_password::ZipPasswordEncoding;
use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};

pub(super) fn decode(
    raw: &[u8],
    flags: u32,
    encoding: Option<ZipPasswordEncoding>,
) -> Result<Option<String>> {
    if flags & 0x00f00000 == 0x00100000 {
        return Ok(Some(raw.iter().map(|&b| char::from(b)).collect()));
    }
    if flags & 0x00f00000 == 0x00500000 || encoding.is_none() {
        return Ok(String::from_utf8(raw.to_vec()).ok());
    }
    let encoding = encoding.unwrap();
    if encoding == ZipPasswordEncoding::Utf8 {
        return String::from_utf8(raw.to_vec()).map(Some).map_err(|_| {
            anyhow::anyhow!(
                "[lzop-encoding] 成员名称不是有效 UTF-8，请选择来源工具的名称编码后重新读取目录"
            )
        });
    }
    decode_legacy(raw, encoding).map(Some)
}

#[cfg(windows)]
fn decode_legacy(raw: &[u8], encoding: ZipPasswordEncoding) -> Result<String> {
    use windows_sys::Win32::Globalization::{GetACP, MultiByteToWideChar, MB_ERR_INVALID_CHARS};
    if raw.is_empty() {
        return Ok(String::new());
    }
    let page = match encoding {
        ZipPasswordEncoding::WindowsLocal => unsafe { GetACP() },
        ZipPasswordEncoding::Cp932 => 932,
        ZipPasswordEncoding::Cp936 => 936,
        ZipPasswordEncoding::Cp950 => 950,
        ZipPasswordEncoding::Cp437 => 437,
        ZipPasswordEncoding::Cp850 => 850,
        ZipPasswordEncoding::Utf8 => 65001,
    };
    let len = i32::try_from(raw.len())?;
    // SAFETY: the input slice is valid for len bytes; null/zero queries capacity.
    let count = unsafe {
        MultiByteToWideChar(
            page,
            MB_ERR_INVALID_CHARS,
            raw.as_ptr(),
            len,
            std::ptr::null_mut(),
            0,
        )
    };
    if count <= 0 {
        bail!("[lzop-encoding] 成员名称不能按 CP{page} 读取，请更换名称编码后重试");
    }
    let mut wide = vec![0u16; count as usize];
    // SAFETY: the output has the queried capacity and input is unchanged.
    let written = unsafe {
        MultiByteToWideChar(
            page,
            MB_ERR_INVALID_CHARS,
            raw.as_ptr(),
            len,
            wide.as_mut_ptr(),
            count,
        )
    };
    if written <= 0 {
        bail!("[lzop-encoding] 成员名称不能按 CP{page} 读取，请更换名称编码后重试");
    }
    wide.truncate(written as usize);
    String::from_utf16(&wide)
        .map_err(|_| anyhow::anyhow!("[lzop-encoding] 成员名称无法无损读取，请更换名称编码后重试"))
}

#[cfg(not(windows))]
fn decode_legacy(raw: &[u8], encoding: ZipPasswordEncoding) -> Result<String> {
    let codec = match encoding {
        ZipPasswordEncoding::Cp932 => encoding_rs::SHIFT_JIS,
        ZipPasswordEncoding::Cp936 => encoding_rs::GBK,
        ZipPasswordEncoding::Cp950 => encoding_rs::BIG5,
        _ => bail!("[lzop-encoding] 当前平台不支持所选 Windows/DOS 名称代码页"),
    };
    let (text, errors) = codec.decode_without_bom_handling(raw);
    if errors {
        bail!("[lzop-encoding] 成员名称无法按所选编码读取，请更换名称编码后重试");
    }
    Ok(text.into_owned())
}

#[derive(Default)]
pub(super) struct Names {
    occupied: HashSet<String>,
    directories: HashMap<String, String>,
}
fn component(raw: &str, index: usize) -> String {
    let mut name: String = raw
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    name = name.trim_end_matches(['.', ' ']).to_owned();
    if name.is_empty() {
        return format!("member-{index:03}.bin");
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|p| {
            stem.strip_prefix(p)
                .is_some_and(|n| matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        })
    {
        name = format!("member-{index}-{name}");
    }
    name
}
fn fit(name: &str, suffix: &str) -> String {
    let (stem, mut ext) = name
        .rsplit_once('.')
        .filter(|(s, _)| !s.is_empty())
        .map(|(s, e)| (s, format!(".{e}")))
        .unwrap_or((name, String::new()));
    while suffix.encode_utf16().count() + ext.encode_utf16().count() >= 255 {
        ext.pop();
    }
    let mut base = stem.to_owned();
    while base.encode_utf16().count() + suffix.encode_utf16().count() + ext.encode_utf16().count()
        > 255
        && !base.is_empty()
    {
        base.pop();
    }
    format!("{base}{suffix}{ext}")
}
impl Names {
    fn unique(&mut self, parent: &str, name: &str, index: usize) -> String {
        let mut suffix = String::new();
        let mut n = index;
        loop {
            let leaf = fit(name, &suffix);
            let path = if parent.is_empty() {
                leaf
            } else {
                format!("{parent}/{leaf}")
            };
            if self.occupied.insert(path.to_lowercase()) {
                return path;
            }
            suffix = format!(" (member {n})");
            n += 1;
        }
    }
    pub(super) fn allocate(&mut self, decoded: Option<&str>, flags: u32, index: usize) -> String {
        self.allocate_path(decoded, flags, index, false)
    }
    pub(super) fn allocate_directory(&mut self, decoded: &str, index: usize) -> String {
        self.allocate_path(Some(decoded), 0x2000, index, true)
    }
    fn allocate_path(
        &mut self,
        decoded: Option<&str>,
        flags: u32,
        index: usize,
        directory: bool,
    ) -> String {
        let decoded = decoded.unwrap_or("");
        let valid_path = flags & 0x2000 != 0
            && !decoded.starts_with(['/', '\\'])
            && !decoded.contains(':')
            && !decoded.split(['/', '\\']).any(|p| p == "..");
        let parts: Vec<&str> = if valid_path {
            decoded
                .split(['/', '\\'])
                .filter(|p| !p.is_empty() && *p != ".")
                .collect()
        } else {
            vec![decoded.rsplit(['/', '\\']).next().unwrap_or("")]
        };
        let mut parent = String::new();
        let mut original = String::new();
        for part in parts.iter().take(if directory {
            parts.len()
        } else {
            parts.len().saturating_sub(1)
        }) {
            if !original.is_empty() {
                original.push('/');
            }
            original.push_str(part);
            let key = original.to_lowercase();
            parent = if let Some(path) = self.directories.get(&key) {
                path.clone()
            } else {
                let path = self.unique(&parent, &component(part, index), index);
                self.directories.insert(key, path.clone());
                path
            };
        }
        if directory {
            parent
        } else {
            self.unique(
                &parent,
                &component(parts.last().copied().unwrap_or(""), index),
                index,
            )
        }
    }
}

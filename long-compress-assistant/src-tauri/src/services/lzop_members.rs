//! Index member framing without decoding unrelated payloads. Full verification is separate.
use super::{damaged, numeric_error, source_input, LzopReader};
use crate::models::compression::DecompressOptions;
use crate::services::{
    extraction_transaction,
    native_extraction::{self, ExtractionRuntime},
    numeric_volume_reader::ZipInputReader,
};
use anyhow::{Context, Result};
use super::names::{decode, Names};
use crate::services::zip_password::ZipPasswordEncoding;
use std::{
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Debug, Clone)]
pub(crate) struct Member {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    offset: u64,
    flags: u32,
}
pub(crate) fn member_index(
    path: &Path,
    check: impl FnMut() -> Result<()>,
) -> Result<Vec<Member>> {
    member_index_with_encoding(path, None, check)
}
pub(crate) fn member_index_with_encoding(
    path: &Path, encoding: Option<ZipPasswordEncoding>,
    mut check: impl FnMut() -> Result<()>,
) -> Result<Vec<Member>> {
    check()?;
    let (mut input, numeric) = source_input(path)?;
    let length = input.seek(SeekFrom::End(0))?;
    input.seek(SeekFrom::Start(0))?;
    let mut result = Vec::new();
    let mut names = Names::default();
    while input.stream_position()? < length {
        check()?;
        anyhow::ensure!(
            result.len() < 100_000,
            "LZOP 成员超过 100000 个，请分批处理"
        );
        let offset = input.stream_position()?;
        let mut member =
            LzopReader::with_mode(input, true).map_err(|e| numeric_error(e, numeric))?;
        let decoded = decode(&member.name, member.flags, encoding)?;
        let name = names.allocate(decoded.as_deref(), member.flags, result.len() + 1);
        let mut size = 0u64;
        loop {
            check()?;
            let Some(block) = member
                .block_header()
                .map_err(|e| numeric_error(e, numeric))?
            else {
                break;
            };
            size = size
                .checked_add(block.unpacked as u64)
                .context("LZOP 成员大小超出范围")?;
            let end = member
                .input
                .stream_position()?
                .checked_add(block.packed as u64)
                .context("LZOP 块位置超出范围")?;
            if end > length {
                return Err(numeric_error(damaged("数据不完整"), numeric).into());
            }
            member.input.seek(SeekFrom::Start(end))?;
        }
        let end = member.input.stream_position()?;
        result.push(Member {
            name,
            size,
            compressed_size: end - offset,
            offset,
            flags: member.flags,
        });
        input = member.input;
    }
    anyhow::ensure!(
        !result.is_empty(),
        "[archive-inspection:damaged] LZOP 文件为空"
    );
    if result.len() == 1 && result[0].flags & 0x2400 == 0 {
        result[0].name = native_extraction::single_stream::output_name(path, &[".lzo"]);
    }
    Ok(result)
}
pub(crate) fn multi_index(
    path: &Path,
    check: impl FnMut() -> Result<()>,
) -> Result<Option<Vec<Member>>> {
    multi_index_with_encoding(path, None, check)
}
pub(crate) fn multi_index_with_encoding(path: &Path, encoding: Option<ZipPasswordEncoding>, check: impl FnMut() -> Result<()>) -> Result<Option<Vec<Member>>> {
    let index = member_index_with_encoding(path, encoding, check)?;
    Ok((index.len() > 1 || index[0].flags & 0x2400 != 0).then_some(index))
}
struct CheckedMember {
    inner: LzopReader<ZipInputReader>,
    name: String,
}
impl Read for CheckedMember {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.inner.read(bytes).map_err(|e| {
            if crate::services::archive_io::io_kind(&e).is_some() {
                e
            } else {
                io::Error::new(e.kind(), format!("LZOP 成员“{}”：{e}", self.name))
            }
        })
    }
}
pub(crate) fn member_reader(path: &Path, member: &Member) -> Result<Box<dyn Read + Send>> {
    let (mut input, numeric) = source_input(path)?;
    input.seek(SeekFrom::Start(member.offset))?;
    let mut reader = LzopReader::with_mode(input, true).map_err(|e| numeric_error(e, numeric))?;
    reader.numeric = numeric;
    Ok(Box::new(CheckedMember {
        inner: reader,
        name: member.name.clone(),
    }))
}
pub(crate) fn extract_members<R: ExtractionRuntime>(
    runtime: &R,
    window: Option<&tauri::Window>,
    task_id: &str,
    path: &Path,
    output: &Path,
    options: &DecompressOptions,
) -> Result<bool> {
    let Some(members) = multi_index_with_encoding(path, options.zip_password_encoding, || runtime.check_cancellation())? else {
        return Ok(false);
    };
    let filter = extraction_transaction::compile_file_filter(options.file_filter.as_deref());
    let mut total = 0u64;
    let mut flat_names = Names::default();
    let mut selected_options = options.clone();
    selected_options.selected_entries.clear();
    selected_options.file_filter = None;
    for (index, member) in members.into_iter().enumerate() {
        let destination = if options.preserve_paths { member.name.clone() } else { flat_names.allocate(Some(&member.name), 0, index + 1) };
        runtime.check_cancellation()?;
        let relative = Path::new(&member.name);
        if !extraction_transaction::matches_selected_entries(relative, &options.selected_entries)
            || !extraction_transaction::matches_compiled_file_filter(relative, &filter)
        {
            continue;
        }
        total = total
            .checked_add(member.size)
            .context("LZOP 展开大小超出范围")?;
        anyhow::ensure!(
            total <= extraction_transaction::MAX_EXTRACTED_BYTES,
            "LZOP 展开大小超出支持范围"
        );
        native_extraction::single_stream::extract_inner(
            runtime,
            window,
            task_id,
            member_reader(path, &member)?,
            output,
            destination,
            &selected_options,
        )?;
    }
    Ok(true)
}

//! Read only the checksum-confirmed LZOP prefix. Never search for a later TAR header.
use super::{
    recovery::{check, Entry, Report, Source},
    LzopReader, ADLER_D, CRC_D,
};
use crate::services::{
    extraction_transaction::MAX_EXTRACTED_BYTES, numeric_volume_reader::ZipInputReader,
    zip_password::ZipPasswordEncoding,
};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, Write},
    path::{Path,PathBuf},
    sync::atomic::AtomicBool,
};

struct Verified<'a, F> {
    reader: LzopReader<ZipInputReader>,
    cancelled: &'a AtomicBool,
    progress: F,
    total: u64,
    position: u64,
}
impl<F: FnMut(u64, u64)> Read for Verified<'_, F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        check(self.cancelled).map_err(io::Error::other)?;
        if self.reader.flags & (ADLER_D | CRC_D) == 0 {
            return Err(io::Error::other(
                "外层 LZOP 未提供内容校验，无法确认内部文件",
            ));
        }
        // LzopReader publishes a block only after decoding and checking its entire data.
        let n = self.reader.read(buf)?;
        self.position = self
            .position
            .checked_add(n as u64)
            .ok_or_else(|| io::Error::other("TAR 大小超出范围"))?;
        if self.position > MAX_EXTRACTED_BYTES {
            return Err(io::Error::other("TAR 展开大小超过本次支持范围"));
        }
        (self.progress)(self.reader.input.stream_position()?, self.total);
        Ok(n)
    }
}
fn padding(reader: &mut impl Read, size: u64) -> Result<()> {
    let count = ((512 - size % 512) % 512) as usize;
    let mut bytes = [0; 512];
    reader
        .read_exact(&mut bytes[..count])
        .context("TAR 条目填充未完整到达")?;
    anyhow::ensure!(bytes[..count].iter().all(|b| *b == 0), "TAR 条目填充无效");
    Ok(())
}
fn local_metadata(fields:&crate::services::tar_pax::Fields)->Result<Option<crate::services::tar_sparse_recovery::PaxVersion>>{
    let ordinary=fields.iter().filter(|(k,_)|k!="linkpath").cloned().collect();
    let sparse=crate::services::tar_sparse_recovery::validate_local(&ordinary)?;
    if let Some((_,target))=fields.iter().find(|(k,_)|k=="linkpath"){
        anyhow::ensure!(!target.is_empty()&&!target.contains(['\0','\n'])&&sparse.is_none(),"[archive-inspection:unsupported] TAR 链接目标为空、无效或与稀疏元数据冲突");
    }
    Ok(sparse)
}
fn archive_key(path:&str)->Result<String>{
    anyhow::ensure!(!path.starts_with(['/','\\'])&&!path.contains([':', '\0'])&&!path.split(['/','\\']).any(|p|p==".."),"TAR 路径无法确认，停止在此处");
    Ok(path.split('/').filter(|p|!p.is_empty()&&*p!=".").collect::<Vec<_>>().join("/"))
}
fn parse<F: FnMut(u64, u64)>(
    reader: &mut Verified<'_, F>,
    report: &mut Report,
    encoding: Option<ZipPasswordEncoding>,
    complete_links: bool,
) -> Result<()> {
    let mut names = super::names::Names::default();
    let mut pending: Option<crate::services::tar_pax::Fields> = None;
    let mut global = crate::services::tar_pax::State::default();
    let mut long_name: Option<String> = None;
    let mut long_link: Option<String> = None;
    let mut records = 0usize;
    let mut logical_total = 0u64;
    let mut sparse_segments = 0usize;
    // Original archive names stay case sensitive and separate from Windows repair.
    let mut sources=std::collections::HashMap::<String,Option<usize>>::new();
    loop {
        report.stopped_offset = reader.position;
        report.stopped_member = None;
        let mut block = [0u8; 512];
        reader
            .read_exact(&mut block)
            .context("TAR 文件头或结束标记未完整到达")?;
        if block.iter().all(|b| *b == 0) {
            anyhow::ensure!(pending.is_none() && long_name.is_none() && long_link.is_none(), "TAR 在名称或链接目标元数据之后缺少对应条目");
            reader
                .read_exact(&mut block)
                .context("TAR 第二个结束标记未完整到达")?;
            anyhow::ensure!(block.iter().all(|b| *b == 0), "TAR 结束标记不完整");
            let mut buf = [0; 65536];
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                anyhow::ensure!(
                    buf[..n].iter().all(|b| *b == 0),
                    "TAR 结束后还有非零内容，后续清单未确认"
                );
            }
            anyhow::ensure!(
                reader.reader.input.stream_position()? == reader.total,
                "外层 LZOP 成员结束后仍有数据，后续内容未确认"
            );
            if complete_links { resolve_complete_links(report, || check(reader.cancelled))?; }
            report.complete = true;
            report.stopped_offset = reader.position;
            return Ok(());
        }
        records += 1;
        anyhow::ensure!(
            records <= 200_000 && report.entries.len() < 100_000,
            "TAR 条目超过本次支持范围"
        );
        let header = tar::Header::from_byte_slice(&block);
        let sum = block
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if (148..156).contains(&i) {
                    32
                } else {
                    *b as u32
                }
            })
            .sum::<u32>();
        anyhow::ensure!(header.cksum()? == sum, "TAR 文件头校验失败");
        let kind = header.entry_type();
        report.hard_links_seen |= kind.is_hard_link();
        anyhow::ensure!(kind.is_pax_global_extensions() || kind.is_pax_local_extensions() || kind.is_gnu_longname() || kind.is_gnu_longlink() || kind.is_file() || kind.is_dir() || kind.is_gnu_sparse() || kind.is_hard_link(), "[archive-inspection:unsupported] TAR.LZO 部分取回暂不支持此 TAR 条目类型（{}），已停止在此处", kind.as_byte());
        let size = header.entry_size()?;
        anyhow::ensure!(size <= MAX_EXTRACTED_BYTES, "TAR 条目大小超过本次支持范围");
        if kind.is_pax_global_extensions() || kind.is_pax_local_extensions() || kind.is_gnu_longname() || kind.is_gnu_longlink() {
            anyhow::ensure!(
                kind.is_pax_global_extensions()
                    || (kind.is_gnu_longname() && long_name.is_none())
                    || (kind.is_gnu_longlink() && long_link.is_none())
                    || (kind.is_pax_local_extensions() && pending.is_none()),
                "[archive-inspection:unsupported] TAR 名称元数据重复，已停止在此处"
            );
            anyhow::ensure!(size <= 1024 * 1024, "TAR 名称元数据超过本次支持范围");
            let mut bytes = vec![0; size as usize];
            reader
                .read_exact(&mut bytes)
                .context("TAR 名称元数据未完整到达")?;
            padding(reader, size)?;
            // Extension records use their own physical size. Apply their
            // contents only after the checked payload AND padding arrive.
            if kind.is_gnu_longname() || kind.is_gnu_longlink() {
                let label=if kind.is_gnu_longlink(){"GNU 长链接目标"}else{"GNU 长名称"};
                let end = bytes.iter().position(|byte| *byte == 0)
                    .with_context(||format!("{label}缺少结束标记"))?;
                anyhow::ensure!(end > 0 && bytes[end..].iter().all(|byte| *byte == 0), "{label}为空或结束后仍有其他内容");
                let decoded=super::names::decode(&bytes[..end],0,encoding)?
                    .with_context(||format!("[lzop-encoding] {label}不是 UTF-8，请选择名称编码后重新检查"))?;
                if kind.is_gnu_longlink(){long_link=Some(decoded);}else{long_name=Some(decoded);}
            } else {
                let fields = crate::services::tar_pax::parse(&bytes)?;
                if kind.is_pax_global_extensions() { global.update(fields)?; }
                else { local_metadata(&fields)?; pending = Some(fields); }
            }
            continue;
        }
        anyhow::ensure!(kind.is_file() || kind.is_dir() || kind.is_gnu_sparse() || kind.is_hard_link(),"[archive-inspection:unsupported] TAR.LZO 部分取回暂不支持此 TAR 条目类型（{}），已停止在此处",kind.as_byte());
        // GNU sparse records have a different layout even when their path looks ordinary.
        if let Some(gnu) = header.as_gnu().filter(|_| !kind.is_gnu_sparse()) {
            anyhow::ensure!(
                gnu.isextended[0] == 0 && gnu.sparse.iter().all(|s| s.is_empty()),
                "[archive-inspection:unsupported] TAR 稀疏条目暂不支持部分取回"
            );
        }
        let local = pending.take().unwrap_or_default();
        anyhow::ensure!(kind.is_hard_link()||long_link.is_none(),"[archive-inspection:unsupported] GNU 长链接目标缺少对应硬链接条目");
        if let (Some(gnu),Some((_,pax)))=(&long_link,local.iter().find(|(k,_)|k=="linkpath")){
            anyhow::ensure!(gnu==pax,"[archive-inspection:unsupported] GNU 长链接目标与局部 PAX linkpath 不一致，停止在此处");
        }
        let pax_version=local_metadata(&local)?;
        anyhow::ensure!(kind.is_hard_link()||!local.iter().any(|(k,_)|k=="linkpath"),"[archive-inspection:unsupported] TAR linkpath 仅支持硬链接条目");
        let pax_sparse=pax_version.is_some();
        anyhow::ensure!(!pax_sparse||(kind.is_file()&&long_name.is_none()),"[archive-inspection:unsupported] GNU.sparse PAX 仅支持普通文件且不能叠加 GNU 长名称");
        if let Some(name) = &long_name { crate::services::tar_pax::check_mixed_path(name.as_bytes(), &local)?; }
        let meta = global.effective(&local);
        anyhow::ensure!(!pax_sparse||crate::services::tar_pax::size(&meta)?.is_none_or(|physical|physical==size),"GNU.sparse 物理大小与 PAX size 冲突");
        let size = crate::services::tar_pax::size(&meta)?.unwrap_or(size);
        anyhow::ensure!(
            size <= MAX_EXTRACTED_BYTES && (!kind.is_dir() || size == 0),
            "TAR 条目大小无效"
        );
        // 0.1/1.0 replace a placeholder via sparse.name; 0.0 uses the ordinary path.
        let sparse_name=local.iter().find(|(k,_)|k=="GNU.sparse.name").map(|(_,value)|value.clone());
        let decoded = if let Some(path) = sparse_name.or_else(||long_name.take()).or_else(|| crate::services::tar_pax::path(&meta).map(str::to_owned)) {
            path
        } else {
            super::names::decode(&header.path_bytes(), 0, encoding)?
                .context("[lzop-encoding] TAR 路径不是 UTF-8，请选择名称编码后重新检查")?
        };
        anyhow::ensure!(
            !decoded.starts_with(['/', '\\'])
                && !decoded.contains([':', '\0'])
                && !decoded.split(['/', '\\']).any(|p| p == ".."),
            "TAR 路径无法确认，停止在此处"
        );
        if decoded.split(['/', '\\']).all(|s| s.is_empty() || s == ".") {
            anyhow::ensure!(kind.is_dir(), "TAR 文件名为空");
            continue;
        }
        let name = if kind.is_dir() {
            names.allocate_directory(&decoded, report.entries.len() + 1)
        } else {
            names.allocate(Some(&decoded), 0x2000, report.entries.len() + 1)
        };
        report.stopped_member = Some(name.clone());
        let key=archive_key(&decoded)?;
        let link=if kind.is_hard_link(){
            anyhow::ensure!(header.entry_size()?==0&&size==0,"[archive-inspection:unsupported] TAR 硬链接带有正文或冲突大小，暂不取回");
            let target=if let Some(value)=long_link.take().or_else(||local.iter().find(|(k,_)|k=="linkpath").map(|(_,v)|v.clone())){value}else{
                super::names::decode(&header.link_name_bytes().context("[archive-inspection:unsupported] TAR 硬链接缺少可确认目标")?,0,encoding)?.context("[lzop-encoding] TAR 链接目标不是 UTF-8，请选择名称编码后重新检查")?
            };
            let target_key=archive_key(&target)?;
            anyhow::ensure!(!target_key.is_empty()&&target_key!=key,"[archive-inspection:unsupported] TAR 硬链接目标为空或指向自身");
            let id=if complete_links {None} else {
                let id=sources.get(&target_key).copied().flatten().context("[archive-inspection:unsupported] TAR 硬链接目标尚未确认或名称有歧义，停止在此处")?;
                let entry=&report.entries[id];
                anyhow::ensure!(!entry.is_directory&&entry.link_source.is_none(),"[archive-inspection:unsupported] TAR 硬链接目标不是前面已确认的普通内容文件，暂不支持链接链条");
                Some(id)
            };
            Some((target,id))
        }else{None};
        let link_id=link.as_ref().and_then(|(_,id)|*id);
        let sparse = if let Some(id)=link_id {report.entries[id].sparse.clone()} else if kind.is_gnu_sparse() {
            anyhow::ensure!(!meta.iter().any(|(key,_)|key=="size"), "[archive-inspection:unsupported] 旧 GNU 稀疏条目暂不支持叠加 PAX size，已停止在此处");
            Some(crate::services::tar_sparse_recovery::Layout::parse(header, reader)?)
        } else if let Some(version)=pax_version {
            use crate::services::tar_sparse_recovery::{Layout,PaxVersion};
            Some(match version{PaxVersion::Pairs00=>Layout::parse_pax00(&local,size)?,PaxVersion::Map01=>Layout::parse_pax01(&local,size)?,PaxVersion::Text10=>Layout::parse_pax10(&local,size,reader)?})
        } else { None };
        let bytes = link_id.map_or_else(||sparse.as_ref().map_or(size, |layout|layout.logical),|id|report.entries[id].bytes);
        logical_total = logical_total.checked_add(bytes).context("TAR 完整大小溢出")?;
        anyhow::ensure!(logical_total <= MAX_EXTRACTED_BYTES, "[archive-inspection:unsupported] TAR 完整大小总量超过支持范围");
        sparse_segments += sparse.as_ref().map_or(0, |layout|layout.ranges.len());
        anyhow::ensure!(sparse_segments <= 1_000_000, "[archive-inspection:unsupported] TAR 稀疏映射总量超过支持范围");
        let offset = link_id.map_or(reader.position,|id|report.entries[id].offset);
        let digest = if let Some(id)=link_id {report.entries[id].sha256.clone()} else if let Some(layout) = &sparse {
            let cancelled = reader.cancelled;
            layout.process(reader, None, cancelled, |_|{})?
        } else {
            let mut remaining = size;
            let mut hash = Sha256::new();
            let mut buf = [0;65536];
            while remaining > 0 {
                let count = remaining.min(buf.len() as u64) as usize;
                reader.read_exact(&mut buf[..count]).context("TAR 文件内容未完整到达")?;
                hash.update(&buf[..count]); remaining -= count as u64;
            }
            format!("{:x}",hash.finalize())
        };
        padding(reader, size)?;
        let id=report.entries.len();
        sources.entry(key).and_modify(|previous|*previous=None).or_insert(Some(id));
        report.entries.push(Entry { archive_name:Some(decoded.clone()),
            link_target: link.as_ref().map(|(target,_)|target.clone()),
            link_source: link.and_then(|(_,id)|id),
            stored_bytes: sparse.as_ref().map(|layout|layout.stored),
            sparse,
            modified_time: {
                use crate::services::tar_recovery_time::ModificationTime;
                if let Some((_, value)) = meta.iter().find(|(k, _)| k == "mtime") {
                    ModificationTime::pax(value)
                } else { ModificationTime::header(header) }
            },
            time_warning: None,
            id: report.entries.len(),
            name,
            bytes,
            sha256: digest,
            is_directory: kind.is_dir(),
            offset,
        });
    }
}
pub(super) fn scan_source(
    src: &Source,
    encoding: Option<ZipPasswordEncoding>,
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
) -> Result<Report> {
    scan_source_with_links(src, encoding, cancelled, progress, false)
}

/// Only complete readers may resolve a reference to later content. Prefix
/// recovery retains its original stop-at-the-first-unconfirmed-entry policy.
pub(super) fn scan_complete_source(
    src: &Source, encoding: Option<ZipPasswordEncoding>, cancelled: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
) -> Result<Report> {
    scan_source_with_links(src, encoding, cancelled, progress, true)
}

fn resolve_complete_links(report: &mut Report, mut check: impl FnMut() -> Result<()>) -> Result<()> {
    use crate::services::tar_link_targets::{Record, resolve};
    let records = report.entries.iter().map(|e| Ok(Record {
        key: archive_key(e.archive_name.as_deref().unwrap_or(&e.name))?,
        content: !e.is_directory && e.link_target.is_none(),
        target: e.link_target.as_deref().map(archive_key).transpose()?,
    })).collect::<Result<Vec<_>>>()?;
    let targets = resolve(&records, &mut check)?;
    for (index, target) in targets.into_iter().enumerate() {
        check()?;
        if let Some(target) = target {
            let source = report.entries[target].clone();
            let entry = &mut report.entries[index];
            entry.link_source = Some(target);
            entry.bytes = source.bytes;
            entry.sha256 = source.sha256;
            entry.offset = source.offset;
            entry.sparse = source.sparse;
            entry.stored_bytes = source.stored_bytes;
        }
    }
    let mut total = 0u64; let mut segments = 0usize;
    for entry in &report.entries {
        check()?;
        total = total.checked_add(entry.bytes).context("TAR 完整大小溢出")?;
        segments += entry.sparse.as_ref().map_or(0, |s| s.ranges.len());
        anyhow::ensure!(total <= MAX_EXTRACTED_BYTES && segments <= 1_000_000,
            "[archive-inspection:unsupported] TAR.LZO 链接逻辑总量或稀疏映射超过支持范围");
    }
    Ok(())
}

fn scan_source_with_links(
    src: &Source, encoding: Option<ZipPasswordEncoding>, cancelled: &AtomicBool,
    progress: &mut impl FnMut(u64, u64), complete_links: bool,
) -> Result<Report> {
    let mut report = Report {
        hard_links_seen: false,
        format: "TAR.LZO",
        revision: String::new(),
        entries: Vec::new(),
        complete: false,
        stop_reason: String::new(),
        stopped_member: None,
        stopped_offset: 0,
        available_volumes: src.paths.len(),
        ignored_volumes: src.ignored,
    };
    let checked = (|| -> Result<()> {
        let reader = LzopReader::with_mode(src.open()?, true)?;
        let mut reader = Verified {
            reader,
            cancelled,
            progress: &mut *progress,
            total: src.length,
            position: 0,
        };
        parse(&mut reader, &mut report, encoding, complete_links)
    })();
    if let Err(e) = checked {
        check(cancelled)?;
        if crate::services::archive_io::kind(&e).is_some() {
            return Err(e);
        }
        report.stop_reason = format!("{e:#}");
    }
    if let Some(boundary) = &src.boundary {
        report.complete = false;
        report.stop_reason = format!("{}；{boundary}", report.stop_reason);
    }
    if report.stop_reason.is_empty() {
        report.stop_reason = "TAR 条目及外层 LZOP 已完整结束".into();
    }
    check(cancelled)?;
    let mut hash = Sha256::new();
    hash.update(src.stamp.as_bytes());
    hash.update(serde_json::to_vec(&report)?);
    for e in &report.entries {
        hash.update(e.offset.to_le_bytes());
        if let Some(id)=e.link_source {hash.update(b"hardlink");hash.update((id as u64).to_le_bytes());}
        if let Some(layout) = &e.sparse { hash.update(serde_json::to_vec(layout)?); }
    }
    report.revision = format!("{:x}", hash.finalize());
    progress(src.length, src.length);
    Ok(report)
}
fn destination(stage:&Path,name:&str)->Result<PathBuf>{
    let relative=Path::new(name);
    anyhow::ensure!(relative.components().all(|c|matches!(c,std::path::Component::Normal(_))),"TAR 条目路径无法取回");
    Ok(stage.join(relative))
}
/// Present only the stored extents of our newly written, verified logical file.
/// This lets the same sparse writer copy aliases without filling their holes.
struct StoredCopy {file:File,ranges:Option<Vec<(u64,u64)>>,next:usize,left:u64}
impl Read for StoredCopy{
    fn read(&mut self,buf:&mut[u8])->io::Result<usize>{
        let Some(ranges)=&self.ranges else{return self.file.read(buf)};
        while self.left==0{
            let Some(&(offset,length))=ranges.get(self.next)else{return Ok(0)};
            self.next+=1;self.file.seek(std::io::SeekFrom::Start(offset))?;self.left=length;
        }
        let n=(buf.len()as u64).min(self.left)as usize;let n=self.file.read(&mut buf[..n])?;self.left-=n as u64;Ok(n)
    }
}
fn copy_entry(input:&mut impl Read,stage:&Path,entry:&mut Entry,cancelled:&AtomicBool,done:&mut u64,progress:&mut impl FnMut(u64)->Result<()>)->Result<PathBuf>{
    let target=destination(stage,&entry.name)?;
    if let Some(parent)=target.parent(){fs::create_dir_all(parent)?;}
    let mut output=OpenOptions::new().write(true).create_new(true).open(&target)?;
    let digest=if let Some(layout)=&entry.sparse{
        let before=*done;let digest=layout.process_checked(input,Some(&mut output),cancelled,|logical|progress(before+logical))?;*done+=entry.bytes;digest
    }else{
        let mut remaining=entry.bytes;let mut hash=Sha256::new();let mut buf=[0;65536];
        while remaining>0{check(cancelled)?;let n=remaining.min(buf.len()as u64)as usize;input.read_exact(&mut buf[..n])?;output.write_all(&buf[..n])?;hash.update(&buf[..n]);remaining-=n as u64;*done+=n as u64;progress(*done)?;}
        format!("{:x}",hash.finalize())
    };
    anyhow::ensure!(digest==entry.sha256,"TAR 文件在取回期间已变化，请重新检查");
    output.flush()?;check(cancelled)?;
    entry.time_warning=match &entry.modified_time{Some(time)=>time.apply(&output).err().map(str::to_owned),None=>Some("原修改时间缺失，或其范围、精度暂不支持；文件内容已取回".into())};
    Ok(target)
}
pub(super) fn copy_selected(src:&Source,stage:&Path,entries:&mut[Entry],cancelled:&AtomicBool,progress:&mut impl FnMut(u64))->Result<()> {
    copy_selected_controlled(src,stage,entries,cancelled,&mut|bytes|{progress(bytes);Ok(())})
}
pub(super) fn copy_selected_controlled(
    src: &Source,
    stage: &Path,
    entries: &mut [Entry],
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(u64)->Result<()>,
) -> Result<()> {
    let reader = LzopReader::with_mode(src.open()?, true)?;
    let mut reader = Verified {
        reader,
        cancelled,
        progress: |_, _| {},
        total: src.length,
        position: 0,
    };
    let mut done = 0;
    let mut buf = [0; 65536];
    for entry in entries.iter().filter(|e|e.is_directory){check(cancelled)?;fs::create_dir_all(destination(stage,&entry.name)?)?;}
    // Decode source bytes once, in physical order. A first selected alias can
    // supply subsequent aliases even when the original target is unselected.
    let mut order=(0..entries.len()).filter(|i|!entries[*i].is_directory).collect::<Vec<_>>();
    order.sort_by_key(|i|(entries[*i].offset,entries[*i].link_source.unwrap_or(entries[*i].id)));
    let mut previous:Option<(u64,usize,PathBuf)>=None;
    for index in order{
        check(cancelled)?;let entry=&mut entries[index];let source_id=entry.link_source.unwrap_or(entry.id);
        if let Some((_,_,path))=previous.as_ref().filter(|(offset,id,_)|*offset==entry.offset&&*id==source_id){
            let mut stored=StoredCopy{file:File::open(path)?,ranges:entry.sparse.as_ref().map(|layout|layout.ranges.clone()),next:0,left:0};
            copy_entry(&mut stored,stage,entry,cancelled,&mut done,progress)?;
        }else{
            anyhow::ensure!(entry.offset>=reader.position,"TAR 取回位置无效");
            while reader.position<entry.offset{let n=(entry.offset-reader.position).min(buf.len()as u64)as usize;reader.read_exact(&mut buf[..n])?;}
            let path=copy_entry(&mut reader,stage,entry,cancelled,&mut done,progress)?;
            previous=Some((entry.offset,source_id,path));
        }
    }
    // Child creation changes parent mtimes. Apply only explicitly selected
    // directory entries after all contents exist, deepest directories first.
    let mut directories: Vec<_> = entries.iter().enumerate().filter(|(_, e)| e.is_directory)
        .map(|(index, e)| (index, Path::new(&e.name).components().count())).collect();
    directories.sort_by_key(|(_, depth)| std::cmp::Reverse(*depth));
    let directory_count = directories.len();
    for (step, (index, _)) in directories.into_iter().enumerate() {
        check(cancelled)?;
        let entry = &mut entries[index];
        entry.time_warning = match &entry.modified_time {
            Some(time) => time.apply_directory(&stage.join(&entry.name)).err().map(str::to_owned),
            None => Some("目录原修改时间缺失，或其范围、精度暂不支持；已取回内容仍保留".into()),
        };
        // These updates carry the same completed byte count. Batch them for
        // large directory sets, while cancellation remains checked per item.
        if step == 0 || (step + 1) % 64 == 0 || step + 1 == directory_count { progress(done)?; }
    }
    Ok(())
}

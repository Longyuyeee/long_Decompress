use super::ExtractionRuntime;
use crate::models::compression::DecompressOptions;
use crate::services::progress_gate::ProgressGate;
use anyhow::Result;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Window;
use zip_aes::{CompressionMethod, ZipArchive};

// Shared by all native ZIP operations, including sequential fallback. A task
// owns one slot; it may borrow one more, never wait while holding it for another.
static ACTIVE_WORKERS: AtomicUsize = AtomicUsize::new(0);
pub(super) struct WorkerSlot<'a>(&'a AtomicUsize);
impl WorkerSlot<'static> {
    fn try_acquire() -> Option<Self> {
        let limit = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(4);
        Self::from_budget(&ACTIVE_WORKERS, limit)
    }
    pub(super) fn acquire(runtime: &impl ExtractionRuntime) -> Result<Self> {
        loop {
            runtime.check_cancellation()?;
            if let Some(slot) = Self::try_acquire() {
                return Ok(slot);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl<'a> WorkerSlot<'a> {
    fn from_budget(counter: &'a AtomicUsize, limit: usize) -> Option<Self> {
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < limit).then_some(n + 1)
            })
            .ok()
            .map(|_| Self(counter))
    }
}
impl Drop for WorkerSlot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

struct Entry {
    index: usize,
    name: String,
    destination: PathBuf,
    size: u64,
    modified: Option<std::time::SystemTime>,
}

fn plan<R: Read + Seek>(
    runtime: &impl ExtractionRuntime,
    archive: &mut ZipArchive<R>,
    output: &Path,
    options: &DecompressOptions,
) -> Result<Option<Vec<Entry>>> {
    if !options.parallel_zip
        || !options.preserve_paths
        || options.skip_corrupted
        || options.extract_only_newer
        || options.file_filter.is_some()
        || !options.selected_entries.is_empty()
    {
        return Ok(None);
    }
    let mut paths = HashSet::new();
    let mut files = HashSet::new();
    let mut entries = Vec::new();
    let mut large_files = 0;
    for index in 0..archive.len() {
        runtime.check_cancellation()?;
        let entry = archive.by_index_raw(index)?;
        if entry.encrypted()
            || entry.is_symlink()
            || !matches!(
                entry.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            )
        {
            return Ok(None);
        }
        let Some(relative) = entry.enclosed_name() else {
            return Ok(None);
        };
        // Windows aliases, ambiguous names and duplicate targets use the
        // established sequential policy, never race against another writer.
        let key = relative.to_string_lossy().replace('\\', "/").to_lowercase();
        if key.is_empty()
            || key
                .split('/')
                .any(|part| part.contains(':') || part.ends_with(['.', ' ']))
            || !paths.insert(key.clone())
        {
            return Ok(None);
        }
        let destination = output.join(&relative);
        if entry.is_dir() {
            continue;
        }
        files.insert(key);
        if entry.size() >= 256 * 1024 {
            large_files += 1;
        }
        let modified = entry.last_modified().and_then(|t| {
            super::zip::system_time(
                t.year(),
                t.month(),
                t.day(),
                t.hour(),
                t.minute(),
                t.second(),
            )
        });
        entries.push(Entry {
            index,
            name: entry.name().into(),
            destination,
            size: entry.size(),
            modified,
        });
    }
    // Reject small-file collections from archive metadata before any per-target
    // filesystem probes; otherwise opting in penalizes the sequential fallback.
    if entries.len() != archive.len() || large_files < 2 {
        return Ok(None);
    }
    for path in &paths {
        let mut ancestor = path.as_str();
        while let Some((parent, _)) = ancestor.rsplit_once('/') {
            if files.contains(parent) {
                return Ok(None);
            }
            ancestor = parent;
        }
    }
    if entries.iter().any(|entry| entry.destination.exists()) {
        return Ok(None);
    }
    Ok(Some(entries))
}

pub(super) fn try_extract<R: ExtractionRuntime + Sync, Rd: Read + Seek>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    file: &str,
    output: &str,
    archive: &mut ZipArchive<Rd>,
    options: &DecompressOptions,
) -> Result<bool> {
    let Some(entries) = plan(runtime, archive, Path::new(output), options)? else {
        return Ok(false);
    };
    let Some(extra_slot) = WorkerSlot::try_acquire() else {
        return Ok(false);
    };
    // Prepare readers before any output is written. After writing starts an I/O
    // or CRC error is returned, never hidden by a second sequential extraction.
    let first = ZipArchive::new(crate::services::numeric_volume_reader::ZipInputReader::open(Path::new(file))?)?;
    let second = ZipArchive::new(crate::services::numeric_volume_reader::ZipInputReader::open(Path::new(file))?)?;
    if let Some(window) = window {
        runtime.emit_log(window, task_id, "ZIP 并行解压：使用 2 个共享预算工作线程；完成后仍按原冲突策略提交结果", crate::models::compression::TaskLogSeverity::Info);
    }
    let next = AtomicUsize::new(0);
    let stopped = AtomicBool::new(false);
    let failure = Mutex::new(None);
    let total_bytes = entries
        .iter()
        .try_fold(0u64, |sum, entry| sum.checked_add(entry.size))
        .ok_or_else(|| anyhow::anyhow!("ZIP entry sizes exceed supported total"))?;
    let progress = Mutex::new((0usize, 0u64, ProgressGate::new()));
    let worker = |mut reader: ZipArchive<crate::services::numeric_volume_reader::ZipInputReader>| {
        let result = (|| -> Result<()> {
            let mut buffer = vec![0u8; 256 * 1024];
            loop {
                runtime.check_cancellation()?;
                if stopped.load(Ordering::Acquire) {
                    return Ok(());
                }
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(entry) = entries.get(index) else {
                    return Ok(());
                };
                if let Some(parent) = entry.destination.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut input = reader.by_index(entry.index)?;
                let mut output = File::options()
                    .write(true)
                    .create_new(true)
                    .open(&entry.destination)?;
                loop {
                    runtime.check_cancellation()?;
                    if stopped.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    let count = input.read(&mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    output.write_all(&buffer[..count])?;
                    let mut state = progress.lock().unwrap();
                    state.1 = state.1.saturating_add(count as u64);
                    if state.2.allow(false) {
                        if let Some(window) = window {
                            runtime.emit_progress(
                                window,
                                task_id,
                                (state.1 as f32 / total_bytes.max(1) as f32).min(0.999),
                                Some(entry.name.clone()),
                                state.1,
                                total_bytes,
                            );
                        }
                    }
                }
                output.flush()?;
                if options.preserve_timestamps {
                    if let Some(modified) = entry.modified {
                        filetime::set_file_mtime(
                            &entry.destination,
                            filetime::FileTime::from_system_time(modified),
                        )?;
                    }
                }
                let mut state = progress.lock().unwrap();
                state.0 += 1;
                let finished = state.0 == entries.len();
                if finished {
                    if let Some(window) = window {
                        runtime.emit_progress(
                            window,
                            task_id,
                            1.0,
                            Some(entry.name.clone()),
                            state.1,
                            total_bytes,
                        );
                    }
                }
            }
        })();
        if let Err(error) = result {
            stopped.store(true, Ordering::Release);
            let mut first_error = failure.lock().unwrap();
            if first_error.is_none() {
                *first_error = Some(error);
            }
        }
    };
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let _slot = extra_slot;
            worker(second);
        });
        worker(first);
    });
    if let Some(error) = failure.into_inner().unwrap() {
        return Err(error);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::compression_service::CompressionService;
    use sha2::{Digest, Sha256};
    use std::io::{Cursor, SeekFrom};
    use zip_aes::write::SimpleFileOptions;

    #[test]
    fn task_and_entry_workers_share_a_bounded_budget_released_on_error() {
        let counter = AtomicUsize::new(0);
        let task = WorkerSlot::from_budget(&counter, 2).unwrap();
        let failed_entry = (|| -> Result<()> {
            let _entry = WorkerSlot::from_budget(&counter, 2).unwrap();
            assert!(WorkerSlot::from_budget(&counter, 2).is_none());
            anyhow::bail!("simulated entry error");
        })();
        assert!(failed_entry.is_err());
        assert_eq!(counter.load(Ordering::Acquire), 1);
        drop(task);
        assert_eq!(counter.load(Ordering::Acquire), 0);
    }

    struct CancelDuringCopy {
        inner: CompressionService,
        checks: AtomicUsize,
    }
    impl ExtractionRuntime for CancelDuringCopy {
        fn check_cancellation(&self) -> Result<()> {
            if self.checks.fetch_add(1, Ordering::Relaxed) >= 10 {
                anyhow::bail!("CANCELLED_DURING_COPY");
            }
            Ok(())
        }
        fn buffer_pool(&self) -> &crate::services::io_buffer_pool::IOBufferPool {
            &self.inner.buffer_pool
        }
        fn copy_buffer_size(&self) -> usize {
            256 * 1024
        }
        fn normalized_archive_path(&self, path: &Path, preserve: bool) -> Option<PathBuf> {
            ExtractionRuntime::normalized_archive_path(&self.inner, path, preserve)
        }
        fn emit_log(
            &self,
            _: &Window,
            _: &str,
            _: &str,
            _: crate::models::compression::TaskLogSeverity,
        ) {
        }
        fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
    }
    #[test]
    fn cancellation_inside_parallel_copy_returns_the_actual_cancellation() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("input.zip");
        fixture(
            &file,
            &["a.bin", "b.bin"],
            4 * 1024 * 1024,
            CompressionMethod::Stored,
            None,
        );
        let runtime = CancelDuringCopy {
            inner: CompressionService::for_testing(),
            checks: AtomicUsize::new(0),
        };
        let error = super::super::zip::extract_with_optional_window(
            &runtime,
            None,
            "cancel",
            file.to_str().unwrap(),
            temp.path().join("out").to_str().unwrap(),
            None,
            &options(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("CANCELLED_DURING_COPY"));
        assert!(runtime.checks.load(Ordering::Relaxed) > 10);
    }

    fn fixture(
        path: &Path,
        names: &[&str],
        bytes: usize,
        method: CompressionMethod,
        password: Option<&str>,
    ) -> Vec<u8> {
        let mut payload = vec![0u8; bytes];
        let mut state = 0x6D2B79F5u32;
        for byte in &mut payload {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        let mut writer = zip_aes::ZipWriter::new(File::create(path).unwrap());
        let mut options = SimpleFileOptions::default()
            .compression_method(method)
            .last_modified_time(
                zip_aes::DateTime::from_date_and_time(2020, 5, 6, 12, 34, 56).unwrap(),
            );
        if let Some(password) = password {
            options = options.with_aes_encryption(zip_aes::AesMode::Aes256, password);
        }
        for name in names {
            writer.start_file(*name, options).unwrap();
            writer.write_all(&payload).unwrap();
        }
        writer.finish().unwrap();
        payload
    }
    fn options() -> DecompressOptions {
        DecompressOptions {
            parallel_zip: true,
            preserve_paths: true,
            preserve_timestamps: true,
            ..Default::default()
        }
    }

    #[test]
    fn parallel_store_and_deflate_preserve_every_byte_and_release_worker_slots() {
        for method in [CompressionMethod::Stored, CompressionMethod::Deflated] {
            let temp = tempfile::tempdir().unwrap();
            let file = temp.path().join("input.zip");
            let output = temp.path().join("out");
            let payload = fixture(
                &file,
                &["a.bin", "sub/b.bin", "sub/c.bin"],
                512 * 1024,
                method,
                None,
            );
            let runtime = CompressionService::for_testing();
            super::super::zip::extract_with_optional_window(
                &runtime,
                None,
                "parallel",
                file.to_str().unwrap(),
                output.to_str().unwrap(),
                None,
                &options(),
            )
            .unwrap();
            for name in ["a.bin", "sub/b.bin", "sub/c.bin"] {
                assert_eq!(std::fs::read(output.join(name)).unwrap(), payload);
                let modified = std::fs::metadata(output.join(name))
                    .unwrap()
                    .modified()
                    .unwrap();
                assert_eq!(
                    modified,
                    super::super::zip::system_time(2020, 5, 6, 12, 34, 56).unwrap()
                );
            }
        }
    }

    #[test]
    fn ineligible_inputs_fall_back_before_writing_and_encrypted_fallback_still_extracts() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("input.zip");
        let runtime = CompressionService::for_testing();
        let output = temp.path().join("out");
        fixture(
            &file,
            &["A.bin", "a.bin"],
            512 * 1024,
            CompressionMethod::Stored,
            None,
        );
        let mut archive = ZipArchive::new(File::open(&file).unwrap()).unwrap();
        assert!(plan(&runtime, &mut archive, &output, &options())
            .unwrap()
            .is_none());
        assert!(!output.exists());
        let payload = fixture(
            &file,
            &["one.bin", "two.bin"],
            512 * 1024,
            CompressionMethod::Deflated,
            Some("secret"),
        );
        let mut archive = ZipArchive::new(File::open(&file).unwrap()).unwrap();
        assert!(plan(&runtime, &mut archive, &output, &options())
            .unwrap()
            .is_none());
        super::super::zip::extract_with_optional_window(
            &runtime,
            None,
            "encrypted",
            file.to_str().unwrap(),
            output.to_str().unwrap(),
            Some("secret"),
            &options(),
        )
        .unwrap();
        assert_eq!(std::fs::read(output.join("one.bin")).unwrap(), payload);
        let mut writer = zip_aes::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .add_directory("empty/", SimpleFileOptions::default())
            .unwrap();
        let mut archive = ZipArchive::new(writer.finish().unwrap()).unwrap();
        assert!(plan(&runtime, &mut archive, &output, &options())
            .unwrap()
            .is_none());
    }

    #[test]
    fn crc_write_failure_and_cancel_are_not_hidden_by_sequential_retry() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("input.zip");
        let runtime = CompressionService::for_testing();
        fixture(
            &file,
            &["blocked/one.bin", "two.bin"],
            512 * 1024,
            CompressionMethod::Stored,
            None,
        );
        let output = temp.path().join("out");
        std::fs::create_dir(&output).unwrap();
        std::fs::write(output.join("blocked"), b"keep this file").unwrap();
        assert!(super::super::zip::extract_with_optional_window(
            &runtime,
            None,
            "failure",
            file.to_str().unwrap(),
            output.to_str().unwrap(),
            None,
            &options()
        )
        .is_err());
        assert_eq!(
            std::fs::read(output.join("blocked")).unwrap(),
            b"keep this file"
        );
        fixture(
            &file,
            &["one.bin", "two.bin"],
            512 * 1024,
            CompressionMethod::Stored,
            None,
        );
        let offset = ZipArchive::new(File::open(&file).unwrap())
            .unwrap()
            .by_index_raw(0)
            .unwrap()
            .data_start();
        let mut writer = File::options().write(true).open(&file).unwrap();
        writer
            .seek(SeekFrom::Start(offset.expect("entry data offset")))
            .unwrap();
        writer.write_all(b"broken").unwrap();
        drop(writer);
        assert!(super::super::zip::extract_with_optional_window(
            &runtime,
            None,
            "crc",
            file.to_str().unwrap(),
            temp.path().join("crc").to_str().unwrap(),
            None,
            &options()
        )
        .is_err());
        runtime.cancellation_flag.store(true, Ordering::Release);
        let cancelled_output = temp.path().join("cancelled");
        assert!(super::super::zip::extract_with_optional_window(
            &runtime,
            None,
            "cancelled",
            file.to_str().unwrap(),
            cancelled_output.to_str().unwrap(),
            None,
            &options()
        )
        .is_err());
        assert!(!cancelled_output.exists());
    }

    #[test]
    #[ignore = "explicit same-machine sequential/parallel ZIP A/B"]
    fn parallel_zip_ab() {
        let temp = tempfile::tempdir().unwrap();
        let names = ["a.bin", "b.bin", "c.bin", "d.bin"];
        for method in [CompressionMethod::Stored, CompressionMethod::Deflated] {
            let file = temp.path().join(format!("{method:?}.zip"));
            let payload = fixture(&file, &names, 8 * 1024 * 1024, method, None);
            let expected = Sha256::digest(&payload);
            for round in 0..3 {
                for parallel in [false, true] {
                    let output = temp.path().join(format!("{method:?}-{round}-{parallel}"));
                    let started = std::time::Instant::now();
                    super::super::zip::extract_with_optional_window(
                        &CompressionService::for_testing(),
                        None,
                        "ab",
                        file.to_str().unwrap(),
                        output.to_str().unwrap(),
                        None,
                        &DecompressOptions {
                            parallel_zip: parallel,
                            ..options()
                        },
                    )
                    .unwrap();
                    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
                    for name in names {
                        assert_eq!(
                            Sha256::digest(std::fs::read(output.join(name)).unwrap()),
                            expected
                        );
                    }
                    println!("ZIP_PARALLEL_AB method={method:?} round={round} parallel={parallel} ms={elapsed:.2} input_bytes={} sha256={expected:x}", payload.len() * names.len());
                }
            }
        }
    }
}

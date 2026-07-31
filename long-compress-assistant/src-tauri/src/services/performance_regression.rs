use super::compression_service::CompressionService;
use super::{native_compression, native_extraction};
use crate::models::compression::{CompressionOptions, DecompressOptions};
use std::fs::File;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tempfile::tempdir;

const MIB: u64 = 1024 * 1024;
const COPY_BUFFER_SIZE: usize = 256 * 1024;

fn update_peak(peak: &AtomicU64, value: u64) {
    let mut current = peak.load(Ordering::Relaxed);
    while value > current {
        match peak.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

fn start_memory_sampler() -> (Arc<AtomicBool>, Arc<AtomicU64>, std::thread::JoinHandle<()>) {
    let running = Arc::new(AtomicBool::new(true));
    let peak = Arc::new(AtomicU64::new(0));
    let sampler_running = running.clone();
    let sampler_peak = peak.clone();
    let handle = std::thread::spawn(move || {
        let pid = sysinfo::get_current_pid().expect("current process id");
        let mut system = System::new();
        while sampler_running.load(Ordering::Relaxed) {
            if system.refresh_process(pid) {
                if let Some(process) = system.process(pid) {
                    update_peak(&sampler_peak, process.memory());
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    (running, peak, handle)
}

fn current_process_memory() -> u64 {
    let pid = sysinfo::get_current_pid().expect("current process id");
    let mut system = System::new();
    assert!(system.refresh_process(pid), "refresh benchmark process");
    system.process(pid).expect("benchmark process").memory()
}

fn write_fixture(path: &std::path::Path, size_mib: u64) -> u32 {
    let mut file = File::create(path).expect("create 7z performance fixture");
    let mut hasher = crc32fast::Hasher::new();
    let mut state = 0x6D2B_79F5u32;
    let mut chunk = vec![0u8; MIB as usize];
    for _ in 0..size_mib {
        for byte in &mut chunk {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        file.write_all(&chunk)
            .expect("write 7z performance fixture");
        hasher.update(&chunk);
    }
    file.flush().expect("flush 7z performance fixture");
    hasher.finalize()
}

fn crc32_file(path: &std::path::Path) -> u32 {
    let mut file = File::open(path).expect("open 7z checksum input");
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
    loop {
        let read = file.read(&mut buffer).expect("read 7z checksum input");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher.finalize()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "real 7z performance benchmark; run explicitly on a fixed Windows machine"]
async fn real_7z_large_file_baseline() {
    let size_mib = std::env::var("LONG_DECOMPRESS_PERF_SIZE_MIB")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(100)
        .clamp(16, 2048);
    let temp = tempdir().expect("7z performance temp dir");
    let source = temp.path().join("source.bin");
    let archive = temp.path().join("source.7z");
    let output = temp.path().join("output");
    let expected_crc = write_fixture(&source, size_mib);
    let service = CompressionService::for_testing();

    let baseline_memory = current_process_memory();
    let (running, peak_memory, sampler) = start_memory_sampler();
    let compression_started = Instant::now();
    native_compression::seven_zip::compress(
        &service,
        None,
        "performance-7z",
        &[source.to_string_lossy().into_owned()],
        archive.to_string_lossy().as_ref(),
        CompressionOptions::default(),
    )
    .expect("compress 7z performance fixture");
    let compression_time = compression_started.elapsed();

    let extraction_started = Instant::now();
    native_extraction::seven_zip::extract(
        &service,
        None,
        "performance-7z",
        archive.to_string_lossy().as_ref(),
        output.to_string_lossy().as_ref(),
        None,
        &DecompressOptions::default(),
    )
    .expect("extract 7z performance fixture");
    let extraction_time = extraction_started.elapsed();
    running.store(false, Ordering::Relaxed);
    sampler.join().expect("join 7z memory sampler");

    let extracted = output.join("source.bin");
    assert_eq!(crc32_file(&extracted), expected_crc);
    assert_eq!(
        std::fs::metadata(&extracted)
            .expect("7z extracted metadata")
            .len(),
        size_mib * MIB
    );

    let peak_delta = peak_memory
        .load(Ordering::Relaxed)
        .saturating_sub(baseline_memory);
    println!(
        "PERF_JSON {}",
        serde_json::json!({
            "scenario": "seven_zip_large_file",
            "fixture_mib": size_mib,
            "compression_mib_s": size_mib as f64 / compression_time.as_secs_f64(),
            "extraction_mib_s": size_mib as f64 / extraction_time.as_secs_f64(),
            "compression_ms": compression_time.as_millis(),
            "extraction_ms": extraction_time.as_millis(),
            "peak_working_set_delta_mib": peak_delta as f64 / MIB as f64,
            "archive_bytes": std::fs::metadata(&archive).expect("7z archive metadata").len(),
        })
    );
    assert!(
        peak_delta < 512 * MIB,
        "native 7z path used more than 512 MiB of additional working set"
    );
}

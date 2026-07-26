use long_compress_assistant::models::compression::CompressionOptions;
use long_compress_assistant::services::compression_service::CompressionService;
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
        match peak.compare_exchange_weak(
            current,
            value,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
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
        if system.refresh_process(pid) {
            if let Some(process) = system.process(pid) {
                update_peak(&sampler_peak, process.memory());
            }
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

fn write_deterministic_fixture(path: &std::path::Path, size_mib: u64) -> u32 {
    let mut file = File::create(path).expect("create performance fixture");
    let mut hasher = crc32fast::Hasher::new();
    let mut state = 0x9E37_79B9u32;
    let mut chunk = vec![0u8; MIB as usize];

    for _ in 0..size_mib {
        for byte in &mut chunk {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        file.write_all(&chunk).expect("write performance fixture");
        hasher.update(&chunk);
    }
    file.flush().expect("flush performance fixture");
    hasher.finalize()
}

fn crc32_file(path: &std::path::Path) -> u32 {
    let mut file = File::open(path).expect("open checksum input");
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
    loop {
        let read = file.read(&mut buffer).expect("read checksum input");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher.finalize()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "real 100 MiB+ performance benchmark; run explicitly before performance releases"]
async fn real_zip_compress_extract_baseline() {
    let size_mib = std::env::var("LONG_DECOMPRESS_PERF_SIZE_MIB")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(100)
        .clamp(16, 2048);
    let temp = tempdir().expect("performance temp dir");
    let source = temp.path().join("source.bin");
    let archive = temp.path().join("source.zip");
    let extracted = temp.path().join("extracted.bin");
    let expected_crc = write_deterministic_fixture(&source, size_mib);
    let source_bytes = size_mib * MIB;

    let baseline_memory = current_process_memory();
    let (running, peak_memory, sampler) = start_memory_sampler();
    let service = CompressionService::for_testing();
    let started = Instant::now();
    service
        .compress_zip_enhanced(
            &[source.to_string_lossy().into_owned()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .await
        .expect("compress performance fixture");
    let compression_time = started.elapsed();

    let archive_file = File::open(&archive).expect("open performance archive");
    let mut zip = zip::ZipArchive::new(archive_file).expect("read performance archive");
    let mut entry = zip.by_index(0).expect("performance archive entry");
    let mut output = File::create(&extracted).expect("create extracted fixture");
    let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
    let extraction_started = Instant::now();
    loop {
        let read = entry.read(&mut buffer).expect("decompress performance fixture");
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .expect("write extracted fixture");
    }
    output.flush().expect("flush extracted fixture");
    let extraction_time = extraction_started.elapsed();
    running.store(false, Ordering::Relaxed);
    sampler.join().expect("join memory sampler");

    assert_eq!(
        std::fs::metadata(&extracted).expect("extracted metadata").len(),
        source_bytes,
    );
    assert_eq!(crc32_file(&extracted), expected_crc);

    let compression_mib_s = size_mib as f64 / compression_time.as_secs_f64();
    let extraction_mib_s = size_mib as f64 / extraction_time.as_secs_f64();
    let peak_delta = peak_memory
        .load(Ordering::Relaxed)
        .saturating_sub(baseline_memory);
    println!(
        "PERF size={}MiB compression={:.2}MiB/s extraction={:.2}MiB/s \
         compression_ms={} extraction_ms={} peak_working_set_delta={:.2}MiB archive_bytes={}",
        size_mib,
        compression_mib_s,
        extraction_mib_s,
        compression_time.as_millis(),
        extraction_time.as_millis(),
        peak_delta as f64 / MIB as f64,
        std::fs::metadata(&archive).expect("archive metadata").len(),
    );

    assert!(
        peak_delta < 256 * MIB,
        "streaming ZIP path used more than 256 MiB of additional working set"
    );
}

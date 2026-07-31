use long_compress_assistant::services::aes_stream_v2::{AesStreamKind, AesStreamV2};
use std::fs::File;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tempfile::tempdir;

const MIB: u64 = 1024 * 1024;
const COPY_BUFFER_SIZE: usize = 256 * 1024;
const MAX_PEAK_DELTA_MIB: u64 = 192;

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
    let mut file = File::create(path).expect("create AES performance fixture");
    let mut hasher = crc32fast::Hasher::new();
    let mut state = 0x7F4A_7C15u32;
    let mut chunk = vec![0u8; MIB as usize];
    for _ in 0..size_mib {
        for byte in &mut chunk {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        file.write_all(&chunk).expect("write AES fixture");
        hasher.update(&chunk);
    }
    file.flush().expect("flush AES fixture");
    hasher.finalize()
}

fn crc32_file(path: &std::path::Path) -> u32 {
    let mut file = File::open(path).expect("open AES checksum input");
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
    loop {
        let read = file.read(&mut buffer).expect("read AES checksum input");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher.finalize()
}

fn run_aes_stream_baseline(size_mib: u64) {
    let temp = tempdir().expect("AES performance temp dir");
    let source = temp.path().join("source.bin");
    let encrypted = temp.path().join("source.bin.aes");
    let decrypted = temp.path().join("decrypted.bin");
    let expected_crc = write_deterministic_fixture(&source, size_mib);
    let expected_bytes = size_mib * MIB;

    let baseline_memory = current_process_memory();
    let (running, peak_memory, sampler) = start_memory_sampler();
    let encryption_started = Instant::now();
    AesStreamV2::encrypt_file(
        &source,
        &encrypted,
        "long-decompress-performance-fixture",
        AesStreamKind::Generic,
    )
    .expect("encrypt AES performance fixture");
    let encryption_time = encryption_started.elapsed();

    let decryption_started = Instant::now();
    AesStreamV2::decrypt_file(
        &encrypted,
        &decrypted,
        "long-decompress-performance-fixture",
        AesStreamKind::Generic,
    )
    .expect("decrypt AES performance fixture");
    let decryption_time = decryption_started.elapsed();
    running.store(false, Ordering::Relaxed);
    sampler.join().expect("join AES memory sampler");

    assert_eq!(
        std::fs::metadata(&decrypted)
            .expect("decrypted metadata")
            .len(),
        expected_bytes
    );
    assert_eq!(crc32_file(&decrypted), expected_crc);

    let peak_delta = peak_memory
        .load(Ordering::Relaxed)
        .saturating_sub(baseline_memory);
    println!(
        "AES_PERF size={}MiB encryption={:.2}MiB/s decryption={:.2}MiB/s \
         encryption_ms={} decryption_ms={} peak_working_set_delta={:.2}MiB container_bytes={}",
        size_mib,
        size_mib as f64 / encryption_time.as_secs_f64(),
        size_mib as f64 / decryption_time.as_secs_f64(),
        encryption_time.as_millis(),
        decryption_time.as_millis(),
        peak_delta as f64 / MIB as f64,
        std::fs::metadata(&encrypted)
            .expect("encrypted metadata")
            .len(),
    );
    println!(
        "PERF_JSON {}",
        serde_json::json!({
            "scenario": "aes_v2_large_file",
            "fixture_mib": size_mib,
            "encryption_mib_s": size_mib as f64 / encryption_time.as_secs_f64(),
            "decryption_mib_s": size_mib as f64 / decryption_time.as_secs_f64(),
            "encryption_ms": encryption_time.as_millis(),
            "decryption_ms": decryption_time.as_millis(),
            "peak_working_set_delta_mib": peak_delta as f64 / MIB as f64,
            "container_bytes": std::fs::metadata(&encrypted)
                .expect("encrypted metadata")
                .len(),
        })
    );
    assert!(
        peak_delta < MAX_PEAK_DELTA_MIB * MIB,
        "AES v2 used more than {MAX_PEAK_DELTA_MIB} MiB of additional working set"
    );
}

#[test]
#[ignore = "real 100 MiB AES benchmark; run explicitly before performance releases"]
fn real_aes_stream_100_mib_baseline() {
    let size_mib = std::env::var("LONG_DECOMPRESS_PERF_AES_SIZE_MIB")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(100)
        .clamp(16, 2048);
    run_aes_stream_baseline(size_mib);
}

#[test]
#[ignore = "real 1 GiB AES benchmark; run explicitly on a machine with sufficient free disk"]
fn real_aes_stream_1_gib_baseline() {
    run_aes_stream_baseline(1024);
}

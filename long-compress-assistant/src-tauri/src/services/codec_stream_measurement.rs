//! Explicit engine/disk-space observations, not desktop or physical I/O timings.
use super::*;
use crate::models::compression::TaskLogSeverity;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{atomic::{AtomicBool, AtomicU64, Ordering}, Arc};
use std::time::Instant;

#[derive(Default)]
struct Runtime {
    // Only a separate untimed observation scans for temporary files.
    workspace: Option<PathBuf>,
    output: PathBuf,
    peak_temporary_bytes: AtomicU64,
}
impl CompressionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> {
        if let Some(workspace) = &self.workspace {
            let total = std::fs::read_dir(workspace)?.try_fold(0u64, |total, entry| -> Result<u64> {
                let path = entry?.path();
                // Open a fresh handle: Windows path attributes may retain a
                // stale length while the temporary file is still open.
                Ok(total + if path != self.output && path.is_file() { File::open(&path)?.metadata()?.len() } else { 0 })
            })?;
            self.peak_temporary_bytes.fetch_max(total, Ordering::Relaxed);
        }
        Ok(())
    }
    fn cancellation_flag(&self) -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
    fn copy_buffer_size(&self) -> usize { 64 * 1024 }
    fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}

fn hashes(output: &Path, format: &str) -> BTreeMap<String, String> {
    let mut archive = ::tar::Archive::new(decoder(output, format).unwrap());
    let mut hashes = BTreeMap::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        if entry.header().entry_type().is_dir() { continue; }
        let path = entry.path().unwrap().to_string_lossy().replace('\\', "/");
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = entry.read(&mut buffer).unwrap();
            if read == 0 { break; }
            digest.update(&buffer[..read]);
        }
        assert!(hashes.insert(path, format!("{:x}", digest.finalize())).is_none());
    }
    // Drain through the codec footer, not merely TAR's end-of-entries marker.
    std::io::copy(&mut archive.into_inner(), &mut std::io::sink()).unwrap();
    hashes
}

#[test]
#[ignore = "explicit TAR codec creation/temporary-space measurement with real files"]
fn tar_creation_measurement() {
    use rand::{RngCore, SeedableRng};
    let root = tempfile::tempdir().unwrap();
    for scenario in ["large", "small"] {
        let source = root.path().join(scenario).join("input");
        std::fs::create_dir_all(&source).unwrap();
        let mut random = rand::rngs::StdRng::seed_from_u64(1415);
        let mut expected = BTreeMap::new();
        let (count, size) = if scenario == "large" { (1, 32 * 1024 * 1024) } else { (1000, 4096) };
        for index in 0..count {
            let mut bytes = vec![0; size];
            random.fill_bytes(&mut bytes);
            let name = format!("内容-{index:04}.bin");
            std::fs::write(source.join(&name), &bytes).unwrap();
            expected.insert(format!("input/{name}"), format!("{:x}", Sha256::digest(&bytes)));
        }
        let fixture_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(&expected).unwrap()));
        for format in ["tar.lz4", "tar.br"] {
            let workspace = root.path().join(scenario).join(format);
            std::fs::create_dir(&workspace).unwrap();
            let output = workspace.join(format!("result.{format}"));
            let options = CompressionOptions { level: 6, preserve_paths: Some(true), ..Default::default() };
            let observer = Runtime { workspace: Some(workspace), output: output.clone(), ..Default::default() };
            compress(&observer, None, "temporary-observation", &[source.to_string_lossy().into_owned()],
                output.to_str().unwrap(), options.clone(), format).unwrap();
            assert_eq!(hashes(&output, format), expected);
            let temporary_peak = observer.peak_temporary_bytes.load(Ordering::Relaxed);
            for round in 1..=3 {
                let started = Instant::now();
                compress(&Runtime::default(), None, "timed-creation", &[source.to_string_lossy().into_owned()],
                    output.to_str().unwrap(), options.clone(), format).unwrap();
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(hashes(&output, format), expected);
                println!("TAR_CREATION_JSON {}", serde_json::json!({
                    "scenario": scenario, "format": format, "round": round, "level": 6,
                    "input_bytes": count * size, "files": count, "fixture_sha256": fixture_sha256,
                    "compression_ms": elapsed_ms, "output_bytes": output.metadata().unwrap().len(),
                    "temporary_peak_bytes_untimed": temporary_peak, "payload_hashes_verified": true,
                }));
            }
        }
    }
}

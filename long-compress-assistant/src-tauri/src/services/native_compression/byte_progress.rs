use crate::services::compression_entries::CompressionEntry;
use anyhow::Result;

pub(crate) const PROGRESS_EMIT_INTERVAL_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct ByteProgress {
    processed: u64,
    total: u64,
    last_emitted: u64,
}

impl ByteProgress {
    pub(crate) fn new(total: u64) -> Self {
        Self {
            processed: 0,
            total,
            last_emitted: 0,
        }
    }

    pub(crate) fn total(&self) -> u64 {
        self.total
    }

    pub(crate) fn record(&mut self, read: u64, force: bool) -> Option<(f32, u64, u64)> {
        self.processed = self.processed.saturating_add(read);
        let should_emit = self.total > 0
            && (force
                || self.processed.saturating_sub(self.last_emitted)
                    >= PROGRESS_EMIT_INTERVAL_BYTES
                || self.processed >= self.total);
        if !should_emit || self.processed == self.last_emitted {
            return None;
        }
        self.last_emitted = self.processed;
        Some((
            (self.processed as f32 / self.total as f32).clamp(0.0, 1.0),
            self.processed,
            self.total,
        ))
    }
}

pub(crate) fn source_total_bytes(entries: &[CompressionEntry]) -> Result<u64> {
    entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .try_fold(0u64, |total, entry| {
            Ok(total.saturating_add(entry.path.metadata()?.len()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_real_intermediate_and_final_totals() {
        let total = PROGRESS_EMIT_INTERVAL_BYTES * 2 + 17;
        let mut progress = ByteProgress::new(total);
        assert_eq!(progress.record(1024, false), None);
        let first = progress
            .record(PROGRESS_EMIT_INTERVAL_BYTES, false)
            .expect("intermediate progress");
        assert!(first.0 > 0.0 && first.0 < 1.0);
        assert_eq!(first.1, PROGRESS_EMIT_INTERVAL_BYTES + 1024);
        assert_eq!(first.2, total);
        assert_eq!(
            progress.record(total - first.1, false),
            Some((1.0, total, total))
        );
    }
}

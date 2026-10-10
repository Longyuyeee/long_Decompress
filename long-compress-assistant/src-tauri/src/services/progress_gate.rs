use std::time::{Duration, Instant};

/// Per-operation UI progress budget. This never gates logs, cancellation checks,
/// or task completion/failure events. A completed entry is not a completed task.
pub(crate) struct ProgressGate {
    last: Option<Instant>,
}

impl ProgressGate {
    pub(crate) fn new() -> Self {
        Self { last: None }
    }

    pub(crate) fn allow(&mut self, final_progress: bool) -> bool {
        self.allow_at(Instant::now(), final_progress)
    }

    fn allow_at(&mut self, now: Instant, final_progress: bool) -> bool {
        if final_progress
            || self
                .last
                .is_none_or(|last| now.duration_since(last) >= Duration::from_millis(100))
        {
            self.last = Some(now);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thousand_small_entries_emit_first_periodic_and_final_progress() {
        let mut gate = ProgressGate::new();
        let start = Instant::now();
        let accepted: Vec<_> = (0..1000)
            .filter(|index| gate.allow_at(start + Duration::from_millis(*index), *index == 999))
            .collect();
        assert_eq!(
            accepted,
            vec![0, 100, 200, 300, 400, 500, 600, 700, 800, 900, 999]
        );
    }

    #[test]
    fn separate_operations_and_final_updates_are_never_stuck_behind_a_previous_entry() {
        let start = Instant::now();
        let mut first = ProgressGate::new();
        let mut second = ProgressGate::new();
        assert!(first.allow_at(start, false));
        assert!(!first.allow_at(start, false));
        assert!(second.allow_at(start, false));
        assert!(first.allow_at(start, true));
        assert!(first.allow_at(start, true));
    }
}

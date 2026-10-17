//! Evidence-based per-entry rescue results. Unknown is deliberately not called
//! missing or damaged when only a read/decryption failure is known.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStatus { Recovered, Missing, ChecksumFailed, Unknown }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEntryResult {
    pub name: Option<String>,
    pub status: RecoveryStatus,
    pub reason: String,
}
impl RecoveryEntryResult {
    pub fn new(name: Option<String>, status: RecoveryStatus, reason: impl Into<String>) -> Self {
        Self { name, status, reason: reason.into() }
    }
}

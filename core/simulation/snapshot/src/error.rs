use property_simulation_month::Month;
use std::fmt;

/// Failure to construct a consistent completed-month Snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    JournalEntryMonthMismatch {
        snapshot_month: Month,
        entry_month: Month,
    },
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JournalEntryMonthMismatch {
                snapshot_month,
                entry_month,
            } => write!(
                f,
                "journal entry month {entry_month} does not match snapshot month {snapshot_month}"
            ),
        }
    }
}

impl std::error::Error for SnapshotError {}

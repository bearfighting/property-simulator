use property_simulation_month::Month;
use property_simulation_slice_catalogue::SliceResult;

use crate::SnapshotError;

/// The completed result for one simulation month.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    month: Month,
    slice_results: Vec<SliceResult>,
}

impl Snapshot {
    /// Create a completed-month record after the caller has settled its Ledger
    /// entries.
    pub fn new(month: Month, slice_results: Vec<SliceResult>) -> Result<Self, SnapshotError> {
        for result in &slice_results {
            for entry in result.journal_entries() {
                if entry.month() != month {
                    return Err(SnapshotError::JournalEntryMonthMismatch {
                        snapshot_month: month,
                        entry_month: entry.month(),
                    });
                }
            }
        }
        Ok(Self {
            month,
            slice_results,
        })
    }
}

impl Snapshot {
    pub const fn month(&self) -> Month {
        self.month
    }

    pub fn slice_results(&self) -> &[SliceResult] {
        &self.slice_results
    }
}

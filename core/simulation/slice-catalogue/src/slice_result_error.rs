use std::fmt;

use crate::{Event, SliceKind};
use property_simulation_month::Month;

/// Failure to match Slice result contents to their declared kind or month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceResultError {
    InitialMonthIsNotCalculated,
    EventKindMismatch {
        slice_kind: SliceKind,
        event: Event,
    },
    JournalEntryMonthMismatch {
        result_month: Month,
        entry_month: Month,
    },
}

impl fmt::Display for SliceResultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitialMonthIsNotCalculated => {
                f.write_str("Month 0 is initialization, not a calculated Slice period")
            }
            Self::EventKindMismatch { slice_kind, event } => {
                write!(
                    f,
                    "event {event:?} does not belong to Slice kind {slice_kind:?}"
                )
            }
            Self::JournalEntryMonthMismatch {
                result_month,
                entry_month,
            } => write!(
                f,
                "journal entry month {entry_month} does not match Slice result month {result_month}"
            ),
        }
    }
}

impl std::error::Error for SliceResultError {}

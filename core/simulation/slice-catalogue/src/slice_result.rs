use property_simulation_journal_entry::JournalEntry;
use property_simulation_month::Month;

use crate::{Event, SliceKind, SliceResultError};

/// Typed output contract for one Slice evaluated for one period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceResult {
    month: Month,
    kind: SliceKind,
    events: Vec<Event>,
    journal_entries: Vec<JournalEntry>,
}

impl SliceResult {
    /// Create one month's result after checking kind and journal-month consistency.
    pub fn new(
        month: Month,
        kind: SliceKind,
        events: Vec<Event>,
        journal_entries: Vec<JournalEntry>,
    ) -> Result<Self, SliceResultError> {
        if month.is_initial() {
            return Err(SliceResultError::InitialMonthIsNotCalculated);
        }

        for event in &events {
            if event.kind() != kind {
                return Err(SliceResultError::EventKindMismatch {
                    slice_kind: kind,
                    event: *event,
                });
            }
        }
        for entry in &journal_entries {
            if entry.month() != month {
                return Err(SliceResultError::JournalEntryMonthMismatch {
                    result_month: month,
                    entry_month: entry.month(),
                });
            }
        }

        Ok(Self {
            month,
            kind,
            events,
            journal_entries,
        })
    }

    /// Create an empty result for a Slice that has no output this period.
    pub fn empty(month: Month, kind: SliceKind) -> Result<Self, SliceResultError> {
        if month.is_initial() {
            return Err(SliceResultError::InitialMonthIsNotCalculated);
        }

        Ok(Self {
            month,
            kind,
            events: Vec::new(),
            journal_entries: Vec::new(),
        })
    }
}

impl SliceResult {
    pub const fn month(&self) -> Month {
        self.month
    }

    pub const fn kind(&self) -> SliceKind {
        self.kind
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn journal_entries(&self) -> &[JournalEntry] {
        &self.journal_entries
    }

    /// Consume this result so Application can collect its outputs.
    pub fn into_parts(self) -> (Month, SliceKind, Vec<Event>, Vec<JournalEntry>) {
        (self.month, self.kind, self.events, self.journal_entries)
    }
}

impl Event {
    const fn kind(self) -> SliceKind {
        match self {
            Self::RentCollected => SliceKind::Rental,
            Self::RepairOccurred => SliceKind::RepairOccurrence,
            Self::RepairCostSettled => SliceKind::RepairCost,
            Self::LoanPayment => SliceKind::Loan,
            Self::FixedExpensePaid => SliceKind::FixedExpense,
        }
    }
}

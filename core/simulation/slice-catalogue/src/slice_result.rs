use property_simulation_journal_entry::JournalEntry;

use crate::{Effect, Event, SliceKind, SliceResultError};

/// Typed output contract for one Slice evaluated for one period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceResult {
    kind: SliceKind,
    events: Vec<Event>,
    effects: Vec<Effect>,
    journal_entries: Vec<JournalEntry>,
}

impl SliceResult {
    /// Create a result after checking that its events and effects belong to its Slice kind.
    pub fn new(
        kind: SliceKind,
        events: Vec<Event>,
        effects: Vec<Effect>,
        journal_entries: Vec<JournalEntry>,
    ) -> Result<Self, SliceResultError> {
        for event in &events {
            if event.kind() != kind {
                return Err(SliceResultError::EventKindMismatch {
                    slice_kind: kind,
                    event: *event,
                });
            }
        }
        for effect in &effects {
            if effect.kind() != kind {
                return Err(SliceResultError::EffectKindMismatch {
                    slice_kind: kind,
                    effect: *effect,
                });
            }
        }

        Ok(Self {
            kind,
            events,
            effects,
            journal_entries,
        })
    }

    /// Create an empty result for a Slice that has no output this period.
    pub fn empty(kind: SliceKind) -> Self {
        Self {
            kind,
            events: Vec::new(),
            effects: Vec::new(),
            journal_entries: Vec::new(),
        }
    }
}

impl SliceResult {
    pub const fn kind(&self) -> SliceKind {
        self.kind
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    pub fn journal_entries(&self) -> &[JournalEntry] {
        &self.journal_entries
    }

    /// Consume the result so Application can combine outputs from several Slices.
    pub fn into_parts(self) -> (SliceKind, Vec<Event>, Vec<Effect>, Vec<JournalEntry>) {
        (self.kind, self.events, self.effects, self.journal_entries)
    }
}

impl Event {
    const fn kind(self) -> SliceKind {
        match self {
            Self::RentCollected { .. } => SliceKind::Rental,
            Self::RepairOccurred => SliceKind::RepairOccurrence,
            Self::RepairCostSettled { .. } => SliceKind::RepairCost,
            Self::LoanPayment { .. } => SliceKind::Loan,
            Self::FixedExpensePaid { .. } => SliceKind::FixedExpense,
        }
    }
}

impl Effect {
    const fn kind(self) -> SliceKind {
        match self {
            Self::LoanRemainingTermUpdated { .. } => SliceKind::Loan,
            Self::RepairCostBaselineUpdated { .. } => SliceKind::RepairCost,
        }
    }
}

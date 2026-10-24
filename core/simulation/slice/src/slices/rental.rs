use std::fmt;

use property_simulation_account::DebitCredit;
use property_simulation_journal_entry::{JournalEntry, JournalEntryError, JournalLine};
use property_simulation_slice_catalogue::{
    Event, RentalParameters, RentalSliceInput, SliceKind, SliceResult, SliceResultError,
};

use crate::DeterministicSlice;

/// Produces the recurring rent receipt for one calculated month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RentalSlice {
    parameters: RentalParameters,
    input: RentalSliceInput,
}

impl RentalSlice {
    /// Construct a one-period rental behavior from settings and resolved inputs.
    pub const fn new(parameters: RentalParameters, input: RentalSliceInput) -> Self {
        Self { parameters, input }
    }
}

impl DeterministicSlice for RentalSlice {
    type Error = RentalSliceError;

    fn calculate(&self) -> Result<SliceResult, Self::Error> {
        (|| -> Result<SliceResult, RentalSliceError> {
            let amount = self
                .parameters
                .monthly_rent()
                .rounded_for_posting()
                .map_err(JournalEntryError::from)?;
            if amount.is_zero() {
                return Ok(SliceResult::empty(SliceKind::Rental));
            }

            let cash_line = JournalLine::new(self.input.cash_account, DebitCredit::Debit, amount)?;
            let rental_income_line = JournalLine::new(
                self.input.rental_income_account,
                DebitCredit::Credit,
                amount,
            )?;
            let entry = JournalEntry::new(self.input.month, vec![cash_line, rental_income_line])?;
            let event = Event::RentCollected { amount };

            Ok(SliceResult::new(
                SliceKind::Rental,
                vec![event],
                Vec::new(),
                vec![entry],
            )?)
        })()
    }
}

/// Failure while evaluating rental output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RentalSliceError {
    JournalEntry(JournalEntryError),
    SliceResult(SliceResultError),
}

impl From<JournalEntryError> for RentalSliceError {
    fn from(error: JournalEntryError) -> Self {
        Self::JournalEntry(error)
    }
}

impl From<SliceResultError> for RentalSliceError {
    fn from(error: SliceResultError) -> Self {
        Self::SliceResult(error)
    }
}

impl fmt::Display for RentalSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JournalEntry(error) => write!(f, "rental journal entry failed: {error}"),
            Self::SliceResult(error) => write!(f, "invalid rental Slice result: {error}"),
        }
    }
}

impl std::error::Error for RentalSliceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::JournalEntry(error) => Some(error),
            Self::SliceResult(error) => Some(error),
        }
    }
}

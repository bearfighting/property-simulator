use std::fmt;

use property_simulation_random_engine::RandomInput;
use property_simulation_slice_catalogue::SliceResultError;
use property_simulation_slice_catalogue::{
    Event, RepairOccurrenceSliceInput, SliceKind, SliceResult,
};

use crate::StochasticSlice;

/// Determines whether a repair occurs in one simulation month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairOccurrenceSlice {
    input: RepairOccurrenceSliceInput,
}

impl RepairOccurrenceSlice {
    pub const fn new(input: RepairOccurrenceSliceInput) -> Self {
        Self { input }
    }
}

impl StochasticSlice for RepairOccurrenceSlice {
    type Error = RepairOccurrenceSliceError;

    fn simulate(
        &self,
        random_input: &mut dyn RandomInput,
    ) -> Result<SliceResult, RepairOccurrenceSliceError> {
        (|| -> Result<SliceResult, RepairOccurrenceSliceError> {
            let threshold = occurrence_threshold(self.input.months_since_repair_cost);
            let sample = sample_below(random_input, 10_000);
            if sample >= threshold {
                return Ok(SliceResult::empty(
                    self.input.month,
                    SliceKind::RepairOccurrence,
                )?);
            }

            Ok(SliceResult::new(
                self.input.month,
                SliceKind::RepairOccurrence,
                vec![Event::RepairOccurred],
                Vec::new(),
            )?)
        })()
    }
}

const fn occurrence_threshold(months_since_repair_cost: u64) -> u64 {
    let age = months_since_repair_cost as u128;
    (25 + (125 * age) / (age + 60)) as u64
}

fn sample_below(random_input: &mut dyn RandomInput, upper_bound: u64) -> u64 {
    let rejection_threshold = upper_bound.wrapping_neg() % upper_bound;
    loop {
        let sample = random_input.next_u64();
        if sample >= rejection_threshold {
            return sample % upper_bound;
        }
    }
}

/// Failure while evaluating repair occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairOccurrenceSliceError {
    SliceResult(SliceResultError),
}

impl From<SliceResultError> for RepairOccurrenceSliceError {
    fn from(error: SliceResultError) -> Self {
        Self::SliceResult(error)
    }
}

impl fmt::Display for RepairOccurrenceSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SliceResult(error) => {
                write!(f, "invalid repair occurrence Slice result: {error}")
            }
        }
    }
}

impl std::error::Error for RepairOccurrenceSliceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SliceResult(error) => Some(error),
        }
    }
}

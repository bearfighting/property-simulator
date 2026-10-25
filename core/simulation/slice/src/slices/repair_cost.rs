use std::{fmt, num::NonZeroU32};

use property_simulation_account::DebitCredit;
use property_simulation_journal_entry::{JournalEntry, JournalEntryError, JournalLine};
use property_simulation_money::{Money, MoneyError};
use property_simulation_month::{Month, MonthError};
use property_simulation_random_engine::RandomInput;
use property_simulation_slice_catalogue::{
    Event, RepairCostParameters, RepairCostSliceInput, SliceKind, SliceResult, SliceResultError,
};

use crate::StochasticSlice;

/// Samples and posts one generated repair cost in its configured effective month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairCostSlice {
    parameters: RepairCostParameters,
    input: RepairCostSliceInput,
}

impl RepairCostSlice {
    pub const fn new(parameters: RepairCostParameters, input: RepairCostSliceInput) -> Self {
        Self { parameters, input }
    }
}

impl StochasticSlice for RepairCostSlice {
    type Error = RepairCostSliceError;

    fn simulate(
        &self,
        random_input: &mut dyn RandomInput,
    ) -> Result<SliceResult, RepairCostSliceError> {
        (|| -> Result<SliceResult, RepairCostSliceError> {
            validate_effective_month(
                self.input.occurrence_month,
                self.parameters.delay_months(),
                self.input.month,
            )?;

            let maximum_minor_units = maximum_minor_units(self.parameters.maximum_amount())?;
            let sampled_minor_units = if maximum_minor_units == u64::MAX {
                random_input.next_u64()
            } else {
                sample_below(random_input, maximum_minor_units + 1)
            };
            let amount = Money::from_parts(
                i128::from(sampled_minor_units),
                self.parameters
                    .maximum_amount()
                    .currency()
                    .minor_unit_digits() as u8,
                self.parameters.maximum_amount().currency(),
            )?;

            let mut journal_entries = Vec::new();
            if !amount.is_zero() {
                let expense_line =
                    JournalLine::new(self.input.expense_account, DebitCredit::Debit, amount)?;
                let cash_line =
                    JournalLine::new(self.input.cash_account, DebitCredit::Credit, amount)?;
                journal_entries.push(JournalEntry::new(
                    self.input.month,
                    vec![expense_line, cash_line],
                )?);
            }

            Ok(SliceResult::new(
                self.input.month,
                SliceKind::RepairCost,
                vec![Event::RepairCostSettled],
                journal_entries,
            )?)
        })()
    }
}

fn validate_effective_month(
    occurrence_month: Month,
    delay_months: NonZeroU32,
    effective_month: Month,
) -> Result<(), RepairCostSliceError> {
    let expected_month = occurrence_month
        .checked_add(
            u16::try_from(delay_months.get())
                .map_err(|_| RepairCostSliceError::Month(MonthError::OutOfRange))?,
        )
        .map_err(RepairCostSliceError::Month)?;
    if effective_month != expected_month {
        return Err(RepairCostSliceError::InvalidEffectiveMonth {
            expected: expected_month,
            actual: effective_month,
        });
    }
    Ok(())
}

fn maximum_minor_units(maximum: Money) -> Result<u64, RepairCostSliceError> {
    if maximum.is_negative() {
        return Err(RepairCostSliceError::NegativeMaximumAmount);
    }

    let minor_unit_digits = maximum.currency().minor_unit_digits() as u8;
    let scaled = if maximum.scale() <= minor_unit_digits {
        maximum
            .coefficient()
            .checked_mul(
                10_i128
                    .checked_pow(u32::from(minor_unit_digits - maximum.scale()))
                    .ok_or(RepairCostSliceError::CalculationOverflow)?,
            )
            .ok_or(RepairCostSliceError::CalculationOverflow)?
    } else {
        maximum.coefficient()
            / 10_i128
                .checked_pow(u32::from(maximum.scale() - minor_unit_digits))
                .ok_or(RepairCostSliceError::CalculationOverflow)?
    };

    u64::try_from(scaled).map_err(|_| RepairCostSliceError::MaximumAmountOutOfRange)
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

/// Failure while validating or evaluating a repair cost instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairCostSliceError {
    NegativeMaximumAmount,
    MaximumAmountOutOfRange,
    CalculationOverflow,
    Month(MonthError),
    InvalidEffectiveMonth { expected: Month, actual: Month },
    Money(MoneyError),
    JournalEntry(JournalEntryError),
    SliceResult(SliceResultError),
}

impl From<MoneyError> for RepairCostSliceError {
    fn from(error: MoneyError) -> Self {
        Self::Money(error)
    }
}

impl From<JournalEntryError> for RepairCostSliceError {
    fn from(error: JournalEntryError) -> Self {
        Self::JournalEntry(error)
    }
}

impl From<SliceResultError> for RepairCostSliceError {
    fn from(error: SliceResultError) -> Self {
        Self::SliceResult(error)
    }
}

impl fmt::Display for RepairCostSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeMaximumAmount => f.write_str("maximum repair cost cannot be negative"),
            Self::MaximumAmountOutOfRange => {
                f.write_str("maximum repair cost exceeds the supported sampling range")
            }
            Self::CalculationOverflow => f.write_str("repair cost calculation overflowed"),
            Self::Month(error) => write!(f, "invalid repair cost month: {error}"),
            Self::InvalidEffectiveMonth { expected, actual } => write!(
                f,
                "repair cost is due in month {expected}, not month {actual}"
            ),
            Self::Money(error) => write!(f, "invalid repair cost amount: {error}"),
            Self::JournalEntry(error) => write!(f, "invalid repair cost journal entry: {error}"),
            Self::SliceResult(error) => write!(f, "invalid repair cost Slice result: {error}"),
        }
    }
}

impl std::error::Error for RepairCostSliceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Month(error) => Some(error),
            Self::Money(error) => Some(error),
            Self::JournalEntry(error) => Some(error),
            Self::SliceResult(error) => Some(error),
            Self::NegativeMaximumAmount
            | Self::MaximumAmountOutOfRange
            | Self::CalculationOverflow
            | Self::InvalidEffectiveMonth { .. } => None,
        }
    }
}

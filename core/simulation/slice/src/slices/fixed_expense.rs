use std::fmt;

use property_simulation_account::DebitCredit;
use property_simulation_journal_entry::{JournalEntry, JournalEntryError, JournalLine};
use property_simulation_money::{Money, MoneyError};
use property_simulation_slice_catalogue::{
    Event, FixedExpenseMethod, FixedExpenseParameters, FixedExpenseSliceInput, SliceKind,
    SliceResult, SliceResultError,
};
use rust_decimal::Decimal;

use crate::DeterministicSlice;

/// Calculates and posts one month's recurring fixed expense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedExpenseSlice {
    parameters: FixedExpenseParameters,
    input: FixedExpenseSliceInput,
}

impl FixedExpenseSlice {
    /// Construct one month's fixed-expense calculation from settings and resolved inputs.
    pub const fn new(parameters: FixedExpenseParameters, input: FixedExpenseSliceInput) -> Self {
        Self { parameters, input }
    }
}

impl DeterministicSlice for FixedExpenseSlice {
    type Error = FixedExpenseSliceError;

    fn calculate(&self) -> Result<SliceResult, Self::Error> {
        (|| -> Result<SliceResult, FixedExpenseSliceError> {
            let monthly_amount = match self.parameters.method() {
                FixedExpenseMethod::MonthlyAmount(amount) => *amount,
                FixedExpenseMethod::AnnualPropertyValueRate(annual_rate) => {
                    if self.input.property_value.is_negative() {
                        return Err(FixedExpenseSliceError::NegativePropertyValue);
                    }
                    calculate_monthly_property_expense(self.input.property_value, *annual_rate)?
                }
            }
            .rounded_for_posting()?;

            if monthly_amount.is_zero() {
                return Ok(SliceResult::empty(SliceKind::FixedExpense));
            }

            let expense_line = JournalLine::new(
                self.input.expense_account,
                DebitCredit::Debit,
                monthly_amount,
            )?;
            let cash_line =
                JournalLine::new(self.input.cash_account, DebitCredit::Credit, monthly_amount)?;
            let entry = JournalEntry::new(self.input.month, vec![expense_line, cash_line])?;
            let event = Event::FixedExpensePaid {
                amount: monthly_amount,
            };

            Ok(SliceResult::new(
                SliceKind::FixedExpense,
                vec![event],
                Vec::new(),
                vec![entry],
            )?)
        })()
    }
}

fn calculate_monthly_property_expense(
    property_value: Money,
    annual_rate: Decimal,
) -> Result<Money, FixedExpenseSliceError> {
    let currency = property_value.currency();
    let property_value = Decimal::try_from_i128_with_scale(
        property_value.coefficient(),
        u32::from(property_value.scale()),
    )
    .map_err(|_| FixedExpenseSliceError::CalculationOverflow)?;
    let monthly_rate = annual_rate
        .checked_div(Decimal::from(12))
        .ok_or(FixedExpenseSliceError::CalculationOverflow)?;
    let amount = property_value
        .checked_mul(monthly_rate)
        .ok_or(FixedExpenseSliceError::CalculationOverflow)?;

    Ok(Money::new(amount, currency))
}

/// Failure while evaluating a fixed expense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedExpenseSliceError {
    NegativePropertyValue,
    CalculationOverflow,
    Money(MoneyError),
    JournalEntry(JournalEntryError),
    SliceResult(SliceResultError),
}

impl From<MoneyError> for FixedExpenseSliceError {
    fn from(error: MoneyError) -> Self {
        Self::Money(error)
    }
}

impl From<JournalEntryError> for FixedExpenseSliceError {
    fn from(error: JournalEntryError) -> Self {
        Self::JournalEntry(error)
    }
}

impl From<SliceResultError> for FixedExpenseSliceError {
    fn from(error: SliceResultError) -> Self {
        Self::SliceResult(error)
    }
}

impl fmt::Display for FixedExpenseSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativePropertyValue => f.write_str("property value cannot be negative"),
            Self::CalculationOverflow => f.write_str("fixed expense calculation overflowed"),
            Self::Money(error) => write!(f, "invalid fixed expense amount: {error}"),
            Self::JournalEntry(error) => write!(f, "invalid fixed expense journal entry: {error}"),
            Self::SliceResult(error) => write!(f, "invalid fixed expense Slice result: {error}"),
        }
    }
}

impl std::error::Error for FixedExpenseSliceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Money(error) => Some(error),
            Self::JournalEntry(error) => Some(error),
            Self::SliceResult(error) => Some(error),
            Self::NegativePropertyValue | Self::CalculationOverflow => None,
        }
    }
}

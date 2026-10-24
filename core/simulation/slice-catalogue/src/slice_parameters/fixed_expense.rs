use std::fmt;

use property_simulation_money::Money;
use rust_decimal::Decimal;

/// The mutually exclusive calculation methods for one recurring fixed expense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedExpenseMethod {
    /// A fixed amount charged each month.
    MonthlyAmount(Money),
    /// An annual rate applied to the simulation's unchanged total property value.
    /// The rate is divided by twelve to calculate the monthly charge.
    AnnualPropertyValueRate(Decimal),
}

/// Settings for the recurring FixedExpense Slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedExpenseParameters {
    method: FixedExpenseMethod,
}

impl FixedExpenseParameters {
    pub fn new(method: FixedExpenseMethod) -> Result<Self, FixedExpenseParametersError> {
        match &method {
            FixedExpenseMethod::MonthlyAmount(amount) if amount.is_negative() => {
                Err(FixedExpenseParametersError::NegativeMonthlyAmount)
            }
            FixedExpenseMethod::AnnualPropertyValueRate(rate) if *rate < Decimal::ZERO => {
                Err(FixedExpenseParametersError::NegativeAnnualRate)
            }
            _ => Ok(Self { method }),
        }
    }

    pub const fn method(&self) -> &FixedExpenseMethod {
        &self.method
    }
}

/// Invalid recurring fixed expense settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedExpenseParametersError {
    NegativeMonthlyAmount,
    NegativeAnnualRate,
}

impl fmt::Display for FixedExpenseParametersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeMonthlyAmount => f.write_str("monthly expense cannot be negative"),
            Self::NegativeAnnualRate => f.write_str("annual expense rate cannot be negative"),
        }
    }
}

impl std::error::Error for FixedExpenseParametersError {}

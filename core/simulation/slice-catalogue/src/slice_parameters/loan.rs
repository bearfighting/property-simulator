use std::{fmt, num::NonZeroU32};

use rust_decimal::Decimal;

/// Settings required to calculate loan activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanParameters {
    annual_interest_rate: Decimal,
    /// Remaining payment term at the simulation start, used for scheduled payment.
    remaining_term_months_at_start: NonZeroU32,
}

impl LoanParameters {
    /// Create fixed-rate amortizing loan terms. The annual rate is a decimal
    /// ratio, so `0.05` represents five percent.
    pub fn new(
        annual_interest_rate: Decimal,
        remaining_term_months_at_start: NonZeroU32,
    ) -> Result<Self, LoanParametersError> {
        if annual_interest_rate < Decimal::ZERO {
            return Err(LoanParametersError::NegativeAnnualInterestRate);
        }
        if u16::try_from(remaining_term_months_at_start.get()).is_err() {
            return Err(LoanParametersError::TermOutOfRange);
        }

        Ok(Self {
            annual_interest_rate,
            remaining_term_months_at_start,
        })
    }
}

impl LoanParameters {
    pub const fn annual_interest_rate(&self) -> Decimal {
        self.annual_interest_rate
    }

    pub const fn remaining_term_months_at_start(&self) -> NonZeroU32 {
        self.remaining_term_months_at_start
    }
}

/// Failure to construct valid loan parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoanParametersError {
    NegativeAnnualInterestRate,
    TermOutOfRange,
}

impl fmt::Display for LoanParametersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeAnnualInterestRate => {
                f.write_str("annual interest rate cannot be negative")
            }
            Self::TermOutOfRange => f.write_str("loan term exceeds the supported month range"),
        }
    }
}

impl std::error::Error for LoanParametersError {}

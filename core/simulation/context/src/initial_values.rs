use property_simulation_money::{Currency, Money};
use property_simulation_slice_catalogue::LoanParameters;

/// User-provided opening facts for the simulation.
///
/// **Incomplete:** this captures currency, property value, loan balance, and
/// initial loan terms, but not every opening fact or the final persistence
/// mapping. Consumers must not assume that all opening Ledger accounts can be
/// created from this value. Complete it when the opening account set and
/// initialization workflow are finalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialValues {
    currency: Currency,
    property_value: Money,
    loan_balance: Money,
    loan_parameters: LoanParameters,
}

impl InitialValues {
    /// Create opening values with one explicit simulation currency.
    pub fn new(
        currency: Currency,
        property_value: Money,
        loan_balance: Money,
        loan_parameters: LoanParameters,
    ) -> Result<Self, InitialValuesError> {
        if property_value.currency() != currency || loan_balance.currency() != currency {
            return Err(InitialValuesError::CurrencyMismatch);
        }
        if property_value.is_negative() {
            return Err(InitialValuesError::NegativePropertyValue);
        }
        if loan_balance.is_negative() {
            return Err(InitialValuesError::NegativeLoanBalance);
        }

        Ok(Self {
            currency,
            property_value,
            loan_balance,
            loan_parameters,
        })
    }
}

impl InitialValues {
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    pub const fn property_value(&self) -> Money {
        self.property_value
    }

    pub const fn loan_balance(&self) -> Money {
        self.loan_balance
    }

    pub const fn loan_parameters(&self) -> &LoanParameters {
        &self.loan_parameters
    }

    /// Derive opening equity as property value less opening loan balance.
    pub fn opening_equity(&self) -> Result<Money, property_simulation_money::MoneyError> {
        self.property_value.checked_sub(self.loan_balance)
    }
}

/// Failure to construct consistent opening values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialValuesError {
    CurrencyMismatch,
    NegativePropertyValue,
    NegativeLoanBalance,
}

impl std::fmt::Display for InitialValuesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrencyMismatch => {
                f.write_str("opening values must use the simulation currency")
            }
            Self::NegativePropertyValue => f.write_str("property value cannot be negative"),
            Self::NegativeLoanBalance => f.write_str("loan balance cannot be negative"),
        }
    }
}

impl std::error::Error for InitialValuesError {}

use property_simulation_account::AccountCode;
use property_simulation_money::{Currency, Money, MoneyError};
use property_simulation_month::Month;
use std::fmt;

/// Failure to initialize or append facts to a ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    InvalidOpeningBalance,
    DuplicateOpeningBalance(AccountCode),
    UnknownAccount(AccountCode),
    CurrencyMismatch {
        expected: Currency,
        actual: Currency,
    },
    UnbalancedOpeningBalances {
        debits: Money,
        credits: Money,
    },
    MonthRegression {
        previous: Month,
        attempted: Month,
    },
    Money(MoneyError),
}

impl From<MoneyError> for LedgerError {
    fn from(error: MoneyError) -> Self {
        Self::Money(error)
    }
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOpeningBalance => f.write_str("opening balance amount must be positive"),
            Self::DuplicateOpeningBalance(account) => {
                write!(f, "duplicate opening balance for account {account}")
            }
            Self::UnknownAccount(account) => write!(f, "unknown ledger account {account}"),
            Self::CurrencyMismatch { expected, actual } => {
                write!(f, "ledger currency is {expected}, received {actual}")
            }
            Self::UnbalancedOpeningBalances { debits, credits } => write!(
                f,
                "opening balances are unbalanced: debits {debits}, credits {credits}"
            ),
            Self::MonthRegression {
                previous,
                attempted,
            } => write!(
                f,
                "journal month {attempted} precedes the latest journal month {previous}"
            ),
            Self::Money(error) => write!(f, "ledger money arithmetic failed: {error}"),
        }
    }
}

impl std::error::Error for LedgerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Money(error) => Some(error),
            _ => None,
        }
    }
}

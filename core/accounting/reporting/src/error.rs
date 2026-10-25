use property_simulation_account::AccountCode;
use property_simulation_money::{Currency, Money, MoneyError};
use property_simulation_month::Month;
use std::fmt;

/// Failure while deriving a report from supplied ledger facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportingError {
    UnknownAccount(AccountCode),
    MissingCashFlowClassification(AccountCode),
    DuplicateCashFlowClassification(AccountCode),
    CashFlowClassificationMismatch {
        journal_position: usize,
        cash_amount: Money,
        classified_amount: Money,
    },
    CurrencyMismatch {
        expected: Currency,
        actual: Currency,
    },
    UnbalancedJournalEntry {
        month: Month,
        debits: Money,
        credits: Money,
    },
    UnbalancedTrialBalance {
        debits: Money,
        credits: Money,
    },
    UnbalancedBalanceSheet {
        assets: Money,
        liabilities_and_equity: Money,
    },
    MonthRegression {
        previous: Month,
        attempted: Month,
    },
    InvalidPeriod {
        start: Month,
        end: Month,
    },
    Money(MoneyError),
}

impl From<MoneyError> for ReportingError {
    fn from(error: MoneyError) -> Self {
        Self::Money(error)
    }
}

impl fmt::Display for ReportingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAccount(account) => write!(f, "unknown reporting account {account}"),
            Self::MissingCashFlowClassification(account) => write!(
                f,
                "cash-flow classification is missing for counterpart account {account}"
            ),
            Self::DuplicateCashFlowClassification(account) => write!(
                f,
                "cash-flow classification is duplicated for account {account}"
            ),
            Self::CashFlowClassificationMismatch {
                journal_position,
                cash_amount,
                classified_amount,
            } => write!(
                f,
                "cash-flow classifications for journal entry {journal_position} total {classified_amount}, but cash lines total {cash_amount}"
            ),
            Self::CurrencyMismatch { expected, actual } => {
                write!(f, "report currency is {expected}, received {actual}")
            }
            Self::UnbalancedJournalEntry {
                month,
                debits,
                credits,
            } => write!(
                f,
                "journal entry in month {month} is unbalanced: debits {debits}, credits {credits}"
            ),
            Self::UnbalancedTrialBalance { debits, credits } => write!(
                f,
                "trial balance is unbalanced: debits {debits}, credits {credits}"
            ),
            Self::UnbalancedBalanceSheet {
                assets,
                liabilities_and_equity,
            } => write!(
                f,
                "balance sheet is unbalanced: assets {assets}, liabilities and equity {liabilities_and_equity}"
            ),
            Self::MonthRegression {
                previous,
                attempted,
            } => write!(
                f,
                "journal month {attempted} precedes the latest journal month {previous}"
            ),
            Self::InvalidPeriod { start, end } => {
                write!(
                    f,
                    "report period {start}..={end} must start at Month 1 or later and be ordered"
                )
            }
            Self::Money(error) => write!(f, "report money arithmetic failed: {error}"),
        }
    }
}

impl std::error::Error for ReportingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Money(error) => Some(error),
            _ => None,
        }
    }
}

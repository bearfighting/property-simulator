use property_simulation_money::{Money, MoneyError};
use std::fmt;

/// Failure to construct a valid, balanced journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalEntryError {
    TooFewLines,
    NonPositiveAmount,
    InvalidNoteCode,
    Unbalanced { debits: Money, credits: Money },
    Money(MoneyError),
}

impl From<MoneyError> for JournalEntryError {
    fn from(error: MoneyError) -> Self {
        Self::Money(error)
    }
}

impl fmt::Display for JournalEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewLines => f.write_str("journal entry must contain at least two lines"),
            Self::NonPositiveAmount => f.write_str("journal line amount must be positive"),
            Self::InvalidNoteCode => f.write_str("journal entry note code is invalid"),
            Self::Unbalanced { debits, credits } => {
                write!(
                    f,
                    "journal entry is unbalanced: debits {debits}, credits {credits}"
                )
            }
            Self::Money(error) => write!(f, "invalid journal entry amount: {error}"),
        }
    }
}

impl std::error::Error for JournalEntryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Money(error) => Some(error),
            _ => None,
        }
    }
}

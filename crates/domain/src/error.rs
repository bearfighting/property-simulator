use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidYearMonth,
    MonthOutOfRange,
    InvalidCurrency(String),
    CurrencyMismatch,
    ArithmeticOverflow,
    NegativePostingAmount,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidYearMonth => {
                write!(f, "invalid year-month; expected a month in years 0001–9999")
            }
            Self::MonthOutOfRange => write!(f, "year-month arithmetic exceeded years 0001–9999"),
            Self::InvalidCurrency(code) => write!(f, "invalid ISO 4217 currency code: {code}"),
            Self::CurrencyMismatch => write!(f, "currency mismatch"),
            Self::ArithmeticOverflow => write!(f, "decimal arithmetic overflow"),
            Self::NegativePostingAmount => write!(f, "posting amount cannot be negative"),
        }
    }
}

impl std::error::Error for DomainError {}

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidYearMonth,
    MonthOutOfRange,
    InvalidCurrency(String),
    CurrencyMismatch,
    ArithmeticOverflow,
    InvalidMoneyAmount,
    InvalidMoneyScale(u8),
    MoneyOutOfRange,
    NegativePostingAmount,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidYearMonth => {
                write!(
                    f,
                    "invalid year-month; expected a positive year and month 01–12"
                )
            }
            Self::MonthOutOfRange => {
                write!(f, "year-month arithmetic exceeded the supported year type")
            }
            Self::InvalidCurrency(code) => write!(f, "invalid ISO 4217 currency code: {code}"),
            Self::CurrencyMismatch => write!(f, "currency mismatch"),
            Self::ArithmeticOverflow => write!(f, "decimal arithmetic overflow"),
            Self::InvalidMoneyAmount => write!(f, "invalid decimal money amount"),
            Self::InvalidMoneyScale(scale) => {
                write!(f, "money scale {scale} exceeds the supported precision")
            }
            Self::MoneyOutOfRange => write!(
                f,
                "money amount is outside the supported i128 coefficient range"
            ),
            Self::NegativePostingAmount => write!(f, "posting amount cannot be negative"),
        }
    }
}

impl std::error::Error for DomainError {}

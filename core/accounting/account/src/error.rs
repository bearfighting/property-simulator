use std::fmt;

use crate::AccountCode;

/// Invalid account definition or catalogue input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountError {
    InvalidCode,
    EmptyName,
    EmptyRole,
    DuplicateCode(AccountCode),
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCode => f.write_str("account code must be positive"),
            Self::EmptyName => f.write_str("account name cannot be empty"),
            Self::EmptyRole => f.write_str("account role cannot be empty"),
            Self::DuplicateCode(code) => write!(f, "duplicate account code: {code}"),
        }
    }
}

impl std::error::Error for AccountError {}

use std::fmt;

use crate::{AccountCode, AccountRole};

/// Invalid account definition or catalogue input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountError {
    InvalidCode,
    EmptyName,
    EmptyRole,
    DuplicateCode(AccountCode),
    DuplicateRole(AccountRole),
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCode => f.write_str("account code must be positive"),
            Self::EmptyName => f.write_str("account name cannot be empty"),
            Self::EmptyRole => f.write_str("account role cannot be empty"),
            Self::DuplicateCode(code) => write!(f, "duplicate account code: {code}"),
            Self::DuplicateRole(role) => {
                write!(f, "duplicate account role: {}", role.as_str())
            }
        }
    }
}

impl std::error::Error for AccountError {}

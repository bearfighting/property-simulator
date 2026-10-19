use std::fmt;

use property_simulation_domain::AccountId;

use crate::{AccountCategory, AccountCode};

/// Validation failures for account definitions and account tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountError {
    InvalidCode(u16),
    CodeCategoryMismatch {
        code: AccountCode,
        category: AccountCategory,
    },
    ContraNotAllowed(AccountCategory),
    EmptyName,
    DuplicateId(AccountId),
    DuplicateCode(AccountCode),
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCode(code) => write!(f, "account code {code} is outside 1000–5999"),
            Self::CodeCategoryMismatch { code, category } => {
                write!(f, "account code {code} does not belong to {category:?}")
            }
            Self::ContraNotAllowed(category) => {
                write!(f, "contra {category:?} accounts are not allowed")
            }
            Self::EmptyName => f.write_str("account name must not be empty"),
            Self::DuplicateId(id) => write!(f, "duplicate account ID {id:?}"),
            Self::DuplicateCode(code) => write!(f, "duplicate account code {code}"),
        }
    }
}

impl std::error::Error for AccountError {}

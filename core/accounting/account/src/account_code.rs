use std::fmt;

use crate::AccountError;

/// Numeric account reference, unique within an account catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountCode(u32);

impl AccountCode {
    /// Create a positive numeric account code.
    pub const fn new(value: u32) -> Result<Self, AccountError> {
        if value == 0 {
            Err(AccountError::InvalidCode)
        } else {
            Ok(Self(value))
        }
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for AccountCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

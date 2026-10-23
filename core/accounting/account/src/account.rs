use crate::{AccountClass, AccountCode, AccountError, AccountRole};

/// One postable account in the simulation's account set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    code: AccountCode,
    name: String,
    class: AccountClass,
    role: Option<AccountRole>,
}

impl Account {
    /// Create an account with a valid code, non-empty name, and accounting class.
    pub fn new(
        code: AccountCode,
        name: impl Into<String>,
        class: AccountClass,
        role: Option<AccountRole>,
    ) -> Result<Self, AccountError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(AccountError::EmptyName);
        }

        Ok(Self {
            code,
            name,
            class,
            role,
        })
    }

    pub const fn code(&self) -> AccountCode {
        self.code
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn class(&self) -> AccountClass {
        self.class
    }

    pub fn role(&self) -> Option<&AccountRole> {
        self.role.as_ref()
    }
}

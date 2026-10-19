use property_simulation_domain::AccountId;

use crate::{AccountCode, AccountError};

/// The statement class of an account. The code is descriptive; this type
/// determines normal balance behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountCategory {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

/// The side on which an account normally carries a balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BalanceSide {
    Debit,
    Credit,
}

impl AccountCategory {
    /// Returns the normal balance side, reversing it for a contra account.
    pub const fn normal_balance(self, contra: bool) -> Option<BalanceSide> {
        let normal = match self {
            Self::Asset | Self::Expense => BalanceSide::Debit,
            Self::Liability | Self::Equity | Self::Income => BalanceSide::Credit,
        };
        if contra && !matches!(self, Self::Asset | Self::Expense) {
            None
        } else if contra {
            Some(match normal {
                BalanceSide::Debit => BalanceSide::Credit,
                BalanceSide::Credit => BalanceSide::Debit,
            })
        } else {
            Some(normal)
        }
    }

    pub(super) const fn code_range(self) -> (u16, u16) {
        match self {
            Self::Asset => (1000, 1999),
            Self::Liability => (2000, 2999),
            Self::Equity => (3000, 3999),
            Self::Income => (4000, 4999),
            Self::Expense => (5000, 5999),
        }
    }
}

/// An account definition. IDs are supplied by the caller and codes remain
/// separate from account identity. Cash-position behavior is explicit metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    code: AccountCode,
    category: AccountCategory,
    contra: bool,
    cash_position: bool,
    name: String,
}

impl Account {
    pub fn new(
        id: AccountId,
        code: AccountCode,
        category: AccountCategory,
        contra: bool,
        name: impl Into<String>,
    ) -> Result<Self, AccountError> {
        code.validate_category(category)?;
        category
            .normal_balance(contra)
            .ok_or(AccountError::ContraNotAllowed(category))?;
        let name = name.into();
        if name.trim().is_empty() {
            return Err(AccountError::EmptyName);
        }
        Ok(Self {
            id,
            code,
            category,
            contra,
            cash_position: false,
            name,
        })
    }

    /// Designates this asset as an in-scope signed property cash position.
    pub fn new_cash_position(
        id: AccountId,
        code: AccountCode,
        name: impl Into<String>,
    ) -> Result<Self, AccountError> {
        let mut account = Self::new(id, code, AccountCategory::Asset, false, name)?;
        account.cash_position = true;
        Ok(account)
    }

    pub const fn id(&self) -> AccountId {
        self.id
    }

    pub const fn code(&self) -> AccountCode {
        self.code
    }

    pub const fn category(&self) -> AccountCategory {
        self.category
    }

    pub const fn is_contra(&self) -> bool {
        self.contra
    }

    pub const fn is_cash_position(&self) -> bool {
        self.cash_position
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn normal_balance(&self) -> BalanceSide {
        // Account construction guarantees this combination is valid.
        match self.category.normal_balance(self.contra) {
            Some(side) => side,
            None => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_balance_is_derived_from_category_and_contra_status() {
        assert_eq!(
            AccountCategory::Asset.normal_balance(false),
            Some(BalanceSide::Debit)
        );
        assert_eq!(
            AccountCategory::Liability.normal_balance(false),
            Some(BalanceSide::Credit)
        );
        assert_eq!(
            AccountCategory::Equity.normal_balance(false),
            Some(BalanceSide::Credit)
        );
        assert_eq!(
            AccountCategory::Income.normal_balance(false),
            Some(BalanceSide::Credit)
        );
        assert_eq!(
            AccountCategory::Expense.normal_balance(false),
            Some(BalanceSide::Debit)
        );
        assert_eq!(
            AccountCategory::Asset.normal_balance(true),
            Some(BalanceSide::Credit)
        );
        assert_eq!(
            AccountCategory::Expense.normal_balance(true),
            Some(BalanceSide::Credit)
        );
        for category in [
            AccountCategory::Liability,
            AccountCategory::Equity,
            AccountCategory::Income,
        ] {
            assert_eq!(category.normal_balance(true), None);
        }
    }

    #[test]
    fn rejects_disallowed_contra_accounts_and_empty_names() {
        let id = AccountId::from_bytes([1; 16]);
        assert!(matches!(
            Account::new(
                id,
                AccountCode::new(2001).unwrap(),
                AccountCategory::Liability,
                true,
                "bad"
            ),
            Err(AccountError::ContraNotAllowed(AccountCategory::Liability))
        ));
        assert!(matches!(
            Account::new(
                id,
                AccountCode::new(1000).unwrap(),
                AccountCategory::Asset,
                false,
                " "
            ),
            Err(AccountError::EmptyName)
        ));
    }
}

use std::collections::{BTreeMap, BTreeSet};

use property_simulation_domain::AccountId;

use crate::{Account, AccountCode, AccountError};

/// A validated account table with unique IDs and codes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChartOfAccounts {
    by_id: BTreeMap<AccountId, Account>,
    codes: BTreeSet<AccountCode>,
}

impl ChartOfAccounts {
    pub fn new(accounts: impl IntoIterator<Item = Account>) -> Result<Self, AccountError> {
        let mut chart = Self::default();
        for account in accounts {
            chart.insert(account)?;
        }
        Ok(chart)
    }

    pub fn insert(&mut self, account: Account) -> Result<(), AccountError> {
        if self.by_id.contains_key(&account.id()) {
            return Err(AccountError::DuplicateId(account.id()));
        }
        if self.codes.contains(&account.code()) {
            return Err(AccountError::DuplicateCode(account.code()));
        }
        self.codes.insert(account.code());
        self.by_id.insert(account.id(), account);
        Ok(())
    }

    pub fn get(&self, id: AccountId) -> Option<&Account> {
        self.by_id.get(&id)
    }

    /// Accounts in numeric code order, suitable for chart presentation.
    pub fn accounts_by_code(&self) -> impl Iterator<Item = &Account> {
        let mut accounts: Vec<_> = self.by_id.values().collect();
        accounts.sort_by_key(|account| account.code());
        accounts.into_iter()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccountCategory;

    fn account(n: u8, code: u16, category: AccountCategory) -> Account {
        let mut bytes = [0; 16];
        bytes[15] = n;
        Account::new(
            AccountId::from_bytes(bytes),
            AccountCode::new(code).unwrap(),
            category,
            false,
            format!("Account {n}"),
        )
        .unwrap()
    }

    #[test]
    fn enforces_unique_ids_and_codes_and_returns_numeric_order() {
        let mut chart = ChartOfAccounts::default();
        chart
            .insert(account(1, 1100, AccountCategory::Asset))
            .unwrap();
        chart
            .insert(account(2, 2000, AccountCategory::Liability))
            .unwrap();
        assert_eq!(chart.len(), 2);
        assert_eq!(
            chart
                .get(account(1, 1100, AccountCategory::Asset).id())
                .unwrap()
                .code()
                .value(),
            1100
        );
        assert_eq!(
            chart
                .accounts_by_code()
                .map(|account| account.code().value())
                .collect::<Vec<_>>(),
            [1100, 2000]
        );
        assert!(matches!(
            chart.insert(account(1, 1101, AccountCategory::Asset)),
            Err(AccountError::DuplicateId(_))
        ));
        assert!(matches!(
            chart.insert(account(3, 1100, AccountCategory::Asset)),
            Err(AccountError::DuplicateCode(_))
        ));
    }
}

use std::collections::{HashMap, hash_map::Entry};

use crate::{Account, AccountCode, AccountError};

/// The accounts available to one ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountCatalogue {
    accounts: HashMap<AccountCode, Account>,
}

impl AccountCatalogue {
    /// Create a catalogue, rejecting duplicate account codes.
    pub fn new(accounts: Vec<Account>) -> Result<Self, AccountError> {
        let mut by_code = HashMap::with_capacity(accounts.len());
        for account in accounts {
            let code = account.code();
            match by_code.entry(code) {
                Entry::Vacant(entry) => {
                    entry.insert(account);
                }
                Entry::Occupied(_) => return Err(AccountError::DuplicateCode(code)),
            }
        }

        Ok(Self { accounts: by_code })
    }

    pub fn get(&self, code: AccountCode) -> Option<&Account> {
        self.accounts.get(&code)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}

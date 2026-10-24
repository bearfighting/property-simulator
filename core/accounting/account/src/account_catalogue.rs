use std::collections::{HashMap, hash_map::Entry};

use crate::{Account, AccountCode, AccountError, AccountRole};

/// The accounts available to one ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountCatalogue {
    accounts: HashMap<AccountCode, Account>,
    codes_by_role: HashMap<AccountRole, AccountCode>,
}

impl AccountCatalogue {
    /// Create a catalogue, rejecting duplicate account codes or roles.
    pub fn new(accounts: Vec<Account>) -> Result<Self, AccountError> {
        let mut by_code = HashMap::with_capacity(accounts.len());
        let mut codes_by_role = HashMap::with_capacity(accounts.len());
        for account in accounts {
            let code = account.code();
            let role = account.role().clone();
            match by_code.entry(code) {
                Entry::Vacant(entry) => {
                    entry.insert(account);
                }
                Entry::Occupied(_) => return Err(AccountError::DuplicateCode(code)),
            }

            if codes_by_role.insert(role.clone(), code).is_some() {
                return Err(AccountError::DuplicateRole(role));
            }
        }

        Ok(Self {
            accounts: by_code,
            codes_by_role,
        })
    }

    pub fn get(&self, code: AccountCode) -> Option<&Account> {
        self.accounts.get(&code)
    }

    /// Resolve one semantic account role to its posting account code.
    pub fn code_for_role(&self, role: &AccountRole) -> Option<AccountCode> {
        self.codes_by_role.get(role).copied()
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

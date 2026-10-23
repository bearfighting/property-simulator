use crate::{LedgerError, OpeningBalance};
use property_simulation_account::{AccountCatalogue, AccountCode, DebitCredit};
use property_simulation_journal_entry::JournalEntry;
use property_simulation_money::{Currency, Money};
use std::collections::HashMap;

/// One append-only journal and its current per-account net balances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    accounts: AccountCatalogue,
    currency: Currency,
    opening_balances: Vec<OpeningBalance>,
    journal: Vec<JournalEntry>,
    net_debit_balances: HashMap<AccountCode, Money>,
}

impl Ledger {
    /// Create a ledger from the supplied account set and opening facts.
    pub fn new(
        accounts: AccountCatalogue,
        currency: Currency,
        opening_balances: Vec<OpeningBalance>,
    ) -> Result<Self, LedgerError> {
        let mut balances = HashMap::with_capacity(accounts.len());
        for account in accounts.iter() {
            balances.insert(account.code(), Money::from_parts(0, 0, currency)?);
        }

        let mut seen_opening_accounts = HashMap::with_capacity(opening_balances.len());
        let mut debits = Money::from_parts(0, 0, currency)?;
        let mut credits = Money::from_parts(0, 0, currency)?;

        for opening in &opening_balances {
            if accounts.get(opening.account()).is_none() {
                return Err(LedgerError::UnknownAccount(opening.account()));
            }
            if seen_opening_accounts
                .insert(opening.account(), ())
                .is_some()
            {
                return Err(LedgerError::DuplicateOpeningBalance(opening.account()));
            }
            Self::validate_currency(currency, opening.amount().currency())?;

            let balance = balances
                .get_mut(&opening.account())
                .ok_or(LedgerError::UnknownAccount(opening.account()))?;
            *balance = Self::apply_signed_amount(*balance, opening.side(), opening.amount())?;

            match opening.side() {
                DebitCredit::Debit => debits = debits.checked_add(opening.amount())?,
                DebitCredit::Credit => credits = credits.checked_add(opening.amount())?,
            }
        }

        if debits.checked_cmp(credits)? != std::cmp::Ordering::Equal {
            return Err(LedgerError::UnbalancedOpeningBalances { debits, credits });
        }

        Ok(Self {
            accounts,
            currency,
            opening_balances,
            journal: Vec::new(),
            net_debit_balances: balances,
        })
    }
}

impl Ledger {
    /// Append a journal entry if it fits this ledger's account set, currency, and period order.
    /// The ledger is unchanged if any validation or balance calculation fails.
    pub fn append(&mut self, entry: JournalEntry) -> Result<(), LedgerError> {
        if let Some(previous) = self.journal.last() {
            if entry.month() < previous.month() {
                return Err(LedgerError::MonthRegression {
                    previous: previous.month(),
                    attempted: entry.month(),
                });
            }
        }

        let mut next_balances = self.net_debit_balances.clone();
        for line in entry.lines() {
            if self.accounts.get(line.account()).is_none() {
                return Err(LedgerError::UnknownAccount(line.account()));
            }
            Self::validate_currency(self.currency, line.amount().currency())?;

            let balance = next_balances
                .get_mut(&line.account())
                .ok_or(LedgerError::UnknownAccount(line.account()))?;
            *balance = Self::apply_signed_amount(*balance, line.side(), line.amount())?;
        }

        self.journal.push(entry);
        self.net_debit_balances = next_balances;
        Ok(())
    }
}

impl Ledger {
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    pub fn accounts(&self) -> &AccountCatalogue {
        &self.accounts
    }

    pub fn opening_balances(&self) -> &[OpeningBalance] {
        &self.opening_balances
    }

    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }

    /// Return the signed net balance: positive is debit, negative is credit.
    pub fn balance(&self, account: AccountCode) -> Result<Money, LedgerError> {
        self.net_debit_balances
            .get(&account)
            .copied()
            .ok_or(LedgerError::UnknownAccount(account))
    }
}

impl Ledger {
    fn apply_signed_amount(
        balance: Money,
        side: DebitCredit,
        amount: Money,
    ) -> Result<Money, LedgerError> {
        let signed = match side {
            DebitCredit::Debit => amount,
            DebitCredit::Credit => amount.checked_neg()?,
        };
        Ok(balance.checked_add(signed)?)
    }

    fn validate_currency(expected: Currency, actual: Currency) -> Result<(), LedgerError> {
        if expected == actual {
            Ok(())
        } else {
            Err(LedgerError::CurrencyMismatch { expected, actual })
        }
    }
}

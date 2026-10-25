use crate::LedgerError;
use property_simulation_account::{AccountCatalogue, AccountCode, DebitCredit};
use property_simulation_journal_entry::JournalEntry;
use property_simulation_money::{Currency, Money};
use std::collections::BTreeMap;

/// An append-only journal and balances derived from its entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    currency: Currency,
    journal: Vec<JournalEntry>,
}

impl Ledger {
    /// Create an empty ledger for one currency.
    pub const fn new(currency: Currency) -> Self {
        Self {
            currency,
            journal: Vec::new(),
        }
    }

    /// Append an entry after validating its accounts, currency, and period order.
    /// The catalogue is supplied by the caller and is not owned by the Ledger.
    pub fn append(
        &mut self,
        entry: JournalEntry,
        accounts: &AccountCatalogue,
    ) -> Result<(), LedgerError> {
        if let Some(previous) = self.journal.last() {
            if entry.month() < previous.month() {
                return Err(LedgerError::MonthRegression {
                    previous: previous.month(),
                    attempted: entry.month(),
                });
            }
        }

        for line in entry.lines() {
            if accounts.get(line.account()).is_none() {
                return Err(LedgerError::UnknownAccount(line.account()));
            }
            if line.amount().currency() != self.currency {
                return Err(LedgerError::CurrencyMismatch {
                    expected: self.currency,
                    actual: line.amount().currency(),
                });
            }
        }

        self.journal.push(entry);
        Ok(())
    }

    /// Derive the ending net debit balance for every account in the supplied catalogue.
    /// Positive amounts are debit balances; negative amounts are credit balances.
    pub fn ending_balances(
        &self,
        accounts: &AccountCatalogue,
    ) -> Result<BTreeMap<AccountCode, Money>, LedgerError> {
        let zero = Money::from_parts(0, 0, self.currency)?;
        let mut balances = accounts
            .iter()
            .map(|account| (account.code(), zero))
            .collect::<BTreeMap<_, _>>();

        for entry in &self.journal {
            for line in entry.lines() {
                let balance = balances
                    .get_mut(&line.account())
                    .ok_or(LedgerError::UnknownAccount(line.account()))?;
                let amount = match line.side() {
                    DebitCredit::Debit => line.amount(),
                    DebitCredit::Credit => line.amount().checked_neg()?,
                };
                *balance = balance.checked_add(amount)?;
            }
        }

        Ok(balances)
    }
}

impl Ledger {
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }
}

use property_simulation_account::AccountCatalogue;
use property_simulation_journal_entry::JournalEntry;
use property_simulation_money::Currency;

/// Read-only ledger facts supplied to reporting calculations.
#[derive(Debug, Clone, Copy)]
pub struct ReportingInput<'a> {
    accounts: &'a AccountCatalogue,
    currency: Currency,
    journal: &'a [JournalEntry],
}

impl<'a> ReportingInput<'a> {
    pub const fn new(
        accounts: &'a AccountCatalogue,
        currency: Currency,
        journal: &'a [JournalEntry],
    ) -> Self {
        Self {
            accounts,
            currency,
            journal,
        }
    }
}

impl ReportingInput<'_> {
    pub const fn accounts(&self) -> &AccountCatalogue {
        self.accounts
    }

    pub const fn currency(&self) -> Currency {
        self.currency
    }

    pub const fn journal(&self) -> &[JournalEntry] {
        self.journal
    }
}

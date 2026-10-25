use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_journal_entry::{JournalEntry, JournalLine};
use property_simulation_money::Currency;
use property_simulation_month::Month;

use crate::{DtoError, dto::MoneyDto};

/// One JournalEntry and its lines, independent of a backend's row/document layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntryDto {
    pub simulation_id: String,
    pub month: Month,
    pub sequence: u32,
    pub lines: Vec<JournalLineDto>,
}

/// A posting to one account and side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalLineDto {
    pub account_code: AccountCode,
    pub side: DebitCredit,
    pub amount: MoneyDto,
}

impl JournalLineDto {
    pub fn from_domain(line: &JournalLine) -> Self {
        Self {
            account_code: line.account(),
            side: line.side(),
            amount: MoneyDto::from_domain(line.amount()),
        }
    }

    pub fn domain_line(&self, currency: Currency) -> Result<JournalLine, DtoError> {
        Ok(JournalLine::new(
            self.account_code,
            self.side,
            self.amount.to_domain(currency)?,
        )?)
    }
}

impl JournalEntryDto {
    pub fn from_domain(simulation_id: String, sequence: u32, entry: &JournalEntry) -> Self {
        Self {
            simulation_id,
            month: entry.month(),
            sequence,
            lines: entry
                .lines()
                .iter()
                .map(JournalLineDto::from_domain)
                .collect(),
        }
    }

    /// Rebuilds and validates the balanced Core entry.
    pub fn domain_entry(&self, currency: Currency) -> Result<JournalEntry, DtoError> {
        let domain_lines = self
            .lines
            .iter()
            .map(|line| line.domain_line(currency))
            .collect::<Result<Vec<_>, _>>()?;
        if self.month.is_initial() {
            Ok(JournalEntry::initial(domain_lines)?)
        } else {
            Ok(JournalEntry::new(self.month, domain_lines)?)
        }
    }
}

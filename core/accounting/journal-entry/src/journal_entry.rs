use crate::{JournalEntryError, JournalLine};
use property_simulation_account::DebitCredit;
use property_simulation_money::Money;
use property_simulation_month::Month;

/// A balanced set of accounting lines for one simulation month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    month: Month,
    lines: Vec<JournalLine>,
}

impl JournalEntry {
    /// Construct an entry after verifying that its lines balance.
    pub fn new(month: Month, lines: Vec<JournalLine>) -> Result<Self, JournalEntryError> {
        if lines.len() < 2 {
            return Err(JournalEntryError::TooFewLines);
        }

        let mut debits: Option<Money> = None;
        let mut credits: Option<Money> = None;

        for line in &lines {
            let total = match line.side() {
                DebitCredit::Debit => &mut debits,
                DebitCredit::Credit => &mut credits,
            };

            *total = Some(match *total {
                Some(current) => current.checked_add(line.amount())?,
                None => line.amount(),
            });
        }

        let debits = debits.ok_or(JournalEntryError::TooFewLines)?;
        let credits = credits.ok_or(JournalEntryError::TooFewLines)?;

        if debits.checked_cmp(credits)? != std::cmp::Ordering::Equal {
            return Err(JournalEntryError::Unbalanced { debits, credits });
        }

        Ok(Self { month, lines })
    }
}

impl JournalEntry {
    pub const fn month(&self) -> Month {
        self.month
    }

    pub fn lines(&self) -> &[JournalLine] {
        &self.lines
    }
}

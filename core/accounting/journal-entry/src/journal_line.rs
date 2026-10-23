use crate::JournalEntryError;
use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_money::Money;

/// One positive, currency-rounded amount assigned to an account and side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalLine {
    account: AccountCode,
    side: DebitCredit,
    amount: Money,
}

impl JournalLine {
    /// Construct a line, rounding the positive amount to its currency's minor unit.
    pub fn new(
        account: AccountCode,
        side: DebitCredit,
        amount: Money,
    ) -> Result<Self, JournalEntryError> {
        let amount = amount.rounded_for_posting()?;
        if amount.is_zero() {
            return Err(JournalEntryError::NonPositiveAmount);
        }

        Ok(Self {
            account,
            side,
            amount,
        })
    }
}

impl JournalLine {
    pub const fn account(&self) -> AccountCode {
        self.account
    }

    pub const fn side(&self) -> DebitCredit {
        self.side
    }

    pub const fn amount(&self) -> Money {
        self.amount
    }
}

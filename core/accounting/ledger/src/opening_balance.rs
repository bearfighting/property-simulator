use crate::LedgerError;
use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_money::Money;

/// An account balance that was true before the first simulated month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpeningBalance {
    account: AccountCode,
    side: DebitCredit,
    amount: Money,
}

impl OpeningBalance {
    /// Create a positive opening balance. Zero balances should be omitted.
    pub fn new(
        account: AccountCode,
        side: DebitCredit,
        amount: Money,
    ) -> Result<Self, LedgerError> {
        if amount.is_zero() || amount.is_negative() {
            return Err(LedgerError::InvalidOpeningBalance);
        }

        Ok(Self {
            account,
            side,
            amount,
        })
    }
}

impl OpeningBalance {
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

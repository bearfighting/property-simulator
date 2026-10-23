use crate::DebitCredit;

/// Broad accounting class of an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountClass {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

impl AccountClass {
    /// Return the debit/credit side that increases this account class.
    pub const fn normal_balance(self) -> DebitCredit {
        match self {
            Self::Asset | Self::Expense => DebitCredit::Debit,
            Self::Liability | Self::Equity | Self::Income => DebitCredit::Credit,
        }
    }
}

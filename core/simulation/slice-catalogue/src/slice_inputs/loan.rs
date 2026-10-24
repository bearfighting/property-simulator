use std::num::NonZeroU32;

use property_simulation_account::AccountCode;
use property_simulation_money::Money;
use property_simulation_month::Month;

/// All non-configuration inputs needed to calculate one month's loan payment.
///
/// Application assembles this value from the current Month, initial settings,
/// Ledger opening balance, the latest Loan effect or initial term, and account
/// mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoanSliceInput {
    pub month: Month,
    pub initial_principal: Money,
    pub opening_balance: Money,
    pub remaining_term_months: NonZeroU32,
    pub cash_account: AccountCode,
    pub loan_liability_account: AccountCode,
    pub interest_expense_account: AccountCode,
}

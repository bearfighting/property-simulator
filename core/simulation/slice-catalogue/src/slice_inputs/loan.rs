use std::num::NonZeroU32;

use property_simulation_account::AccountCode;
use property_simulation_money::Money;
use property_simulation_month::Month;

/// All non-configuration inputs needed to calculate one month's loan payment.
///
/// Application assembles this value from the current Month, initial settings,
/// Ledger opening balance, the remaining term derived from the original term
/// and current Month, and account mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoanSliceInput {
    pub month: Month,
    pub initial_principal: Money,
    pub opening_balance: Money,
    /// Remaining payments at the start of this input month.
    pub remaining_term_months_at_month_start: NonZeroU32,
    pub cash_account: AccountCode,
    pub loan_liability_account: AccountCode,
    pub interest_expense_account: AccountCode,
}

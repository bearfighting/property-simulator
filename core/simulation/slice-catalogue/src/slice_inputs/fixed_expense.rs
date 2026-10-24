use property_simulation_account::AccountCode;
use property_simulation_money::Money;
use property_simulation_month::Month;

/// Inputs resolved for one month's fixed expense evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedExpenseSliceInput {
    pub month: Month,
    pub property_value: Money,
    pub cash_account: AccountCode,
    pub expense_account: AccountCode,
}

use property_simulation_account::AccountCode;
use property_simulation_month::Month;

/// Inputs resolved for one generated repair cost instance's settlement month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairCostSliceInput {
    pub month: Month,
    pub occurrence_month: Month,
    pub cash_account: AccountCode,
    pub expense_account: AccountCode,
}

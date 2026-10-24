use property_simulation_account::AccountCode;
use property_simulation_month::Month;

/// Inputs resolved for one month's rental evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RentalSliceInput {
    pub month: Month,
    pub cash_account: AccountCode,
    pub rental_income_account: AccountCode,
}

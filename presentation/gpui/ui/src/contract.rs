use std::{future::Future, pin::Pin, rc::Rc};

/// Values collected by the simulation initialization page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializationFormData {
    pub name: String,
    pub property_value: String,
    pub cash_balance: String,
    pub stop_after_months: String,
    pub stop_when_all_loans_paid_off: bool,
    pub loan_enabled: bool,
    pub loan_balance: String,
    pub annual_interest_rate_percent: String,
    pub loan_term_months: String,
    pub rental_enabled: bool,
    pub monthly_rent: String,
    pub fixed_expense_enabled: bool,
    pub monthly_expenses: String,
    pub repairs_enabled: bool,
    pub repair_maximum_amount: String,
    pub repair_delay_months: String,
}

/// Result returned by the host after it coordinates initialization.
pub type SubmissionResult = Result<(), String>;

/// Async host callback. The future is local to GPUI's foreground executor.
pub type InitializationHandler =
    Rc<dyn Fn(InitializationFormData) -> Pin<Box<dyn Future<Output = SubmissionResult> + 'static>>>;

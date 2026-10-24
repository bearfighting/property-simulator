//! Pure domain calculations. Modules may own formulas specific to one domain
//! behavior while keeping inputs and outputs independent from application flow,
//! Ledger state, and IO.

mod loan;

pub use loan::{
    LoanCalculationError, LoanInstallment, calculate_installment, calculate_scheduled_payment,
};

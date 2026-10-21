mod loan;
#[cfg(test)]
#[path = "test_support/loan_coordinator.rs"]
mod loan_coordinator;
mod property_tax;
mod rental;
mod repair;
mod utility;

#[cfg(test)]
pub use loan::LoanCoordinatedMonth;
#[cfg(test)]
pub use loan::LoanMonthCommand;
pub use loan::{
    LoanCalculation, LoanContinuation, LoanEvent, LoanMonthInput, LoanMonthOutput, LoanRunUpdate,
    LoanSlice, LoanStateEffect, LoanStateEffectProposal, LoanTermsUpdate, apply_loan_terms_update,
    validate_loan_input, validate_loan_terms_update,
};
#[cfg(test)]
pub use loan_coordinator::{coordinate_loan_month, coordinate_month_with_loans};
pub use property_tax::PropertyTaxSlice;
pub use rental::RentalSlice;
pub use repair::RepairSlice;
pub use utility::UtilitySlice;

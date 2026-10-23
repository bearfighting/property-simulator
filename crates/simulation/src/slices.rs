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
pub use loan::LoanRunUpdate;
pub use loan::{LoanCalculation, LoanContinuation, LoanEvent, LoanStateEffect};
pub(crate) use loan::{LoanMonthOutput, LoanSlice};
pub(crate) use loan::{validate_loan_calculation, validate_loan_opening_state};
#[cfg(test)]
pub use loan_coordinator::{coordinate_loan_month, coordinate_month_with_loans};
pub use property_tax::PropertyTaxSlice;
pub use rental::RentalSlice;
pub use repair::{RepairCostSlice, RepairOccurrenceSlice};
pub use utility::UtilitySlice;

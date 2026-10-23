mod accounting;
mod proposal;
mod terms;
#[cfg(test)]
mod tests;
mod types;

#[cfg(test)]
pub(in crate::slices) use accounting::{cash_classified_entry, validate_loan_accounts};
pub(crate) use accounting::{validate_loan_calculation, validate_loan_opening_state};
#[cfg(test)]
pub use accounting::{validate_loan_input, validate_loan_terms_update};
pub use terms::LoanCalculation;
#[cfg(test)]
pub use terms::apply_loan_terms_update;
pub(crate) use terms::map_model_error;
#[cfg(test)]
pub use types::LoanCoordinatedMonth;
#[cfg(test)]
pub use types::LoanMonthCommand;
#[cfg(test)]
pub use types::LoanRunUpdate;
pub use types::{
    LoanContinuation, LoanEvent, LoanMonthOutput, LoanSlice, LoanStateEffect,
    LoanStateEffectProposal,
};
#[cfg(test)]
pub use types::{LoanMonthInput, LoanTermsUpdate};

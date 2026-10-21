mod accounting;
mod proposal;
mod terms;
#[cfg(test)]
mod tests;
mod types;

#[cfg(test)]
pub(in crate::slices) use accounting::{cash_classified_entry, validate_loan_accounts};
pub use accounting::{validate_loan_input, validate_loan_terms_update};
pub(in crate::slices) use terms::map_model_error;
pub use terms::{LoanCalculation, apply_loan_terms_update};
#[cfg(test)]
pub use types::LoanCoordinatedMonth;
#[cfg(test)]
pub use types::LoanMonthCommand;
#[cfg(test)]
pub use types::LoanRunUpdate;
pub use types::{
    LoanContinuation, LoanEvent, LoanMonthInput, LoanMonthOutput, LoanSlice, LoanStateEffect,
    LoanStateEffectProposal, LoanTermsUpdate,
};

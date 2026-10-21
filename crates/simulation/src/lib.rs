//! Independent simulation calculations and typed Slice interfaces.

mod activity;
#[cfg(test)]
#[path = "test_support/archive.rs"]
mod archive;
mod composition;
#[cfg(test)]
#[path = "test_support/coordinator.rs"]
mod coordinator;
mod error;
mod proposals;
#[cfg(test)]
#[path = "test_support/run_control/mod.rs"]
mod run_control;
mod slices;
mod types;
mod validation;

pub use activity::*;
#[cfg(test)]
pub use archive::{
    ArchiveError, BranchRecord, RunCashFlowSummary, SavedMonth, SimulationArchive, UserSettings,
    saved_month,
};
pub use composition::{SliceResources, validate_composition};
#[cfg(test)]
pub use coordinator::coordinate_month;
pub use error::{FatalSimulationFailure, FatalSimulationPhase, SimulationError};
pub use proposals::{identify_proposal, merge_proposals};
#[cfg(test)]
pub use run_control::{
    MonthSnapshot, RunCallResult, RunCommand, RunConfig, RunControlError, RunIdentity, RunStatus,
    RunUpdate, RunUpdateValue, SimulationRun, SingleMonthExecutor, StopCondition, StopReason,
};
#[cfg(test)]
pub use slices::LoanCoordinatedMonth;
#[cfg(test)]
pub use slices::LoanMonthCommand;
pub use slices::{
    LoanCalculation, LoanContinuation, LoanEvent, LoanMonthInput, LoanMonthOutput, LoanRunUpdate,
    LoanSlice, LoanStateEffect, LoanStateEffectProposal, LoanTermsUpdate, PropertyTaxSlice,
    RentalSlice, RepairSlice, UtilitySlice, apply_loan_terms_update,
};
#[cfg(test)]
pub use slices::{coordinate_loan_month, coordinate_month_with_loans};
pub use types::*;
#[cfg(test)]
pub use validation::validate_proposal_transactions;
pub use validation::{
    AccountFacts, LedgerRead, ValidatedInput, ValidationErrors, ValidationIssue, validate_input,
};
#[cfg(test)]
pub use validation::{ProposalValidationErrors, ProposalValidationIssue};

#[cfg(test)]
extern crate self as property_simulation_simulation;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod monthly_composition_tests;

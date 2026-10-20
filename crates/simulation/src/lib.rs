//! Core simulation orchestration and monthly lifecycle, built on domain, accounting, and models.

mod activity;
mod archive;
mod composition;
mod coordinator;
mod error;
mod proposals;
mod run_control;
mod slices;
mod types;
mod validation;

pub use activity::*;
pub use archive::{
    ArchiveError, BranchRecord, RunCashFlowSummary, SavedMonth, SimulationArchive, UserSettings,
    saved_month,
};
pub use composition::{
    SliceResources, account_resource_key, plan_slice_stages, validate_composition,
};
pub use coordinator::coordinate_month;
pub use error::{FatalSimulationFailure, FatalSimulationPhase, SimulationError};
pub use proposals::{OutputIdentity, identify_proposal, merge_proposals};
pub use run_control::{
    MonthSnapshot, RunCallResult, RunCommand, RunConfig, RunControlError, RunIdentity, RunStatus,
    RunUpdate, RunUpdateValue, SimulationRun, SingleMonthExecutor, StopCondition, StopReason,
};
pub use slices::{RentalSlice, RepairSlice};
pub use types::*;
pub use validation::{
    ProposalValidationErrors, ProposalValidationIssue, ValidatedInput, ValidationErrors,
    ValidationIssue, validate_input, validate_proposal_transactions,
};

#[cfg(test)]
mod tests;

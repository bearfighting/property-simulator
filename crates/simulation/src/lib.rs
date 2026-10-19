//! Core simulation orchestration and monthly lifecycle, built on domain, accounting, and models.

mod activity;
mod composition;
mod error;
mod proposals;
mod slices;
mod types;
mod validation;

pub use activity::*;
pub use composition::{
    SliceResources, account_resource_key, plan_slice_stages, validate_composition,
};
pub use error::SimulationError;
pub use proposals::{OutputIdentity, identify_proposal, merge_proposals};
pub use slices::{RentalSlice, RepairSlice};
pub use types::*;
pub use validation::{
    ProposalValidationErrors, ProposalValidationIssue, ValidatedInput, ValidationErrors,
    ValidationIssue, validate_input, validate_proposal_transactions,
};

#[cfg(test)]
mod tests;

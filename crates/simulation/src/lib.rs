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
mod monthly;
mod proposals;
#[cfg(test)]
#[path = "test_support/run_control/mod.rs"]
mod run_control;
mod settlement;
mod slices;
#[cfg(test)]
mod test_support_api;
mod types;
mod validation;

pub use activity::{ActivitySample, ActivitySampleValue};
pub use error::{FatalSimulationFailure, FatalSimulationPhase, SimulationError};
pub use settlement::{SliceMonthlyRuntime, calculate_monthly_slice_result};
pub use slices::{LoanCalculation, LoanContinuation, LoanEvent, LoanStateEffect};
pub use types::{
    ActivityConfiguration, ActivityInstance, AssetCondition, FrozenInput, IdentifiedProposal,
    InstanceLifecycleChange, InstanceLifecycleOperation, LoanConfig, MonthlyCashFlowContribution,
    MonthlyInput, MonthlyResult, OpeningState, OutputIdAllocator, PropertyTaxCadence,
    PropertyTaxConfig, ProposedEffect, ProposedEvent, ProposedTransaction, RepairCostConfig,
    RepairCostTemplate, RepairHistory, RepairOccurrenceConfig, SliceRandomInputKey, UtilityConfig,
};
pub use validation::{AccountFacts, LedgerRead};

// Internal cross-module names. These are crate-private and are not part of the
// supported caller interface above.
pub(crate) use proposals::merge_proposals;
pub(crate) use slices::{
    PropertyTaxSlice, RentalSlice, RepairCostSlice, RepairOccurrenceSlice, UtilitySlice,
};
pub(crate) use types::{EventProposal, SliceProposal, TransactionProposal};

// Test-only helpers are kept out of the production API above. These are
// crate-visible only while compiling this crate's unit tests.
#[cfg(test)]
pub(crate) use test_support_api::*;

#[cfg(test)]
mod tests;

//! Unit-test-only APIs kept separate from the production crate interface.

pub(crate) use crate::activity::{
    SimulationSlice, SliceOutput, UTILITY_SAMPLE_PURPOSE, UtilitySamplingError,
    ValidatedActivitySample, propose_slice, sample_utility,
};
pub(crate) use crate::archive::{
    ArchiveError, BranchRecord, SimulationArchive, UserSettings, saved_month,
};
pub(crate) use crate::composition::{SliceResources, validate_composition};
pub(crate) use crate::coordinator::coordinate_month;
pub(crate) use crate::proposals::identify_proposal;
pub(crate) use crate::run_control::{
    MonthSnapshot, RunConfig, RunIdentity, RunUpdate, RunUpdateValue, SimulationRun, StopCondition,
};
pub(crate) use crate::slices::{LoanRunUpdate, coordinate_loan_month, coordinate_month_with_loans};
pub(crate) use crate::types::*;
pub(crate) use crate::validation::{
    ValidatedInput, ValidationErrors, validate_proposal_transactions,
};

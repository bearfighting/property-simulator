use property_simulation_accounting::AccountError;
use property_simulation_domain::{ActivityInstanceId, BranchId, DomainError, TrialId, YearMonth};

use crate::validation::ProposalValidationErrors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatalSimulationPhase {
    SliceTransition,
    LedgerCommit,
    ClosingStateDerivation,
    MonthlyResultConstruction,
}

/// Diagnostic for an unexpected failure after the relevant validation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FatalSimulationFailure {
    pub phase: FatalSimulationPhase,
    pub period: YearMonth,
    pub trial_id: Option<TrialId>,
    pub branch_id: Option<BranchId>,
    pub activity_instance_id: Option<ActivityInstanceId>,
    pub source: Box<SimulationError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationError {
    InvalidInput(&'static str),
    InvalidProposal(&'static str),
    Domain(DomainError),
    Accounting(AccountError),
    InvalidTransactions(ProposalValidationErrors),
    DuplicateOutputId,
    Fatal(FatalSimulationFailure),
}

impl FatalSimulationFailure {
    pub(crate) fn new(
        phase: FatalSimulationPhase,
        period: YearMonth,
        trial_id: Option<TrialId>,
        branch_id: Option<BranchId>,
        activity_instance_id: Option<ActivityInstanceId>,
        source: SimulationError,
    ) -> Self {
        Self {
            phase,
            period,
            trial_id,
            branch_id,
            activity_instance_id,
            source: Box::new(source),
        }
    }
}
impl From<DomainError> for SimulationError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}
impl From<AccountError> for SimulationError {
    fn from(e: AccountError) -> Self {
        Self::Accounting(e)
    }
}

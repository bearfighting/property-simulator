use property_simulation_accounting::AccountError;
use property_simulation_domain::{ActivityInstanceId, DomainError, YearMonth};
#[cfg(test)]
use property_simulation_domain::{BranchId, TrialId};

#[cfg(test)]
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
    #[cfg(test)]
    pub trial_id: Option<TrialId>,
    #[cfg(test)]
    pub branch_id: Option<BranchId>,
    pub activity_instance_id: Option<ActivityInstanceId>,
    pub source: Box<SimulationError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationError {
    InvalidInput(&'static str),
    InputValidation(crate::validation::ValidationErrors),
    InvalidProposal(&'static str),
    Domain(DomainError),
    Accounting(AccountError),
    #[cfg(test)]
    InvalidTransactions(ProposalValidationErrors),
    DuplicateOutputId,
    Fatal(FatalSimulationFailure),
}

impl FatalSimulationFailure {
    pub(crate) fn new(
        phase: FatalSimulationPhase,
        period: YearMonth,
        activity_instance_id: Option<ActivityInstanceId>,
        source: SimulationError,
    ) -> Self {
        Self {
            phase,
            period,
            #[cfg(test)]
            trial_id: None,
            #[cfg(test)]
            branch_id: None,
            activity_instance_id,
            source: Box::new(source),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_run_identity(
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

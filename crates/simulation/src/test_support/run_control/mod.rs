use property_simulation_accounting::{Ledger, LedgerTransaction};
use property_simulation_domain::{BranchId, Money, TrialId, YearMonth};

use crate::{
    FrozenInput, LoanContinuation, LoanRunUpdate, OpeningState, SimulationError, TestMonthlyResult,
    ValidatedInput,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    pub start: YearMonth,
    pub end: Option<YearMonth>,
    pub stop_conditions: Vec<StopCondition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCondition {
    MonthlyNetCashChangeAtMost(Money),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunUpdateValue {
    Rent(Money),
    Condition(crate::AssetCondition),
    Activities(Vec<crate::ActivityInstance>),
    LoanTerms(Box<LoanRunUpdate>),
    /// A known state adjustment and its balanced accounting posting are one update.
    StateAdjustment {
        state: OpeningState,
        transaction: LedgerTransaction,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunUpdate {
    pub effective_month: YearMonth,
    pub source: String,
    pub value: RunUpdateValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunCommand {
    Continue,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    User,
    EndMonth(YearMonth),
    Condition(usize),
    CalendarLimit(YearMonth),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    Running { next_month: YearMonth },
    Stopped(StopReason),
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthSnapshot {
    result: TestMonthlyResult,
    ledger: Ledger,
}

impl MonthSnapshot {
    pub fn result(&self) -> &TestMonthlyResult {
        &self.result
    }
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCallResult {
    pub monthly_result: Option<TestMonthlyResult>,
    pub latest_completed_month: Option<YearMonth>,
    pub status: RunStatus,
    pub stop_reason: Option<StopReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunControlError {
    InvalidConfig(&'static str),
    InvalidUpdate(&'static str),
    InvalidState,
    Input(crate::ValidationErrors),
    InvalidExecutionInput(SimulationError),
    Simulation(SimulationError),
    Randomness(&'static str),
}

/// Executes one already frozen month. Implementations compose sampling, Slices and
/// `coordinate_month`; run control owns the lifecycle and never duplicates those rules.
/// Non-fatal errors must mean validation/preflight failed before commit. Errors after
/// an accepted transition or ledger commit must be returned as `SimulationError::Fatal`.
pub trait SingleMonthExecutor {
    /// Performs deterministic, side-effect-free validation of executor-specific
    /// configuration and activity composition. It must not sample or transition state.
    fn validate_inputs(
        &self,
        input: &ValidatedInput,
        opening_ledger: &Ledger,
        activities: &[crate::ActivityInstance],
    ) -> Result<(), SimulationError>;

    /// Validate loan continuation and month-start terms updates before transition.
    /// Executors without loan support keep the default rejection behavior.
    fn validate_loan_context(
        &self,
        _input: &ValidatedInput,
        _opening_ledger: &Ledger,
        loans: &[LoanContinuation],
        updates: &[LoanRunUpdate],
    ) -> Result<(), SimulationError> {
        if loans.is_empty() && updates.is_empty() {
            Ok(())
        } else {
            Err(SimulationError::InvalidInput(
                "monthly executor does not support loan state or updates",
            ))
        }
    }

    fn requires_random_streams(&self) -> bool {
        false
    }

    /// Optional explicit stream allocation hook. Random-aware executors may use
    /// an injected OS-backed (or test-keyed) factory; default executors stay deterministic.
    fn create_random_streams(
        &mut self,
        _identity: Option<RunIdentity>,
    ) -> Result<Option<property_simulation_randomness::RandomStreams>, &'static str> {
        Ok(None)
    }

    fn execute(
        &mut self,
        input: &ValidatedInput,
        opening_ledger: &Ledger,
        activities: &[crate::ActivityInstance],
        identity: Option<RunIdentity>,
    ) -> Result<(TestMonthlyResult, Ledger), SimulationError>;

    fn execute_with_random_streams(
        &mut self,
        input: &ValidatedInput,
        opening_ledger: &Ledger,
        activities: &[crate::ActivityInstance],
        identity: Option<RunIdentity>,
        _random_streams: Option<&property_simulation_randomness::RandomStreams>,
    ) -> Result<(TestMonthlyResult, Ledger), SimulationError> {
        self.execute(input, opening_ledger, activities, identity)
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_with_loan_context(
        &mut self,
        input: &ValidatedInput,
        opening_ledger: &Ledger,
        activities: &[crate::ActivityInstance],
        loans: &[LoanContinuation],
        updates: &[LoanRunUpdate],
        identity: Option<RunIdentity>,
        random_streams: Option<&property_simulation_randomness::RandomStreams>,
    ) -> Result<(TestMonthlyResult, Ledger), SimulationError> {
        if !loans.is_empty() || !updates.is_empty() {
            return Err(SimulationError::InvalidInput(
                "monthly executor does not support loan state or updates",
            ));
        }
        self.execute_with_random_streams(
            input,
            opening_ledger,
            activities,
            identity,
            random_streams,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunIdentity {
    pub trial_id: TrialId,
    pub branch_id: BranchId,
}

pub struct SimulationRun {
    config: RunConfig,
    next_month: YearMonth,
    opening_state: OpeningState,
    frozen_input: FrozenInput,
    activities: Vec<crate::ActivityInstance>,
    loan_continuations: Vec<LoanContinuation>,
    ledger: Ledger,
    history: Vec<RunUpdate>,
    latest: Option<MonthSnapshot>,
    run_identity: Option<RunIdentity>,
    randomness_mode: Option<bool>,
    random_streams: Option<property_simulation_randomness::RandomStreams>,
    status: RunStatus,
}

mod call;
mod candidate;
mod execution;
mod output;
mod stopping;
#[cfg(test)]
mod tests;
mod validation;

impl SimulationRun {
    pub fn new(
        config: RunConfig,
        opening_state: OpeningState,
        frozen_input: FrozenInput,
        ledger: Ledger,
    ) -> Result<Self, RunControlError> {
        Self::new_with_activities(config, opening_state, frozen_input, ledger, vec![])
    }

    pub fn new_with_activities(
        config: RunConfig,
        opening_state: OpeningState,
        frozen_input: FrozenInput,
        ledger: Ledger,
        activities: Vec<crate::ActivityInstance>,
    ) -> Result<Self, RunControlError> {
        Self::new_with_activities_and_loans(
            config,
            opening_state,
            frozen_input,
            ledger,
            activities,
            vec![],
        )
    }

    pub fn new_with_activities_and_loans(
        config: RunConfig,
        opening_state: OpeningState,
        frozen_input: FrozenInput,
        ledger: Ledger,
        activities: Vec<crate::ActivityInstance>,
        loan_continuations: Vec<LoanContinuation>,
    ) -> Result<Self, RunControlError> {
        if config.end.is_some_and(|end| end < config.start) {
            return Err(RunControlError::InvalidConfig(
                "end month precedes start month",
            ));
        }
        if validation::has_duplicate_activity_ids(&activities) {
            return Err(RunControlError::InvalidConfig(
                "active activity instance IDs must be unique",
            ));
        }
        let loan_activities: std::collections::BTreeMap<_, _> = activities
            .iter()
            .filter(|activity| activity.configuration.is_loan())
            .map(|activity| (activity.id, activity))
            .collect();
        let mut loan_ids = std::collections::BTreeSet::new();
        if loan_continuations.iter().any(|loan| {
            !loan_ids.insert(loan.instance_id)
                || loan_activities
                    .get(&loan.instance_id)
                    .is_none_or(|activity| activity.slice_id != loan.slice_id)
        }) || loan_ids.len() != loan_activities.len()
        {
            return Err(RunControlError::InvalidConfig(
                "loan continuations must match active loan activities exactly",
            ));
        }
        for continuation in &loan_continuations {
            if let Some(loan) = continuation.loan {
                loan.validate().map_err(|_| {
                    RunControlError::InvalidConfig("active loan continuation is invalid")
                })?;
                loan.derive_payment().map_err(|_| {
                    RunControlError::InvalidConfig("active loan continuation is invalid")
                })?;
                if loan.opening_balance.currency() != ledger.currency() {
                    return Err(RunControlError::InvalidConfig(
                        "loan continuation currency differs from ledger",
                    ));
                }
            }
        }
        if frozen_input.period != config.start {
            return Err(RunControlError::InvalidConfig(
                "initial input month must equal run start",
            ));
        }
        if ledger
            .transactions()
            .any(|transaction| transaction.period() >= config.start)
        {
            return Err(RunControlError::InvalidConfig(
                "opening ledger contains a posting at or after the run start month",
            ));
        }
        if config
            .stop_conditions
            .iter()
            .any(|condition| match condition {
                StopCondition::MonthlyNetCashChangeAtMost(value) => {
                    value.currency() != ledger.currency()
                }
            })
        {
            return Err(RunControlError::InvalidConfig(
                "stop threshold currency differs from ledger",
            ));
        }
        Ok(Self {
            config,
            next_month: frozen_input.period,
            opening_state,
            frozen_input,
            activities,
            loan_continuations,
            ledger,
            history: Vec::new(),
            latest: None,
            run_identity: None,
            randomness_mode: None,
            random_streams: None,
            status: RunStatus::Running {
                next_month: frozen_input.period,
            },
        })
    }

    /// Construct a run with the trial and branch identity required by
    /// identity-bound outputs and random streams.
    pub fn new_with_identity(
        config: RunConfig,
        opening_state: OpeningState,
        frozen_input: FrozenInput,
        ledger: Ledger,
        identity: RunIdentity,
    ) -> Result<Self, RunControlError> {
        Self::new_with_activities_and_identity(
            config,
            opening_state,
            frozen_input,
            ledger,
            vec![],
            identity,
        )
    }

    pub fn new_with_activities_and_identity(
        config: RunConfig,
        opening_state: OpeningState,
        frozen_input: FrozenInput,
        ledger: Ledger,
        activities: Vec<crate::ActivityInstance>,
        identity: RunIdentity,
    ) -> Result<Self, RunControlError> {
        Self::new_with_activities_and_loans_and_identity(
            config,
            opening_state,
            frozen_input,
            ledger,
            activities,
            vec![],
            identity,
        )
    }

    pub fn new_with_activities_and_loans_and_identity(
        config: RunConfig,
        opening_state: OpeningState,
        frozen_input: FrozenInput,
        ledger: Ledger,
        activities: Vec<crate::ActivityInstance>,
        loans: Vec<LoanContinuation>,
        identity: RunIdentity,
    ) -> Result<Self, RunControlError> {
        let mut run = Self::new_with_activities_and_loans(
            config,
            opening_state,
            frozen_input,
            ledger,
            activities,
            loans,
        )?;
        run.set_run_identity(identity);
        Ok(run)
    }

    pub fn status(&self) -> &RunStatus {
        &self.status
    }
    pub fn history(&self) -> &[RunUpdate] {
        &self.history
    }
    pub fn latest_snapshot(&self) -> Option<&MonthSnapshot> {
        self.latest.as_ref()
    }
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }
    pub fn opening_state(&self) -> &OpeningState {
        &self.opening_state
    }
    pub fn config(&self) -> &RunConfig {
        &self.config
    }
    pub fn next_month(&self) -> YearMonth {
        self.next_month
    }
    pub fn frozen_input(&self) -> FrozenInput {
        self.frozen_input
    }
    pub fn activities(&self) -> &[crate::ActivityInstance] {
        &self.activities
    }
    pub fn loan_continuations(&self) -> &[LoanContinuation] {
        &self.loan_continuations
    }
    pub(crate) fn set_run_identity(&mut self, identity: RunIdentity) {
        self.run_identity = Some(identity);
    }
}

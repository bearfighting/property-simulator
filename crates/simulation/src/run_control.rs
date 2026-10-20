use property_simulation_accounting::{BalanceSide, Ledger, LedgerTransaction};
use property_simulation_domain::{BranchId, Money, TrialId, YearMonth};
use property_simulation_models::RepairCostDistribution;

use crate::{
    FrozenInput, MonthlyResult, OpeningState, SimulationError, ValidatedInput, validate_input,
    validation::validate_cashflow_support,
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
    RepairCostLimit(Money),
    Condition(crate::AssetCondition),
    Activities(Vec<crate::ActivityInstance>),
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
    result: MonthlyResult,
    ledger: Ledger,
}

impl MonthSnapshot {
    pub fn result(&self) -> &MonthlyResult {
        &self.result
    }
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCallResult {
    pub monthly_result: Option<MonthlyResult>,
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
}

/// Executes one already frozen month. Implementations compose sampling, Slices and
/// `coordinate_month`; run control owns the lifecycle and never duplicates those rules.
pub trait SingleMonthExecutor {
    /// Performs deterministic, side-effect-free validation of executor-specific
    /// configuration and activity composition. It must not sample or transition state.
    fn validate_inputs(
        &self,
        input: &ValidatedInput,
        opening_ledger: &Ledger,
        activities: &[crate::ActivityInstance],
    ) -> Result<(), SimulationError>;

    fn execute(
        &mut self,
        input: &ValidatedInput,
        opening_ledger: &Ledger,
        activities: &[crate::ActivityInstance],
        identity: Option<RunIdentity>,
    ) -> Result<(MonthlyResult, Ledger), SimulationError>;
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
    ledger: Ledger,
    history: Vec<RunUpdate>,
    latest: Option<MonthSnapshot>,
    identity: Option<(TrialId, BranchId)>,
    execution_identity: Option<RunIdentity>,
    status: RunStatus,
}

fn has_duplicate_activity_ids(activities: &[crate::ActivityInstance]) -> bool {
    let mut ids = std::collections::BTreeSet::new();
    activities.iter().any(|activity| !ids.insert(activity.id))
}

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
        if config.end.is_some_and(|end| end < config.start) {
            return Err(RunControlError::InvalidConfig(
                "end month precedes start month",
            ));
        }
        if has_duplicate_activity_ids(&activities) {
            return Err(RunControlError::InvalidConfig(
                "active activity instance IDs must be unique",
            ));
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
            ledger,
            history: Vec::new(),
            latest: None,
            identity: None,
            execution_identity: None,
            status: RunStatus::Running {
                next_month: frozen_input.period,
            },
        })
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
    pub(crate) fn set_execution_identity(&mut self, trial_id: TrialId, branch_id: BranchId) {
        self.execution_identity = Some(RunIdentity {
            trial_id,
            branch_id,
        });
    }

    pub fn call(
        &mut self,
        command: RunCommand,
        updates: &[RunUpdate],
        executor: &mut dyn SingleMonthExecutor,
    ) -> Result<RunCallResult, RunControlError> {
        if !matches!(self.status, RunStatus::Running { .. }) {
            return Err(RunControlError::InvalidState);
        }
        if command == RunCommand::Stop {
            self.status = RunStatus::Stopped(StopReason::User);
            return Ok(self.response(None));
        }

        let mut input = self.frozen_input;
        input.period = self.next_month;
        let mut candidate_ledger = self.ledger.clone();
        let mut state = self.opening_state.clone();
        let mut activities = self.activities.clone();
        for update in updates {
            if update.source.trim().is_empty() || update.effective_month != self.next_month {
                return Err(RunControlError::InvalidUpdate(
                    "update source and effective month are required",
                ));
            }
            match &update.value {
                RunUpdateValue::Rent(value) => input.rent = *value,
                RunUpdateValue::RepairCostLimit(value) => input.repair_cost_limit = *value,
                RunUpdateValue::Condition(value) => state.condition = *value,
                RunUpdateValue::Activities(value) => activities = value.clone(),
                RunUpdateValue::StateAdjustment {
                    state: updated,
                    transaction,
                } => {
                    if transaction.period() != self.next_month {
                        return Err(RunControlError::InvalidUpdate(
                            "state adjustment must post in its effective month",
                        ));
                    }
                    candidate_ledger.post(transaction.clone()).map_err(|_| {
                        RunControlError::InvalidUpdate("state adjustment transaction is invalid")
                    })?;
                    let (side, amount) =
                        candidate_ledger.balance(input.cash_account).map_err(|_| {
                            RunControlError::InvalidUpdate(
                                "state adjustment cash balance is unavailable",
                            )
                        })?;
                    let cash = if side == BalanceSide::Debit {
                        amount
                    } else {
                        amount.checked_neg().map_err(|_| {
                            RunControlError::InvalidUpdate(
                                "state adjustment cash balance cannot be represented",
                            )
                        })?
                    };
                    if updated.cash != cash {
                        return Err(RunControlError::InvalidUpdate(
                            "state adjustment cash must match its ledger posting",
                        ));
                    }
                    state = updated.clone();
                }
            }
        }
        if has_duplicate_activity_ids(&activities) {
            return Err(RunControlError::InvalidUpdate(
                "active activity instance IDs must be unique",
            ));
        }
        let distribution = RepairCostDistribution {
            maximum: input.repair_cost_limit,
        };
        let validated = validate_input(&input, &distribution, &candidate_ledger, &state)
            .map_err(RunControlError::Input)?;
        validate_cashflow_support(&input, &activities)
            .map_err(RunControlError::InvalidExecutionInput)?;
        if let Err(error) = executor.validate_inputs(&validated, &candidate_ledger, &activities) {
            if matches!(error, SimulationError::Fatal(_)) {
                self.status = RunStatus::Failed;
                return Err(RunControlError::Simulation(error));
            }
            return Err(RunControlError::InvalidExecutionInput(error));
        }

        let (result, ledger) = match executor.execute(
            &validated,
            &candidate_ledger,
            &activities,
            self.execution_identity,
        ) {
            Ok(completed) => completed,
            Err(error) => {
                self.status = RunStatus::Failed;
                return Err(RunControlError::Simulation(error));
            }
        };
        let period = result.period();
        let opening_transactions: Vec<_> = candidate_ledger.transactions().cloned().collect();
        let returned_transactions: Vec<_> = ledger.transactions().cloned().collect();
        let returned_month_transactions = if returned_transactions.len()
            >= opening_transactions.len()
            && returned_transactions.starts_with(&opening_transactions)
        {
            &returned_transactions[opening_transactions.len()..]
        } else {
            &[]
        };
        let reported_transactions: Vec<_> = result
            .proposal()
            .transactions()
            .iter()
            .cloned()
            .map(crate::ProposedTransaction::into_ledger_transaction)
            .collect();
        let mut expected_activities = activities.clone();
        expected_activities.sort_by_key(|activity| activity.id);
        let cash_matches = ledger
            .balance(input.cash_account)
            .ok()
            .and_then(|(side, amount)| {
                if side == BalanceSide::Debit {
                    Some(amount)
                } else {
                    amount.checked_neg().ok()
                }
            })
            == Some(result.closing_state().cash);
        let output_is_consistent = period == self.next_month
            && self.execution_identity.is_none_or(|identity| {
                identity
                    == RunIdentity {
                        trial_id: result.trial_id(),
                        branch_id: result.branch_id(),
                    }
            })
            && self
                .identity
                .is_none_or(|identity| identity == (result.trial_id(), result.branch_id()))
            && result.opening_state() == validated.opening_state()
            && result.frozen_input() == &input
            && result.activities() == expected_activities
            && cash_matches
            && ledger.chart() == candidate_ledger.chart()
            && ledger.opening_balances() == candidate_ledger.opening_balances()
            && ledger.cash_accounts().eq(candidate_ledger.cash_accounts())
            && returned_transactions.starts_with(&opening_transactions)
            && returned_month_transactions
                .iter()
                .all(|transaction| transaction.period() == self.next_month)
            && returned_month_transactions == reported_transactions;
        if !output_is_consistent {
            self.status = RunStatus::Failed;
            return Err(RunControlError::Simulation(SimulationError::InvalidInput(
                "single-month executor returned an inconsistent result or ledger",
            )));
        }
        let mut reason = if self.config.end == Some(period) {
            Some(StopReason::EndMonth(period))
        } else if self.config.stop_conditions.is_empty() {
            None
        } else {
            let accounts = ledger.cash_accounts().collect::<Vec<_>>();
            let projection = ledger
                .validate_projection(period, period, &accounts)
                .map_err(|error| {
                    self.status = RunStatus::Failed;
                    RunControlError::Simulation(error.into())
                })?
                .project()
                .map_err(|error| {
                    self.status = RunStatus::Failed;
                    RunControlError::Simulation(error.into())
                })?;
            let change = projection
                .cash_flow_statement
                .totals
                .net_change()
                .map_err(|error| {
                    self.status = RunStatus::Failed;
                    RunControlError::Simulation(error.into())
                })?;
            let mut satisfied = None;
            for (index, condition) in self.config.stop_conditions.iter().enumerate() {
                let StopCondition::MonthlyNetCashChangeAtMost(threshold) = condition;
                let within_threshold = change.checked_cmp(*threshold).map_err(|error| {
                    self.status = RunStatus::Failed;
                    RunControlError::Simulation(error.into())
                })? != std::cmp::Ordering::Greater;
                if within_threshold {
                    satisfied = Some(StopReason::Condition(index));
                    break;
                }
            }
            satisfied
        };
        let next_month = if reason.is_none() {
            match period.checked_add_months(1) {
                Ok(next_month) => Some(next_month),
                Err(_) => {
                    reason = Some(StopReason::CalendarLimit(period));
                    None
                }
            }
        } else {
            None
        };

        self.opening_state = result.closing_state().clone();
        self.frozen_input = input;
        self.activities = activities;
        self.ledger = ledger.clone();
        self.history.extend_from_slice(updates);
        self.latest = Some(MonthSnapshot {
            result: result.clone(),
            ledger,
        });
        self.identity = Some((result.trial_id(), result.branch_id()));
        if let Some(reason) = reason {
            self.status = RunStatus::Stopped(reason);
        } else {
            self.next_month = next_month.expect("an unstopped month has a representable successor");
            self.frozen_input.period = self.next_month;
            self.status = RunStatus::Running {
                next_month: self.next_month,
            };
        }
        Ok(self.response(Some(result)))
    }

    fn response(&self, monthly_result: Option<MonthlyResult>) -> RunCallResult {
        let stop_reason = match self.status {
            RunStatus::Stopped(reason) => Some(reason),
            _ => None,
        };
        RunCallResult {
            monthly_result,
            latest_completed_month: self
                .latest
                .as_ref()
                .map(|snapshot| snapshot.result.period()),
            status: self.status.clone(),
            stop_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActivitySample, ActivitySampleValue, AssetCondition, IdentifiedProposal};
    use property_simulation_accounting::{
        Account, AccountCategory, AccountCode, BalanceSide, CashFlowCategory, ChartOfAccounts,
        LedgerEntry, OpeningBalances,
    };
    use property_simulation_domain::{AccountId, BranchId, Currency, TrialId};

    fn month(m: u8) -> YearMonth {
        YearMonth::new(2025, m).unwrap()
    }
    fn id(n: u8) -> AccountId {
        AccountId::from_bytes([n; 16])
    }
    fn fixture(end: Option<YearMonth>, conditions: Vec<StopCondition>) -> SimulationRun {
        let currency = Currency::from_code("CAD").unwrap();
        let cash_account = id(1);
        let rent_account = id(2);
        let repair_account = id(3);
        let equity_account = id(4);
        let chart = ChartOfAccounts::new([
            Account::new_cash_position(cash_account, AccountCode::new(1000).unwrap(), "Cash")
                .unwrap(),
            Account::new(
                rent_account,
                AccountCode::new(4000).unwrap(),
                AccountCategory::Income,
                false,
                "Rent",
            )
            .unwrap(),
            Account::new(
                equity_account,
                AccountCode::new(3000).unwrap(),
                AccountCategory::Equity,
                false,
                "Opening equity",
            )
            .unwrap(),
            Account::new(
                repair_account,
                AccountCode::new(5000).unwrap(),
                AccountCategory::Expense,
                false,
                "Repairs",
            )
            .unwrap(),
        ])
        .unwrap();
        let opening =
            OpeningBalances::new(&chart, [], Some(Money::new(0.into(), currency))).unwrap();
        let ledger = Ledger::new(chart, opening).unwrap();
        let frozen_input = FrozenInput {
            period: month(1),
            rent: Money::new(0.into(), currency),
            rent_account,
            cash_account,
            repair_expense_account: repair_account,
            repair_cost_limit: Money::new(0.into(), currency),
        };
        let state = OpeningState {
            cash: Money::new(0.into(), currency),
            condition: AssetCondition::Available,
        };
        SimulationRun::new(
            RunConfig {
                start: month(1),
                end,
                stop_conditions: conditions,
            },
            state,
            frozen_input,
            ledger,
        )
        .unwrap()
    }

    fn make_fixture_ledger() -> (Ledger, FrozenInput, OpeningState) {
        let currency = Currency::from_code("CAD").unwrap();
        let cash_account = id(1);
        let rent_account = id(2);
        let repair_account = id(3);
        let equity_account = id(4);
        let chart = ChartOfAccounts::new([
            Account::new_cash_position(cash_account, AccountCode::new(1000).unwrap(), "Cash")
                .unwrap(),
            Account::new(
                rent_account,
                AccountCode::new(4000).unwrap(),
                AccountCategory::Income,
                false,
                "Rent",
            )
            .unwrap(),
            Account::new(
                equity_account,
                AccountCode::new(3000).unwrap(),
                AccountCategory::Equity,
                false,
                "Opening equity",
            )
            .unwrap(),
            Account::new(
                repair_account,
                AccountCode::new(5000).unwrap(),
                AccountCategory::Expense,
                false,
                "Repairs",
            )
            .unwrap(),
        ])
        .unwrap();
        let opening =
            OpeningBalances::new(&chart, [], Some(Money::new(0.into(), currency))).unwrap();
        let ledger = Ledger::new(chart, opening).unwrap();
        let input = FrozenInput {
            period: month(1),
            rent: Money::new(0.into(), currency),
            rent_account,
            cash_account,
            repair_expense_account: repair_account,
            repair_cost_limit: Money::new(0.into(), currency),
        };
        let state = OpeningState {
            cash: Money::new(0.into(), currency),
            condition: AssetCondition::Available,
        };
        (ledger, input, state)
    }

    #[derive(Default)]
    struct Executor {
        calls: usize,
        fail: bool,
        seen_activity_count: Option<usize>,
        identity: Option<RunIdentity>,
    }
    impl SingleMonthExecutor for Executor {
        fn validate_inputs(
            &self,
            _input: &ValidatedInput,
            _ledger: &Ledger,
            _activities: &[crate::ActivityInstance],
        ) -> Result<(), SimulationError> {
            Ok(())
        }

        fn execute(
            &mut self,
            input: &ValidatedInput,
            ledger: &Ledger,
            activities: &[crate::ActivityInstance],
            identity: Option<RunIdentity>,
        ) -> Result<(MonthlyResult, Ledger), SimulationError> {
            self.identity = identity;
            self.calls += 1;
            self.seen_activity_count = Some(activities.len());
            if self.fail {
                return Err(SimulationError::InvalidInput("test failure"));
            }
            let state = input.opening_state().clone();
            let samples = activities
                .iter()
                .map(|activity| {
                    let value = match activity.sample_kind {
                        crate::ActivitySampleKind::Rental => ActivitySampleValue::Rental,
                        crate::ActivitySampleKind::Repair => {
                            ActivitySampleValue::Repair(crate::RepairSample::NoRepair)
                        }
                    };
                    ActivitySample::new(activity, input.frozen_input().period, value)
                })
                .collect();
            let result = MonthlyResult::new(
                self.identity
                    .map_or(TrialId::from_bytes([4; 16]), |id| id.trial_id),
                self.identity
                    .map_or(BranchId::from_bytes([5; 16]), |id| id.branch_id),
                input.frozen_input().period,
                state.clone(),
                *input.frozen_input(),
                activities.to_vec(),
                samples,
                state,
                IdentifiedProposal::default(),
            )?;
            Ok((result, ledger.clone()))
        }
    }

    fn update(effective_month: YearMonth) -> RunUpdate {
        RunUpdate {
            effective_month,
            source: "user".into(),
            value: RunUpdateValue::Rent(Money::new(1.into(), Currency::from_code("CAD").unwrap())),
        }
    }

    #[test]
    fn stop_does_not_apply_updates_or_execute_and_retains_latest_month() {
        let mut run = fixture(Some(month(3)), vec![]);
        let mut executor = Executor::default();
        let response = run
            .call(RunCommand::Stop, &[update(month(1))], &mut executor)
            .unwrap();
        assert_eq!(executor.calls, 0);
        assert_eq!(run.history().len(), 0);
        assert_eq!(response.latest_completed_month, None);
        assert_eq!(response.stop_reason, Some(StopReason::User));
    }

    #[test]
    fn end_month_is_inclusive_and_each_continue_advances_one_month() {
        let mut run = fixture(Some(month(2)), vec![]);
        let mut executor = Executor::default();
        let first = run
            .call(RunCommand::Continue, &[update(month(1))], &mut executor)
            .unwrap();
        assert_eq!(first.monthly_result.unwrap().period(), month(1));
        assert_eq!(
            first.status,
            RunStatus::Running {
                next_month: month(2)
            }
        );
        let second = run.call(RunCommand::Continue, &[], &mut executor).unwrap();
        assert_eq!(second.latest_completed_month, Some(month(2)));
        assert_eq!(
            second.status,
            RunStatus::Stopped(StopReason::EndMonth(month(2)))
        );
        assert_eq!(executor.calls, 2);
        assert_eq!(
            run.call(RunCommand::Continue, &[], &mut executor),
            Err(RunControlError::InvalidState)
        );
    }

    #[test]
    fn final_representable_month_completes_then_stops_at_calendar_limit() {
        let final_month = YearMonth::new(u32::MAX, 12).unwrap();
        let (ledger, mut input, state) = make_fixture_ledger();
        input.period = final_month;
        let mut run = SimulationRun::new(
            RunConfig {
                start: final_month,
                end: None,
                stop_conditions: vec![],
            },
            state,
            input,
            ledger,
        )
        .unwrap();
        let mut executor = Executor::default();
        let response = run.call(RunCommand::Continue, &[], &mut executor).unwrap();
        assert_eq!(response.latest_completed_month, Some(final_month));
        assert_eq!(
            response.status,
            RunStatus::Stopped(StopReason::CalendarLimit(final_month))
        );
        assert!(run.latest_snapshot().is_some());
    }

    #[test]
    fn no_configured_stop_boundary_keeps_running_and_user_stop_keeps_last_month() {
        let mut run = fixture(None, vec![]);
        let mut executor = Executor::default();
        let completed = run.call(RunCommand::Continue, &[], &mut executor).unwrap();
        assert_eq!(
            completed.status,
            RunStatus::Running {
                next_month: month(2)
            }
        );
        let stopped = run
            .call(RunCommand::Stop, &[update(month(2))], &mut executor)
            .unwrap();
        assert_eq!(stopped.latest_completed_month, Some(month(1)));
        assert_eq!(stopped.status, RunStatus::Stopped(StopReason::User));
        assert_eq!(executor.calls, 1);
        assert!(run.history().is_empty());
    }

    #[test]
    fn activity_updates_are_passed_to_executor_and_recorded() {
        let mut run = fixture(None, vec![]);
        let activity = crate::ActivityInstance {
            id: property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]),
            slice_id: property_simulation_domain::SliceId::from_bytes([8; 16]),
            version: property_simulation_domain::ModelVersionId::from_bytes([9; 16]),
            sample_kind: crate::ActivitySampleKind::Rental,
        };
        let update = RunUpdate {
            effective_month: month(1),
            source: "scenario edit".into(),
            value: RunUpdateValue::Activities(vec![activity]),
        };
        let mut executor = Executor::default();
        run.call(
            RunCommand::Continue,
            std::slice::from_ref(&update),
            &mut executor,
        )
        .unwrap();
        assert_eq!(executor.seen_activity_count, Some(1));
        assert_eq!(run.history(), &[update]);
    }

    #[test]
    fn equal_cash_threshold_stops_after_successful_month() {
        let zero = Money::new(0.into(), Currency::from_code("CAD").unwrap());
        let mut run = fixture(None, vec![StopCondition::MonthlyNetCashChangeAtMost(zero)]);
        let mut executor = Executor::default();
        let response = run.call(RunCommand::Continue, &[], &mut executor).unwrap();
        assert_eq!(response.latest_completed_month, Some(month(1)));
        assert_eq!(response.stop_reason, Some(StopReason::Condition(0)));
    }

    #[test]
    fn negative_month_end_cash_does_not_implicitly_stop_an_unbounded_run() {
        struct NegativeCashExecutor;
        impl SingleMonthExecutor for NegativeCashExecutor {
            fn validate_inputs(
                &self,
                _input: &ValidatedInput,
                _ledger: &Ledger,
                activities: &[crate::ActivityInstance],
            ) -> Result<(), SimulationError> {
                if activities.len() == 1 {
                    Ok(())
                } else {
                    Err(SimulationError::InvalidInput("one test activity required"))
                }
            }

            fn execute(
                &mut self,
                input: &ValidatedInput,
                opening_ledger: &Ledger,
                activities: &[crate::ActivityInstance],
                _identity: Option<RunIdentity>,
            ) -> Result<(MonthlyResult, Ledger), SimulationError> {
                let currency = input.opening_state().cash.currency();
                let amount = Money::new(1.into(), currency);
                let mut ledger = opening_ledger.clone();
                let transaction_id =
                    property_simulation_domain::LedgerTransactionId::from_bytes([10; 16]);
                let instance_id =
                    property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]);
                let slice_id = property_simulation_domain::SliceId::from_bytes([8; 16]);
                let version = property_simulation_domain::ModelVersionId::from_bytes([9; 16]);
                let key = "negative-cash-test";
                let entries = [
                    LedgerEntry::new(id(3), BalanceSide::Debit, amount),
                    LedgerEntry::new(id(1), BalanceSide::Credit, amount)
                        .with_cash_flow_category(CashFlowCategory::Operating),
                ];
                ledger.post(
                    property_simulation_accounting::LedgerTransaction::new(
                        transaction_id,
                        input.frozen_input().period,
                        entries,
                    )
                    .with_source(
                        property_simulation_accounting::LedgerTransactionSource {
                            instance_id,
                            slice_id,
                            model_version: version,
                            output_key: key.into(),
                            source_event_id: None,
                        },
                    ),
                )?;
                let activity = crate::ActivityInstance {
                    id: instance_id,
                    slice_id,
                    version,
                    sample_kind: crate::ActivitySampleKind::Repair,
                };
                if activities != [activity.clone()] {
                    return Err(SimulationError::InvalidInput(
                        "unexpected test activity instance",
                    ));
                }
                let trial = TrialId::from_bytes([4; 16]);
                let branch = BranchId::from_bytes([5; 16]);
                let proposal = IdentifiedProposal {
                    identity: Some((trial, branch, input.frozen_input().period)),
                    transactions: vec![crate::ProposedTransaction {
                        id: transaction_id,
                        instance_id: activity.id,
                        slice_id: activity.slice_id,
                        version: activity.version,
                        period: input.frozen_input().period,
                        key: key.into(),
                        source_event_id: None,
                        entries: entries.to_vec(),
                    }],
                    ..IdentifiedProposal::default()
                };
                let closing = OpeningState {
                    cash: amount.checked_neg()?,
                    condition: input.opening_state().condition,
                };
                let result = MonthlyResult::new(
                    trial,
                    branch,
                    input.frozen_input().period,
                    input.opening_state().clone(),
                    *input.frozen_input(),
                    vec![activity.clone()],
                    vec![ActivitySample::new(
                        &activity,
                        input.frozen_input().period,
                        ActivitySampleValue::Repair(crate::RepairSample::NoRepair),
                    )],
                    closing,
                    proposal,
                )?;
                Ok((result, ledger))
            }
        }

        let (ledger, mut input, state) = make_fixture_ledger();
        input.repair_cost_limit = Money::new(15.into(), input.repair_cost_limit.currency());
        let activity = crate::ActivityInstance {
            id: property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]),
            slice_id: property_simulation_domain::SliceId::from_bytes([8; 16]),
            version: property_simulation_domain::ModelVersionId::from_bytes([9; 16]),
            sample_kind: crate::ActivitySampleKind::Repair,
        };
        let mut run = SimulationRun::new_with_activities(
            RunConfig {
                start: month(1),
                end: None,
                stop_conditions: vec![],
            },
            state,
            input,
            ledger,
            vec![activity],
        )
        .unwrap();
        let mut executor = NegativeCashExecutor;
        let response = run.call(RunCommand::Continue, &[], &mut executor).unwrap();
        assert_eq!(
            response.status,
            RunStatus::Running {
                next_month: month(2)
            }
        );
        assert!(run.opening_state().cash.is_negative());
    }

    #[test]
    fn stop_conditions_use_or_semantics() {
        let currency = Currency::from_code("CAD").unwrap();
        let mut run = fixture(
            None,
            vec![
                StopCondition::MonthlyNetCashChangeAtMost(Money::new((-1).into(), currency)),
                StopCondition::MonthlyNetCashChangeAtMost(Money::new(0.into(), currency)),
            ],
        );
        let mut executor = Executor::default();
        let result = run.call(RunCommand::Continue, &[], &mut executor).unwrap();
        assert_eq!(result.stop_reason, Some(StopReason::Condition(1)));
    }

    #[test]
    fn economic_state_update_requires_matching_balanced_ledger_posting() {
        use property_simulation_accounting::LedgerTransaction;
        let mut run = fixture(None, vec![]);
        let currency = Currency::from_code("CAD").unwrap();
        let updated_state = OpeningState {
            cash: Money::new(5.into(), currency),
            condition: AssetCondition::NeedsRepair,
        };
        let transaction = LedgerTransaction::new(
            property_simulation_domain::LedgerTransactionId::from_bytes([8; 16]),
            month(1),
            [
                LedgerEntry::new(id(1), BalanceSide::Debit, updated_state.cash)
                    .with_cash_flow_category(CashFlowCategory::Financing),
                LedgerEntry::new(id(4), BalanceSide::Credit, updated_state.cash),
            ],
        );
        let update = RunUpdate {
            effective_month: month(1),
            source: "opening state correction".into(),
            value: RunUpdateValue::StateAdjustment {
                state: updated_state,
                transaction,
            },
        };
        let mut executor = Executor::default();
        let result = run
            .call(
                RunCommand::Continue,
                std::slice::from_ref(&update),
                &mut executor,
            )
            .unwrap();
        assert_eq!(result.latest_completed_month, Some(month(1)));
        assert_eq!(run.history(), &[update]);
        assert_eq!(
            run.ledger().balance(id(1)).unwrap().1,
            Money::new(5.into(), currency)
        );

        let mut rejected = fixture(None, vec![]);
        let invalid = RunUpdate {
            effective_month: month(1),
            source: "mismatched state".into(),
            value: RunUpdateValue::StateAdjustment {
                state: OpeningState {
                    cash: Money::new(6.into(), currency),
                    condition: AssetCondition::Available,
                },
                transaction: LedgerTransaction::new(
                    property_simulation_domain::LedgerTransactionId::from_bytes([9; 16]),
                    month(1),
                    [
                        LedgerEntry::new(id(1), BalanceSide::Debit, Money::new(5.into(), currency))
                            .with_cash_flow_category(CashFlowCategory::Financing),
                        LedgerEntry::new(
                            id(4),
                            BalanceSide::Credit,
                            Money::new(5.into(), currency),
                        ),
                    ],
                ),
            },
        };
        let mut executor = Executor::default();
        assert!(matches!(
            rejected.call(RunCommand::Continue, &[invalid], &mut executor),
            Err(RunControlError::InvalidUpdate(_))
        ));
        assert_eq!(executor.calls, 0);
        assert!(rejected.ledger().transactions().next().is_none());
    }

    #[test]
    fn invalid_update_does_not_call_executor_or_change_run_state() {
        let mut run = fixture(None, vec![]);
        let mut executor = Executor::default();
        let result = run.call(RunCommand::Continue, &[update(month(2))], &mut executor);
        assert!(matches!(result, Err(RunControlError::InvalidUpdate(_))));
        assert_eq!(executor.calls, 0);
        assert_eq!(
            run.status(),
            &RunStatus::Running {
                next_month: month(1)
            }
        );
        assert!(run.latest_snapshot().is_none());
    }

    #[test]
    fn failed_month_has_no_successful_snapshot_and_run_cannot_continue() {
        let mut run = fixture(None, vec![]);
        let mut executor = Executor {
            calls: 0,
            fail: true,
            seen_activity_count: None,
            identity: None,
        };
        assert!(matches!(
            run.call(RunCommand::Continue, &[], &mut executor),
            Err(RunControlError::Simulation(_))
        ));
        assert_eq!(run.status(), &RunStatus::Failed);
        assert!(run.latest_snapshot().is_none());
        assert_eq!(
            run.call(RunCommand::Continue, &[], &mut executor),
            Err(RunControlError::InvalidState)
        );
    }

    #[test]
    fn rejects_duplicate_activity_updates_before_execution() {
        let mut run = fixture(None, vec![]);
        let activity = crate::ActivityInstance {
            id: property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]),
            slice_id: property_simulation_domain::SliceId::from_bytes([8; 16]),
            version: property_simulation_domain::ModelVersionId::from_bytes([9; 16]),
            sample_kind: crate::ActivitySampleKind::Rental,
        };
        let update = RunUpdate {
            effective_month: month(1),
            source: "duplicate activity".into(),
            value: RunUpdateValue::Activities(vec![activity.clone(), activity]),
        };
        let mut executor = Executor::default();
        assert!(matches!(
            run.call(RunCommand::Continue, &[update], &mut executor),
            Err(RunControlError::InvalidUpdate(_))
        ));
        assert_eq!(executor.calls, 0);
        assert!(run.history().is_empty());
        assert!(run.latest_snapshot().is_none());
    }

    #[test]
    fn executor_specific_input_rejection_is_retryable_and_does_not_execute() {
        struct RejectingExecutor;
        impl SingleMonthExecutor for RejectingExecutor {
            fn validate_inputs(
                &self,
                _input: &ValidatedInput,
                _ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
            ) -> Result<(), SimulationError> {
                Err(SimulationError::InvalidInput(
                    "unsupported activity configuration",
                ))
            }

            fn execute(
                &mut self,
                _input: &ValidatedInput,
                _ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
                _identity: Option<RunIdentity>,
            ) -> Result<(MonthlyResult, Ledger), SimulationError> {
                panic!("input validation failure must prevent the monthly transition")
            }
        }

        let mut run = fixture(None, vec![]);
        let result = run.call(RunCommand::Continue, &[], &mut RejectingExecutor);
        assert!(matches!(
            result,
            Err(RunControlError::InvalidExecutionInput(_))
        ));
        assert_eq!(
            run.status(),
            &RunStatus::Running {
                next_month: month(1)
            }
        );
        assert!(run.latest_snapshot().is_none());
        assert!(run.history().is_empty());
    }

    #[test]
    fn fatal_executor_preflight_error_fails_the_run() {
        struct FatalPreflightExecutor;
        impl SingleMonthExecutor for FatalPreflightExecutor {
            fn validate_inputs(
                &self,
                _input: &ValidatedInput,
                _ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
            ) -> Result<(), SimulationError> {
                Err(SimulationError::Fatal(crate::FatalSimulationFailure {
                    phase: crate::FatalSimulationPhase::SliceTransition,
                    period: month(1),
                    trial_id: None,
                    branch_id: None,
                    activity_instance_id: None,
                    source: Box::new(SimulationError::InvalidInput("internal preflight defect")),
                }))
            }

            fn execute(
                &mut self,
                _input: &ValidatedInput,
                _ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
                _identity: Option<RunIdentity>,
            ) -> Result<(MonthlyResult, Ledger), SimulationError> {
                panic!("fatal preflight errors must prevent the monthly transition")
            }
        }

        let mut run = fixture(None, vec![]);
        assert!(matches!(
            run.call(RunCommand::Continue, &[], &mut FatalPreflightExecutor),
            Err(RunControlError::Simulation(SimulationError::Fatal(_)))
        ));
        assert_eq!(run.status(), &RunStatus::Failed);
        assert!(run.latest_snapshot().is_none());
    }

    #[test]
    fn inconsistent_executor_output_fails_without_committing_a_snapshot() {
        struct InconsistentExecutor;
        impl SingleMonthExecutor for InconsistentExecutor {
            fn validate_inputs(
                &self,
                _input: &ValidatedInput,
                _ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
            ) -> Result<(), SimulationError> {
                Ok(())
            }

            fn execute(
                &mut self,
                input: &ValidatedInput,
                ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
                _identity: Option<RunIdentity>,
            ) -> Result<(MonthlyResult, Ledger), SimulationError> {
                let mut opening = input.opening_state().clone();
                opening.condition = AssetCondition::NeedsRepair;
                let result = MonthlyResult::new(
                    TrialId::from_bytes([4; 16]),
                    BranchId::from_bytes([5; 16]),
                    input.frozen_input().period,
                    opening.clone(),
                    *input.frozen_input(),
                    vec![],
                    vec![],
                    opening,
                    IdentifiedProposal::default(),
                )?;
                Ok((result, ledger.clone()))
            }
        }

        let (ledger, input, state) = make_fixture_ledger();
        let activity = crate::ActivityInstance {
            id: property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]),
            slice_id: property_simulation_domain::SliceId::from_bytes([8; 16]),
            version: property_simulation_domain::ModelVersionId::from_bytes([9; 16]),
            sample_kind: crate::ActivitySampleKind::Repair,
        };
        let mut run = SimulationRun::new_with_activities(
            RunConfig {
                start: month(1),
                end: None,
                stop_conditions: vec![],
            },
            state,
            input,
            ledger,
            vec![activity],
        )
        .unwrap();
        let mut executor = InconsistentExecutor;
        assert!(matches!(
            run.call(RunCommand::Continue, &[], &mut executor),
            Err(RunControlError::Simulation(_))
        ));
        assert_eq!(run.status(), &RunStatus::Failed);
        assert!(run.latest_snapshot().is_none());
        assert_eq!(run.opening_state().condition, AssetCondition::Available);
    }

    #[test]
    fn opening_ledger_cannot_contain_postings_in_the_run_interval() {
        let (mut ledger, input, state) = make_fixture_ledger();
        let currency = Currency::from_code("CAD").unwrap();
        let amount = Money::new(1.into(), currency);
        ledger
            .post(property_simulation_accounting::LedgerTransaction::new(
                property_simulation_domain::LedgerTransactionId::from_bytes([11; 16]),
                month(1),
                [
                    LedgerEntry::new(id(3), BalanceSide::Debit, amount),
                    LedgerEntry::new(id(1), BalanceSide::Credit, amount)
                        .with_cash_flow_category(CashFlowCategory::Operating),
                ],
            ))
            .unwrap();
        assert!(matches!(
            SimulationRun::new(
                RunConfig {
                    start: month(1),
                    end: None,
                    stop_conditions: vec![],
                },
                state,
                input,
                ledger,
            ),
            Err(RunControlError::InvalidConfig(_))
        ));
    }

    #[test]
    fn rejects_ledger_postings_that_disagree_with_monthly_result() {
        struct MismatchedPostingExecutor;
        impl SingleMonthExecutor for MismatchedPostingExecutor {
            fn validate_inputs(
                &self,
                _input: &ValidatedInput,
                _ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
            ) -> Result<(), SimulationError> {
                Ok(())
            }

            fn execute(
                &mut self,
                input: &ValidatedInput,
                opening_ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
                _identity: Option<RunIdentity>,
            ) -> Result<(MonthlyResult, Ledger), SimulationError> {
                let amount = Money::new(1.into(), input.opening_state().cash.currency());
                let mut ledger = opening_ledger.clone();
                let transaction_id =
                    property_simulation_domain::LedgerTransactionId::from_bytes([12; 16]);
                ledger.post(property_simulation_accounting::LedgerTransaction::new(
                    transaction_id,
                    input.frozen_input().period,
                    [
                        LedgerEntry::new(id(3), BalanceSide::Debit, amount),
                        LedgerEntry::new(id(1), BalanceSide::Credit, amount)
                            .with_cash_flow_category(CashFlowCategory::Operating),
                    ],
                ))?;
                let result = MonthlyResult::new(
                    TrialId::from_bytes([4; 16]),
                    BranchId::from_bytes([5; 16]),
                    input.frozen_input().period,
                    input.opening_state().clone(),
                    *input.frozen_input(),
                    vec![],
                    vec![],
                    OpeningState {
                        cash: amount.checked_neg()?,
                        condition: input.opening_state().condition,
                    },
                    IdentifiedProposal {
                        transactions: vec![crate::ProposedTransaction {
                            id: transaction_id,
                            instance_id: property_simulation_domain::ActivityInstanceId::from_bytes(
                                [7; 16],
                            ),
                            slice_id: property_simulation_domain::SliceId::from_bytes([8; 16]),
                            version: property_simulation_domain::ModelVersionId::from_bytes(
                                [9; 16],
                            ),
                            period: input.frozen_input().period,
                            key: "reported-posting".into(),
                            source_event_id: None,
                            // Keep the same transaction ID and cash movement, but report
                            // a different expense account than the ledger actually posted.
                            entries: vec![
                                LedgerEntry::new(id(2), BalanceSide::Debit, amount),
                                LedgerEntry::new(id(1), BalanceSide::Credit, amount)
                                    .with_cash_flow_category(CashFlowCategory::Operating),
                            ],
                        }],
                        ..IdentifiedProposal::default()
                    },
                )?;
                Ok((result, ledger))
            }
        }

        let mut run = fixture(None, vec![]);
        assert!(matches!(
            run.call(RunCommand::Continue, &[], &mut MismatchedPostingExecutor),
            Err(RunControlError::Simulation(_))
        ));
        assert_eq!(run.status(), &RunStatus::Failed);
        assert!(run.latest_snapshot().is_none());
    }

    #[test]
    fn trial_and_branch_identity_cannot_change_mid_run() {
        struct ChangingIdentityExecutor {
            calls: u8,
        }
        impl SingleMonthExecutor for ChangingIdentityExecutor {
            fn validate_inputs(
                &self,
                _input: &ValidatedInput,
                _ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
            ) -> Result<(), SimulationError> {
                Ok(())
            }

            fn execute(
                &mut self,
                input: &ValidatedInput,
                ledger: &Ledger,
                _activities: &[crate::ActivityInstance],
                _identity: Option<RunIdentity>,
            ) -> Result<(MonthlyResult, Ledger), SimulationError> {
                self.calls += 1;
                let state = input.opening_state().clone();
                let result = MonthlyResult::new(
                    TrialId::from_bytes([self.calls; 16]),
                    BranchId::from_bytes([5; 16]),
                    input.frozen_input().period,
                    state.clone(),
                    *input.frozen_input(),
                    vec![],
                    vec![],
                    state,
                    IdentifiedProposal::default(),
                )?;
                Ok((result, ledger.clone()))
            }
        }

        let mut run = fixture(None, vec![]);
        let mut executor = ChangingIdentityExecutor { calls: 0 };
        let first = run.call(RunCommand::Continue, &[], &mut executor).unwrap();
        assert_eq!(first.latest_completed_month, Some(month(1)));
        assert!(matches!(
            run.call(RunCommand::Continue, &[], &mut executor),
            Err(RunControlError::Simulation(_))
        ));
        assert_eq!(run.status(), &RunStatus::Failed);
        assert_eq!(
            run.latest_snapshot()
                .map(|snapshot| snapshot.result.period()),
            Some(month(1))
        );
    }

    #[test]
    fn archive_branches_from_saved_snapshot_and_keeps_latest_settings_independent() {
        use crate::{BranchRecord, SimulationArchive, saved_month};
        use property_simulation_domain::{RunId, SnapshotId};

        let mut run = fixture(None, vec![]);
        let mut executor = Executor::default();
        run.call(RunCommand::Continue, &[], &mut executor).unwrap();
        let snapshot = run.latest_snapshot().unwrap().clone();
        let parent_id = SnapshotId::from_bytes([9; 16]);
        let run_id = RunId::from_bytes([7; 16]);
        let branch_id = snapshot.result.branch_id();
        let mut archive = SimulationArchive::default();
        archive
            .register_branch(BranchRecord {
                id: branch_id,
                run_id,
                trial_id: snapshot.result.trial_id(),
                start_month: month(1),
                parent_snapshot: None,
                fork_month: None,
            })
            .unwrap();
        archive
            .save_month(saved_month(parent_id, run_id, snapshot.clone(), None))
            .unwrap();
        let parent_summary = archive.cash_flow_summary(run_id).unwrap();
        assert_eq!(
            parent_summary.by_month.keys().copied().collect::<Vec<_>>(),
            vec![month(1)]
        );
        assert!(parent_summary.by_activity.is_empty());
        let mut mismatched_start_archive = SimulationArchive::default();
        mismatched_start_archive
            .register_branch(BranchRecord {
                id: branch_id,
                run_id,
                trial_id: snapshot.result.trial_id(),
                start_month: month(2),
                parent_snapshot: None,
                fork_month: None,
            })
            .unwrap();
        assert_eq!(
            mismatched_start_archive.save_month(saved_month(
                parent_id,
                run_id,
                snapshot.clone(),
                None
            )),
            Err(crate::ArchiveError::NonContiguousMonth)
        );
        let child_branch = BranchId::from_bytes([6; 16]);
        let mut child = archive
            .create_branch(
                parent_id,
                RunId::from_bytes([8; 16]),
                child_branch,
                TrialId::from_bytes([3; 16]),
                Some(month(3)),
                vec![],
            )
            .unwrap();
        assert_eq!(child.next_month(), month(2));
        assert_eq!(child.opening_state(), snapshot.result.closing_state());
        assert_eq!(child.ledger(), &snapshot.ledger);
        assert_eq!(archive.snapshot(parent_id).unwrap().snapshot, snapshot);
        assert_eq!(
            archive.branch(child_branch).unwrap().fork_month,
            Some(month(1))
        );
        let mut branch_executor = Executor::default();
        let branch_result = child
            .call(RunCommand::Continue, &[], &mut branch_executor)
            .unwrap()
            .monthly_result
            .unwrap();
        assert_eq!(branch_result.trial_id(), TrialId::from_bytes([3; 16]));
        assert_eq!(branch_result.branch_id(), child_branch);
        assert_eq!(branch_result.period(), month(2));
        let child_run_id = RunId::from_bytes([8; 16]);
        let child_snapshot_id = SnapshotId::from_bytes([10; 16]);
        archive
            .save_month(saved_month(
                child_snapshot_id,
                child_run_id,
                child.latest_snapshot().unwrap().clone(),
                Some(parent_id),
            ))
            .unwrap();
        let child_summary = archive.cash_flow_summary(child_run_id).unwrap();
        assert_eq!(
            child_summary.by_month.keys().copied().collect::<Vec<_>>(),
            vec![month(2)]
        );
        assert!(!child_summary.by_month.contains_key(&month(1)));
        assert_eq!(
            child_summary.cumulative.operating,
            Money::new(0.into(), child_summary.cumulative.operating.currency())
        );
        let mut latest_input = snapshot.result.frozen_input().to_owned();
        latest_input.rent = Money::new(125.into(), latest_input.rent.currency());
        archive.update_latest_settings(crate::UserSettings {
            config: run.config().clone(),
            opening_state: snapshot.result.closing_state().clone(),
            frozen_input: latest_input,
            activities: snapshot.result.activities().to_vec(),
        });
        assert_eq!(
            archive.latest_settings().unwrap().frozen_input.rent,
            latest_input.rent
        );
        assert_eq!(archive.snapshot(parent_id).unwrap().snapshot, snapshot);
    }

    #[test]
    fn archive_cashflow_summary_derives_nonzero_month_activity_and_run_totals() {
        use crate::{
            BranchRecord, IdentifiedProposal, ProposedTransaction, SimulationArchive, saved_month,
        };
        use property_simulation_domain::{
            ActivityInstanceId, ModelVersionId, RunId, SliceId, SnapshotId,
        };

        let (mut ledger, mut input, state) = make_fixture_ledger();
        input.rent = Money::new(25.into(), input.rent.currency());
        let activity = crate::ActivityInstance {
            id: ActivityInstanceId::from_bytes([21; 16]),
            slice_id: SliceId::from_bytes([22; 16]),
            version: ModelVersionId::from_bytes([23; 16]),
            sample_kind: crate::ActivitySampleKind::Rental,
        };
        let trial = TrialId::from_bytes([24; 16]);
        let branch = BranchId::from_bytes([25; 16]);
        let transaction = ProposedTransaction {
            id: property_simulation_domain::LedgerTransactionId::from_bytes([26; 16]),
            instance_id: activity.id,
            slice_id: activity.slice_id,
            version: activity.version,
            period: month(1),
            key: "rent.received".into(),
            source_event_id: None,
            entries: vec![
                LedgerEntry::new(id(1), BalanceSide::Debit, input.rent)
                    .with_cash_flow_category(CashFlowCategory::Operating),
                LedgerEntry::new(id(2), BalanceSide::Credit, input.rent),
            ],
        };
        ledger
            .post(transaction.clone().into_ledger_transaction())
            .unwrap();
        let proposal = IdentifiedProposal {
            identity: Some((trial, branch, month(1))),
            events: vec![],
            effects: vec![],
            transactions: vec![transaction],
        };
        let closing = OpeningState {
            cash: input.rent,
            condition: state.condition,
        };
        let result = MonthlyResult::new(
            trial,
            branch,
            month(1),
            state.clone(),
            input,
            vec![activity.clone()],
            vec![ActivitySample::new(
                &activity,
                month(1),
                ActivitySampleValue::Rental,
            )],
            closing,
            proposal,
        )
        .unwrap();
        let snapshot = MonthSnapshot { result, ledger };
        let run = RunId::from_bytes([27; 16]);
        let snapshot_id = SnapshotId::from_bytes([28; 16]);
        let mut archive = SimulationArchive::default();
        archive
            .register_branch(BranchRecord {
                id: branch,
                run_id: run,
                trial_id: trial,
                start_month: month(1),
                parent_snapshot: None,
                fork_month: None,
            })
            .unwrap();
        archive
            .save_month(saved_month(snapshot_id, run, snapshot, None))
            .unwrap();
        let summary = archive.cash_flow_summary(run).unwrap();
        assert_eq!(
            summary.by_month[&month(1)].operating,
            Money::new(25.into(), state.cash.currency())
        );
        assert_eq!(
            summary.by_activity[&activity.id].operating,
            Money::new(25.into(), state.cash.currency())
        );
        assert_eq!(
            summary.cumulative.operating,
            Money::new(25.into(), state.cash.currency())
        );
    }
}

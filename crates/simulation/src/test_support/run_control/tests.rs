use super::*;
use crate::{ActivitySample, ActivitySampleValue, AssetCondition, IdentifiedProposal};
use property_simulation_accounting::{
    Account, AccountCategory, AccountCode, BalanceSide, CashFlowCategory, ChartOfAccounts,
    LedgerEntry, OpeningBalance, OpeningBalances,
};
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, Currency, SliceId, TrialId,
};
use property_simulation_models::RandomSource;

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
        Account::new_cash_position(cash_account, AccountCode::new(1000).unwrap(), "Cash").unwrap(),
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
    let opening = OpeningBalances::new(&chart, [], Some(Money::new(0.into(), currency))).unwrap();
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
        Account::new_cash_position(cash_account, AccountCode::new(1000).unwrap(), "Cash").unwrap(),
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
    let opening = OpeningBalances::new(&chart, [], Some(Money::new(0.into(), currency))).unwrap();
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
    require_randomness: bool,
}
impl SingleMonthExecutor for Executor {
    fn requires_random_streams(&self) -> bool {
        self.require_randomness
    }

    fn create_random_streams(
        &mut self,
        _identity: Option<RunIdentity>,
    ) -> Result<Option<property_simulation_randomness::RandomStreams>, &'static str> {
        Ok(None)
    }

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
            return Err(SimulationError::Fatal(crate::FatalSimulationFailure {
                phase: crate::FatalSimulationPhase::LedgerCommit,
                period: input.frozen_input().period,
                trial_id: identity.map(|value| value.trial_id),
                branch_id: identity.map(|value| value.branch_id),
                activity_instance_id: None,
                source: Box::new(SimulationError::InvalidInput("test failure")),
            }));
        }
        let state = input.opening_state().clone();
        let samples = activities
            .iter()
            .map(|activity| {
                let value = match activity.configuration.sample_kind() {
                    crate::ActivitySampleKind::Rental => ActivitySampleValue::Rental,
                    crate::ActivitySampleKind::Repair => {
                        ActivitySampleValue::Repair(crate::RepairSample::NoRepair)
                    }
                    crate::ActivitySampleKind::Loan => ActivitySampleValue::Loan(None),
                    crate::ActivitySampleKind::PropertyTax => ActivitySampleValue::PropertyTax,
                    crate::ActivitySampleKind::Utility => ActivitySampleValue::Utility(0),
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

#[test]
fn requesting_random_streams_without_run_identity_is_an_execution_error() {
    let mut run = fixture(None, vec![]);
    let mut executor = Executor {
        require_randomness: true,
        ..Executor::default()
    };
    let error = run
        .call(RunCommand::Continue, &[], &mut executor)
        .unwrap_err();
    assert_eq!(
        error,
        RunControlError::Randomness("random streams require a trial identity")
    );
    assert_eq!(executor.calls, 0);
    assert!(run.latest_snapshot().is_none());
}

#[test]
fn random_run_key_is_allocated_once_and_reused_across_months() {
    struct RandomExecutor {
        inner: Executor,
        factory: property_simulation_randomness::RunKeyFactory,
        allocations: usize,
        draws: Vec<u64>,
    }

    impl SingleMonthExecutor for RandomExecutor {
        fn validate_inputs(
            &self,
            input: &ValidatedInput,
            ledger: &Ledger,
            activities: &[crate::ActivityInstance],
        ) -> Result<(), SimulationError> {
            self.inner.validate_inputs(input, ledger, activities)
        }

        fn requires_random_streams(&self) -> bool {
            true
        }

        fn create_random_streams(
            &mut self,
            identity: Option<RunIdentity>,
        ) -> Result<Option<property_simulation_randomness::RandomStreams>, &'static str> {
            self.allocations += 1;
            Ok(Some(self.factory.run_streams(
                identity.ok_or("missing run identity")?.trial_id,
            )))
        }

        fn execute(
            &mut self,
            input: &ValidatedInput,
            ledger: &Ledger,
            activities: &[crate::ActivityInstance],
            identity: Option<RunIdentity>,
        ) -> Result<(MonthlyResult, Ledger), SimulationError> {
            self.inner.execute(input, ledger, activities, identity)
        }

        fn execute_with_random_streams(
            &mut self,
            input: &ValidatedInput,
            ledger: &Ledger,
            activities: &[crate::ActivityInstance],
            identity: Option<RunIdentity>,
            streams: Option<&property_simulation_randomness::RandomStreams>,
        ) -> Result<(MonthlyResult, Ledger), SimulationError> {
            let run_identity = identity.unwrap();
            let mut stream = streams
                .unwrap()
                .stream(&property_simulation_randomness::SemanticIdentity {
                    trial_id: run_identity.trial_id,
                    category: property_simulation_randomness::RandomCategory::Property,
                    slice_id: SliceId::from_bytes([2; 16]),
                    activity_id: ActivityInstanceId::from_bytes([3; 16]),
                    period: input.frozen_input().period,
                    purpose: "test".into(),
                })
                .unwrap();
            self.draws.push(stream.next_u64().unwrap());
            self.inner.execute(input, ledger, activities, identity)
        }
    }

    let trial_id = TrialId::from_bytes([8; 16]);
    let branch_id = BranchId::from_bytes([9; 16]);
    let (ledger, frozen_input, opening_state) = make_fixture_ledger();
    let mut run = SimulationRun::new_with_identity(
        RunConfig {
            start: month(1),
            end: Some(month(2)),
            stop_conditions: vec![],
        },
        opening_state,
        frozen_input,
        ledger,
        RunIdentity {
            trial_id,
            branch_id,
        },
    )
    .unwrap();
    let mut executor = RandomExecutor {
        inner: Executor::default(),
        factory: property_simulation_randomness::RunKeyFactory::new(
            property_simulation_randomness::RootKey::from_seed([7; 32]),
        ),
        allocations: 0,
        draws: Vec::new(),
    };

    run.call(RunCommand::Continue, &[], &mut executor).unwrap();
    run.call(RunCommand::Continue, &[], &mut executor).unwrap();

    assert_eq!(executor.allocations, 1);
    assert_eq!(executor.draws.len(), 2);
    assert_ne!(executor.draws[0], executor.draws[1]);
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
        configuration: crate::ActivityConfiguration::Rental,
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
    assert_eq!(run.history(), std::slice::from_ref(&update));
}

#[test]
fn loan_run_requires_a_continuation_for_each_loan_activity() {
    let (ledger, input, state) = make_fixture_ledger();
    let activity = crate::ActivityInstance {
        id: ActivityInstanceId::from_bytes([31; 16]),
        slice_id: SliceId::from_bytes([32; 16]),
        configuration: crate::ActivityConfiguration::Loan,
    };
    let config = RunConfig {
        start: month(1),
        end: None,
        stop_conditions: vec![],
    };
    assert!(matches!(
        SimulationRun::new_with_activities(
            config.clone(),
            state.clone(),
            input,
            ledger.clone(),
            vec![activity.clone()],
        ),
        Err(RunControlError::InvalidConfig(_))
    ));
    let loan = property_simulation_models::FixedRateLoan {
        opening_balance: Money::new(100.into(), input.rent.currency()),
        remaining_months: 12,
        annual_rate: rust_decimal::Decimal::new(5, 2),
        scheduled_payment: None,
    };
    let continuation = LoanContinuation {
        instance_id: activity.id,
        slice_id: activity.slice_id,
        loan: Some(loan),
    };
    let run = SimulationRun::new_with_activities_and_loans(
        config,
        state,
        input,
        ledger,
        vec![activity],
        vec![continuation.clone()],
    )
    .unwrap();
    assert_eq!(run.loan_continuations(), &[continuation]);
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
            let instance_id = property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]);
            let slice_id = property_simulation_domain::SliceId::from_bytes([8; 16]);
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
                        output_key: key.into(),
                        source_event_id: None,
                    },
                ),
            )?;
            let activity = crate::ActivityInstance {
                id: instance_id,
                slice_id,
                configuration: crate::ActivityConfiguration::Repair,
            };
            if activities != [activity.clone()] {
                return Err(SimulationError::InvalidInput(
                    "unexpected test activity instance",
                ));
            }
            let trial = TrialId::from_bytes([4; 16]);
            let branch = BranchId::from_bytes([5; 16]);
            let proposal = IdentifiedProposal {
                period: Some(input.frozen_input().period),
                transactions: vec![crate::ProposedTransaction {
                    id: transaction_id,
                    instance_id: activity.id,
                    slice_id: activity.slice_id,
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
        configuration: crate::ActivityConfiguration::Repair,
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
            state: updated_state.clone(),
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
    assert_eq!(run.history(), std::slice::from_ref(&update));
    let completed = run.latest_snapshot().unwrap().result();
    assert_eq!(completed.input_postings().len(), 1);
    assert_eq!(completed.input_updates(), std::slice::from_ref(&update));
    assert_eq!(completed.input_effects().len(), 1);
    assert_eq!(
        completed.input_postings()[0].source,
        "opening state correction"
    );
    assert_eq!(
        completed.input_postings()[0].transaction.id(),
        property_simulation_domain::LedgerTransactionId::from_bytes([8; 16])
    );
    assert_eq!(completed.input_cash_flow_contributions().len(), 1);
    assert_eq!(completed.input_effects()[0].id.as_bytes(), &[8; 16]);
    assert_eq!(
        completed.input_effects()[0].source,
        "opening state correction"
    );
    assert_eq!(completed.input_effects()[0].period, month(1));
    assert_eq!(
        completed.input_effects()[0].previous.cash,
        Money::new(0.into(), currency)
    );
    assert_eq!(
        completed.input_effects()[0].previous.condition,
        AssetCondition::Available
    );
    assert_eq!(completed.input_effects()[0].updated, updated_state);
    assert_eq!(
        completed.input_effects()[0].transaction_id.as_bytes(),
        &[8; 16]
    );
    assert_eq!(
        completed.input_cash_flow_contributions()[0].amount,
        updated_state.cash
    );
    assert_eq!(
        completed.cash_flow_totals().unwrap().financing,
        updated_state.cash
    );
    let projected = run
        .ledger()
        .validate_projection(month(1), month(1), &[id(1)])
        .unwrap()
        .project()
        .unwrap();
    assert_eq!(
        projected.cash_flow_statement.totals.financing,
        updated_state.cash
    );
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
                    LedgerEntry::new(id(4), BalanceSide::Credit, Money::new(5.into(), currency)),
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
    let committed_ledger = run.ledger().clone();
    let committed_state = run.opening_state().clone();
    let mut executor = Executor {
        calls: 0,
        fail: true,
        seen_activity_count: None,
        identity: None,
        ..Executor::default()
    };
    assert!(matches!(
        run.call(RunCommand::Continue, &[], &mut executor),
        Err(RunControlError::Simulation(_))
    ));
    assert_eq!(run.status(), &RunStatus::Failed);
    assert!(run.latest_snapshot().is_none());
    assert!(run.history().is_empty());
    assert_eq!(run.ledger(), &committed_ledger);
    assert_eq!(run.opening_state(), &committed_state);
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
        configuration: crate::ActivityConfiguration::Rental,
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
fn invalid_sample_from_executor_is_retryable_before_any_snapshot() {
    struct SampleBoundaryExecutor {
        reject_once: bool,
        fallback: Executor,
        activity: crate::ActivityInstance,
        key_factory: property_simulation_randomness::RunKeyFactory,
        stream_allocations: u8,
    }
    impl SingleMonthExecutor for SampleBoundaryExecutor {
        fn validate_inputs(
            &self,
            input: &ValidatedInput,
            ledger: &Ledger,
            activities: &[crate::ActivityInstance],
        ) -> Result<(), SimulationError> {
            self.fallback.validate_inputs(input, ledger, activities)
        }

        fn requires_random_streams(&self) -> bool {
            true
        }

        fn create_random_streams(
            &mut self,
            identity: Option<RunIdentity>,
        ) -> Result<Option<property_simulation_randomness::RandomStreams>, &'static str> {
            self.stream_allocations += 1;
            Ok(Some(
                self.key_factory
                    .run_streams(identity.ok_or("missing identity")?.trial_id),
            ))
        }

        fn execute(
            &mut self,
            input: &ValidatedInput,
            ledger: &Ledger,
            activities: &[crate::ActivityInstance],
            identity: Option<RunIdentity>,
        ) -> Result<(MonthlyResult, Ledger), SimulationError> {
            if self.reject_once {
                self.reject_once = false;
                let invalid = ActivitySample::new(
                    &self.activity,
                    input.frozen_input().period,
                    crate::RepairSample::Repair {
                        cost: Money::new(
                            1.into(),
                            input.frozen_input().repair_cost_limit.currency(),
                        ),
                    },
                );
                let slice = crate::RepairSlice {
                    instance: self.activity.clone(),
                };
                return Err(crate::propose_slice(&slice, input, &invalid).unwrap_err());
            }
            self.fallback.execute(input, ledger, activities, identity)
        }
    }

    let (ledger, input, state) = make_fixture_ledger();
    let opening_ledger = ledger.clone();
    let activity = crate::ActivityInstance {
        id: ActivityInstanceId::from_bytes([7; 16]),
        slice_id: SliceId::from_bytes([8; 16]),
        configuration: crate::ActivityConfiguration::Repair,
    };
    let mut run = SimulationRun::new_with_activities(
        RunConfig {
            start: month(1),
            end: None,
            stop_conditions: vec![],
        },
        state.clone(),
        input,
        ledger,
        vec![activity.clone()],
    )
    .unwrap();
    run.set_run_identity(RunIdentity {
        trial_id: TrialId::from_bytes([31; 16]),
        branch_id: BranchId::from_bytes([32; 16]),
    });
    let mut executor = SampleBoundaryExecutor {
        reject_once: true,
        fallback: Executor::default(),
        activity,
        key_factory: property_simulation_randomness::RunKeyFactory::new(
            property_simulation_randomness::RootKey::from_seed([33; 32]),
        ),
        stream_allocations: 0,
    };
    assert!(matches!(
        run.call(RunCommand::Continue, &[], &mut executor),
        Err(RunControlError::InvalidExecutionInput(
            SimulationError::InvalidInput("repair sample outside support")
        ))
    ));
    assert_eq!(
        run.status(),
        &RunStatus::Running {
            next_month: month(1)
        }
    );
    assert!(run.latest_snapshot().is_none());
    assert!(run.history().is_empty());
    assert_eq!(run.ledger(), &opening_ledger);
    assert_eq!(run.opening_state(), &state);
    assert_eq!(executor.stream_allocations, 1);

    assert!(
        run.call(RunCommand::Continue, &[], &mut executor)
            .unwrap()
            .monthly_result
            .is_some()
    );
    assert_eq!(run.latest_snapshot().unwrap().result().period(), month(1));
    assert_eq!(
        run.status(),
        &RunStatus::Running {
            next_month: month(2)
        }
    );
    assert_eq!(executor.fallback.calls, 1);
    assert_eq!(run.ledger(), &opening_ledger);
    assert_eq!(executor.stream_allocations, 1);
}

#[test]
fn unrepresentable_loan_and_input_cashflow_support_rejects_before_executor() {
    use std::cell::Cell;

    struct CountingExecutor {
        calls: Cell<usize>,
    }
    impl SingleMonthExecutor for CountingExecutor {
        fn validate_inputs(
            &self,
            _input: &ValidatedInput,
            _ledger: &Ledger,
            _activities: &[crate::ActivityInstance],
        ) -> Result<(), SimulationError> {
            self.calls.set(self.calls.get() + 1);
            Ok(())
        }

        fn validate_loan_context(
            &self,
            _input: &ValidatedInput,
            _ledger: &Ledger,
            loans: &[LoanContinuation],
            _updates: &[LoanRunUpdate],
        ) -> Result<(), SimulationError> {
            self.calls.set(self.calls.get() + 1);
            assert_eq!(loans.len(), 1);
            Ok(())
        }

        fn execute(
            &mut self,
            _input: &ValidatedInput,
            _ledger: &Ledger,
            _activities: &[crate::ActivityInstance],
            _identity: Option<RunIdentity>,
        ) -> Result<(MonthlyResult, Ledger), SimulationError> {
            self.calls.set(self.calls.get() + 1);
            panic!("support validation must precede monthly execution")
        }
    }

    let currency = Currency::from_code("CAD").unwrap();
    let cash = id(41);
    let rent = id(42);
    let repairs = id(43);
    let liability = id(44);
    let equity = id(45);
    let chart = ChartOfAccounts::new([
        Account::new_cash_position(cash, AccountCode::new(1000).unwrap(), "Cash").unwrap(),
        Account::new(
            rent,
            AccountCode::new(4000).unwrap(),
            AccountCategory::Income,
            false,
            "Rent",
        )
        .unwrap(),
        Account::new(
            repairs,
            AccountCode::new(5000).unwrap(),
            AccountCategory::Expense,
            false,
            "Repairs",
        )
        .unwrap(),
        Account::new(
            liability,
            AccountCode::new(2000).unwrap(),
            AccountCategory::Liability,
            false,
            "Loan",
        )
        .unwrap(),
        Account::new(
            equity,
            AccountCode::new(3000).unwrap(),
            AccountCategory::Equity,
            false,
            "Opening equity",
        )
        .unwrap(),
    ])
    .unwrap();
    // This loan payment is individually representable by the loan Decimal model,
    // while the state adjustment nearly fills Money's wider fixed-point range.
    // Together their financing contributions cannot be aggregated.
    let principal = Money::parse("70000000000000000000000000000", currency).unwrap();
    let opening = OpeningBalances::new(
        &chart,
        [
            OpeningBalance::new(liability, BalanceSide::Credit, principal).unwrap(),
            OpeningBalance::new(equity, BalanceSide::Debit, principal).unwrap(),
        ],
        None,
    )
    .unwrap();
    let ledger = Ledger::new(chart, opening).unwrap();
    let input = FrozenInput {
        period: month(1),
        rent: Money::new(0.into(), currency),
        rent_account: rent,
        cash_account: cash,
        repair_expense_account: repairs,
        repair_cost_limit: Money::new(0.into(), currency),
    };
    let state = OpeningState {
        cash: Money::new(0.into(), currency),
        condition: AssetCondition::Available,
    };
    let loan_activity = crate::ActivityInstance {
        id: ActivityInstanceId::from_bytes([46; 16]),
        slice_id: SliceId::from_bytes([47; 16]),
        configuration: crate::ActivityConfiguration::Loan,
    };
    let continuation = LoanContinuation {
        instance_id: loan_activity.id,
        slice_id: loan_activity.slice_id,
        loan: Some(property_simulation_models::FixedRateLoan {
            opening_balance: principal,
            remaining_months: 1,
            annual_rate: rust_decimal::Decimal::ZERO,
            scheduled_payment: None,
        }),
    };
    let mut run = SimulationRun::new_with_activities_and_loans(
        RunConfig {
            start: month(1),
            end: None,
            stop_conditions: vec![],
        },
        state.clone(),
        input,
        ledger.clone(),
        vec![loan_activity],
        vec![continuation],
    )
    .unwrap();
    let adjustment = Money::from_parts(i128::MAX - 1, 2, currency).unwrap();
    let update = RunUpdate {
        effective_month: month(1),
        source: "opening cash correction".into(),
        value: RunUpdateValue::StateAdjustment {
            state: OpeningState {
                cash: adjustment,
                condition: AssetCondition::Available,
            },
            transaction: LedgerTransaction::new(
                property_simulation_domain::LedgerTransactionId::from_bytes([49; 16]),
                month(1),
                [
                    LedgerEntry::new(cash, BalanceSide::Debit, adjustment)
                        .with_cash_flow_category(CashFlowCategory::Financing),
                    LedgerEntry::new(equity, BalanceSide::Credit, adjustment),
                ],
            ),
        },
    };
    let mut executor = CountingExecutor {
        calls: Cell::new(0),
    };

    assert!(matches!(
        run.call(RunCommand::Continue, &[update], &mut executor),
        Err(RunControlError::InvalidExecutionInput(
            SimulationError::InvalidInput(
                "active activity cash-flow absolute support exceeds the representable Money range"
            )
        ))
    ));
    assert_eq!(executor.calls.get(), 0);
    assert_eq!(
        run.status(),
        &RunStatus::Running {
            next_month: month(1)
        }
    );
    assert!(run.latest_snapshot().is_none());
    assert!(run.history().is_empty());
    assert_eq!(run.ledger(), &ledger);
    assert_eq!(run.opening_state(), &state);
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
    let committed_ledger = run.ledger().clone();
    let committed_state = run.opening_state().clone();
    assert!(matches!(
        run.call(RunCommand::Continue, &[], &mut FatalPreflightExecutor),
        Err(RunControlError::Simulation(SimulationError::Fatal(_)))
    ));
    assert_eq!(run.status(), &RunStatus::Failed);
    assert!(run.latest_snapshot().is_none());
    assert!(run.history().is_empty());
    assert_eq!(run.ledger(), &committed_ledger);
    assert_eq!(run.opening_state(), &committed_state);
    assert_eq!(
        run.call(RunCommand::Continue, &[], &mut FatalPreflightExecutor),
        Err(RunControlError::InvalidState)
    );
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
        configuration: crate::ActivityConfiguration::Repair,
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
        Err(RunControlError::Simulation(SimulationError::Fatal(
            crate::FatalSimulationFailure {
                phase: crate::FatalSimulationPhase::MonthlyResultConstruction,
                period,
                ..
            }
        ))) if period == month(1)
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
                vec![crate::ActivityInstance {
                    id: property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]),
                    slice_id: property_simulation_domain::SliceId::from_bytes([8; 16]),
                    configuration: crate::ActivityConfiguration::Rental,
                }],
                vec![ActivitySample::new(
                    &crate::ActivityInstance {
                        id: property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]),
                        slice_id: property_simulation_domain::SliceId::from_bytes([8; 16]),
                        configuration: crate::ActivityConfiguration::Rental,
                    },
                    input.frozen_input().period,
                    crate::ActivitySampleValue::Rental,
                )],
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

    let activity = crate::ActivityInstance {
        id: property_simulation_domain::ActivityInstanceId::from_bytes([7; 16]),
        slice_id: property_simulation_domain::SliceId::from_bytes([8; 16]),
        configuration: crate::ActivityConfiguration::Rental,
    };
    let (ledger, mut input, state) = make_fixture_ledger();
    input.rent = Money::new(1.into(), input.rent.currency());
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
    let outcome = run.call(RunCommand::Continue, &[], &mut MismatchedPostingExecutor);
    assert!(
        matches!(outcome, Err(RunControlError::Simulation(_))),
        "{outcome:?}"
    );
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
        mismatched_start_archive.save_month(saved_month(parent_id, run_id, snapshot.clone(), None)),
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
        loan_continuations: snapshot.result.loan_continuations().to_vec(),
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
    use property_simulation_domain::{ActivityInstanceId, RunId, SliceId, SnapshotId};

    let (mut ledger, mut input, state) = make_fixture_ledger();
    input.rent = Money::new(25.into(), input.rent.currency());
    let activity = crate::ActivityInstance {
        id: ActivityInstanceId::from_bytes([21; 16]),
        slice_id: SliceId::from_bytes([22; 16]),
        configuration: crate::ActivityConfiguration::Rental,
    };
    let trial = TrialId::from_bytes([24; 16]);
    let branch = BranchId::from_bytes([25; 16]);
    let transaction = ProposedTransaction {
        id: property_simulation_domain::LedgerTransactionId::from_bytes([26; 16]),
        instance_id: activity.id,
        slice_id: activity.slice_id,
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
        period: Some(month(1)),
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

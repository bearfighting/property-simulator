use property_simulation_accounting::{
    Account, AccountCategory, AccountCode, BalanceSide, CashFlowCategory, ChartOfAccounts, Ledger,
    LedgerEntry, LedgerTransaction, OpeningBalance, OpeningBalances,
};
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, Currency, EffectId, EventId, LedgerTransactionId,
    ModelVersionId, Money, RunId, SliceId, SnapshotId, TrialId, YearMonth,
};
use property_simulation_models::{
    FixedRateLoan, RepairDistribution, RepairSample, RepairSampler, UtilityUsageDistribution,
    UtilityUsageSampler,
};
use property_simulation_randomness::{RootKey, RunKeyFactory};
use property_simulation_simulation::{
    ActivityConfiguration, ActivityInstance, ActivitySample, ActivitySampleKind,
    ActivitySampleValue, AssetCondition, BranchRecord, CashFlowContributionTotals, FrozenInput,
    LoanContinuation, LoanMonthInput, LoanRunUpdate, LoanSlice, MonthlyResult, OpeningState,
    OutputIdFactory, PropertyTaxCadence, PropertyTaxConfig, PropertyTaxSlice, RentalSlice,
    RepairSlice, RunCommand, RunConfig, RunIdentity, RunUpdate, RunUpdateValue, SimulationArchive,
    SimulationError, SimulationRun, SimulationSlice, SingleMonthExecutor, StopCondition,
    StopReason, UtilityConfig, UtilitySlice, ValidatedInput, coordinate_month_with_loans,
    propose_slice, sample_repair_from_streams, sample_utility_from_streams, saved_month,
};

fn month(number: u8) -> YearMonth {
    YearMonth::new(2026, number).unwrap()
}

fn money(value: &str) -> Money {
    Money::parse(value, Currency::from_code("CAD").unwrap()).unwrap()
}

fn account_id(value: u8) -> AccountId {
    AccountId::from_bytes([value; 16])
}

fn activity(value: u8, kind: ActivitySampleKind) -> ActivityInstance {
    ActivityInstance {
        id: ActivityInstanceId::from_bytes([value; 16]),
        slice_id: SliceId::from_bytes([value + 10; 16]),
        version: ModelVersionId::from_bytes([value + 20; 16]),
        configuration: match kind {
            ActivitySampleKind::Rental => ActivityConfiguration::Rental,
            ActivitySampleKind::Repair => ActivityConfiguration::Repair,
            ActivitySampleKind::Loan => ActivityConfiguration::Loan,
            ActivitySampleKind::PropertyTax => panic!("tax test helper requires configuration"),
            ActivitySampleKind::Utility => panic!("utility test helper requires configuration"),
        },
    }
}

struct Ids;

fn derived_id(
    tag: u8,
    trial: TrialId,
    branch: BranchId,
    period: YearMonth,
    instance: ActivityInstanceId,
    key: &str,
) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[0] = tag;
    bytes[1..5].copy_from_slice(&period.year().to_le_bytes());
    bytes[5] = period.month();
    bytes[6] = key.bytes().fold(0_u8, u8::wrapping_add);
    bytes[7..11].copy_from_slice(&trial.as_bytes()[..4]);
    bytes[11..15].copy_from_slice(&branch.as_bytes()[..4]);
    bytes[15] = instance.as_bytes()[0];
    bytes
}

impl OutputIdFactory for Ids {
    fn event_id(
        &self,
        trial: TrialId,
        branch: BranchId,
        period: YearMonth,
        instance: ActivityInstanceId,
        key: &str,
    ) -> EventId {
        EventId::from_bytes(derived_id(1, trial, branch, period, instance, key))
    }
    fn effect_id(
        &self,
        trial: TrialId,
        branch: BranchId,
        period: YearMonth,
        instance: ActivityInstanceId,
        key: &str,
    ) -> EffectId {
        EffectId::from_bytes(derived_id(2, trial, branch, period, instance, key))
    }
    fn transaction_id(
        &self,
        trial: TrialId,
        branch: BranchId,
        period: YearMonth,
        instance: ActivityInstanceId,
        key: &str,
    ) -> LedgerTransactionId {
        LedgerTransactionId::from_bytes(derived_id(3, trial, branch, period, instance, key))
    }
}

struct CompositionExecutor {
    key_factory: RunKeyFactory,
    rental: RentalSlice,
    repair: RepairSlice,
    loans: Vec<LoanSlice>,
    distribution: RepairDistribution,
    sampler: RepairSampler,
    utility_sampler: UtilityUsageSampler,
    ids: Ids,
}

impl SingleMonthExecutor for CompositionExecutor {
    fn validate_inputs(
        &self,
        _input: &ValidatedInput,
        _ledger: &Ledger,
        activities: &[ActivityInstance],
    ) -> Result<(), SimulationError> {
        let expected = [
            self.rental.instance.id,
            self.repair.instance.id,
            self.loans[0].instance_id,
            self.loans[1].instance_id,
        ];
        if activities.len() < expected.len()
            || expected
                .iter()
                .any(|id| !activities.iter().any(|item| item.id == *id))
            || activities.iter().any(|item| {
                !expected.contains(&item.id)
                    && !matches!(
                        item.configuration,
                        ActivityConfiguration::PropertyTax(_) | ActivityConfiguration::Utility(_)
                    )
            })
        {
            return Err(SimulationError::InvalidInput(
                "composition executor received an unexpected activity set",
            ));
        }
        Ok(())
    }

    fn validate_loan_context(
        &self,
        _input: &ValidatedInput,
        _ledger: &Ledger,
        loans: &[LoanContinuation],
        updates: &[LoanRunUpdate],
    ) -> Result<(), SimulationError> {
        if loans.len() == 2 && updates.is_empty() {
            Ok(())
        } else {
            Err(SimulationError::InvalidInput(
                "expected two active loans and no loan terms updates",
            ))
        }
    }

    fn requires_random_streams(&self) -> bool {
        true
    }

    fn create_random_streams(
        &mut self,
        identity: Option<RunIdentity>,
    ) -> Result<Option<property_simulation_randomness::RandomStreams>, &'static str> {
        self.key_factory
            .run_streams(identity.ok_or("missing run identity")?.trial_id)
            .map(Some)
            .map_err(|_| "run stream allocation failed")
    }

    fn execute(
        &mut self,
        _input: &ValidatedInput,
        _opening_ledger: &Ledger,
        _activities: &[ActivityInstance],
        _identity: Option<RunIdentity>,
    ) -> Result<(MonthlyResult, Ledger), SimulationError> {
        Err(SimulationError::InvalidInput(
            "random streams and loan context are required",
        ))
    }

    fn execute_with_loan_context(
        &mut self,
        input: &ValidatedInput,
        opening_ledger: &Ledger,
        activities: &[ActivityInstance],
        continuations: &[LoanContinuation],
        updates: &[LoanRunUpdate],
        identity: Option<RunIdentity>,
        streams: Option<&property_simulation_randomness::RandomStreams>,
    ) -> Result<(MonthlyResult, Ledger), SimulationError> {
        if !updates.is_empty() {
            return Err(SimulationError::InvalidInput("unexpected loan update"));
        }
        let identity = identity.ok_or(SimulationError::InvalidInput("missing run identity"))?;
        let streams = streams.ok_or(SimulationError::InvalidInput("missing random streams"))?;
        let period = input.frozen_input().period;
        let repair_sample = sample_repair_from_streams(
            streams,
            identity.trial_id,
            self.repair.instance(),
            period,
            &self.distribution,
            &mut self.sampler,
        )
        .map_err(|_| SimulationError::InvalidInput("repair sampling failed"))?;
        let rental_sample = ActivitySample::new(self.rental.instance(), period, ());
        let repair_value = repair_sample.value;
        let repair_purpose = repair_sample.purpose;
        let repair_sample = ActivitySample {
            instance_id: self.repair.instance.id,
            slice_id: self.repair.instance.slice_id,
            version: self.repair.instance.version,
            period,
            purpose: repair_purpose,
            value: repair_value,
        };
        let rental_output = propose_slice(&self.rental, input, &rental_sample)
            .map_err(|_| SimulationError::InvalidInput("rental Slice failed"))?;
        let repair_output = propose_slice(&self.repair, input, &repair_sample)
            .map_err(|_| SimulationError::InvalidInput("repair Slice failed"))?;
        let mut samples = vec![
            ActivitySample::new(self.rental.instance(), period, ActivitySampleValue::Rental),
            ActivitySample {
                instance_id: self.repair.instance.id,
                slice_id: self.repair.instance.slice_id,
                version: self.repair.instance.version,
                period,
                purpose: repair_sample.purpose.clone(),
                value: ActivitySampleValue::Repair(repair_sample.value),
            },
        ];
        let mut outputs = vec![rental_output, repair_output];
        for instance in activities {
            match &instance.configuration {
                ActivityConfiguration::PropertyTax(_) => {
                    let slice = PropertyTaxSlice {
                        instance: instance.clone(),
                    };
                    let proposal_sample = ActivitySample::new(instance, period, ());
                    outputs.push(
                        propose_slice(&slice, input, &proposal_sample).map_err(|_| {
                            SimulationError::InvalidInput("property-tax Slice failed")
                        })?,
                    );
                    samples.push(ActivitySample::new(
                        instance,
                        period,
                        ActivitySampleValue::PropertyTax,
                    ));
                }
                ActivityConfiguration::Utility(config) => {
                    let slice = UtilitySlice {
                        instance: instance.clone(),
                    };
                    let typed = sample_utility_from_streams(
                        input,
                        streams,
                        identity.trial_id,
                        instance,
                        period,
                        &UtilityUsageDistribution {
                            maximum: config.maximum_usage,
                        },
                        &mut self.utility_sampler,
                    )
                    .map_err(|_| SimulationError::InvalidInput("utility sampling failed"))?;
                    let sample = ActivitySample {
                        instance_id: typed.instance_id,
                        slice_id: typed.slice_id,
                        version: typed.version,
                        period: typed.period,
                        purpose: typed.purpose.clone(),
                        value: ActivitySampleValue::Utility(typed.value),
                    };
                    outputs.push(
                        propose_slice(&slice, input, &typed)
                            .map_err(|_| SimulationError::InvalidInput("utility Slice failed"))?,
                    );
                    samples.push(sample);
                }
                _ => {}
            }
        }
        let loan_inputs = self
            .loans
            .iter()
            .map(|slice| {
                let continuation = continuations
                    .iter()
                    .find(|item| item.instance_id == slice.instance_id)
                    .ok_or(SimulationError::InvalidInput(
                        "loan continuation is missing",
                    ))?;
                Ok(LoanMonthInput {
                    slice: slice.clone(),
                    loan: continuation.loan,
                    update: None,
                })
            })
            .collect::<Result<Vec<_>, SimulationError>>()?;
        coordinate_month_with_loans(
            input,
            opening_ledger,
            outputs,
            samples,
            identity.trial_id,
            identity.branch_id,
            &self.ids,
            loan_inputs,
        )
    }
}

fn fixture() -> (
    SimulationRun,
    CompositionExecutor,
    Ledger,
    Vec<LoanContinuation>,
) {
    let cash_id = account_id(1);
    let rent_id = account_id(2);
    let repair_id = account_id(3);
    let property_id = account_id(4);
    let equity_id = account_id(5);
    let liability_ids = [account_id(6), account_id(7)];
    let interest_id = account_id(8);
    let chart = ChartOfAccounts::new([
        Account::new_cash_position(cash_id, AccountCode::new(1000).unwrap(), "Cash").unwrap(),
        Account::new(
            rent_id,
            AccountCode::new(4000).unwrap(),
            AccountCategory::Income,
            false,
            "Rent",
        )
        .unwrap(),
        Account::new(
            repair_id,
            AccountCode::new(5000).unwrap(),
            AccountCategory::Expense,
            false,
            "Repairs",
        )
        .unwrap(),
        Account::new(
            property_id,
            AccountCode::new(1500).unwrap(),
            AccountCategory::Asset,
            false,
            "Property",
        )
        .unwrap(),
        Account::new(
            equity_id,
            AccountCode::new(3000).unwrap(),
            AccountCategory::Equity,
            false,
            "Opening equity",
        )
        .unwrap(),
        Account::new(
            liability_ids[0],
            AccountCode::new(2001).unwrap(),
            AccountCategory::Liability,
            false,
            "Loan one",
        )
        .unwrap(),
        Account::new(
            liability_ids[1],
            AccountCode::new(2002).unwrap(),
            AccountCategory::Liability,
            false,
            "Loan two",
        )
        .unwrap(),
        Account::new(
            interest_id,
            AccountCode::new(5100).unwrap(),
            AccountCategory::Expense,
            false,
            "Interest",
        )
        .unwrap(),
    ])
    .unwrap();
    let first = FixedRateLoan {
        opening_balance: money("1200"),
        remaining_months: 3,
        annual_rate: rust_decimal::Decimal::new(600, 4),
        scheduled_payment: None,
    };
    let second = FixedRateLoan {
        opening_balance: money("2400"),
        remaining_months: 6,
        annual_rate: rust_decimal::Decimal::new(500, 4),
        scheduled_payment: None,
    };
    let opening = OpeningBalances::new(
        &chart,
        [
            OpeningBalance::new(property_id, BalanceSide::Debit, money("10000")).unwrap(),
            OpeningBalance::new(liability_ids[0], BalanceSide::Credit, first.opening_balance)
                .unwrap(),
            OpeningBalance::new(
                liability_ids[1],
                BalanceSide::Credit,
                second.opening_balance,
            )
            .unwrap(),
            OpeningBalance::new(equity_id, BalanceSide::Credit, money("6400")).unwrap(),
        ],
        Some(money("6400")),
    )
    .unwrap();
    let ledger = Ledger::new(chart, opening).unwrap();
    let frozen = FrozenInput {
        period: month(1),
        rent: money("80"),
        rent_account: rent_id,
        cash_account: cash_id,
        repair_expense_account: repair_id,
        repair_cost_limit: money("10"),
    };
    let rental = RentalSlice {
        instance: activity(1, ActivitySampleKind::Rental),
    };
    let repair = RepairSlice {
        instance: activity(2, ActivitySampleKind::Repair),
    };
    let loans = vec![
        LoanSlice {
            instance_id: ActivityInstanceId::from_bytes([3; 16]),
            slice_id: SliceId::from_bytes([13; 16]),
            model_version: ModelVersionId::from_bytes([23; 16]),
            liability_account: liability_ids[0],
            interest_expense_account: interest_id,
            cash_account: cash_id,
        },
        LoanSlice {
            instance_id: ActivityInstanceId::from_bytes([4; 16]),
            slice_id: SliceId::from_bytes([14; 16]),
            model_version: ModelVersionId::from_bytes([24; 16]),
            liability_account: liability_ids[1],
            interest_expense_account: interest_id,
            cash_account: cash_id,
        },
    ];
    let continuations = vec![
        LoanContinuation {
            instance_id: loans[0].instance_id,
            slice_id: loans[0].slice_id,
            model_version: loans[0].model_version,
            loan: Some(first),
        },
        LoanContinuation {
            instance_id: loans[1].instance_id,
            slice_id: loans[1].slice_id,
            model_version: loans[1].model_version,
            loan: Some(second),
        },
    ];
    let activities = vec![
        rental.instance.clone(),
        repair.instance.clone(),
        activity(3, ActivitySampleKind::Loan),
        activity(4, ActivitySampleKind::Loan),
    ];
    let run = SimulationRun::new_with_activities_and_loans_and_identity(
        RunConfig {
            start: month(1),
            end: Some(month(2)),
            stop_conditions: vec![StopCondition::MonthlyNetCashChangeAtMost(money("-100000"))],
        },
        OpeningState {
            cash: money("0"),
            condition: AssetCondition::NeedsRepair,
        },
        frozen,
        ledger.clone(),
        activities,
        continuations.clone(),
        RunIdentity {
            trial_id: TrialId::from_bytes([31; 16]),
            branch_id: BranchId::from_bytes([41; 16]),
        },
    )
    .unwrap();
    let executor = CompositionExecutor {
        key_factory: RunKeyFactory::new(RootKey::from_seed([8; 32])),
        rental,
        repair,
        loans,
        distribution: RepairDistribution {
            occurrence_numerator: 1,
            occurrence_denominator: 2,
            cost: property_simulation_models::RepairCostDistribution {
                maximum: money("10"),
            },
        },
        sampler: RepairSampler,
        utility_sampler: UtilityUsageSampler,
        ids: Ids,
    };
    (run, executor, ledger, continuations)
}

fn operating_activities(tax_amount: &str, utility_rates: [&str; 2]) -> Vec<ActivityInstance> {
    let tax = ActivityInstance {
        id: ActivityInstanceId::from_bytes([6; 16]),
        slice_id: SliceId::from_bytes([16; 16]),
        version: ModelVersionId::from_bytes([26; 16]),
        configuration: ActivityConfiguration::PropertyTax(PropertyTaxConfig {
            amount: money(tax_amount),
            cadence: PropertyTaxCadence::Monthly,
            first_due: month(1),
            cash_account: account_id(1),
            expense_account: account_id(3),
        }),
    };
    let utility = |id: u8, unit: &str, rate: &str| ActivityInstance {
        id: ActivityInstanceId::from_bytes([id; 16]),
        slice_id: SliceId::from_bytes([id + 10; 16]),
        version: ModelVersionId::from_bytes([id + 20; 16]),
        configuration: ActivityConfiguration::Utility(UtilityConfig {
            usage_unit: unit.into(),
            maximum_usage: 20,
            rate_per_unit: money(rate),
            cash_account: account_id(1),
            expense_account: account_id(8),
        }),
    };
    vec![
        activity(1, ActivitySampleKind::Rental),
        activity(2, ActivitySampleKind::Repair),
        activity(3, ActivitySampleKind::Loan),
        activity(4, ActivitySampleKind::Loan),
        tax,
        utility(7, "kWh", utility_rates[0]),
        utility(8, "m3", utility_rates[1]),
    ]
}

fn activity_update(activities: Vec<ActivityInstance>, source: &str) -> RunUpdate {
    RunUpdate {
        effective_month: month(2),
        source: source.into(),
        value: RunUpdateValue::Activities(activities),
    }
}

#[test]
fn m46_operating_activities_follow_month_updates_and_branch_isolation() {
    let (_, mut executor, ledger, loans) = fixture();
    let parent_trial = TrialId::from_bytes([31; 16]);
    let parent_branch = BranchId::from_bytes([41; 16]);
    let parent_run_id = RunId::from_bytes([71; 16]);
    let jan_snapshot_id = SnapshotId::from_bytes([72; 16]);
    let initial = operating_activities("25", ["2", "3"]);
    let initial_frozen = FrozenInput {
        period: month(1),
        rent: money("80"),
        rent_account: account_id(2),
        cash_account: account_id(1),
        repair_expense_account: account_id(3),
        repair_cost_limit: money("10"),
    };
    let mut parent = SimulationRun::new_with_activities_and_loans_and_identity(
        RunConfig {
            start: month(1),
            end: Some(month(2)),
            stop_conditions: vec![],
        },
        OpeningState {
            cash: money("0"),
            condition: AssetCondition::NeedsRepair,
        },
        initial_frozen,
        ledger.clone(),
        initial,
        loans.clone(),
        RunIdentity {
            trial_id: parent_trial,
            branch_id: parent_branch,
        },
    )
    .unwrap();
    let mut archive = SimulationArchive::default();
    archive
        .register_branch(BranchRecord {
            id: parent_branch,
            run_id: parent_run_id,
            trial_id: parent_trial,
            start_month: month(1),
            parent_snapshot: None,
            fork_month: None,
        })
        .unwrap();
    let jan = parent
        .call(RunCommand::Continue, &[], &mut executor)
        .unwrap()
        .monthly_result
        .unwrap();
    assert_eq!(jan.activities().len(), 7);
    let jan_tax = jan
        .activities()
        .iter()
        .find_map(|a| match &a.configuration {
            ActivityConfiguration::PropertyTax(c) => Some(c.amount),
            _ => None,
        })
        .unwrap();
    assert_eq!(jan_tax, money("25"));
    for sample in jan
        .samples()
        .iter()
        .filter(|s| matches!(s.value, ActivitySampleValue::Utility(_)))
    {
        assert_eq!(sample.purpose.as_deref(), Some("utility.usage"));
        assert_eq!(sample.period, month(1));
    }
    assert_utility_sample_provenance(&jan);
    let jan_saved = parent.latest_snapshot().unwrap().clone();
    archive
        .save_month(saved_month(
            jan_snapshot_id,
            parent_run_id,
            jan_saved.clone(),
            None,
        ))
        .unwrap();
    let jan_archived = archive.snapshot(jan_snapshot_id).unwrap().snapshot.clone();

    let parent_feb_activities = operating_activities("35", ["4", "5"]);
    let parent_update = activity_update(parent_feb_activities.clone(), "parent operating update");
    let parent_feb = parent
        .call(RunCommand::Continue, &[parent_update], &mut executor)
        .unwrap();
    assert_eq!(parent_feb.stop_reason, Some(StopReason::EndMonth(month(2))));
    let parent_result = parent_feb.monthly_result.unwrap();
    assert_eq!(
        parent_result.input_updates()[0].source,
        "parent operating update"
    );
    assert_eq!(parent_result.activities(), parent_feb_activities.as_slice());
    assert_eq!(parent_result.input_updates()[0].effective_month, month(2));
    assert_source_and_ids(&parent_result);
    assert_utility_sample_provenance(&parent_result);
    assert_cash_flow_matches_snapshot(
        &parent_result,
        parent.latest_snapshot().unwrap().ledger(),
        account_id(1),
    );
    let parent_feb_snapshot = parent.latest_snapshot().unwrap().clone();

    let (_, mut reordered_executor, _, baseline_loans) = fixture();
    reordered_executor.loans.reverse();
    let mut reordered = SimulationRun::new_with_activities_and_loans_and_identity(
        RunConfig {
            start: month(1),
            end: Some(month(2)),
            stop_conditions: vec![],
        },
        OpeningState {
            cash: money("0"),
            condition: AssetCondition::NeedsRepair,
        },
        FrozenInput {
            period: month(1),
            rent: money("80"),
            rent_account: account_id(2),
            cash_account: account_id(1),
            repair_expense_account: account_id(3),
            repair_cost_limit: money("10"),
        },
        ledger.clone(),
        operating_activities("25", ["2", "3"]),
        baseline_loans,
        RunIdentity {
            trial_id: parent_trial,
            branch_id: parent_branch,
        },
    )
    .unwrap();
    reordered
        .call(RunCommand::Continue, &[], &mut reordered_executor)
        .unwrap();
    let mut reversed_activities = parent_feb_activities.clone();
    reversed_activities.reverse();
    let reversed_update = activity_update(reversed_activities, "parent operating update");
    let reordered_feb = reordered
        .call(
            RunCommand::Continue,
            &[reversed_update],
            &mut reordered_executor,
        )
        .unwrap();
    let reordered_result = reordered_feb.monthly_result.unwrap();
    assert_eq!(parent_result.activities(), reordered_result.activities());
    assert_eq!(parent_result.samples(), reordered_result.samples());
    assert_eq!(parent_result.proposal(), reordered_result.proposal());
    assert_eq!(
        parent_result.cash_flow_contributions(),
        reordered_result.cash_flow_contributions()
    );
    assert_eq!(
        parent_result.loan_continuations(),
        reordered_result.loan_continuations()
    );
    assert_eq!(
        parent.latest_snapshot().unwrap().ledger(),
        reordered.latest_snapshot().unwrap().ledger()
    );

    let child_trial = TrialId::from_bytes([73; 16]);
    let child_branch = BranchId::from_bytes([74; 16]);
    let child_run_id = RunId::from_bytes([75; 16]);
    let mut child = archive
        .create_branch(
            jan_snapshot_id,
            child_run_id,
            child_branch,
            child_trial,
            Some(month(2)),
            vec![],
        )
        .unwrap();
    let (_, mut child_executor, _, _) = fixture();
    child_executor.key_factory = RunKeyFactory::new(RootKey::from_seed([9; 32]));
    let child_activities = operating_activities("45", ["6", "7"]);
    let child_update = activity_update(child_activities.clone(), "child operating update");
    let child_call = child
        .call(RunCommand::Continue, &[child_update], &mut child_executor)
        .unwrap();
    assert_eq!(child_call.stop_reason, Some(StopReason::EndMonth(month(2))));
    let child_result = child_call.monthly_result.unwrap();
    assert_eq!(child_result.trial_id(), child_trial);
    assert_eq!(child_result.branch_id(), child_branch);
    assert_eq!(child_result.opening_state(), jan.closing_state());
    assert_eq!(child_result.activities(), child_activities.as_slice());
    assert_eq!(
        child_result.input_updates()[0].source,
        "child operating update"
    );
    assert_source_and_ids(&child_result);
    assert_utility_sample_provenance(&child_result);
    assert_cash_flow_matches_snapshot(
        &child_result,
        child.latest_snapshot().unwrap().ledger(),
        account_id(1),
    );
    assert_eq!(
        archive.snapshot(jan_snapshot_id).unwrap().snapshot,
        jan_archived
    );
    assert_eq!(jan_saved, jan_archived);
    assert_eq!(parent.latest_snapshot().unwrap(), &parent_feb_snapshot);
    assert_eq!(parent.latest_snapshot().unwrap().result(), &parent_result);
    assert_eq!(child.latest_snapshot().unwrap().result(), &child_result);
    assert_ne!(
        parent.latest_snapshot().unwrap().ledger(),
        child.latest_snapshot().unwrap().ledger()
    );

    assert!(!parent_result.proposal().transactions().is_empty());
    assert_eq!(parent_result.period(), month(2));
    assert_eq!(child_result.period(), month(2));
    assert_ne!(parent_result.branch_id(), child_result.branch_id());
}

#[test]
fn m45_operating_slices_and_multiple_loans_share_one_monthly_batch() {
    let (_run, executor, ledger, continuations) = fixture();
    let period = YearMonth::new(u32::MAX, 12).unwrap();
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::NeedsRepair,
    };
    let frozen = FrozenInput {
        period,
        rent: money("80"),
        rent_account: account_id(2),
        cash_account: account_id(1),
        repair_expense_account: account_id(3),
        repair_cost_limit: money("10"),
    };
    let mut validated = property_simulation_simulation::validate_input(
        &frozen,
        &property_simulation_models::RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &ledger,
        &state,
    )
    .unwrap();

    let tax = ActivityInstance {
        id: ActivityInstanceId::from_bytes([6; 16]),
        slice_id: SliceId::from_bytes([16; 16]),
        version: ModelVersionId::from_bytes([26; 16]),
        configuration: ActivityConfiguration::PropertyTax(PropertyTaxConfig {
            amount: money("25"),
            cadence: PropertyTaxCadence::Monthly,
            first_due: period,
            cash_account: account_id(1),
            expense_account: account_id(3),
        }),
    };
    let utility_a = ActivityInstance {
        id: ActivityInstanceId::from_bytes([7; 16]),
        slice_id: SliceId::from_bytes([17; 16]),
        version: ModelVersionId::from_bytes([27; 16]),
        configuration: ActivityConfiguration::Utility(UtilityConfig {
            usage_unit: "kWh".into(),
            maximum_usage: 20,
            rate_per_unit: money("2"),
            cash_account: account_id(1),
            expense_account: account_id(8),
        }),
    };
    let utility_b = ActivityInstance {
        id: ActivityInstanceId::from_bytes([8; 16]),
        slice_id: SliceId::from_bytes([18; 16]),
        version: ModelVersionId::from_bytes([28; 16]),
        configuration: ActivityConfiguration::Utility(UtilityConfig {
            usage_unit: "m³".into(),
            maximum_usage: 20,
            rate_per_unit: money("3"),
            cash_account: account_id(1),
            expense_account: account_id(8),
        }),
    };
    let repair_b = activity(5, ActivitySampleKind::Repair);
    let tax_slice = PropertyTaxSlice {
        instance: tax.clone(),
    };
    let utility_a_slice = UtilitySlice {
        instance: utility_a.clone(),
    };
    let utility_b_slice = UtilitySlice {
        instance: utility_b.clone(),
    };
    let repair_b_slice = RepairSlice {
        instance: repair_b.clone(),
    };
    let active_instances = vec![
        executor.rental.instance.clone(),
        executor.repair.instance.clone(),
        repair_b.clone(),
        tax.clone(),
        utility_a.clone(),
        utility_b.clone(),
        activity(3, ActivitySampleKind::Loan),
        activity(4, ActivitySampleKind::Loan),
    ];
    let loan_support = continuations.iter().flat_map(|continuation| {
        let month_result = continuation.loan.unwrap().calculate_month().unwrap();
        [
            (CashFlowCategory::Operating, month_result.interest),
            (CashFlowCategory::Financing, month_result.principal),
        ]
    });
    validated
        .validate_activities_before_sampling(&active_instances, loan_support)
        .unwrap();

    let rental_sample = ActivitySample::new(executor.rental.instance(), period, ());
    let repair_sample = ActivitySample::new(
        executor.repair.instance(),
        period,
        RepairSample::Repair { cost: money("5") },
    );
    let repair_b_sample =
        ActivitySample::new(&repair_b, period, RepairSample::Repair { cost: money("5") });
    let tax_sample = ActivitySample::new(&tax, period, ());
    let utility_a_sample = ActivitySample::new(&utility_a, period, 2_u64);
    let utility_b_sample = ActivitySample::new(&utility_b, period, 3_u64);
    let outputs = vec![
        propose_slice(&executor.rental, &validated, &rental_sample).unwrap(),
        propose_slice(&executor.repair, &validated, &repair_sample).unwrap(),
        propose_slice(&repair_b_slice, &validated, &repair_b_sample).unwrap(),
        propose_slice(&tax_slice, &validated, &tax_sample).unwrap(),
        propose_slice(&utility_a_slice, &validated, &utility_a_sample).unwrap(),
        propose_slice(&utility_b_slice, &validated, &utility_b_sample).unwrap(),
    ];
    let samples = vec![
        ActivitySample::new(
            executor.rental.instance(),
            period,
            ActivitySampleValue::Rental,
        ),
        ActivitySample::new(
            executor.repair.instance(),
            period,
            ActivitySampleValue::Repair(repair_sample.value),
        ),
        ActivitySample::new(
            &repair_b,
            period,
            ActivitySampleValue::Repair(repair_b_sample.value),
        ),
        ActivitySample::new(&tax, period, ActivitySampleValue::PropertyTax),
        ActivitySample::new(&utility_a, period, ActivitySampleValue::Utility(2)),
        ActivitySample::new(&utility_b, period, ActivitySampleValue::Utility(3)),
    ];
    let loan_inputs: Vec<_> = executor
        .loans
        .iter()
        .map(|slice| LoanMonthInput {
            slice: slice.clone(),
            loan: continuations
                .iter()
                .find(|item| item.instance_id == slice.instance_id)
                .unwrap()
                .loan,
            update: None,
        })
        .collect();
    let trial_id = TrialId::from_bytes([91; 16]);
    let branch_id = BranchId::from_bytes([92; 16]);
    let forward = coordinate_month_with_loans(
        &validated,
        &ledger,
        outputs.clone(),
        samples.clone(),
        trial_id,
        branch_id,
        &executor.ids,
        loan_inputs.clone(),
    )
    .unwrap();
    let reversed = coordinate_month_with_loans(
        &validated,
        &ledger,
        outputs.into_iter().rev().collect(),
        samples.into_iter().rev().collect(),
        trial_id,
        branch_id,
        &executor.ids,
        loan_inputs.into_iter().rev().collect(),
    )
    .unwrap();
    assert_eq!(forward.0.activities(), reversed.0.activities());
    assert_eq!(forward.0.samples(), reversed.0.samples());
    assert_eq!(forward.0.proposal(), reversed.0.proposal());
    assert_eq!(
        forward.0.cash_flow_contributions(),
        reversed.0.cash_flow_contributions()
    );
    assert_eq!(
        forward.0.loan_continuations(),
        reversed.0.loan_continuations()
    );
    assert_eq!(forward.0.loan_events(), reversed.0.loan_events());
    assert_eq!(forward.0.loan_effects(), reversed.0.loan_effects());
    assert_eq!(forward.0.closing_state(), reversed.0.closing_state());
    assert_eq!(forward.1, reversed.1);

    let (result, committed) = forward;
    assert_eq!(result.activities().len(), 8);
    assert_eq!(result.proposal().transactions().len(), 8);
    assert_eq!(result.proposal().effects().len(), 2);
    assert_eq!(result.closing_state().condition, AssetCondition::Available);
    assert_eq!(result.loan_continuations().len(), 2);
    assert_source_and_ids(&result);
    assert_cash_flow_matches_snapshot(&result, &committed, account_id(1));
    for continuation in result.loan_continuations() {
        let liability = match continuation.instance_id.as_bytes()[0] {
            3 => account_id(6),
            4 => account_id(7),
            _ => unreachable!(),
        };
        let (side, balance) = committed.balance(liability).unwrap();
        assert_eq!(side, BalanceSide::Credit);
        assert_eq!(balance, continuation.loan.unwrap().opening_balance);
    }
}

#[test]
fn m37_run_sampling_coordination_two_loans_and_snapshot_branch_compose() {
    let (mut parent, mut parent_executor, _opening_ledger, _) = fixture();
    let parent_trial = TrialId::from_bytes([31; 16]);
    let parent_branch = BranchId::from_bytes([41; 16]);
    let parent_run_id = RunId::from_bytes([51; 16]);
    let first_snapshot_id = SnapshotId::from_bytes([61; 16]);
    let second_snapshot_id = SnapshotId::from_bytes([62; 16]);
    let mut archive = SimulationArchive::default();
    archive
        .register_branch(BranchRecord {
            id: parent_branch,
            run_id: parent_run_id,
            trial_id: parent_trial,
            start_month: month(1),
            parent_snapshot: None,
            fork_month: None,
        })
        .unwrap();

    let first = parent
        .call(RunCommand::Continue, &[], &mut parent_executor)
        .unwrap();
    let result = first.monthly_result.unwrap();
    assert_eq!(
        result.samples()[1].purpose.as_deref(),
        Some("repair.sample")
    );
    assert_eq!(result.loan_continuations().len(), 2);
    assert_source_and_ids(&result);
    assert_cash_flow_matches_snapshot(
        &result,
        parent.latest_snapshot().unwrap().ledger(),
        account_id(1),
    );
    let saved_first = parent.latest_snapshot().unwrap().clone();
    archive
        .save_month(saved_month(
            first_snapshot_id,
            parent_run_id,
            saved_first.clone(),
            None,
        ))
        .unwrap();

    let parent_before_fork = archive
        .snapshot(first_snapshot_id)
        .unwrap()
        .snapshot
        .clone();
    let child_trial = TrialId::from_bytes([32; 16]);
    let child_branch = BranchId::from_bytes([42; 16]);
    let child_run_id = RunId::from_bytes([52; 16]);
    let child_snapshot_id = SnapshotId::from_bytes([63; 16]);
    let mut child = archive
        .create_branch(
            first_snapshot_id,
            child_run_id,
            child_branch,
            child_trial,
            Some(month(2)),
            vec![],
        )
        .unwrap();
    let (_, mut child_executor, _, _) = fixture();
    child_executor.key_factory = RunKeyFactory::new(RootKey::from_seed([9; 32]));
    let child_update = RunUpdate {
        effective_month: month(2),
        source: "branch rent adjustment".into(),
        value: RunUpdateValue::Rent(money("90")),
    };
    let child_month = child
        .call(RunCommand::Continue, &[child_update], &mut child_executor)
        .unwrap();
    let child_result = child_month.monthly_result.unwrap();
    assert_eq!(child_result.period(), month(2));
    assert_eq!(child_result.frozen_input().rent, money("90"));
    assert_eq!(child_result.input_updates().len(), 1);
    assert_eq!(
        child_result.input_updates()[0].source,
        "branch rent adjustment"
    );
    assert_eq!(child_result.input_updates()[0].effective_month, month(2));
    assert_eq!(child_result.trial_id(), child_trial);
    assert_eq!(child_result.branch_id(), child_branch);
    assert_cash_flow_matches_snapshot(
        &child_result,
        child.latest_snapshot().unwrap().ledger(),
        account_id(1),
    );
    archive
        .save_month(saved_month(
            child_snapshot_id,
            child_run_id,
            child.latest_snapshot().unwrap().clone(),
            Some(first_snapshot_id),
        ))
        .unwrap();
    assert_eq!(
        archive
            .snapshot(child_snapshot_id)
            .unwrap()
            .snapshot
            .result()
            .input_updates()[0]
            .source,
        "branch rent adjustment"
    );

    assert_eq!(
        archive.snapshot(first_snapshot_id).unwrap().snapshot,
        parent_before_fork
    );
    assert_eq!(
        archive.snapshot(first_snapshot_id).unwrap().snapshot,
        saved_first
    );
    assert_eq!(
        archive
            .cash_flow_summary(child_run_id)
            .unwrap()
            .by_month
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![month(2)]
    );

    let parent_update = RunUpdate {
        effective_month: month(2),
        source: "parent rent adjustment".into(),
        value: RunUpdateValue::Rent(money("80")),
    };
    let parent_month = parent
        .call(RunCommand::Continue, &[parent_update], &mut parent_executor)
        .unwrap();
    assert_eq!(
        parent_month.stop_reason,
        Some(StopReason::EndMonth(month(2)))
    );
    let parent_result = parent_month.monthly_result.unwrap();
    assert_eq!(parent_result.period(), month(2));
    assert_eq!(parent_result.frozen_input().rent, money("80"));
    assert_cash_flow_matches_snapshot(
        &parent_result,
        parent.latest_snapshot().unwrap().ledger(),
        account_id(1),
    );
    archive
        .save_month(saved_month(
            second_snapshot_id,
            parent_run_id,
            parent.latest_snapshot().unwrap().clone(),
            Some(first_snapshot_id),
        ))
        .unwrap();
    assert_eq!(
        archive
            .cash_flow_summary(parent_run_id)
            .unwrap()
            .by_month
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![month(1), month(2)]
    );
}

#[test]
fn m37_mixed_slice_batch_checks_only_the_month_end_cash_balance() {
    let currency = Currency::from_code("CAD").unwrap();
    let cash = account_id(81);
    let rent_account = account_id(82);
    let liability = account_id(83);
    let equity = account_id(84);
    let interest = account_id(85);
    let chart = ChartOfAccounts::new([
        Account::new_cash_position(cash, AccountCode::new(1000).unwrap(), "Cash").unwrap(),
        Account::new(
            rent_account,
            AccountCode::new(4000).unwrap(),
            AccountCategory::Income,
            false,
            "Rent",
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
            "Equity",
        )
        .unwrap(),
        Account::new(
            interest,
            AccountCode::new(5000).unwrap(),
            AccountCategory::Expense,
            false,
            "Interest",
        )
        .unwrap(),
    ])
    .unwrap();
    let opening_cash = Money::from_parts(i128::MAX - 500, 2, currency).unwrap();
    let principal = money("100");
    let opening_equity = opening_cash.checked_sub(principal).unwrap();
    let balances = OpeningBalances::new(
        &chart,
        [
            OpeningBalance::new(cash, BalanceSide::Debit, opening_cash).unwrap(),
            OpeningBalance::new(liability, BalanceSide::Credit, principal).unwrap(),
            OpeningBalance::new(equity, BalanceSide::Credit, opening_equity).unwrap(),
        ],
        Some(opening_equity),
    )
    .unwrap();
    let ledger = Ledger::new(chart, balances).unwrap();
    let period = month(1);
    let frozen = FrozenInput {
        period,
        rent: money("0"),
        rent_account,
        cash_account: cash,
        repair_expense_account: interest,
        repair_cost_limit: money("100"),
    };
    let state = OpeningState {
        cash: opening_cash,
        condition: AssetCondition::NeedsRepair,
    };
    let distribution = property_simulation_models::RepairCostDistribution {
        maximum: money("100"),
    };
    let input =
        property_simulation_simulation::validate_input(&frozen, &distribution, &ledger, &state)
            .unwrap();
    let repair = RepairSlice {
        instance: activity(81, ActivitySampleKind::Repair),
    };
    let repair_sample = property_simulation_simulation::RepairSample::Repair { cost: money("100") };
    let repair_output = propose_slice(
        &repair,
        &input,
        &ActivitySample::new(&repair.instance, period, repair_sample),
    )
    .unwrap();
    let loan_slice = LoanSlice {
        instance_id: ActivityInstanceId::from_bytes([82; 16]),
        slice_id: SliceId::from_bytes([92; 16]),
        model_version: ModelVersionId::from_bytes([102; 16]),
        liability_account: liability,
        interest_expense_account: interest,
        cash_account: cash,
    };
    let loan = FixedRateLoan {
        opening_balance: principal,
        remaining_months: 120,
        annual_rate: rust_decimal::Decimal::ZERO,
        scheduled_payment: None,
    };
    let updated_loan = FixedRateLoan {
        opening_balance: money("200"),
        remaining_months: 120,
        annual_rate: rust_decimal::Decimal::ZERO,
        scheduled_payment: None,
    };
    let increase = money("100");
    let principal_transaction = LedgerTransaction::new(
        LedgerTransactionId::from_bytes([111; 16]),
        period,
        [
            LedgerEntry::new(cash, BalanceSide::Debit, increase)
                .with_cash_flow_category(CashFlowCategory::Financing),
            LedgerEntry::new(liability, BalanceSide::Credit, increase),
        ],
    );
    let update = property_simulation_simulation::apply_loan_terms_update(
        loan,
        period,
        updated_loan.opening_balance,
        updated_loan.remaining_months,
        updated_loan.annual_rate,
        Some(principal_transaction),
    )
    .unwrap();
    let monthly = updated_loan.calculate_month().unwrap();
    let (result, committed) = coordinate_month_with_loans(
        &input,
        &ledger,
        vec![repair_output],
        vec![ActivitySample::new(
            &repair.instance,
            period,
            ActivitySampleValue::Repair(repair_sample),
        )],
        TrialId::from_bytes([91; 16]),
        BranchId::from_bytes([92; 16]),
        &Ids,
        vec![LoanMonthInput {
            slice: loan_slice.clone(),
            loan: Some(loan),
            update: Some(update),
        }],
    )
    .unwrap();
    let expected_cash = opening_cash.checked_sub(monthly.principal).unwrap();
    assert_eq!(result.closing_state().cash, expected_cash);
    assert_eq!(committed.balance(cash).unwrap().1, expected_cash);
    assert_eq!(
        committed.balance(liability).unwrap().1,
        monthly.closing_balance
    );
    let projected = committed
        .project(period, period, &[cash])
        .unwrap()
        .cash_flow_statement
        .totals;
    let contributions = result.cash_flow_totals().unwrap();
    assert_eq!(contributions.operating, projected.operating);
    assert_eq!(contributions.investing, projected.investing);
    assert_eq!(contributions.financing, projected.financing);
}

fn assert_source_and_ids(result: &MonthlyResult) {
    let proposal = result.proposal();
    let events: std::collections::BTreeSet<_> =
        proposal.events().iter().map(|item| item.id).collect();
    let effects: std::collections::BTreeSet<_> =
        proposal.effects().iter().map(|item| item.id).collect();
    let transactions: std::collections::BTreeSet<_> =
        proposal.transactions().iter().map(|item| item.id).collect();
    assert_eq!(events.len(), proposal.events().len());
    assert_eq!(effects.len(), proposal.effects().len());
    assert_eq!(transactions.len(), proposal.transactions().len());
    for transaction in proposal.transactions() {
        if let Some(event_id) = transaction.source_event_id {
            assert!(events.contains(&event_id));
        }
    }
    for effect in proposal.effects() {
        if let Some(event_id) = effect.source_event_id {
            assert!(events.contains(&event_id));
        }
    }
    for contribution in result.cash_flow_contributions() {
        assert!(transactions.contains(&contribution.transaction_id));
        assert_eq!(contribution.trial_id, result.trial_id());
        assert_eq!(contribution.branch_id, result.branch_id());
        assert_eq!(contribution.period, result.period());
    }
}

fn assert_utility_sample_provenance(result: &MonthlyResult) {
    let utility_samples: Vec<_> = result
        .samples()
        .iter()
        .filter(|sample| matches!(sample.value, ActivitySampleValue::Utility(_)))
        .collect();
    assert_eq!(utility_samples.len(), 2);
    let identities: std::collections::BTreeSet<_> = utility_samples
        .iter()
        .map(|sample| sample.instance_id.as_bytes()[0])
        .collect();
    assert_eq!(identities, [7, 8].into_iter().collect());
    for sample in utility_samples {
        let id = sample.instance_id.as_bytes()[0];
        assert_eq!(sample.slice_id.as_bytes()[0], id + 10);
        assert_eq!(sample.version.as_bytes()[0], id + 20);
        assert_eq!(sample.period, result.period());
        assert_eq!(sample.purpose.as_deref(), Some("utility.usage"));
    }
}

fn assert_cash_flow_matches_snapshot(result: &MonthlyResult, ledger: &Ledger, cash: AccountId) {
    let projection = ledger
        .validate_projection(result.period(), result.period(), &[cash])
        .unwrap()
        .project()
        .unwrap();
    let contributions = CashFlowContributionTotals::sum(
        result
            .cash_flow_contributions()
            .iter()
            .map(|item| (item.category, item.amount)),
        result.closing_state().cash.currency(),
    )
    .unwrap();
    assert_eq!(
        contributions.operating,
        projection.cash_flow_statement.totals.operating
    );
    assert_eq!(
        contributions.investing,
        projection.cash_flow_statement.totals.investing
    );
    assert_eq!(
        contributions.financing,
        projection.cash_flow_statement.totals.financing
    );
    for continuation in result.loan_continuations() {
        let loan_account = match continuation.instance_id.as_bytes()[0] {
            3 => account_id(6),
            4 => account_id(7),
            _ => unreachable!(),
        };
        let (side, balance) = ledger.balance(loan_account).unwrap();
        assert_eq!(side, BalanceSide::Credit);
        assert_eq!(
            continuation
                .loan
                .map(|loan| loan.opening_balance)
                .unwrap_or_else(|| money("0")),
            balance
        );
    }
}

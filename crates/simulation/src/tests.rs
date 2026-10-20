use super::*;
use property_simulation_accounting::ChartOfAccounts;
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, Currency, EffectId, EventId, LedgerTransactionId,
    ModelVersionId, Money, SliceId, TrialId, YearMonth,
};
use property_simulation_models::{
    RandomSource, RepairCostDistribution, RepairDistribution, RepairSample, RepairSampler,
    SamplingError,
};

use crate::proposals::identify_proposal_for;
struct TestIds;
fn output_bytes(
    kind: u8,
    trial: TrialId,
    branch: BranchId,
    period: YearMonth,
    instance: ActivityInstanceId,
    key: &str,
) -> [u8; 16] {
    let mut state = 0xcbf29ce484222325u64;
    for byte in [kind]
        .into_iter()
        .chain(trial.as_bytes().iter().copied())
        .chain(branch.as_bytes().iter().copied())
        .chain(period.year().to_le_bytes())
        .chain([period.month()])
        .chain(instance.as_bytes().iter().copied())
        .chain(key.bytes())
    {
        state ^= u64::from(byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    let mut result = [0; 16];
    result[..8].copy_from_slice(&state.to_le_bytes());
    result[8..].copy_from_slice(
        &state
            .rotate_left(29)
            .wrapping_mul(0x9e3779b97f4a7c15)
            .to_le_bytes(),
    );
    result
}
impl OutputIdFactory for TestIds {
    fn event_id(
        &self,
        t: TrialId,
        b: BranchId,
        p: YearMonth,
        i: ActivityInstanceId,
        k: &str,
    ) -> EventId {
        EventId::from_bytes(output_bytes(1, t, b, p, i, k))
    }
    fn effect_id(
        &self,
        t: TrialId,
        b: BranchId,
        p: YearMonth,
        i: ActivityInstanceId,
        k: &str,
    ) -> EffectId {
        EffectId::from_bytes(output_bytes(2, t, b, p, i, k))
    }
    fn transaction_id(
        &self,
        t: TrialId,
        b: BranchId,
        p: YearMonth,
        i: ActivityInstanceId,
        k: &str,
    ) -> LedgerTransactionId {
        LedgerTransactionId::from_bytes(output_bytes(3, t, b, p, i, k))
    }
}
fn identity() -> OutputIdentity<'static> {
    static IDS: TestIds = TestIds;
    OutputIdentity {
        trial: TrialId::from_bytes([6; 16]),
        branch: BranchId::from_bytes([7; 16]),
        factory: &IDS,
    }
}
fn identity_for(trial: u8, branch: u8) -> OutputIdentity<'static> {
    static IDS: TestIds = TestIds;
    OutputIdentity {
        trial: TrialId::from_bytes([trial; 16]),
        branch: BranchId::from_bytes([branch; 16]),
        factory: &IDS,
    }
}
fn money(s: &str) -> Money {
    Money::parse(s, Currency::from_code("CAD").unwrap()).unwrap()
}

struct FixedWords(Vec<u64>);
impl RandomSource for FixedWords {
    type Error = &'static str;
    fn next_u64(&mut self) -> Result<u64, Self::Error> {
        if self.0.is_empty() {
            Err("empty")
        } else {
            Ok(self.0.remove(0))
        }
    }
}

#[test]
fn repair_sampling_composition_binds_activity_and_period_provenance() {
    let instance = activity(4);
    let distribution = RepairDistribution {
        occurrence_numerator: 0,
        occurrence_denominator: 1,
        cost: RepairCostDistribution {
            maximum: money("1.23"),
        },
    };
    let mut sampler = RepairSampler;
    let mut random = FixedWords(vec![]);
    let sample = sample_repair(
        &instance,
        YearMonth::new(2025, 1).unwrap(),
        &distribution,
        &mut sampler,
        &mut random,
    )
    .unwrap();
    assert_eq!(sample.instance_id, instance.id);
    assert_eq!(sample.slice_id, instance.slice_id);
    assert_eq!(sample.version, instance.version);
    assert_eq!(sample.period, YearMonth::new(2025, 1).unwrap());
    assert_eq!(sample.value, RepairSample::NoRepair);

    let mut incompatible = rental_activity(4);
    incompatible.slice_id = instance.slice_id;
    let mut random = FixedWords(vec![]);
    assert_eq!(
        sample_repair(
            &incompatible,
            sample.period,
            &distribution,
            &mut sampler,
            &mut random
        ),
        Err(SamplingError::InvalidDistribution)
    );
}
fn activity(id: u8) -> ActivityInstance {
    activity_with_kind(id, ActivitySampleKind::Repair)
}
fn rental_activity(id: u8) -> ActivityInstance {
    activity_with_kind(id, ActivitySampleKind::Rental)
}
fn activity_with_kind(id: u8, sample_kind: ActivitySampleKind) -> ActivityInstance {
    let mut b = [0; 16];
    b[15] = id;
    ActivityInstance {
        id: ActivityInstanceId::from_bytes(b),
        slice_id: SliceId::from_bytes([1; 16]),
        version: ModelVersionId::from_bytes([2; 16]),
        sample_kind,
    }
}
fn input() -> FrozenInput {
    FrozenInput {
        period: YearMonth::new(2025, 1).unwrap(),
        rent: money("80"),
        rent_account: AccountId::from_bytes([3; 16]),
        cash_account: AccountId::from_bytes([4; 16]),
        repair_expense_account: AccountId::from_bytes([5; 16]),
        repair_cost_limit: money("50"),
    }
}
fn validated_input(state: &OpeningState, frozen: &FrozenInput) -> ValidatedInput {
    validate_input(
        frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &ledger_for_cash(state.cash),
        state,
    )
    .unwrap()
}
fn chart() -> ChartOfAccounts {
    use property_simulation_accounting::{Account, AccountCategory, AccountCode};
    ChartOfAccounts::new([
        Account::new_cash_position(
            AccountId::from_bytes([4; 16]),
            AccountCode::new(1000).unwrap(),
            "Cash",
        )
        .unwrap(),
        Account::new(
            AccountId::from_bytes([3; 16]),
            AccountCode::new(4000).unwrap(),
            AccountCategory::Income,
            false,
            "Rent",
        )
        .unwrap(),
        Account::new(
            AccountId::from_bytes([5; 16]),
            AccountCode::new(5000).unwrap(),
            AccountCategory::Expense,
            false,
            "Repairs",
        )
        .unwrap(),
        Account::new(
            AccountId::from_bytes([6; 16]),
            AccountCode::new(3000).unwrap(),
            AccountCategory::Equity,
            false,
            "Opening equity",
        )
        .unwrap(),
    ])
    .unwrap()
}
fn ledger_for_cash(cash: Money) -> property_simulation_accounting::Ledger {
    use property_simulation_accounting::{BalanceSide, Ledger, OpeningBalance, OpeningBalances};
    let chart = chart();
    let amount = if cash.is_negative() {
        cash.checked_neg().unwrap()
    } else {
        cash
    };
    let entries = if amount.is_zero() {
        Vec::new()
    } else {
        let cash_side = if cash.is_negative() {
            BalanceSide::Credit
        } else {
            BalanceSide::Debit
        };
        let equity_side = if cash_side == BalanceSide::Debit {
            BalanceSide::Credit
        } else {
            BalanceSide::Debit
        };
        vec![
            OpeningBalance::new(input().cash_account, cash_side, amount).unwrap(),
            OpeningBalance::new(AccountId::from_bytes([6; 16]), equity_side, amount).unwrap(),
        ]
    };
    let opening = OpeningBalances::new(&chart, entries, Some(cash)).unwrap();
    Ledger::new(chart, opening).unwrap()
}
fn sample<T>(instance: &ActivityInstance, value: T) -> ActivitySample<T> {
    ActivitySample::new(instance, input().period, value)
}

#[test]
fn coordinator_posts_rent_and_repair_and_derives_month_end_state_from_ledger() {
    use crate::{RentalSlice, RepairSlice, coordinate_month, propose_slice};
    let mut frozen = input();
    frozen.rent = money("80");
    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::NeedsRepair,
    };
    let ledger = ledger_for_cash(state.cash);
    let validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &ledger,
        &state,
    )
    .unwrap();
    let rental = rental_activity(11);
    let repair = activity(12);
    let rental_output = propose_slice(
        &RentalSlice {
            instance: rental.clone(),
        },
        &validated,
        &sample(&rental, ()),
    )
    .unwrap();
    let repair_output = propose_slice(
        &RepairSlice {
            instance: repair.clone(),
        },
        &validated,
        &sample(&repair, RepairSample::Repair { cost: money("10") }),
    )
    .unwrap();
    let samples = vec![
        ActivitySample::new(&rental, frozen.period, ActivitySampleValue::Rental),
        ActivitySample::new(
            &repair,
            frozen.period,
            ActivitySampleValue::Repair(RepairSample::Repair { cost: money("10") }),
        ),
    ];
    let mismatched_samples = vec![
        samples[0].clone(),
        ActivitySample::new(
            &repair,
            frozen.period,
            ActivitySampleValue::Repair(RepairSample::NoRepair),
        ),
    ];
    assert!(
        coordinate_month(
            &validated,
            &ledger,
            vec![repair_output.clone(), rental_output.clone()],
            mismatched_samples,
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            &TestIds,
        )
        .is_err()
    );
    assert_eq!(ledger.transactions().count(), 0);
    let mut changed_frozen = frozen;
    changed_frozen.rent = money("90");
    let changed_input = validate_input(
        &changed_frozen,
        &RepairCostDistribution {
            maximum: changed_frozen.repair_cost_limit,
        },
        &ledger,
        &state,
    )
    .unwrap();
    assert!(
        coordinate_month(
            &changed_input,
            &ledger,
            vec![repair_output.clone(), rental_output.clone()],
            samples.clone(),
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            &TestIds,
        )
        .is_err()
    );
    assert_eq!(ledger.transactions().count(), 0);
    let forward_outputs = vec![repair_output.clone(), rental_output.clone()];
    let (result, closing_ledger) = coordinate_month(
        &validated,
        &ledger,
        forward_outputs.clone(),
        samples.clone(),
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        &TestIds,
    )
    .unwrap();
    assert_eq!(result.closing_state().cash, money("170"));
    assert_eq!(result.closing_state().condition, AssetCondition::Available);
    assert_eq!(
        closing_ledger.balance(frozen.cash_account).unwrap().1,
        money("170")
    );
    assert_eq!(closing_ledger.transactions().count(), 2);
    assert_eq!(result.cash_flow_contributions().len(), 2);
    assert_eq!(result.cash_flow_totals().unwrap().operating, money("70"));
    let projection = closing_ledger
        .project(frozen.period, frozen.period, &[frozen.cash_account])
        .unwrap();
    assert_eq!(
        result.cash_flow_totals().unwrap().operating,
        projection.cash_flow_statement.totals.operating
    );
    let mut amounts = result
        .cash_flow_contributions()
        .iter()
        .map(|c| c.amount)
        .collect::<Vec<_>>();
    amounts.sort_by_key(|amount| amount.coefficient());
    assert_eq!(amounts, vec![money("-10"), money("80")]);
    for contribution in result.cash_flow_contributions() {
        assert!(closing_ledger.transactions().any(|transaction| {
            transaction.id() == contribution.transaction_id
                && transaction.source().is_some_and(|source| {
                    source.instance_id == contribution.instance_id
                        && source.slice_id == contribution.slice_id
                        && source.model_version == contribution.model_version
                        && source.output_key == contribution.output_key
                })
        }));
    }
    assert_eq!(ledger.balance(frozen.cash_account).unwrap().1, money("100"));
    let (permuted_result, permuted_ledger) = coordinate_month(
        &validated,
        &ledger,
        forward_outputs.into_iter().rev().collect(),
        samples.into_iter().rev().collect(),
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        &TestIds,
    )
    .unwrap();
    assert_eq!(permuted_result, result);
    assert_eq!(permuted_ledger, closing_ledger);
}

#[test]
fn coordinator_rejects_a_ledger_other_than_the_validated_opening_ledger() {
    use crate::coordinate_month;
    use property_simulation_accounting::{
        BalanceSide, CashFlowCategory, LedgerEntry, LedgerTransaction,
    };

    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(state.cash);
    let frozen = input();
    let validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &ledger,
        &state,
    )
    .unwrap();
    let mut other_ledger = ledger.clone();
    other_ledger
        .post(LedgerTransaction::new(
            LedgerTransactionId::from_bytes([20; 16]),
            frozen.period,
            [
                LedgerEntry::new(
                    frozen.repair_expense_account,
                    BalanceSide::Debit,
                    money("1"),
                ),
                LedgerEntry::new(frozen.cash_account, BalanceSide::Credit, money("1"))
                    .with_cash_flow_category(CashFlowCategory::Operating),
            ],
        ))
        .unwrap();

    assert!(matches!(
        coordinate_month(
            &validated,
            &other_ledger,
            vec![],
            vec![],
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            &TestIds
        ),
        Err(SimulationError::InvalidInput(
            "opening ledger differs from the ledger used to validate monthly input"
        ))
    ));
    assert_eq!(other_ledger.transactions().count(), 1);
    assert_eq!(ledger.transactions().count(), 0);
}

#[test]
fn coordinator_rejects_unknown_effect_and_invalid_journal_without_partial_commit() {
    use crate::{
        EffectProposal, RepairSlice, SliceProposal, TransactionProposal, coordinate_month,
    };
    use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};
    use property_simulation_domain::ResourceKey;

    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::NeedsRepair,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &ledger,
        &state,
    )
    .unwrap();
    let instance = activity(21);
    let slice = RepairSlice {
        instance: instance.clone(),
    };
    let mut resources = slice.resources(&frozen);
    let unknown_target = ResourceKey::new("property.unknown").unwrap();
    resources.writes.insert(unknown_target.clone());
    let sample = ActivitySample::new(
        &instance,
        frozen.period,
        ActivitySampleValue::Repair(RepairSample::NoRepair),
    );
    let output = |proposal| crate::SliceOutput {
        proposal,
        instance: instance.clone(),
        period: frozen.period,
        opening_state: state.clone(),
        frozen_input: frozen,
        resources: resources.clone(),
        sample: sample.clone(),
    };

    let unknown_effect = SliceProposal {
        effects: vec![EffectProposal {
            period: frozen.period,
            target: unknown_target,
            key: "unknown".into(),
            source_event_key: None,
            condition: AssetCondition::Available,
        }],
        ..Default::default()
    };
    assert!(matches!(
        coordinate_month(
            &validated,
            &ledger,
            vec![output(unknown_effect)],
            vec![sample.clone()],
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            &TestIds,
        ),
        Err(SimulationError::InvalidProposal(
            "unsupported Effect target"
        ))
    ));
    assert_eq!(ledger.transactions().count(), 0);

    let invalid_journal = SliceProposal {
        transactions: vec![TransactionProposal {
            key: "unbalanced".into(),
            period: frozen.period,
            entries: vec![
                LedgerEntry::new(
                    frozen.repair_expense_account,
                    BalanceSide::Debit,
                    money("2"),
                ),
                LedgerEntry::new(frozen.cash_account, BalanceSide::Credit, money("1"))
                    .with_cash_flow_category(CashFlowCategory::Operating),
            ],
            source_event_key: None,
        }],
        ..Default::default()
    };
    assert!(matches!(
        coordinate_month(
            &validated,
            &ledger,
            vec![output(invalid_journal)],
            vec![sample],
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            &TestIds,
        ),
        Err(SimulationError::InvalidTransactions(report))
            if report.0.len() == 1
                && report.0[0].transaction_key.as_deref() == Some("unbalanced")
                && matches!(
                    report.0[0].error,
                    property_simulation_accounting::AccountError::TransactionDoesNotBalance { .. }
                )
    ));
    assert_eq!(ledger.transactions().count(), 0);
    assert_eq!(ledger.balance(frozen.cash_account).unwrap().1, money("100"));
}

#[test]
fn slice_transition_errors_are_fatal_with_activity_and_month_diagnostics() {
    struct BrokenSlice(ActivityInstance);
    impl SimulationSlice for BrokenSlice {
        type Sample = RepairSample;
        const SAMPLE_KIND: ActivitySampleKind = ActivitySampleKind::Repair;
        fn instance(&self) -> &ActivityInstance {
            &self.0
        }
        fn resources(&self, input: &FrozenInput) -> SliceResources {
            RepairSlice {
                instance: self.0.clone(),
            }
            .resources(input)
        }
        fn sample_value(&self, sample: &RepairSample) -> ActivitySampleValue {
            ActivitySampleValue::Repair(*sample)
        }
        fn propose_validated(
            &self,
            _: &OpeningState,
            _: &FrozenInput,
            _: &ValidatedActivitySample<'_, RepairSample>,
        ) -> Result<SliceProposal, SimulationError> {
            Err(property_simulation_domain::DomainError::ArithmeticOverflow.into())
        }
    }

    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &ledger,
        &state,
    )
    .unwrap();
    let slice = BrokenSlice(activity(29));
    let sample = ActivitySample::new(&slice.0, frozen.period, RepairSample::NoRepair);
    let error = propose_slice(&slice, &validated, &sample).unwrap_err();
    let SimulationError::Fatal(failure) = error else {
        panic!("expected fatal failure")
    };
    assert_eq!(failure.phase, FatalSimulationPhase::SliceTransition);
    assert_eq!(failure.period, frozen.period);
    assert_eq!(failure.activity_instance_id, Some(slice.0.id));
    assert_eq!(failure.trial_id, None);
    assert_eq!(failure.branch_id, None);
    assert_eq!(
        *failure.source,
        SimulationError::Domain(property_simulation_domain::DomainError::ArithmeticOverflow)
    );
}

#[test]
fn input_validation_checks_repair_distribution_support_before_sampling() {
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let support = RepairCostDistribution {
        maximum: money("50"),
    };
    let validated =
        validate_input(&input(), &support, &ledger_for_cash(state.cash), &state).unwrap();
    assert_eq!(validated.opening_state(), &state);
    assert_eq!(validated.frozen_input(), &input());
    assert_eq!(validated.repair_distribution(), &support);
    assert!(
        validate_input(
            &input(),
            &RepairCostDistribution {
                maximum: money("49"),
            },
            &ledger_for_cash(state.cash),
            &state,
        )
        .is_err()
    );
    let errors = validate_input(
        &input(),
        &RepairCostDistribution {
            maximum: money("-1"),
        },
        &ledger_for_cash(state.cash),
        &state,
    )
    .unwrap_err();
    assert!(
        errors
            .0
            .iter()
            .any(|issue| issue.field == "repair_distribution.maximum")
    );
    assert!(
        errors
            .0
            .iter()
            .any(|issue| issue.message.contains("must match"))
    );

    let mut oversized_input = input();
    oversized_input.repair_cost_limit = money("184467440737095516.16");
    let oversized_errors = validate_input(
        &oversized_input,
        &RepairCostDistribution {
            maximum: oversized_input.repair_cost_limit,
        },
        &ledger_for_cash(state.cash),
        &state,
    )
    .unwrap_err();
    assert!(oversized_errors.0.iter().any(|issue| {
        issue.field == "repair_distribution.maximum"
            && issue.message.contains("sampler's representable range")
    }));
}

#[test]
fn active_cashflow_support_bound_rejects_unrepresentable_monthly_absolute_sum() {
    let mut frozen = input();
    frozen.rent = Money::from_parts(i128::MAX, 0, frozen.rent.currency()).unwrap();
    frozen.repair_cost_limit = Money::from_parts(i128::MAX, 0, frozen.rent.currency()).unwrap();
    let active = vec![rental_activity(41), activity(42)];
    assert!(crate::validation::validate_cashflow_support(&frozen, &active).is_err());
    assert!(crate::validation::validate_cashflow_support(&frozen, &[rental_activity(43)]).is_ok());
}

#[test]
fn contribution_aggregation_cancels_before_applying_money_range() {
    use property_simulation_accounting::CashFlowCategory;
    let currency = money("0").currency();
    let maximum = Money::from_parts(i128::MAX, 0, currency).unwrap();
    let values = [maximum, maximum.checked_neg().unwrap(), maximum];
    let totals = CashFlowContributionTotals::sum(
        values
            .into_iter()
            .map(|amount| (CashFlowCategory::Operating, amount)),
        currency,
    )
    .unwrap();
    assert_eq!(totals.operating, maximum);
    assert_eq!(
        CashFlowContributionTotals::sum([], currency).unwrap(),
        CashFlowContributionTotals::zero(currency)
    );
    let reversed = CashFlowContributionTotals::sum(
        values
            .into_iter()
            .rev()
            .map(|amount| (CashFlowCategory::Operating, amount)),
        currency,
    )
    .unwrap();
    assert_eq!(reversed, totals);
    assert!(
        CashFlowContributionTotals::sum(
            [maximum, money("1")]
                .into_iter()
                .map(|amount| (CashFlowCategory::Operating, amount)),
            currency,
        )
        .is_err()
    );
    let fractions = CashFlowContributionTotals::sum(
        [money("1.25"), money("-1.2")]
            .into_iter()
            .map(|amount| (CashFlowCategory::Operating, amount)),
        currency,
    )
    .unwrap();
    assert_eq!(fractions.operating, money("0.05"));
}

#[test]
fn input_validation_collects_independent_issues_in_one_pass() {
    let state = OpeningState {
        cash: Money::parse("0", Currency::from_code("USD").unwrap()).unwrap(),
        condition: AssetCondition::Available,
    };
    let mut frozen = input();
    frozen.rent = money("-1");
    frozen.cash_account = AccountId::from_bytes([99; 16]);
    frozen.rent_account = AccountId::from_bytes([98; 16]);

    let errors = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: money("50"),
        },
        &ledger_for_cash(money("0")),
        &state,
    )
    .unwrap_err();

    assert!(errors.0.iter().any(|issue| issue.field == "rent"));
    assert!(errors.0.iter().any(|issue| issue.field == "cash_account"));
    assert!(errors.0.iter().any(|issue| issue.field == "rent_account"));
    assert!(
        errors
            .0
            .iter()
            .any(|issue| issue.field == "opening_state.cash")
    );
}

#[test]
fn input_validation_checks_the_ledger_currency_before_sampling() {
    use property_simulation_accounting::{Ledger, OpeningBalances};

    let chart = chart();
    let usd = Currency::from_code("USD").unwrap();
    let opening = OpeningBalances::new(&chart, [], Some(Money::parse("0", usd).unwrap())).unwrap();
    let usd_ledger = Ledger::new(chart, opening).unwrap();
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };

    let errors = validate_input(
        &input(),
        &RepairCostDistribution {
            maximum: money("50"),
        },
        &usd_ledger,
        &state,
    )
    .unwrap_err();

    assert!(errors.0.iter().any(|issue| issue.field == "rent"));
    assert!(
        errors
            .0
            .iter()
            .any(|issue| issue.field == "repair_cost_limit")
    );
}

#[test]
fn input_validation_rejects_cash_state_that_differs_from_ledger_balance() {
    let state = OpeningState {
        cash: money("10"),
        condition: AssetCondition::Available,
    };
    let errors = validate_input(
        &input(),
        &RepairCostDistribution {
            maximum: money("50"),
        },
        &ledger_for_cash(money("0")),
        &state,
    )
    .unwrap_err();

    assert!(errors.0.iter().any(|issue| {
        issue.field == "opening_state.cash" && issue.message.contains("match the ledger")
    }));
}

#[test]
fn input_validation_rejects_wrong_account_roles() {
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let mut frozen = input();
    frozen.cash_account = input().rent_account;
    frozen.rent_account = input().repair_expense_account;
    frozen.repair_expense_account = input().rent_account;
    let errors = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &ledger_for_cash(state.cash),
        &state,
    )
    .unwrap_err();

    assert!(errors.0.iter().any(|issue| issue.field == "cash_account"));
    assert!(errors.0.iter().any(|issue| issue.field == "rent_account"));
    assert!(
        errors
            .0
            .iter()
            .any(|issue| issue.field == "repair_expense_account")
    );
}

#[test]
fn proposal_transaction_validation_uses_ledger_rules_without_mutating_ledger() {
    use property_simulation_accounting::{
        BalanceSide, CashFlowCategory, Ledger, LedgerEntry, OpeningBalances,
    };

    let chart = chart();
    let opening = OpeningBalances::new(&chart, [], Some(money("0"))).unwrap();
    let ledger = Ledger::new(chart, opening).unwrap();
    let mut proposal = IdentifiedProposal {
        identity: None,
        events: Vec::new(),
        effects: Vec::new(),
        transactions: vec![ProposedTransaction {
            id: LedgerTransactionId::from_bytes([77; 16]),
            instance_id: activity(1).id,
            slice_id: activity(1).slice_id,
            version: activity(1).version,
            period: input().period,
            key: "unbalanced".into(),
            source_event_id: None,
            entries: vec![
                LedgerEntry::new(input().cash_account, BalanceSide::Debit, money("1"))
                    .with_cash_flow_category(CashFlowCategory::Operating),
                LedgerEntry::new(input().rent_account, BalanceSide::Credit, money("2")),
            ],
        }],
    };
    let mut second = proposal.transactions[0].clone();
    second.id = LedgerTransactionId::from_bytes([78; 16]);
    second.key = "also_unbalanced".into();
    proposal.transactions.push(second);

    let errors = validate_proposal_transactions(&ledger, &proposal).unwrap_err();
    assert_eq!(errors.0.len(), 2);
    assert!(errors.0.iter().any(|issue| {
        issue.transaction_key.as_deref() == Some("unbalanced")
            && matches!(
                issue.error,
                property_simulation_accounting::AccountError::TransactionDoesNotBalance { .. }
            )
    }));
    assert!(errors.0.iter().any(|issue| {
        issue.transaction_key.as_deref() == Some("also_unbalanced")
            && matches!(
                issue.error,
                property_simulation_accounting::AccountError::TransactionDoesNotBalance { .. }
            )
    }));
    assert_eq!(ledger.transactions().count(), 0);
}

#[test]
fn fractional_minor_unit_inputs_are_rounded_half_up_before_proposing_postings() {
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let mut frozen = input();
    frozen.rent = money("80.005");
    frozen.repair_cost_limit = money("50.005");
    let support = RepairCostDistribution {
        maximum: frozen.repair_cost_limit,
    };
    assert!(validate_input(&frozen, &support, &ledger_for_cash(state.cash), &state).is_ok());

    let rental = RentalSlice {
        instance: rental_activity(10),
    };
    let rental_proposal = propose_slice(
        &rental,
        &validated_input(&state, &frozen),
        &ActivitySample::new(&rental.instance, frozen.period, ()),
    )
    .unwrap();
    assert_eq!(
        rental_proposal.transactions[0].entries[0].amount(),
        money("80.01")
    );
    assert_eq!(
        rental_proposal.transactions[0].entries[1].amount(),
        money("80.01")
    );

    let repair = RepairSlice {
        instance: activity(11),
    };
    let repair_proposal = propose_slice(
        &repair,
        &validated_input(&state, &frozen),
        &ActivitySample::new(
            &repair.instance,
            frozen.period,
            RepairSample::Repair {
                cost: money("1.00"),
            },
        ),
    )
    .unwrap();
    assert_eq!(
        repair_proposal.transactions[0].entries[0].amount(),
        money("1.00")
    );
    assert_eq!(
        repair_proposal.transactions[0].entries[1].amount(),
        money("1.00")
    );

    frozen.rent = money("0.004");
    let zero_rent = propose_slice(
        &rental,
        &validated_input(&state, &frozen),
        &ActivitySample::new(&rental.instance, frozen.period, ()),
    )
    .unwrap();
    assert!(zero_rent.events.is_empty());
    assert!(zero_rent.transactions.is_empty());
}

#[test]
fn repair_sample_controls_expense_and_condition_effect() {
    let slice = RepairSlice {
        instance: activity(8),
    };
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::NeedsRepair,
    };
    let none = propose_slice(
        &slice,
        &validated_input(&state, &input()),
        &sample(&slice.instance, RepairSample::NoRepair),
    )
    .unwrap();
    assert!(none.transactions.is_empty() && none.effects.is_empty());
    let repaired = propose_slice(
        &slice,
        &validated_input(&state, &input()),
        &sample(&slice.instance, RepairSample::Repair { cost: money("50") }),
    )
    .unwrap();
    assert_eq!(repaired.transactions.len(), 1);
    assert_eq!(repaired.events.len(), 1);
    assert_eq!(repaired.effects[0].condition, AssetCondition::Available);
    assert_eq!(
        repaired.effects[0].source_event_key.as_deref(),
        Some("repair_performed")
    );
    assert_eq!(
        repaired.transactions[0].source_event_key.as_deref(),
        Some("repair_performed")
    );
    assert_eq!(
        repaired.transactions[0]
            .entries
            .iter()
            .map(|e| e.amount())
            .collect::<Vec<_>>(),
        [money("50"), money("50")]
    );
    assert!(
        propose_slice(
            &slice,
            &validated_input(&state, &input()),
            &sample(
                &slice.instance,
                RepairSample::Repair {
                    cost: money("50.01")
                }
            ),
        )
        .is_err()
    );

    let available_state = OpeningState {
        cash: money("10"),
        condition: AssetCondition::Available,
    };
    let preventive = propose_slice(
        &slice,
        &validated_input(&available_state, &input()),
        &sample(&slice.instance, RepairSample::Repair { cost: money("5") }),
    )
    .unwrap();
    assert_eq!(preventive.transactions.len(), 1);
    assert_eq!(preventive.effects[0].condition, AssetCondition::Available);
}

#[test]
fn sample_provenance_must_match_the_slice_instance_and_frozen_period() {
    let slice = RepairSlice {
        instance: activity(8),
    };
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let wrong_instance = sample(&activity(9), RepairSample::NoRepair);
    assert_eq!(
        propose_slice(&slice, &validated_input(&state, &input()), &wrong_instance),
        Err(SimulationError::InvalidInput(
            "sample provenance does not match activity instance or period"
        ))
    );
    let wrong_period = ActivitySample::new(
        &slice.instance,
        input().period.checked_add_months(1).unwrap(),
        RepairSample::NoRepair,
    );
    assert!(propose_slice(&slice, &validated_input(&state, &input()), &wrong_period).is_err());
    let wrong_kind_slice = RepairSlice {
        instance: rental_activity(8),
    };
    let wrong_kind_sample = sample(&wrong_kind_slice.instance, RepairSample::NoRepair);
    assert_eq!(
        propose_slice(
            &wrong_kind_slice,
            &validated_input(&state, &input()),
            &wrong_kind_sample
        ),
        Err(SimulationError::InvalidInput(
            "activity sample kind does not match Slice"
        ))
    );
}
#[test]
fn rental_proposal_is_deterministic_and_merge_has_empty_identity() {
    let slice = RentalSlice {
        instance: rental_activity(9),
    };
    let state = OpeningState {
        cash: money("-2"),
        condition: AssetCondition::Available,
    };
    let validated = validated_input(&state, &input());
    let a = propose_slice(&slice, &validated, &sample(&slice.instance, ())).unwrap();
    let b = propose_slice(&slice, &validated, &sample(&slice.instance, ())).unwrap();
    assert_eq!(a, b);
    let a = identify_proposal(a, identity()).unwrap();
    assert_eq!(a.events().len(), 1);
    let posted = a.transactions()[0].clone().into_ledger_transaction();
    assert_eq!(posted.source().unwrap().instance_id, slice.instance.id);
    assert_eq!(posted.source().unwrap().output_key, "rent");
    assert_eq!(
        posted.source().unwrap().source_event_id,
        Some(a.events()[0].id)
    );
    assert_eq!(merge_proposals([a.clone()]).unwrap(), a);
    assert_eq!(merge_proposals([]).unwrap(), IdentifiedProposal::default());
}

#[test]
fn zero_rent_does_not_create_a_collection_event_or_posting() {
    let slice = RentalSlice {
        instance: rental_activity(9),
    };
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let mut zero_rent = input();
    zero_rent.rent = money("0");
    let proposal = propose_slice(
        &slice,
        &validated_input(&state, &zero_rent),
        &sample(&slice.instance, ()),
    )
    .unwrap();
    assert!(proposal.events.is_empty());
    assert!(proposal.transactions.is_empty());
}

#[test]
fn output_ids_are_repeatable_and_use_all_identity_dimensions() {
    let factory = identity().factory;
    let trial = TrialId::from_bytes([6; 16]);
    let branch = BranchId::from_bytes([7; 16]);
    let period = YearMonth::new(2025, 1).unwrap();
    let instance = activity(4).id;
    let id = factory.transaction_id(trial, branch, period, instance, "rent");
    assert_eq!(
        id,
        factory.transaction_id(trial, branch, period, instance, "rent")
    );
    assert_ne!(
        id,
        factory.transaction_id(trial, branch, period, instance, "repair")
    );
    assert_ne!(
        id,
        factory.transaction_id(
            TrialId::from_bytes([8; 16]),
            branch,
            period,
            instance,
            "rent"
        )
    );
    assert_ne!(
        id,
        factory.transaction_id(
            trial,
            BranchId::from_bytes([8; 16]),
            period,
            instance,
            "rent"
        )
    );
    assert_ne!(
        id,
        factory.transaction_id(
            trial,
            branch,
            period.checked_add_months(1).unwrap(),
            instance,
            "rent"
        )
    );
    assert_ne!(
        id,
        factory.transaction_id(trial, branch, period, activity(5).id, "rent")
    );
    assert_ne!(
        factory
            .event_id(trial, branch, period, instance, "rent")
            .as_bytes(),
        id.as_bytes()
    );

    let slice = RentalSlice {
        instance: rental_activity(4),
    };
    let proposal = propose_slice(
        &slice,
        &validated_input(
            &OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available,
            },
            &input(),
        ),
        &sample(slice.instance(), ()),
    )
    .unwrap();
    let forward = identify_proposal(proposal.clone(), identity()).unwrap();
    let reverse = identify_proposal(proposal, identity()).unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(forward.transactions()[0].instance_id, slice.instance.id);
    assert_eq!(forward.transactions()[0].slice_id, slice.instance.slice_id);
    assert_eq!(forward.transactions()[0].version, slice.instance.version);
    assert_eq!(forward.transactions()[0].period, input().period);
    assert_ne!(
        forward,
        identify_proposal(
            propose_slice(
                &slice,
                &validated_input(
                    &OpeningState {
                        cash: money("0"),
                        condition: AssetCondition::Available
                    },
                    &input()
                ),
                &sample(slice.instance(), ())
            )
            .unwrap(),
            identity_for(8, 7),
        )
        .unwrap()
    );
}

#[test]
fn output_ids_do_not_depend_on_proposal_input_order() {
    let one = RentalSlice {
        instance: rental_activity(1),
    };
    let two = RentalSlice {
        instance: rental_activity(2),
    };
    let three = RentalSlice {
        instance: rental_activity(3),
    };
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let validated = validated_input(&state, &input());
    let proposal_one = propose_slice(&one, &validated, &sample(one.instance(), ())).unwrap();
    let proposal_two = propose_slice(&two, &validated, &sample(two.instance(), ())).unwrap();
    let proposal_three = propose_slice(&three, &validated, &sample(three.instance(), ())).unwrap();
    let identified_one = identify_proposal(proposal_one, identity()).unwrap();
    let identified_two = identify_proposal(proposal_two, identity()).unwrap();
    let identified_three = identify_proposal(proposal_three, identity()).unwrap();
    let forward = merge_proposals([
        identified_one.clone(),
        identified_two.clone(),
        identified_three.clone(),
    ])
    .unwrap();
    let reverse = merge_proposals([
        identified_three.clone(),
        identified_two.clone(),
        identified_one.clone(),
    ])
    .unwrap();
    let grouped = merge_proposals([
        merge_proposals([identified_one, identified_two]).unwrap(),
        identified_three,
    ])
    .unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(forward, grouped);
}

#[test]
fn identification_uses_coordinator_source_and_rejects_wrong_period() {
    let draft = SliceProposal {
        events: vec![EventProposal {
            period: input().period,
            key: "occurred".into(),
        }],
        ..Default::default()
    };
    let authoritative_instance = activity(12);
    let identified = identify_proposal_for(
        draft.clone(),
        identity(),
        &authoritative_instance,
        input().period,
    )
    .unwrap();
    assert_eq!(
        identified.events()[0].instance_id,
        authoritative_instance.id
    );
    assert_eq!(
        identified.events()[0].slice_id,
        authoritative_instance.slice_id
    );
    assert_eq!(
        identified.events()[0].version,
        authoritative_instance.version
    );

    assert_eq!(
        identify_proposal_for(
            draft,
            identity(),
            &authoritative_instance,
            input().period.checked_add_months(1).unwrap(),
        ),
        Err(SimulationError::InvalidProposal(
            "proposal period does not match frozen period"
        ))
    );
}

#[test]
fn transaction_keeps_optional_source_event_link_through_accounting_conversion() {
    let period = input().period;
    let proposal = SliceProposal {
        events: vec![EventProposal {
            period,
            key: "rent_collected".into(),
        }],
        transactions: vec![TransactionProposal {
            key: "rent".into(),
            period,
            entries: Vec::new(),
            source_event_key: Some("rent_collected".into()),
        }],
        ..Default::default()
    };
    let identified = identify_proposal_for(proposal, identity(), &activity(3), period).unwrap();
    let expected_event_id = identified.events()[0].id;
    let transaction = identified.transactions()[0]
        .clone()
        .into_ledger_transaction();
    assert_eq!(
        transaction.source().unwrap().source_event_id,
        Some(expected_event_id)
    );

    let effect_link = SliceProposal {
        events: vec![EventProposal {
            period,
            key: "repair_done".into(),
        }],
        effects: vec![EffectProposal {
            period,
            target: property_simulation_domain::ResourceKey::new("property.condition").unwrap(),
            key: "condition".into(),
            source_event_key: Some("repair_done".into()),
            condition: AssetCondition::Available,
        }],
        ..Default::default()
    };
    let identified = identify_proposal_for(effect_link, identity(), &activity(3), period).unwrap();
    assert_eq!(
        identified.effects()[0].source_event_id,
        Some(identified.events()[0].id)
    );

    let missing_event = SliceProposal {
        transactions: vec![TransactionProposal {
            key: "orphan".into(),
            period,
            entries: Vec::new(),
            source_event_key: Some("missing".into()),
        }],
        ..Default::default()
    };
    assert_eq!(
        identify_proposal_for(missing_event, identity(), &activity(3), period),
        Err(SimulationError::InvalidProposal(
            "transaction references missing event key"
        ))
    );

    let missing_effect_event = SliceProposal {
        effects: vec![EffectProposal {
            period,
            target: property_simulation_domain::ResourceKey::new("property.condition").unwrap(),
            key: "condition".into(),
            source_event_key: Some("missing".into()),
            condition: AssetCondition::Available,
        }],
        ..Default::default()
    };
    assert_eq!(
        identify_proposal_for(missing_effect_event, identity(), &activity(3), period),
        Err(SimulationError::InvalidProposal(
            "effect references missing event key"
        ))
    );
}

#[test]
fn effect_conflicts_are_reported_as_invalid_proposals() {
    let period = input().period;
    let first = SliceProposal {
        effects: vec![EffectProposal {
            period,
            target: property_simulation_domain::ResourceKey::new("property.condition").unwrap(),
            key: "condition".into(),
            source_event_key: None,
            condition: AssetCondition::Available,
        }],
        ..Default::default()
    };
    let second = SliceProposal {
        effects: vec![EffectProposal {
            period,
            target: property_simulation_domain::ResourceKey::new("property.condition").unwrap(),
            key: "condition".into(),
            source_event_key: None,
            condition: AssetCondition::NeedsRepair,
        }],
        ..Default::default()
    };
    let first = identify_proposal_for(first, identity(), &activity(1), period).unwrap();
    let second = identify_proposal_for(second, identity(), &activity(2), period).unwrap();
    assert_eq!(
        merge_proposals([first, second]),
        Err(SimulationError::InvalidProposal(
            "conflicting state effects"
        ))
    );
}

#[test]
fn equal_state_effect_writes_merge_idempotently_and_keep_both_provenances() {
    let period = input().period;
    let proposal = |instance| {
        identify_proposal_for(
            SliceProposal {
                effects: vec![EffectProposal {
                    period,
                    target: property_simulation_domain::ResourceKey::new("property.condition")
                        .unwrap(),
                    key: "condition".into(),
                    source_event_key: None,
                    condition: AssetCondition::Available,
                }],
                ..Default::default()
            },
            identity(),
            &activity(instance),
            period,
        )
        .unwrap()
    };
    let merged = merge_proposals([proposal(1), proposal(2)]).unwrap();
    assert_eq!(merged.effects().len(), 2);
    assert_ne!(merged.effects()[0].id, merged.effects()[1].id);
}

#[test]
fn effects_on_distinct_targets_do_not_conflict_and_keep_event_links() {
    let period = input().period;
    let make = |instance_id: u8, target: &str, event_key: &str| {
        identify_proposal_for(
            SliceProposal {
                events: vec![EventProposal {
                    period,
                    key: event_key.into(),
                }],
                effects: vec![EffectProposal {
                    period,
                    target: property_simulation_domain::ResourceKey::new(target).unwrap(),
                    key: format!("{target}.available"),
                    source_event_key: Some(event_key.into()),
                    condition: AssetCondition::Available,
                }],
                ..Default::default()
            },
            identity(),
            &activity(instance_id),
            period,
        )
        .unwrap()
    };

    let merged = merge_proposals([
        make(1, "property.1.condition", "repair_1"),
        make(2, "property.2.condition", "repair_2"),
    ])
    .unwrap();
    assert_eq!(merged.effects().len(), 2);
    for effect in merged.effects() {
        assert!(
            merged
                .events()
                .iter()
                .any(|event| event.id == effect.source_event_id.unwrap())
        );
    }
}

#[test]
fn monthly_result_keeps_branch_identity_and_rejects_mixed_periods() {
    let period = input().period;
    let sample = ActivitySample {
        instance_id: activity(1).id,
        slice_id: activity(1).slice_id,
        version: activity(1).version,
        period,
        value: ActivitySampleValue::Repair(RepairSample::NoRepair),
    };
    let result = MonthlyResult::new(
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        period,
        OpeningState {
            cash: money("0"),
            condition: AssetCondition::Available,
        },
        input(),
        vec![activity(1)],
        vec![sample.clone()],
        OpeningState {
            cash: money("0"),
            condition: AssetCondition::Available,
        },
        IdentifiedProposal::default(),
    )
    .unwrap();
    assert_eq!(result.trial_id(), TrialId::from_bytes([6; 16]));
    assert_eq!(result.branch_id(), BranchId::from_bytes([7; 16]));
    assert_eq!(
        result.samples()[0].value,
        ActivitySampleValue::Repair(RepairSample::NoRepair)
    );
    let foreign_sample = ActivitySample::new(
        &activity(2),
        period,
        ActivitySampleValue::Repair(RepairSample::NoRepair),
    );
    assert!(
        MonthlyResult::new(
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            period,
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available,
            },
            input(),
            vec![activity(1)],
            vec![foreign_sample],
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available,
            },
            IdentifiedProposal::default(),
        )
        .is_err()
    );
    let rental_instance = rental_activity(1);
    let repair_sample_for_rental = ActivitySample::new(
        &rental_instance,
        period,
        ActivitySampleValue::Repair(RepairSample::NoRepair),
    );
    assert!(
        MonthlyResult::new(
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            period,
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available,
            },
            input(),
            vec![rental_instance],
            vec![repair_sample_for_rental],
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available,
            },
            IdentifiedProposal::default(),
        )
        .is_err()
    );
    let wrong_branch_proposal = identify_proposal_for(
        SliceProposal::default(),
        identity_for(6, 9),
        &activity(1),
        period,
    )
    .unwrap();
    assert!(
        MonthlyResult::new(
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            period,
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available
            },
            input(),
            vec![activity(1)],
            Vec::new(),
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available
            },
            wrong_branch_proposal,
        )
        .is_err()
    );

    let mut wrong_period = sample;
    wrong_period.period = period.checked_add_months(1).unwrap();
    assert!(
        MonthlyResult::new(
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            period,
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available
            },
            input(),
            vec![activity(1)],
            vec![wrong_period],
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available
            },
            IdentifiedProposal::default(),
        )
        .is_err()
    );
}

#[test]
fn monthly_result_rejects_duplicate_ids_without_prior_merge() {
    let period = input().period;
    let duplicate_events = SliceProposal {
        events: vec![
            EventProposal {
                period,
                key: "same".into(),
            },
            EventProposal {
                period,
                key: "same".into(),
            },
        ],
        ..Default::default()
    };
    let proposal =
        identify_proposal_for(duplicate_events, identity(), &activity(1), period).unwrap();
    assert_eq!(
        MonthlyResult::new(
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            period,
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available
            },
            input(),
            vec![activity(1)],
            Vec::new(),
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available
            },
            proposal,
        ),
        Err(SimulationError::DuplicateOutputId)
    );
}

#[test]
fn merge_rejects_duplicate_event_and_effect_ids() {
    let event = EventProposal {
        period: input().period,
        key: "same".into(),
    };
    let p = SliceProposal {
        events: vec![event.clone()],
        ..Default::default()
    };
    let q = SliceProposal {
        events: vec![event],
        ..Default::default()
    };
    let p = identify_proposal_for(p, identity(), &activity(1), input().period).unwrap();
    let q = identify_proposal_for(q, identity(), &activity(1), input().period).unwrap();
    assert_eq!(
        merge_proposals([p, q]),
        Err(SimulationError::DuplicateOutputId)
    );

    let effect = EffectProposal {
        period: input().period,
        target: property_simulation_domain::ResourceKey::new("property.condition").unwrap(),
        key: "same".into(),
        source_event_key: None,
        condition: AssetCondition::Available,
    };
    let p = SliceProposal {
        effects: vec![effect.clone()],
        ..Default::default()
    };
    let q = SliceProposal {
        effects: vec![effect],
        ..Default::default()
    };
    let p = identify_proposal_for(p, identity(), &activity(1), input().period).unwrap();
    let q = identify_proposal_for(q, identity(), &activity(1), input().period).unwrap();
    assert_eq!(
        merge_proposals([p, q]),
        Err(SimulationError::DuplicateOutputId)
    );

    let transaction = TransactionProposal {
        key: "same".into(),
        period: input().period,
        entries: Vec::new(),
        source_event_key: None,
    };
    let p = SliceProposal {
        transactions: vec![transaction.clone()],
        ..Default::default()
    };
    let q = SliceProposal {
        transactions: vec![transaction],
        ..Default::default()
    };
    let p = identify_proposal_for(p, identity(), &activity(1), input().period).unwrap();
    let q = identify_proposal_for(q, identity(), &activity(1), input().period).unwrap();
    assert_eq!(
        merge_proposals([p, q]),
        Err(SimulationError::DuplicateOutputId)
    );
}

#[test]
fn merge_rejects_different_trial_or_branch_identities() {
    let proposal_a = identify_proposal_for(
        SliceProposal::default(),
        identity(),
        &activity(1),
        input().period,
    )
    .unwrap();
    let proposal_b = identify_proposal_for(
        SliceProposal::default(),
        identity_for(6, 8),
        &activity(2),
        input().period,
    )
    .unwrap();
    assert_eq!(
        merge_proposals([proposal_a, proposal_b]),
        Err(SimulationError::InvalidProposal(
            "cannot merge outputs from different trials, branches, or periods"
        ))
    );
}

#[test]
fn merge_rejects_different_periods() {
    let first_period = input().period;
    let second_period = first_period.checked_add_months(1).unwrap();
    let first = identify_proposal_for(
        SliceProposal::default(),
        identity(),
        &activity(1),
        first_period,
    )
    .unwrap();
    let second = identify_proposal_for(
        SliceProposal::default(),
        identity(),
        &activity(2),
        second_period,
    )
    .unwrap();
    assert_eq!(
        merge_proposals([first, second]),
        Err(SimulationError::InvalidProposal(
            "cannot merge outputs from different trials, branches, or periods"
        ))
    );
}

#[test]
fn slice_stage_planning_is_stable_and_rejects_invalid_graphs() {
    let resource = |id: u8, slice: u8, dependencies: Vec<u8>| SliceResources {
        instance_id: ActivityInstanceId::from_bytes([id; 16]),
        slice_id: SliceId::from_bytes([slice; 16]),
        version: ModelVersionId::from_bytes([2; 16]),
        reads: Default::default(),
        writes: Default::default(),
        dependencies: dependencies
            .into_iter()
            .map(|d| ActivityInstanceId::from_bytes([d; 16]))
            .collect(),
    };
    let items = vec![
        resource(3, 3, vec![1]),
        resource(2, 1, vec![]),
        resource(1, 2, vec![]),
    ];
    let plan = plan_slice_stages(&items).unwrap();
    assert_eq!(
        plan,
        vec![
            vec![
                ActivityInstanceId::from_bytes([2; 16]),
                ActivityInstanceId::from_bytes([1; 16])
            ],
            vec![ActivityInstanceId::from_bytes([3; 16])]
        ]
    );
    assert!(plan_slice_stages(&[resource(1, 1, vec![9])]).is_err());
    assert!(plan_slice_stages(&[resource(1, 1, vec![2]), resource(2, 2, vec![1])]).is_err());
}

#[test]
fn composition_accepts_shared_writes_but_rejects_undeclared_effect_and_unordered_read_write() {
    let key = property_simulation_domain::ResourceKey::new("property.condition").unwrap();
    let a = SliceResources {
        instance_id: ActivityInstanceId::from_bytes([1; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        version: ModelVersionId::from_bytes([2; 16]),
        reads: Default::default(),
        writes: [key.clone()].into_iter().collect(),
        dependencies: Default::default(),
    };
    let b = SliceResources {
        instance_id: ActivityInstanceId::from_bytes([2; 16]),
        slice_id: SliceId::from_bytes([2; 16]),
        version: ModelVersionId::from_bytes([2; 16]),
        writes: Default::default(),
        reads: [key.clone()].into_iter().collect(),
        dependencies: Default::default(),
    };
    let same_writer = SliceResources {
        instance_id: ActivityInstanceId::from_bytes([3; 16]),
        slice_id: SliceId::from_bytes([3; 16]),
        version: ModelVersionId::from_bytes([2; 16]),
        reads: Default::default(),
        writes: [key.clone()].into_iter().collect(),
        dependencies: Default::default(),
    };
    assert!(validate_composition(&[a.clone(), same_writer], &[]).is_ok());
    assert!(validate_composition(&[a.clone(), b], &[]).is_err());
    // Being placed in different topological stages by an unrelated dependency
    // does not establish an ordering between the conflicting pair.
    let unrelated = SliceResources {
        instance_id: ActivityInstanceId::from_bytes([4; 16]),
        slice_id: SliceId::from_bytes([4; 16]),
        version: ModelVersionId::from_bytes([2; 16]),
        reads: Default::default(),
        writes: Default::default(),
        dependencies: Default::default(),
    };
    let later_reader = SliceResources {
        instance_id: ActivityInstanceId::from_bytes([5; 16]),
        slice_id: SliceId::from_bytes([5; 16]),
        version: ModelVersionId::from_bytes([2; 16]),
        reads: [key.clone()].into_iter().collect(),
        writes: Default::default(),
        dependencies: [unrelated.instance_id].into_iter().collect(),
    };
    assert!(validate_composition(&[a.clone(), unrelated, later_reader], &[]).is_err());
    let proposal = identify_proposal_for(
        SliceProposal {
            effects: vec![EffectProposal {
                period: input().period,
                target: key,
                key: "condition".into(),
                source_event_key: None,
                condition: AssetCondition::Available,
            }],
            ..Default::default()
        },
        identity(),
        &activity(1),
        input().period,
    )
    .unwrap();
    let active = activity(1);
    let undeclared = SliceResources {
        instance_id: active.id,
        slice_id: active.slice_id,
        version: active.version,
        reads: Default::default(),
        writes: Default::default(),
        dependencies: Default::default(),
    };
    assert!(validate_composition(&[undeclared], std::slice::from_ref(&proposal)).is_err());

    let wrong_version = SliceResources {
        instance_id: active.id,
        slice_id: active.slice_id,
        version: ModelVersionId::from_bytes([9; 16]),
        reads: Default::default(),
        writes: [property_simulation_domain::ResourceKey::new("property.condition").unwrap()]
            .into_iter()
            .collect(),
        dependencies: Default::default(),
    };
    assert!(validate_composition(&[wrong_version], &[proposal]).is_err());
}

#[test]
fn slice_resource_contract_is_used_to_validate_actual_account_writes() {
    let slice = RentalSlice {
        instance: rental_activity(8),
    };
    let period_input = input();
    let declaration = slice.resources(&period_input);
    assert!(
        declaration
            .writes
            .contains(&account_resource_key(period_input.cash_account))
    );
    assert!(
        declaration
            .writes
            .contains(&account_resource_key(period_input.rent_account))
    );

    let proposal = identify_proposal_for(
        SliceProposal {
            transactions: vec![TransactionProposal {
                key: "rent".into(),
                period: period_input.period,
                entries: vec![property_simulation_accounting::LedgerEntry::new(
                    period_input.cash_account,
                    property_simulation_accounting::BalanceSide::Debit,
                    money("1"),
                )],
                source_event_key: None,
            }],
            ..Default::default()
        },
        identity(),
        slice.instance(),
        period_input.period,
    )
    .unwrap();
    assert!(
        validate_composition(
            std::slice::from_ref(&declaration),
            std::slice::from_ref(&proposal)
        )
        .is_ok()
    );

    let mut incomplete = declaration;
    incomplete
        .writes
        .remove(&account_resource_key(period_input.cash_account));
    assert!(validate_composition(&[incomplete], &[proposal]).is_err());
}

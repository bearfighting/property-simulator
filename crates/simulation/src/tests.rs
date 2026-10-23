use super::*;
use crate::proposals::identify_proposal_for;
use property_simulation_accounting::{BalanceSide, CashFlowCategory, ChartOfAccounts, LedgerEntry};
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, Currency, EffectId, EventId, LedgerTransactionId,
    Money, SliceId, TrialId, YearMonth,
};
use property_simulation_models::{
    RandomSource, RepairCostDistribution, Sampler, SamplingError, UtilityUsageDistribution,
    UtilityUsageSampler,
};

impl crate::LedgerRead for property_simulation_accounting::Ledger {
    fn currency(&self) -> Currency {
        property_simulation_accounting::Ledger::currency(self)
    }

    fn account(&self, id: AccountId) -> Option<crate::AccountFacts> {
        let account = self.chart().get(id)?;
        Some(crate::AccountFacts {
            category: account.category(),
            cash_position: account.is_cash_position(),
            contra: account.is_contra(),
        })
    }

    fn signed_balance(
        &self,
        id: AccountId,
    ) -> Result<Money, property_simulation_accounting::AccountError> {
        let (side, amount) = self.balance(id)?;
        if side == property_simulation_accounting::BalanceSide::Debit {
            Ok(amount)
        } else {
            amount.checked_neg().map_err(Into::into)
        }
    }
}

pub(crate) struct TestLoanCalculation;

impl crate::LoanCalculation for TestLoanCalculation {
    fn validate(
        &self,
        loan: property_simulation_models::FixedRateLoan,
    ) -> Result<(), property_simulation_models::LoanModelError> {
        loan.validate()
    }

    fn calculate_month(
        &self,
        loan: property_simulation_models::FixedRateLoan,
    ) -> Result<property_simulation_models::LoanMonth, property_simulation_models::LoanModelError>
    {
        loan.calculate_month()
    }

    fn derive_payment(
        &self,
        loan: property_simulation_models::FixedRateLoan,
    ) -> Result<Money, property_simulation_models::LoanModelError> {
        loan.derive_payment()
    }
}
struct TestIds;
fn semantic_random(
    streams: &property_simulation_randomness::RandomStreams,
    trial_id: TrialId,
    instance: &ActivityInstance,
    period: YearMonth,
    category: property_simulation_randomness::RandomCategory,
    purpose: &str,
) -> property_simulation_randomness::RandomStream {
    streams
        .stream(&property_simulation_randomness::SemanticIdentity {
            trial_id,
            category,
            slice_id: instance.slice_id,
            activity_id: instance.id,
            period,
            purpose: purpose.into(),
        })
        .unwrap()
}
impl OutputIdAllocator for TestIds {
    fn activity_instance_id(
        &self,
        period: YearMonth,
        instance: ActivityInstanceId,
        key: &str,
    ) -> ActivityInstanceId {
        ActivityInstanceId::from_bytes(output_bytes(0, period, instance, key))
    }
    fn event_id(&self, period: YearMonth, instance: ActivityInstanceId, key: &str) -> EventId {
        EventId::from_bytes(output_bytes(1, period, instance, key))
    }
    fn effect_id(&self, period: YearMonth, instance: ActivityInstanceId, key: &str) -> EffectId {
        EffectId::from_bytes(output_bytes(2, period, instance, key))
    }
    fn transaction_id(
        &self,
        period: YearMonth,
        instance: ActivityInstanceId,
        key: &str,
    ) -> LedgerTransactionId {
        LedgerTransactionId::from_bytes(output_bytes(3, period, instance, key))
    }
}

struct EmptyRandom;
impl RandomSource for EmptyRandom {
    type Error = ();
    fn next_u64(&mut self) -> Result<u64, Self::Error> {
        Err(())
    }
}

fn output_bytes(kind: u8, period: YearMonth, instance: ActivityInstanceId, key: &str) -> [u8; 16] {
    let mut state = 0xcbf29ce484222325u64;
    for byte in [kind]
        .into_iter()
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
fn identity() -> &'static TestIds {
    static IDS: TestIds = TestIds;
    &IDS
}
fn money(s: &str) -> Money {
    Money::parse(s, Currency::from_code("CAD").unwrap()).unwrap()
}

#[test]
fn production_monthly_result_validates_month_currency_and_activity_identities() {
    let period = input().period;
    let opening = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let result = |period, activities, samples| {
        crate::MonthlyResult::from_settled_parts(
            period,
            opening.clone(),
            frozen,
            activities,
            samples,
            opening.clone(),
            IdentifiedProposal::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    };
    assert!(result(period, Vec::new(), Vec::new()).is_ok());
    assert!(
        crate::MonthlyResult::from_settled_parts(
            period,
            opening.clone(),
            frozen,
            Vec::new(),
            Vec::new(),
            OpeningState {
                cash: money("101"),
                condition: AssetCondition::Available,
            },
            IdentifiedProposal::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .is_err()
    );
    assert!(
        result(
            period.checked_add_months(1).unwrap(),
            Vec::new(),
            Vec::new()
        )
        .is_err()
    );
    let mut wrong_frozen_currency = frozen;
    wrong_frozen_currency.rent = Money::parse("1", Currency::from_code("USD").unwrap()).unwrap();
    assert!(
        crate::MonthlyResult::from_settled_parts(
            period,
            opening.clone(),
            wrong_frozen_currency,
            Vec::new(),
            Vec::new(),
            opening.clone(),
            IdentifiedProposal::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .is_err()
    );

    let rental = rental_activity(51);
    let sample = ActivitySample::new(&rental, period, ActivitySampleValue::Rental);
    assert!(result(period, vec![rental.clone()], vec![sample.clone()]).is_ok());
    assert!(
        result(
            period,
            vec![rental.clone(), rental.clone()],
            vec![sample.clone(), sample.clone()]
        )
        .is_err()
    );
    let mut wrong_source = sample.clone();
    wrong_source.instance_id = ActivityInstanceId::from_bytes([99; 16]);
    assert!(result(period, vec![rental], vec![wrong_source]).is_err());
}

#[test]
fn monthly_result_preserves_typed_instance_lifecycle_changes() {
    use crate::{InstanceLifecycleChange, InstanceLifecycleOperation};

    let period = input().period;
    let active_loan = activity_with_kind(72, TestActivityKind::Loan);
    let future_instance = ActivityInstance {
        id: ActivityInstanceId::from_bytes([73; 16]),
        slice_id: SliceId::from_bytes([74; 16]),
        configuration: ActivityConfiguration::Rental,
    };
    let continuation = LoanContinuation {
        instance_id: active_loan.id,
        slice_id: active_loan.slice_id,
        loan: None,
    };
    let result = crate::MonthlyResult::from_settled_parts_with_lifecycle(
        period,
        OpeningState {
            cash: money("100"),
            condition: AssetCondition::Available,
        },
        input(),
        vec![active_loan.clone()],
        Vec::new(),
        OpeningState {
            cash: money("100"),
            condition: AssetCondition::Available,
        },
        IdentifiedProposal::default(),
        vec![
            InstanceLifecycleChange {
                effective_period: period.checked_add_months(2).unwrap(),
                operation: InstanceLifecycleOperation::Add(future_instance.clone()),
            },
            InstanceLifecycleChange {
                effective_period: period,
                operation: InstanceLifecycleOperation::Complete {
                    instance_id: active_loan.id,
                },
            },
        ],
        Vec::new(),
        Vec::new(),
        vec![continuation.clone()],
        vec![continuation],
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(result.lifecycle_changes().len(), 2);
    assert_eq!(result.lifecycle_changes()[0].effective_period, period);
    assert_eq!(
        result.lifecycle_changes()[1].effective_period,
        period.checked_add_months(2).unwrap()
    );
    assert!(matches!(
        &result.lifecycle_changes()[1].operation,
        InstanceLifecycleOperation::Add(instance) if instance == &future_instance
    ));
}

#[test]
fn calculate_monthly_result_accepts_an_empty_month_and_preserves_opening_ledger() {
    let frozen = input();
    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(state.cash);
    let before = ledger.clone();
    let monthly = MonthlyInput {
        opening_state: &state,
        frozen_input: &frozen,
        activities: &[],
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut random = EmptyRandom;
    let mut occurrence_random = std::collections::BTreeMap::new();
    let mut cost_random = std::collections::BTreeMap::new();
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut occurrence_random,
        repair_cost_random: &mut cost_random,
    };
    let result = calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap();
    assert_eq!(result.period(), frozen.period);
    assert_eq!(result.closing_state(), &state);
    assert!(result.ledger_transactions().is_empty());
    assert!(result.cash_flow_contributions().is_empty());
    assert_eq!(ledger, before);
}

#[test]
fn production_monthly_entry_combines_rent_and_repair_with_ledger_provenance() {
    let frozen = input();
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::NeedsRepair,
    };
    let opening_ledger = ledger_for_cash(opening_state.cash);
    let opening_copy = opening_ledger.clone();
    let rental = rental_activity(71);
    let repair = ActivityInstance {
        id: ActivityInstanceId::from_bytes([72; 16]),
        slice_id: SliceId::from_bytes([82; 16]),
        configuration: ActivityConfiguration::RepairCost(RepairCostConfig {
            source_event_id: EventId::from_bytes([73; 16]),
            source_instance_id: ActivityInstanceId::from_bytes([74; 16]),
            source_slice_id: SliceId::from_bytes([84; 16]),
            occurrence_period: frozen.period.checked_add_months(-1).unwrap(),
            delay_months: 1,
            effective_period: frozen.period,
            maximum: money("10"),
            expense_account: repair_expense_account(),
        }),
    };
    let monthly = MonthlyInput {
        opening_state: &opening_state,
        frozen_input: &frozen,
        activities: &[rental.clone(), repair.clone()],
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &opening_ledger,
    };

    let mut utility_sampler = UtilityUsageSampler;
    let mut random = FixedWords(vec![]);
    let repair_cost_random = &mut std::collections::BTreeMap::from([(
        SliceRandomInputKey {
            activity_id: repair.id,
            slice_id: repair.slice_id,
            period: frozen.period,
            purpose: "repair.cost",
        },
        FixedWords(vec![500]),
    )]);
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random,
    };
    let result = calculate_monthly_slice_result(&monthly, &opening_ledger, &mut runtime).unwrap();

    assert_eq!(result.opening_state(), &opening_state);
    assert_eq!(result.closing_state().cash, money("175"));
    assert_eq!(result.closing_state().condition, AssetCondition::Available);
    assert_eq!(result.ledger_transactions().len(), 2);
    assert_eq!(opening_ledger, opening_copy);

    let mut settled = opening_ledger.clone();
    settled
        .post_batch(result.ledger_transactions().to_vec())
        .unwrap();
    assert_eq!(
        settled.balance(frozen.cash_account).unwrap().1,
        money("175")
    );
    assert_eq!(result.cash_flow_contributions().len(), 2);
    let rent_flow = result
        .cash_flow_contributions()
        .iter()
        .find(|flow| flow.instance_id == rental.id)
        .unwrap();
    let repair_flow = result
        .cash_flow_contributions()
        .iter()
        .find(|flow| flow.instance_id == repair.id)
        .unwrap();
    assert_eq!(rent_flow.category, CashFlowCategory::Operating);
    assert_eq!(rent_flow.amount, money("80"));
    assert_eq!(repair_flow.category, CashFlowCategory::Operating);
    assert_eq!(repair_flow.amount, money("-5"));
    let operating_total = result
        .cash_flow_contributions()
        .iter()
        .map(|flow| flow.amount)
        .try_fold(money("0"), |sum, amount| sum.checked_add(amount))
        .unwrap();
    assert_eq!(operating_total, money("75"));

    let projected_cash_flows = settled
        .cash_flow_contributions(frozen.period, frozen.period)
        .unwrap()
        .into_iter()
        .filter(|flow| {
            result
                .ledger_transactions()
                .iter()
                .any(|transaction| transaction.id() == flow.transaction_id)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        projected_cash_flows.len(),
        result.cash_flow_contributions().len()
    );

    for transaction in result.ledger_transactions() {
        let source = transaction.source().unwrap();
        let debits = transaction
            .entries()
            .iter()
            .filter(|entry| entry.side() == BalanceSide::Debit)
            .map(|entry| entry.amount())
            .try_fold(money("0"), |sum, amount| sum.checked_add(amount))
            .unwrap();
        let credits = transaction
            .entries()
            .iter()
            .filter(|entry| entry.side() == BalanceSide::Credit)
            .map(|entry| entry.amount())
            .try_fold(money("0"), |sum, amount| sum.checked_add(amount))
            .unwrap();
        assert_eq!(debits, credits);
        let contribution = result
            .cash_flow_contributions()
            .iter()
            .find(|flow| flow.transaction_id == transaction.id())
            .unwrap();
        assert_eq!(contribution.instance_id, source.instance_id);
        assert_eq!(contribution.slice_id, source.slice_id);
        assert_eq!(contribution.output_key, source.output_key);
        assert_eq!(contribution.period, transaction.period());
        let ledger_flow = projected_cash_flows
            .iter()
            .find(|flow| flow.transaction_id == transaction.id())
            .unwrap();
        assert_eq!(contribution.category, ledger_flow.category);
        assert_eq!(contribution.amount, ledger_flow.amount);
        assert_eq!(contribution.category, CashFlowCategory::Operating);
        let ledger_source = ledger_flow.source.as_ref().unwrap();
        assert_eq!(contribution.instance_id, ledger_source.instance_id);
        assert_eq!(contribution.slice_id, ledger_source.slice_id);
        assert_eq!(contribution.output_key, ledger_source.output_key);
        assert_eq!(ledger_source.source_event_id, source.source_event_id);
        assert!(result.activities().iter().any(|activity| {
            activity.id == source.instance_id && activity.slice_id == source.slice_id
        }));
    }
    assert!(
        result
            .proposal()
            .events()
            .iter()
            .any(|event| { event.key == "rent_collected" && event.instance_id == rental.id })
    );
    assert!(
        result
            .proposal()
            .events()
            .iter()
            .any(|event| { event.key == "repair_cost_settled" && event.instance_id == repair.id })
    );
    let rent_tx = result
        .ledger_transactions()
        .iter()
        .find(|tx| tx.source().unwrap().instance_id == rental.id)
        .unwrap();
    let repair_tx = result
        .ledger_transactions()
        .iter()
        .find(|tx| tx.source().unwrap().instance_id == repair.id)
        .unwrap();
    assert_eq!(rent_tx.source().unwrap().output_key, "rent");
    assert_eq!(repair_tx.source().unwrap().output_key, "repair_cost");
    assert_eq!(
        rent_tx.source().unwrap().source_event_id,
        result
            .proposal()
            .events()
            .iter()
            .find(|event| event.key == "rent_collected")
            .map(|event| event.id)
    );
    let repair_event_id = match &repair.configuration {
        ActivityConfiguration::RepairCost(config) => config.source_event_id,
        _ => unreachable!(),
    };
    assert_eq!(
        repair_tx.source().unwrap().source_event_id,
        Some(repair_event_id)
    );
    assert!(result.proposal().effects().iter().any(|effect| {
        effect.instance_id == repair.id
            && effect.slice_id == repair.slice_id
            && effect.target.as_str() == "property.condition"
            && effect.condition == AssetCondition::Available
            && effect.source_event_id == Some(repair_event_id)
    }));
}

#[test]
fn repair_occurrence_creates_a_delayed_cost_instance_and_settles_against_source_event() {
    repair_occurrence_settles_after_delay(1, 0);
    repair_occurrence_settles_after_delay(3, 5_000);
}

#[test]
fn repair_occurrence_monthly_result_is_independent_of_activity_order() {
    let frozen = input();
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(opening_state.cash);
    let template = RepairCostTemplate {
        maximum: money("50"),
        expense_account: repair_expense_account(),
        delay_months: 1,
    };
    let first = ActivityInstance {
        id: ActivityInstanceId::from_bytes([100; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairOccurrence(RepairOccurrenceConfig {
            cost_template: template,
        }),
    };
    let second = ActivityInstance {
        id: ActivityInstanceId::from_bytes([101; 16]),
        slice_id: first.slice_id,
        configuration: first.configuration.clone(),
    };
    let run = |activities: &[ActivityInstance]| {
        let mut utility_sampler = UtilityUsageSampler;
        let mut unused_random = FixedWords(vec![]);
        let mut occurrence_streams = activities
            .iter()
            .map(|activity| {
                (
                    SliceRandomInputKey {
                        activity_id: activity.id,
                        slice_id: activity.slice_id,
                        period: frozen.period,
                        purpose: "repair.occurrence",
                    },
                    FixedWords(vec![0]),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut cost_streams = std::collections::BTreeMap::new();
        let monthly = MonthlyInput {
            opening_state: &opening_state,
            frozen_input: &frozen,
            activities,
            samples: &[],
            loan_continuations: &[],
            opening_ledger: &ledger,
        };
        let mut runtime = SliceMonthlyRuntime {
            utility_sampler: &mut utility_sampler,
            random: &mut unused_random,
            loan_calculation: &TestLoanCalculation,
            ids: identity(),
            repair_history: Some(RepairHistory {
                simulation_start: frozen.period,
                latest_settled_cost: None,
            }),
            repair_occurrence_random: &mut occurrence_streams,
            repair_cost_random: &mut cost_streams,
        };
        crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap()
    };

    let forward = run(&[first.clone(), second.clone()]);
    let reverse = run(&[second.clone(), first.clone()]);
    assert_eq!(forward, reverse);

    let unrelated = rental_activity(102);
    let with_unrelated = run(&[first.clone(), second, unrelated]);
    let occurrence_ids = [first.id, ActivityInstanceId::from_bytes([101; 16])];
    let occurrence_samples = |result: &MonthlyResult| {
        result
            .samples()
            .iter()
            .filter(|sample| occurrence_ids.contains(&sample.instance_id))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        occurrence_samples(&forward),
        occurrence_samples(&with_unrelated)
    );
    let occurrence_events = |result: &MonthlyResult| {
        result
            .proposal()
            .events()
            .iter()
            .filter(|event| occurrence_ids.contains(&event.instance_id))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        occurrence_events(&forward),
        occurrence_events(&with_unrelated)
    );
    let occurrence_lifecycle = |result: &MonthlyResult| {
        result
            .lifecycle_changes()
            .iter()
            .filter(|change| match &change.operation {
                InstanceLifecycleOperation::Add(instance) => {
                    matches!(
                        &instance.configuration,
                        ActivityConfiguration::RepairCost(config)
                            if occurrence_ids.contains(&config.source_instance_id)
                    )
                }
                InstanceLifecycleOperation::Complete { instance_id } => {
                    occurrence_ids.contains(instance_id)
                }
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        occurrence_lifecycle(&forward),
        occurrence_lifecycle(&with_unrelated)
    );
}

fn repair_occurrence_settles_after_delay(delay_months: u32, cost_random_word: u64) {
    let january = YearMonth::new(2025, 1).unwrap();
    let mut frozen = input();
    frozen.period = january;
    let template_expense_account = repair_expense_account();
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::NeedsRepair,
    };
    let ledger = ledger_for_cash(opening_state.cash);
    let occurrence = ActivityInstance {
        id: ActivityInstanceId::from_bytes([81; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairOccurrence(RepairOccurrenceConfig {
            cost_template: RepairCostTemplate {
                maximum: money("50"),
                expense_account: template_expense_account,
                delay_months,
            },
        }),
    };
    let mut second_occurrence = ActivityInstance {
        id: ActivityInstanceId::from_bytes([84; 16]),
        slice_id: occurrence.slice_id,
        configuration: occurrence.configuration.clone(),
    };
    if let ActivityConfiguration::RepairOccurrence(config) = &mut second_occurrence.configuration {
        config.cost_template.delay_months = delay_months + 1;
    }
    let monthly = MonthlyInput {
        opening_state: &opening_state,
        frozen_input: &frozen,
        activities: &[occurrence.clone(), second_occurrence.clone()],
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut unused_random = FixedWords(vec![]);
    let occurrence_random = FixedWords(vec![0]);
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut unused_random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: Some(RepairHistory {
            simulation_start: january,
            latest_settled_cost: None,
        }),
        repair_occurrence_random: &mut std::collections::BTreeMap::from([
            (
                SliceRandomInputKey {
                    activity_id: occurrence.id,
                    slice_id: occurrence.slice_id,
                    period: january,
                    purpose: "repair.occurrence",
                },
                occurrence_random,
            ),
            (
                SliceRandomInputKey {
                    activity_id: second_occurrence.id,
                    slice_id: second_occurrence.slice_id,
                    period: january,
                    purpose: "repair.occurrence",
                },
                FixedWords(vec![0]),
            ),
        ]),
        repair_cost_random: &mut std::collections::BTreeMap::new(),
    };
    let january_result =
        crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap();
    assert!(january_result.ledger_transactions().is_empty());
    assert_eq!(january_result.lifecycle_changes().len(), 2);
    assert!(
        january_result
            .lifecycle_changes()
            .iter()
            .all(|change| matches!(change.operation, InstanceLifecycleOperation::Add(_)))
    );
    let (cost_instance, event_id) = match &january_result.lifecycle_changes()[0].operation {
        InstanceLifecycleOperation::Add(instance) => {
            let ActivityConfiguration::RepairCost(config) = &instance.configuration else {
                panic!("generated instance must settle repair cost")
            };
            assert_eq!(
                config.effective_period,
                january.checked_add_months(delay_months as i32).unwrap()
            );
            assert_eq!(config.delay_months, delay_months);
            assert_eq!(config.maximum, money("50"));
            assert_eq!(config.expense_account, template_expense_account);
            let event = january_result
                .proposal()
                .events()
                .iter()
                .find(|event| {
                    event.instance_id == occurrence.id && event.key == "repair_occurrence"
                })
                .unwrap();
            assert_eq!(config.source_event_id, event.id);
            (instance.clone(), event.id)
        }
        _ => panic!("occurrence should add a pending cost instance"),
    };
    let (second_cost_instance, second_event_id) = match &january_result.lifecycle_changes()[1]
        .operation
    {
        InstanceLifecycleOperation::Add(instance) => {
            let ActivityConfiguration::RepairCost(config) = &instance.configuration else {
                panic!("second occurrence should add a pending cost instance")
            };
            assert_eq!(config.delay_months, delay_months + 1);
            let event = january_result
                .proposal()
                .events()
                .iter()
                .find(|event| {
                    event.instance_id == second_occurrence.id && event.key == "repair_occurrence"
                })
                .unwrap();
            assert_eq!(config.source_event_id, event.id);
            (instance.clone(), event.id)
        }
        _ => panic!("second occurrence should add a pending cost instance"),
    };

    let mut changed_template = occurrence.clone();
    if let ActivityConfiguration::RepairOccurrence(config) = &mut changed_template.configuration {
        config.cost_template.maximum = money("1");
        config.cost_template.expense_account = AccountId::from_bytes([99; 16]);
        config.cost_template.delay_months = 9;
    }
    let ActivityConfiguration::RepairOccurrence(changed_config) = &changed_template.configuration
    else {
        panic!("source template should remain an occurrence configuration")
    };
    assert_eq!(changed_config.cost_template.maximum, money("1"));
    assert_eq!(changed_config.cost_template.delay_months, 9);
    let ActivityConfiguration::RepairCost(snapshot) = &cost_instance.configuration else {
        panic!("generated instance must retain its cost configuration snapshot")
    };
    assert_eq!(snapshot.maximum, money("50"));
    assert_eq!(snapshot.expense_account, template_expense_account);
    assert_eq!(snapshot.delay_months, delay_months);

    let settlement_period = january.checked_add_months(delay_months as i32).unwrap();
    frozen.period = settlement_period;
    let february_input = MonthlyInput {
        opening_state: &opening_state,
        frozen_input: &frozen,
        activities: &[occurrence.clone(), cost_instance.clone()],
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut unused_random = FixedWords(vec![]);
    let cost_random = FixedWords(vec![cost_random_word]);
    let occurrence_random = FixedWords(vec![0]);
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut unused_random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: Some(RepairHistory {
            simulation_start: january,
            latest_settled_cost: None,
        }),
        repair_occurrence_random: &mut std::collections::BTreeMap::from([(
            SliceRandomInputKey {
                activity_id: occurrence.id,
                slice_id: occurrence.slice_id,
                period: settlement_period,
                purpose: "repair.occurrence",
            },
            occurrence_random,
        )]),
        repair_cost_random: &mut std::collections::BTreeMap::from([(
            SliceRandomInputKey {
                activity_id: cost_instance.id,
                slice_id: cost_instance.slice_id,
                period: settlement_period,
                purpose: "repair.cost",
            },
            cost_random,
        )]),
    };
    let february_result =
        crate::calculate_monthly_slice_result(&february_input, &ledger, &mut runtime).unwrap();
    let transaction = &february_result.ledger_transactions()[0];
    assert_eq!(transaction.period(), settlement_period);
    assert_eq!(
        transaction.source().unwrap().source_event_id,
        Some(event_id)
    );
    let expected_cost = if cost_random_word == 0 {
        money("0")
    } else {
        money("50")
    };
    assert_eq!(transaction.entries()[0].amount(), expected_cost);
    assert_eq!(
        transaction.entries()[0].account_id(),
        template_expense_account
    );
    assert_eq!(transaction.entries()[1].amount(), expected_cost);
    assert!(
        february_result
            .proposal()
            .events()
            .iter()
            .any(|event| event.key == "repair_cost_settled")
    );
    assert_eq!(
        february_result
            .lifecycle_changes()
            .iter()
            .filter(|change| matches!(
                change.operation,
                InstanceLifecycleOperation::Complete { instance_id }
                    if instance_id == cost_instance.id
            ))
            .count(),
        1
    );
    assert!(february_result.samples().iter().any(|sample| {
        sample.instance_id == occurrence.id
            && sample.value
                == ActivitySampleValue::RepairOccurrence(
                    property_simulation_models::RepairOccurrenceSample(true),
                )
    }));
    assert!(february_result.lifecycle_changes().iter().any(|change| matches!(
        change.operation,
        InstanceLifecycleOperation::Complete { instance_id } if instance_id == cost_instance.id
    )));
    assert!(
        february_result
            .lifecycle_changes()
            .iter()
            .any(|change| matches!(
                &change.operation,
                InstanceLifecycleOperation::Add(instance)
                    if matches!(instance.configuration, ActivityConfiguration::RepairCost(_))
            ))
    );

    let mut later_ledger = ledger.clone();
    later_ledger
        .post_batch(february_result.ledger_transactions().to_vec())
        .unwrap();
    let later_state = february_result.closing_state().clone();
    let later_period = january
        .checked_add_months((delay_months + 1) as i32)
        .unwrap();
    frozen.period = later_period;
    let later_input = MonthlyInput {
        opening_state: &later_state,
        frozen_input: &frozen,
        activities: std::slice::from_ref(&second_cost_instance),
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &later_ledger,
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut unused_random = FixedWords(vec![]);
    let mut later_cost_random = std::collections::BTreeMap::from([(
        SliceRandomInputKey {
            activity_id: second_cost_instance.id,
            slice_id: second_cost_instance.slice_id,
            period: later_period,
            purpose: "repair.cost",
        },
        FixedWords(vec![0]),
    )]);
    let mut later_runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut unused_random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut later_cost_random,
    };
    let later_result =
        crate::calculate_monthly_slice_result(&later_input, &later_ledger, &mut later_runtime)
            .unwrap();
    assert_eq!(later_result.ledger_transactions().len(), 1);
    assert_eq!(later_result.ledger_transactions()[0].period(), later_period);
    assert_eq!(
        later_result.ledger_transactions()[0]
            .source()
            .unwrap()
            .source_event_id,
        Some(second_event_id)
    );
    assert!(
        later_result
            .samples()
            .iter()
            .all(|sample| !matches!(sample.value, ActivitySampleValue::RepairOccurrence(_)))
    );
    assert!(
        later_result
            .lifecycle_changes()
            .iter()
            .any(|change| matches!(
                change.operation,
                InstanceLifecycleOperation::Complete { instance_id }
                    if instance_id == second_cost_instance.id
            ))
    );
}

#[test]
fn modern_monthly_entry_runs_without_repair_specific_runtime_inputs() {
    let frozen = input();
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(opening_state.cash);
    let utility = utility_activity(80, 3, "2");
    let monthly = MonthlyInput {
        opening_state: &opening_state,
        frozen_input: &frozen,
        activities: std::slice::from_ref(&utility),
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut random = FixedWords(vec![2]);
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut std::collections::BTreeMap::new(),
    };

    let result = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap();
    assert_eq!(result.samples().len(), 1);
    assert_eq!(result.samples()[0].value, ActivitySampleValue::Utility(2));
    assert_eq!(result.ledger_transactions().len(), 1);
    assert_eq!(
        result.ledger_transactions()[0].entries()[0].amount(),
        money("4")
    );
}

#[test]
fn no_repair_occurrence_does_not_require_or_create_a_cost_sample() {
    let frozen = input();
    let template_expense_account = repair_expense_account();
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(opening_state.cash);
    let occurrence = ActivityInstance {
        id: ActivityInstanceId::from_bytes([82; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairOccurrence(RepairOccurrenceConfig {
            cost_template: RepairCostTemplate {
                maximum: money("50"),
                expense_account: template_expense_account,
                delay_months: 1,
            },
        }),
    };
    let monthly = MonthlyInput {
        opening_state: &opening_state,
        frozen_input: &frozen,
        activities: std::slice::from_ref(&occurrence),
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut unused_random = FixedWords(vec![]);
    let mut occurrence_streams = std::collections::BTreeMap::from([(
        SliceRandomInputKey {
            activity_id: occurrence.id,
            slice_id: occurrence.slice_id,
            period: frozen.period,
            purpose: "repair.occurrence",
        },
        FixedWords(vec![25]),
    )]);
    let mut cost_streams = std::collections::BTreeMap::new();
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut unused_random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: Some(RepairHistory {
            simulation_start: frozen.period,
            latest_settled_cost: None,
        }),
        repair_occurrence_random: &mut occurrence_streams,
        repair_cost_random: &mut cost_streams,
    };
    let result = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap();
    assert!(result.proposal().events().is_empty());
    assert!(result.lifecycle_changes().is_empty());
    assert!(result.samples().iter().any(|sample| sample.value
        == ActivitySampleValue::RepairOccurrence(
            property_simulation_models::RepairOccurrenceSample(false)
        )));
    assert!(result.ledger_transactions().is_empty());
}

#[test]
fn repair_occurrence_and_cost_purposes_have_independent_random_streams() {
    let frozen = input();
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(opening_state.cash);
    let occurrence = ActivityInstance {
        id: ActivityInstanceId::from_bytes([93; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairOccurrence(RepairOccurrenceConfig {
            cost_template: RepairCostTemplate {
                maximum: money("50"),
                expense_account: repair_expense_account(),
                delay_months: 1,
            },
        }),
    };
    let cost = ActivityInstance {
        id: ActivityInstanceId::from_bytes([94; 16]),
        slice_id: occurrence.slice_id,
        configuration: ActivityConfiguration::RepairCost(RepairCostConfig {
            source_event_id: EventId::from_bytes([95; 16]),
            source_instance_id: occurrence.id,
            source_slice_id: occurrence.slice_id,
            occurrence_period: frozen.period.checked_add_months(-1).unwrap(),
            delay_months: 1,
            effective_period: frozen.period,
            maximum: money("50"),
            expense_account: repair_expense_account(),
        }),
    };
    let run = |consume_occurrence_first: bool, consume_cost_first: bool| {
        let mut occurrence_stream = FixedWords(vec![0, 0]);
        let mut cost_stream = FixedWords(vec![0, 10]);
        if consume_occurrence_first {
            occurrence_stream.next_u64().unwrap();
        }
        if consume_cost_first {
            cost_stream.next_u64().unwrap();
        }
        let mut occurrence_streams = std::collections::BTreeMap::from([(
            SliceRandomInputKey {
                activity_id: occurrence.id,
                slice_id: occurrence.slice_id,
                period: frozen.period,
                purpose: "repair.occurrence",
            },
            occurrence_stream,
        )]);
        let mut cost_streams = std::collections::BTreeMap::from([(
            SliceRandomInputKey {
                activity_id: cost.id,
                slice_id: cost.slice_id,
                period: frozen.period,
                purpose: "repair.cost",
            },
            cost_stream,
        )]);
        let mut utility_sampler = UtilityUsageSampler;
        let mut unused_random = FixedWords(vec![]);
        let monthly = MonthlyInput {
            opening_state: &opening_state,
            frozen_input: &frozen,
            activities: &[occurrence.clone(), cost.clone()],
            samples: &[],
            loan_continuations: &[],
            opening_ledger: &ledger,
        };
        let mut runtime = SliceMonthlyRuntime {
            utility_sampler: &mut utility_sampler,
            random: &mut unused_random,
            loan_calculation: &TestLoanCalculation,
            ids: identity(),
            repair_history: Some(RepairHistory {
                simulation_start: frozen.period.checked_add_months(-1).unwrap(),
                latest_settled_cost: None,
            }),
            repair_occurrence_random: &mut occurrence_streams,
            repair_cost_random: &mut cost_streams,
        };
        crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap()
    };

    let baseline = run(false, false);
    let occurrence_advanced = run(true, false);
    let cost_advanced = run(false, true);
    let occurrence_sample = |result: &MonthlyResult| {
        result
            .samples()
            .iter()
            .find(|sample| sample.instance_id == occurrence.id)
            .unwrap()
            .value
            .clone()
    };
    let cost_sample = |result: &MonthlyResult| {
        result
            .samples()
            .iter()
            .find(|sample| sample.instance_id == cost.id)
            .unwrap()
            .value
            .clone()
    };
    assert_eq!(cost_sample(&baseline), cost_sample(&occurrence_advanced));
    assert_eq!(
        occurrence_sample(&baseline),
        occurrence_sample(&cost_advanced)
    );
}

#[test]
fn repair_occurrence_uses_month_start_history_when_cost_settles_in_same_month() {
    let mut frozen = input();
    frozen.period = YearMonth::new(2025, 2).unwrap();
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::NeedsRepair,
    };
    let ledger = ledger_for_cash(opening_state.cash);
    let occurrence = ActivityInstance {
        id: ActivityInstanceId::from_bytes([86; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairOccurrence(RepairOccurrenceConfig {
            cost_template: RepairCostTemplate {
                maximum: money("50"),
                expense_account: repair_expense_account(),
                delay_months: 1,
            },
        }),
    };
    let cost = ActivityInstance {
        id: ActivityInstanceId::from_bytes([87; 16]),
        slice_id: occurrence.slice_id,
        configuration: ActivityConfiguration::RepairCost(RepairCostConfig {
            source_event_id: EventId::from_bytes([88; 16]),
            source_instance_id: occurrence.id,
            source_slice_id: occurrence.slice_id,
            occurrence_period: YearMonth::new(2025, 1).unwrap(),
            delay_months: 1,
            effective_period: frozen.period,
            maximum: money("50"),
            expense_account: repair_expense_account(),
        }),
    };
    let monthly = MonthlyInput {
        opening_state: &opening_state,
        frozen_input: &frozen,
        activities: &[occurrence.clone(), cost.clone()],
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut unused_random = FixedWords(vec![]);
    let mut occurrence_random = std::collections::BTreeMap::from([(
        SliceRandomInputKey {
            activity_id: occurrence.id,
            slice_id: occurrence.slice_id,
            period: frozen.period,
            purpose: "repair.occurrence",
        },
        // Age 1 has a 27/10,000 threshold; age 0 has 25/10,000.
        FixedWords(vec![26]),
    )]);
    let mut cost_random = std::collections::BTreeMap::from([(
        SliceRandomInputKey {
            activity_id: cost.id,
            slice_id: cost.slice_id,
            period: frozen.period,
            purpose: "repair.cost",
        },
        FixedWords(vec![0]),
    )]);
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut unused_random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: Some(RepairHistory {
            simulation_start: YearMonth::new(2025, 1).unwrap(),
            latest_settled_cost: None,
        }),
        repair_occurrence_random: &mut occurrence_random,
        repair_cost_random: &mut cost_random,
    };

    let result = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap();
    assert!(result.samples().iter().any(|sample| {
        sample.instance_id == occurrence.id
            && sample.value
                == ActivitySampleValue::RepairOccurrence(
                    property_simulation_models::RepairOccurrenceSample(true),
                )
    }));
    assert!(result
        .lifecycle_changes()
        .iter()
        .any(|change| matches!(change.operation, InstanceLifecycleOperation::Complete { instance_id } if instance_id == cost.id)));
}

#[test]
fn repair_occurrence_preflight_rejects_invalid_or_overflowing_delay_before_sampling() {
    let frozen = input();
    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(state.cash);
    let distribution = RepairCostDistribution {
        maximum: money("50"),
    };
    let occurrence = ActivityInstance {
        id: ActivityInstanceId::from_bytes([83; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairOccurrence(RepairOccurrenceConfig {
            cost_template: RepairCostTemplate {
                maximum: distribution.maximum,
                expense_account: repair_expense_account(),
                delay_months: 0,
            },
        }),
    };
    let mut delayed_occurrence = occurrence.clone();
    let mut monthly = MonthlyInput {
        opening_state: &state,
        frozen_input: &frozen,
        activities: std::slice::from_ref(&occurrence),
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let history = Some(RepairHistory {
        simulation_start: frozen.period,
        latest_settled_cost: None,
    });
    assert!(
        crate::monthly::prepare_monthly_slices(&monthly, history, &ledger, &TestLoanCalculation,)
            .is_err()
    );

    monthly.activities = &[];
    if let ActivityConfiguration::RepairOccurrence(config) = &mut delayed_occurrence.configuration {
        config.cost_template.delay_months = 1;
    }
    monthly.activities = std::slice::from_ref(&delayed_occurrence);
    let mut latest_month = frozen;
    latest_month.period = YearMonth::new(u32::MAX, 12).unwrap();
    monthly.frozen_input = &latest_month;
    let latest_history = Some(RepairHistory {
        simulation_start: latest_month.period,
        latest_settled_cost: None,
    });
    assert!(
        crate::monthly::prepare_monthly_slices(
            &monthly,
            latest_history,
            &ledger,
            &TestLoanCalculation,
        )
        .is_err()
    );
}

#[test]
fn repair_occurrence_template_preflight_rejects_invalid_amount_currency_and_accounts() {
    let frozen = input();
    let ledger = ledger_for_cash(money("100"));
    let occurrence = ActivityInstance {
        id: ActivityInstanceId::from_bytes([89; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairOccurrence(RepairOccurrenceConfig {
            cost_template: RepairCostTemplate {
                maximum: money("50"),
                expense_account: repair_expense_account(),
                delay_months: 1,
            },
        }),
    };
    let history = Some(RepairHistory {
        simulation_start: frozen.period,
        latest_settled_cost: None,
    });

    let mut invalid = occurrence.clone();
    if let ActivityConfiguration::RepairOccurrence(config) = &mut invalid.configuration {
        config.cost_template.maximum = money("-1");
    }
    assert_repair_preflight_rejects(&frozen, &invalid, history, &ledger);

    let mut invalid = occurrence.clone();
    if let ActivityConfiguration::RepairOccurrence(config) = &mut invalid.configuration {
        config.cost_template.maximum = Money::parse(
            "50",
            Currency::from_code("USD").expect("USD is a supported test currency"),
        )
        .unwrap();
    }
    assert_repair_preflight_rejects(&frozen, &invalid, history, &ledger);

    let mut invalid = occurrence.clone();
    if let ActivityConfiguration::RepairOccurrence(config) = &mut invalid.configuration {
        config.cost_template.expense_account = AccountId::from_bytes([99; 16]);
    }
    assert_repair_preflight_rejects(&frozen, &invalid, history, &ledger);

    let mut invalid = occurrence.clone();
    if let ActivityConfiguration::RepairOccurrence(config) = &mut invalid.configuration {
        config.cost_template.expense_account = frozen.cash_account;
    }
    assert_repair_preflight_rejects(&frozen, &invalid, history, &ledger);

    let mut invalid_cash = frozen;
    invalid_cash.cash_account = repair_expense_account();
    assert_repair_preflight_rejects(&invalid_cash, &occurrence, history, &ledger);
}

#[test]
fn repair_cost_preflight_rejects_invalid_amount_currency_accounts_and_period() {
    let frozen = input();
    let ledger = ledger_for_cash(money("100"));
    let effective_period = frozen.period;
    let cost = ActivityInstance {
        id: ActivityInstanceId::from_bytes([90; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairCost(RepairCostConfig {
            source_event_id: EventId::from_bytes([91; 16]),
            source_instance_id: ActivityInstanceId::from_bytes([92; 16]),
            source_slice_id: SliceId::from_bytes([1; 16]),
            occurrence_period: effective_period.checked_add_months(-1).unwrap(),
            delay_months: 1,
            effective_period,
            maximum: money("50"),
            expense_account: repair_expense_account(),
        }),
    };

    let mut invalid = cost.clone();
    if let ActivityConfiguration::RepairCost(config) = &mut invalid.configuration {
        config.maximum = money("-1");
    }
    assert_repair_preflight_rejects(&frozen, &invalid, None, &ledger);

    let mut invalid = cost.clone();
    if let ActivityConfiguration::RepairCost(config) = &mut invalid.configuration {
        config.maximum = Money::parse(
            "50",
            Currency::from_code("USD").expect("USD is a supported test currency"),
        )
        .unwrap();
    }
    assert_repair_preflight_rejects(&frozen, &invalid, None, &ledger);

    let mut invalid = cost.clone();
    if let ActivityConfiguration::RepairCost(config) = &mut invalid.configuration {
        config.expense_account = AccountId::from_bytes([99; 16]);
    }
    assert_repair_preflight_rejects(&frozen, &invalid, None, &ledger);

    let mut invalid = cost.clone();
    if let ActivityConfiguration::RepairCost(config) = &mut invalid.configuration {
        config.expense_account = frozen.cash_account;
    }
    assert_repair_preflight_rejects(&frozen, &invalid, None, &ledger);

    let mut invalid_cash = frozen;
    invalid_cash.cash_account = repair_expense_account();
    assert_repair_preflight_rejects(&invalid_cash, &cost, None, &ledger);

    let mut invalid = cost;
    if let ActivityConfiguration::RepairCost(config) = &mut invalid.configuration {
        config.effective_period = config.effective_period.checked_add_months(1).unwrap();
    }
    assert_repair_preflight_rejects(&input(), &invalid, None, &ledger);
}

#[test]
fn repair_cost_preflight_rejects_future_and_expired_instances() {
    let frozen = input();
    let ledger = ledger_for_cash(money("100"));
    let base_occurrence = ActivityInstanceId::from_bytes([96; 16]);
    let base_slice = SliceId::from_bytes([1; 16]);
    let make_cost = |occurrence_period: YearMonth, delay_months: u32| ActivityInstance {
        id: ActivityInstanceId::from_bytes([97; 16]),
        slice_id: base_slice,
        configuration: ActivityConfiguration::RepairCost(RepairCostConfig {
            source_event_id: EventId::from_bytes([98; 16]),
            source_instance_id: base_occurrence,
            source_slice_id: base_slice,
            occurrence_period,
            delay_months,
            effective_period: occurrence_period
                .checked_add_months(delay_months as i32)
                .unwrap(),
            maximum: money("50"),
            expense_account: repair_expense_account(),
        }),
    };

    let future = make_cost(frozen.period, 1);
    assert_repair_preflight_rejects(&frozen, &future, None, &ledger);

    let expired = make_cost(frozen.period.checked_add_months(-2).unwrap(), 1);
    assert_repair_preflight_rejects(&frozen, &expired, None, &ledger);
}

fn assert_repair_preflight_rejects(
    frozen: &FrozenInput,
    activity: &ActivityInstance,
    history: Option<RepairHistory>,
    ledger: &property_simulation_accounting::Ledger,
) {
    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let monthly = MonthlyInput {
        opening_state: &state,
        frozen_input: frozen,
        activities: std::slice::from_ref(activity),
        samples: &[],
        loan_continuations: &[],
        opening_ledger: ledger,
    };
    let before = ledger.clone();
    let result =
        crate::monthly::prepare_monthly_slices(&monthly, history, ledger, &TestLoanCalculation);
    assert!(result.is_err());
    assert_eq!(*ledger, before);
}

#[test]
fn unified_monthly_entry_dispatches_and_samples_instances_without_adapters() {
    let frozen = input();
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::NeedsRepair,
    };
    let ledger = ledger_for_cash(opening_state.cash);
    let rental = rental_activity(75);
    let repair = ActivityInstance {
        id: ActivityInstanceId::from_bytes([76; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::RepairCost(RepairCostConfig {
            source_event_id: EventId::from_bytes([86; 16]),
            source_instance_id: ActivityInstanceId::from_bytes([87; 16]),
            source_slice_id: SliceId::from_bytes([1; 16]),
            occurrence_period: frozen.period.checked_add_months(-1).unwrap(),
            delay_months: 1,
            effective_period: frozen.period,
            maximum: money("50"),
            expense_account: repair_expense_account(),
        }),
    };
    let tax = ActivityInstance {
        id: ActivityInstanceId::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 77]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::PropertyTax(PropertyTaxConfig {
            amount: money("5"),
            cadence: PropertyTaxCadence::Monthly,
            first_due: frozen.period,
            cash_account: frozen.cash_account,
            expense_account: repair_expense_account(),
        }),
    };
    let utility = ActivityInstance {
        id: ActivityInstanceId::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 78]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::Utility(UtilityConfig {
            usage_unit: "kWh".into(),
            maximum_usage: 3,
            rate_per_unit: money("2"),
            cash_account: frozen.cash_account,
            expense_account: AccountId::from_bytes([9; 16]),
        }),
    };
    let activities = [rental.clone(), repair.clone(), tax.clone(), utility.clone()];
    let monthly = MonthlyInput {
        opening_state: &opening_state,
        frozen_input: &frozen,
        activities: &activities,
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut random = FixedWords(vec![2]);
    let mut repair_cost_random = std::collections::BTreeMap::from([(
        SliceRandomInputKey {
            activity_id: repair.id,
            slice_id: repair.slice_id,
            period: frozen.period,
            purpose: "repair.cost",
        },
        FixedWords(vec![1000]),
    )]);
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut repair_cost_random,
    };
    let result = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap();

    assert_eq!(result.samples().len(), 4);
    assert!(result.samples().iter().any(|sample| {
        sample.instance_id == rental.id && sample.value == ActivitySampleValue::Rental
    }));
    assert!(result.samples().iter().any(|sample| {
        sample.instance_id == repair.id
            && sample.value
                == ActivitySampleValue::RepairCost(property_simulation_models::RepairCostSample(
                    money("10"),
                ))
    }));
    assert!(result.samples().iter().any(|sample| {
        sample.instance_id == tax.id && sample.value == ActivitySampleValue::PropertyTax
    }));
    assert!(result.samples().iter().any(|sample| {
        sample.instance_id == utility.id && sample.value == ActivitySampleValue::Utility(2)
    }));
    assert_eq!(result.ledger_transactions().len(), 4);
    assert_eq!(result.closing_state().cash, money("161"));
}

#[test]
fn unified_monthly_preflight_rejects_before_calling_random_samplers() {
    struct CountingUtilitySampler<'a>(&'a std::cell::Cell<usize>);
    impl Sampler<UtilityUsageDistribution> for CountingUtilitySampler<'_> {
        fn sample<R: RandomSource>(
            &mut self,
            distribution: &UtilityUsageDistribution,
            _random: &mut R,
        ) -> Result<u64, SamplingError<R::Error>> {
            self.0.set(self.0.get() + 1);
            Ok(distribution.maximum)
        }
    }

    let mut frozen = input();
    frozen.rent = money("-1");
    let opening_state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(opening_state.cash);
    let utility = ActivityInstance {
        id: ActivityInstanceId::from_bytes([79; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::Utility(UtilityConfig {
            usage_unit: "kWh".into(),
            maximum_usage: 3,
            rate_per_unit: money("2"),
            cash_account: frozen.cash_account,
            expense_account: AccountId::from_bytes([9; 16]),
        }),
    };
    let monthly = MonthlyInput {
        opening_state: &opening_state,
        frozen_input: &frozen,
        activities: std::slice::from_ref(&utility),
        samples: &[],
        loan_continuations: &[],
        opening_ledger: &ledger,
    };
    let calls = std::cell::Cell::new(0);
    let mut utility_sampler = CountingUtilitySampler(&calls);
    let mut random = FixedWords(vec![0]);
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut std::collections::BTreeMap::new(),
    };

    let error = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap_err();
    assert!(matches!(error, SimulationError::InputValidation(_)));
    assert_eq!(calls.get(), 0);
    assert_eq!(ledger, ledger_for_cash(opening_state.cash));

    let valid_frozen = input();
    let valid_monthly = MonthlyInput {
        frozen_input: &valid_frozen,
        ..monthly
    };
    let mut utility_sampler = UtilityUsageSampler;
    let mut random = EmptyRandom;
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &TestLoanCalculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut std::collections::BTreeMap::new(),
    };
    assert!(matches!(
        crate::calculate_monthly_slice_result(&valid_monthly, &ledger, &mut runtime),
        Err(SimulationError::Fatal(FatalSimulationFailure {
            phase: FatalSimulationPhase::SliceTransition,
            activity_instance_id: Some(id),
            source,
            ..
        })) if id == utility.id && *source == SimulationError::SamplingFailure
    ));
}

#[test]
fn unified_monthly_preflight_validates_loan_outputs_and_cashflow_bounds_before_sampling() {
    use property_simulation_accounting::{
        Account, AccountCategory, AccountCode, OpeningBalance, OpeningBalances,
    };

    struct HugeInterestCalculation {
        calls: std::cell::Cell<usize>,
        maximum: Money,
        invalid_result: bool,
        scheduled_payment_override: Option<Money>,
    }
    impl LoanCalculation for HugeInterestCalculation {
        fn validate(
            &self,
            _loan: property_simulation_models::FixedRateLoan,
        ) -> Result<(), property_simulation_models::LoanModelError> {
            Ok(())
        }

        fn calculate_month(
            &self,
            loan: property_simulation_models::FixedRateLoan,
        ) -> Result<property_simulation_models::LoanMonth, property_simulation_models::LoanModelError>
        {
            self.calls.set(self.calls.get() + 1);
            let zero = Money::new(0.into(), self.maximum.currency());
            Ok(property_simulation_models::LoanMonth {
                interest: if self.invalid_result {
                    zero
                } else {
                    self.maximum
                },
                payment: self.maximum,
                principal: zero,
                closing_balance: loan.opening_balance,
                remaining_months: loan.remaining_months - 1,
                scheduled_payment: self.scheduled_payment_override.unwrap_or(self.maximum),
            })
        }

        fn derive_payment(
            &self,
            _loan: property_simulation_models::FixedRateLoan,
        ) -> Result<Money, property_simulation_models::LoanModelError> {
            Ok(self.maximum)
        }
    }

    struct CountingUtilitySampler<'a>(&'a std::cell::Cell<usize>);
    impl Sampler<UtilityUsageDistribution> for CountingUtilitySampler<'_> {
        fn sample<R: RandomSource>(
            &mut self,
            distribution: &UtilityUsageDistribution,
            _random: &mut R,
        ) -> Result<u64, SamplingError<R::Error>> {
            self.0.set(self.0.get() + 1);
            Ok(distribution.maximum)
        }
    }

    let frozen = input();
    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let chart = ChartOfAccounts::new([
        Account::new_cash_position(frozen.cash_account, AccountCode::new(1000).unwrap(), "Cash")
            .unwrap(),
        Account::new(
            frozen.rent_account,
            AccountCode::new(4000).unwrap(),
            AccountCategory::Income,
            false,
            "Rent",
        )
        .unwrap(),
        Account::new(
            repair_expense_account(),
            AccountCode::new(5000).unwrap(),
            AccountCategory::Expense,
            false,
            "Repairs",
        )
        .unwrap(),
        Account::new(
            AccountId::from_bytes([9; 16]),
            AccountCode::new(5100).unwrap(),
            AccountCategory::Expense,
            false,
            "Utilities",
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
        Account::new(
            AccountId::from_bytes([8; 16]),
            AccountCode::new(2000).unwrap(),
            AccountCategory::Liability,
            false,
            "Mortgage liability",
        )
        .unwrap(),
        Account::new(
            AccountId::from_bytes([7; 16]),
            AccountCode::new(2100).unwrap(),
            AccountCategory::Liability,
            false,
            "Second mortgage liability",
        )
        .unwrap(),
    ])
    .unwrap();
    let opening = OpeningBalances::new(
        &chart,
        [
            OpeningBalance::new(frozen.cash_account, BalanceSide::Debit, state.cash).unwrap(),
            OpeningBalance::new(
                AccountId::from_bytes([8; 16]),
                BalanceSide::Credit,
                money("1"),
            )
            .unwrap(),
            OpeningBalance::new(
                AccountId::from_bytes([7; 16]),
                BalanceSide::Credit,
                money("1"),
            )
            .unwrap(),
            OpeningBalance::new(
                AccountId::from_bytes([6; 16]),
                BalanceSide::Credit,
                money("98"),
            )
            .unwrap(),
        ],
        Some(money("98")),
    )
    .unwrap();
    let ledger = property_simulation_accounting::Ledger::new(chart, opening).unwrap();
    let loan = property_simulation_models::FixedRateLoan {
        opening_balance: money("1"),
        remaining_months: 12,
        annual_rate: rust_decimal::Decimal::ZERO,
        scheduled_payment: None,
    };
    let loan_activity = ActivityInstance {
        id: ActivityInstanceId::from_bytes([73; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: ActivityConfiguration::Loan(crate::LoanConfig {
            liability_account: AccountId::from_bytes([8; 16]),
            interest_expense_account: repair_expense_account(),
            cash_account: frozen.cash_account,
        }),
    };
    let second_loan_activity = ActivityInstance {
        id: ActivityInstanceId::from_bytes([75; 16]),
        slice_id: SliceId::from_bytes([2; 16]),
        configuration: ActivityConfiguration::Loan(crate::LoanConfig {
            liability_account: AccountId::from_bytes([7; 16]),
            interest_expense_account: repair_expense_account(),
            cash_account: frozen.cash_account,
        }),
    };
    let utility = utility_activity(74, 1, "1");
    let continuation = LoanContinuation {
        instance_id: loan_activity.id,
        slice_id: loan_activity.slice_id,
        loan: Some(loan),
    };
    let second_continuation = LoanContinuation {
        instance_id: second_loan_activity.id,
        slice_id: second_loan_activity.slice_id,
        loan: Some(loan),
    };
    let monthly = MonthlyInput {
        opening_state: &state,
        frozen_input: &frozen,
        activities: &[
            loan_activity.clone(),
            second_loan_activity.clone(),
            utility.clone(),
        ],
        samples: &[],
        loan_continuations: &[continuation, second_continuation],
        opening_ledger: &ledger,
    };
    let max_interest = Money::from_parts(i128::MAX / 2, 2, state.cash.currency()).unwrap();
    let loan_calculation = HugeInterestCalculation {
        calls: std::cell::Cell::new(0),
        maximum: max_interest,
        invalid_result: false,
        scheduled_payment_override: None,
    };
    let utility_samples = std::cell::Cell::new(0);
    let mut utility_sampler = CountingUtilitySampler(&utility_samples);
    let mut random = EmptyRandom;
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &loan_calculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut std::collections::BTreeMap::new(),
    };

    let mut invalid_utility = utility.clone();
    let ActivityConfiguration::Utility(ref mut config) = invalid_utility.configuration else {
        unreachable!()
    };
    config.expense_account = frozen.cash_account;
    let invalid_monthly = MonthlyInput {
        activities: &[
            loan_activity.clone(),
            second_loan_activity.clone(),
            invalid_utility,
        ],
        ..monthly
    };
    let error =
        crate::calculate_monthly_slice_result(&invalid_monthly, &ledger, &mut runtime).unwrap_err();
    assert!(matches!(error, SimulationError::InvalidInput(_)));
    assert_eq!(loan_calculation.calls.get(), 0);
    assert_eq!(utility_samples.get(), 0);

    let error = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap_err();
    assert!(matches!(error, SimulationError::InvalidInput(_)));
    assert_eq!(loan_calculation.calls.get(), 2);
    assert_eq!(utility_samples.get(), 0);

    let invalid_loan_calculation = HugeInterestCalculation {
        calls: std::cell::Cell::new(0),
        maximum: money("1"),
        invalid_result: true,
        scheduled_payment_override: None,
    };
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &invalid_loan_calculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut std::collections::BTreeMap::new(),
    };
    let error = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap_err();
    assert!(matches!(error, SimulationError::InvalidInput(_)));
    assert_eq!(invalid_loan_calculation.calls.get(), 1);
    assert_eq!(utility_samples.get(), 0);

    let inconsistent_payment_calculation = HugeInterestCalculation {
        calls: std::cell::Cell::new(0),
        maximum: money("1"),
        invalid_result: false,
        scheduled_payment_override: Some(money("2")),
    };
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &inconsistent_payment_calculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut std::collections::BTreeMap::new(),
    };
    let error = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap_err();
    assert!(matches!(error, SimulationError::InvalidInput(_)));
    assert_eq!(inconsistent_payment_calculation.calls.get(), 1);
    assert_eq!(utility_samples.get(), 0);

    struct UnderpayingCalculation;
    impl LoanCalculation for UnderpayingCalculation {
        fn validate(
            &self,
            loan: property_simulation_models::FixedRateLoan,
        ) -> Result<(), property_simulation_models::LoanModelError> {
            TestLoanCalculation.validate(loan)
        }

        fn derive_payment(
            &self,
            loan: property_simulation_models::FixedRateLoan,
        ) -> Result<Money, property_simulation_models::LoanModelError> {
            TestLoanCalculation.derive_payment(loan)
        }

        fn calculate_month(
            &self,
            loan: property_simulation_models::FixedRateLoan,
        ) -> Result<property_simulation_models::LoanMonth, property_simulation_models::LoanModelError>
        {
            let mut result = TestLoanCalculation.calculate_month(loan)?;
            let payment = Money::from_parts(5, 2, loan.opening_balance.currency())
                .map_err(|_| property_simulation_models::LoanModelError::Arithmetic)?;
            result.payment = payment;
            result.principal = payment
                .checked_sub(result.interest)
                .map_err(|_| property_simulation_models::LoanModelError::Arithmetic)?;
            result.closing_balance = loan
                .opening_balance
                .checked_sub(result.principal)
                .map_err(|_| property_simulation_models::LoanModelError::Arithmetic)?;
            result.remaining_months = if result.closing_balance.is_zero() {
                0
            } else {
                loan.remaining_months - 1
            };
            Ok(result)
        }
    }
    let mut runtime = SliceMonthlyRuntime {
        utility_sampler: &mut utility_sampler,
        random: &mut random,
        loan_calculation: &UnderpayingCalculation,
        ids: identity(),
        repair_history: None,
        repair_occurrence_random: &mut std::collections::BTreeMap::new(),
        repair_cost_random: &mut std::collections::BTreeMap::new(),
    };
    let error = crate::calculate_monthly_slice_result(&monthly, &ledger, &mut runtime).unwrap_err();
    assert!(matches!(error, SimulationError::InvalidInput(_)));
    assert_eq!(utility_samples.get(), 0);
}

#[test]
fn production_monthly_result_keeps_both_loan_states() {
    let period = input().period;
    let opening = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let activity = ActivityInstance {
        id: ActivityInstanceId::from_bytes([52; 16]),
        slice_id: SliceId::from_bytes([53; 16]),
        configuration: ActivityConfiguration::Loan(crate::LoanConfig::default()),
    };
    let continuation = LoanContinuation {
        instance_id: activity.id,
        slice_id: activity.slice_id,
        loan: None,
    };
    let result = crate::MonthlyResult::from_settled_parts(
        period,
        opening.clone(),
        input(),
        vec![activity],
        Vec::new(),
        opening,
        IdentifiedProposal::default(),
        Vec::new(),
        Vec::new(),
        vec![continuation.clone()],
        vec![continuation],
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        result.opening_loan_continuations(),
        result.closing_loan_continuations()
    );
}

#[test]
fn production_monthly_result_audits_loan_events_against_continuations() {
    let period = input().period;
    let activity = ActivityInstance {
        id: ActivityInstanceId::from_bytes([61; 16]),
        slice_id: SliceId::from_bytes([62; 16]),
        configuration: ActivityConfiguration::Loan(crate::LoanConfig::default()),
    };
    let opening_loan = property_simulation_models::FixedRateLoan {
        opening_balance: money("100"),
        remaining_months: 12,
        annual_rate: rust_decimal::Decimal::ZERO,
        scheduled_payment: None,
    };
    let closing_loan = property_simulation_models::FixedRateLoan {
        opening_balance: money("90"),
        remaining_months: 11,
        annual_rate: rust_decimal::Decimal::ZERO,
        scheduled_payment: None,
    };
    let event_id = identity().event_id(period, activity.id, "loan_payment");
    let event = LoanEvent {
        id: event_id,
        instance_id: activity.id,
        slice_id: activity.slice_id,
        period,
        key: "loan_payment".into(),
        previous_balance: Some(money("100")),
        updated_balance: Some(money("90")),
        previous_remaining_months: Some(12),
        updated_remaining_months: Some(11),
        previous_annual_rate: Some(rust_decimal::Decimal::ZERO),
        updated_annual_rate: Some(rust_decimal::Decimal::ZERO),
    };
    let effect = LoanStateEffect {
        id: identity().effect_id(period, activity.id, "loan_payment"),
        instance_id: activity.id,
        slice_id: activity.slice_id,
        period,
        previous: Some(opening_loan),
        updated: Some(closing_loan),
        source_event_id: event_id,
    };
    let opening = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let make_result = |event: LoanEvent, link_transaction: bool| {
        let proposal = identify_proposal_for(
            SliceProposal {
                events: link_transaction
                    .then(|| EventProposal {
                        period,
                        key: "loan_payment".into(),
                    })
                    .into_iter()
                    .collect(),
                transactions: vec![TransactionProposal {
                    key: "loan_payment".into(),
                    period,
                    entries: vec![
                        LedgerEntry::new(
                            AccountId::from_bytes([7; 16]),
                            BalanceSide::Debit,
                            money("10"),
                        ),
                        LedgerEntry::new(input().cash_account, BalanceSide::Credit, money("10"))
                            .with_cash_flow_category(CashFlowCategory::Financing),
                    ],
                    source_event_key: link_transaction.then(|| "loan_payment".into()),
                }],
                ..Default::default()
            },
            identity(),
            &activity,
            period,
        );
        let mut proposal = match proposal {
            Ok(proposal) => proposal,
            Err(error) => return Err(error),
        };
        // The production settlement path keeps loan events in their typed
        // result field and removes them from the generic proposal event list.
        proposal.events.retain(|proposed| proposed.id != event_id);
        let journal = proposal.transactions()[0].clone().into_ledger_transaction();
        let contribution = MonthlyCashFlowContribution {
            period,
            instance_id: activity.id,
            slice_id: activity.slice_id,
            output_key: "loan_payment".into(),
            transaction_id: journal.id(),
            category: CashFlowCategory::Financing,
            amount: money("-10"),
        };
        crate::MonthlyResult::from_settled_parts(
            period,
            opening.clone(),
            input(),
            vec![activity.clone()],
            Vec::new(),
            OpeningState {
                cash: money("90"),
                condition: AssetCondition::Available,
            },
            proposal,
            vec![journal],
            vec![contribution],
            vec![LoanContinuation {
                instance_id: activity.id,
                slice_id: activity.slice_id,
                loan: Some(opening_loan),
            }],
            vec![LoanContinuation {
                instance_id: activity.id,
                slice_id: activity.slice_id,
                loan: Some(closing_loan),
            }],
            vec![event],
            vec![effect.clone()],
        )
    };
    assert!(make_result(event.clone(), true).is_ok());
    assert!(make_result(event.clone(), false).is_err());
    let mut inconsistent = event;
    inconsistent.updated_balance = Some(money("91"));
    assert!(make_result(inconsistent, true).is_err());
}

#[test]
fn production_monthly_result_rejects_duplicate_proposal_ids_and_reconciles_cash_flow() {
    let period = input().period;
    let opening = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let rental = rental_activity(54);
    let sample = ActivitySample::new(&rental, period, ActivitySampleValue::Rental);
    let duplicate_events = identify_proposal_for(
        SliceProposal {
            events: vec![
                EventProposal {
                    period,
                    key: "duplicate".into(),
                },
                EventProposal {
                    period,
                    key: "duplicate".into(),
                },
            ],
            ..Default::default()
        },
        identity(),
        &rental,
        period,
    )
    .unwrap();
    assert!(
        crate::MonthlyResult::from_settled_parts(
            period,
            opening.clone(),
            input(),
            vec![rental.clone()],
            vec![sample.clone()],
            opening.clone(),
            duplicate_events,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .is_err()
    );

    let entries = vec![
        LedgerEntry::new(repair_expense_account(), BalanceSide::Debit, money("10")),
        LedgerEntry::new(input().cash_account, BalanceSide::Credit, money("10"))
            .with_cash_flow_category(CashFlowCategory::Operating),
    ];
    let proposal = identify_proposal_for(
        SliceProposal {
            transactions: vec![TransactionProposal {
                key: "expense".into(),
                period,
                entries,
                source_event_key: None,
            }],
            ..Default::default()
        },
        identity(),
        &rental,
        period,
    )
    .unwrap();
    let transaction = proposal.transactions()[0].clone().into_ledger_transaction();
    let build = |amount| {
        crate::MonthlyResult::from_settled_parts(
            period,
            opening.clone(),
            input(),
            vec![rental.clone()],
            vec![sample.clone()],
            OpeningState {
                cash: money("90"),
                condition: AssetCondition::Available,
            },
            proposal.clone(),
            vec![transaction.clone()],
            vec![MonthlyCashFlowContribution {
                period,
                instance_id: rental.id,
                slice_id: rental.slice_id,
                output_key: "expense".into(),
                transaction_id: transaction.id(),
                category: CashFlowCategory::Operating,
                amount,
            }],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    };
    assert!(build(money("-10")).is_ok());
    assert!(build(money("-9")).is_err());
}

#[test]
fn monthly_slice_preparation_checks_loan_continuation_identity_without_dispatching_loan() {
    let frozen = input();
    let opening = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(opening.cash);
    let mut loan_activity = activity_with_kind(66, TestActivityKind::Loan);
    loan_activity.configuration = ActivityConfiguration::Loan(LoanConfig {
        liability_account: AccountId::from_bytes([8; 16]),
        interest_expense_account: AccountId::from_bytes([5; 16]),
        cash_account: AccountId::from_bytes([4; 16]),
    });
    let continuation = LoanContinuation {
        instance_id: loan_activity.id,
        slice_id: loan_activity.slice_id,
        loan: None,
    };
    fn make_monthly_input<'a>(
        opening: &'a OpeningState,
        frozen: &'a FrozenInput,
        activity: &'a ActivityInstance,
        ledger: &'a property_simulation_accounting::Ledger,
        loan_continuations: &'a [LoanContinuation],
    ) -> MonthlyInput<'a> {
        MonthlyInput {
            opening_state: opening,
            frozen_input: frozen,
            activities: std::slice::from_ref(activity),
            samples: &[],
            loan_continuations,
            opening_ledger: ledger,
        }
    }
    let valid = make_monthly_input(
        &opening,
        &frozen,
        &loan_activity,
        &ledger,
        std::slice::from_ref(&continuation),
    );
    assert!(
        crate::monthly::prepare_monthly_slices(&valid, None, &ledger, &TestLoanCalculation,)
            .is_ok()
    );

    let missing = make_monthly_input(&opening, &frozen, &loan_activity, &ledger, &[]);
    assert!(matches!(
        crate::monthly::prepare_monthly_slices(&missing, None, &ledger, &TestLoanCalculation,),
        Err(SimulationError::InvalidInput(_))
    ));

    let mut wrong_source = continuation.clone();
    wrong_source.slice_id = SliceId::from_bytes([99; 16]);
    let mismatched = make_monthly_input(
        &opening,
        &frozen,
        &loan_activity,
        &ledger,
        std::slice::from_ref(&wrong_source),
    );
    assert!(matches!(
        crate::monthly::prepare_monthly_slices(&mismatched, None, &ledger, &TestLoanCalculation,),
        Err(SimulationError::InvalidInput(_))
    ));

    let duplicate_continuations = [continuation.clone(), continuation];
    let duplicate = make_monthly_input(
        &opening,
        &frozen,
        &loan_activity,
        &ledger,
        &duplicate_continuations,
    );
    assert!(matches!(
        crate::monthly::prepare_monthly_slices(&duplicate, None, &ledger, &TestLoanCalculation,),
        Err(SimulationError::InvalidInput(_))
    ));

    let foreign_currency_loan = LoanContinuation {
        instance_id: loan_activity.id,
        slice_id: loan_activity.slice_id,
        loan: Some(property_simulation_models::FixedRateLoan {
            opening_balance: Money::parse("1", Currency::from_code("USD").unwrap()).unwrap(),
            remaining_months: 12,
            annual_rate: rust_decimal::Decimal::ZERO,
            scheduled_payment: None,
        }),
    };
    let wrong_currency = make_monthly_input(
        &opening,
        &frozen,
        &loan_activity,
        &ledger,
        std::slice::from_ref(&foreign_currency_loan),
    );
    assert!(matches!(
        crate::monthly::prepare_monthly_slices(
            &wrong_currency,
            None,
            &ledger,
            &TestLoanCalculation,
        ),
        Err(SimulationError::InvalidInput(_))
    ));
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
fn semantic_stream_rejects_a_different_trial_identity() {
    use property_simulation_randomness::{RandomnessError, RootKey, RunKeyFactory};

    let configured_trial = TrialId::from_bytes([70; 16]);
    let requested_trial = TrialId::from_bytes([71; 16]);
    let streams = RunKeyFactory::new(RootKey::from_seed([8; 32])).run_streams(configured_trial);
    let instance = activity(46);
    assert!(matches!(
        streams.stream(&property_simulation_randomness::SemanticIdentity {
            trial_id: requested_trial,
            category: property_simulation_randomness::RandomCategory::Property,
            slice_id: instance.slice_id,
            activity_id: instance.id,
            period: input().period,
            purpose: "repair.cost".into(),
        }),
        Err(RandomnessError::TrialIdentityMismatch)
    ));
}

fn activity(id: u8) -> ActivityInstance {
    rental_activity(id)
}
fn rental_activity(id: u8) -> ActivityInstance {
    activity_with_kind(id, TestActivityKind::Rental)
}
fn activity_with_kind(id: u8, sample_kind: TestActivityKind) -> ActivityInstance {
    let mut b = [0; 16];
    b[15] = id;
    ActivityInstance {
        id: ActivityInstanceId::from_bytes(b),
        slice_id: SliceId::from_bytes([1; 16]),
        configuration: match sample_kind {
            TestActivityKind::Rental => ActivityConfiguration::Rental,
            TestActivityKind::Loan => ActivityConfiguration::Loan(crate::LoanConfig::default()),
            TestActivityKind::PropertyTax => {
                panic!("property tax uses its explicit configuration")
            }
            TestActivityKind::Utility => panic!("utility uses its explicit configuration"),
            TestActivityKind::RepairOccurrence => {
                panic!("repair occurrence requires explicit configuration")
            }
            TestActivityKind::RepairCost => panic!("repair cost requires generated provenance"),
        },
    }
}
fn repair_expense_account() -> AccountId {
    AccountId::from_bytes([5; 16])
}

fn input() -> FrozenInput {
    FrozenInput {
        period: YearMonth::new(2025, 1).unwrap(),
        rent: money("80"),
        rent_account: AccountId::from_bytes([3; 16]),
        cash_account: AccountId::from_bytes([4; 16]),
    }
}
fn validated_input(state: &OpeningState, frozen: &FrozenInput) -> ValidatedInput {
    crate::validation::validate_slice_input(frozen, &ledger_for_cash(state.cash), state).unwrap()
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
        Account::new(
            AccountId::from_bytes([8; 16]),
            AccountCode::new(2000).unwrap(),
            AccountCategory::Liability,
            false,
            "Mortgage liability",
        )
        .unwrap(),
        Account::new(
            AccountId::from_bytes([9; 16]),
            AccountCode::new(5100).unwrap(),
            AccountCategory::Expense,
            false,
            "Utilities",
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
    let sample = ActivitySample::new(instance, input().period, value);
    match instance.configuration.test_kind() {
        TestActivityKind::Utility => {
            sample.with_purpose(Some(crate::activity::UTILITY_SAMPLE_PURPOSE.into()))
        }
        _ => sample,
    }
}

#[test]
fn property_tax_posts_on_anchored_due_month_and_retains_marker() {
    let config = PropertyTaxConfig {
        amount: money("100.004"),
        cadence: PropertyTaxCadence::Quarterly,
        first_due: YearMonth::new(2025, 1).unwrap(),
        cash_account: AccountId::from_bytes([4; 16]),
        expense_account: AccountId::from_bytes([5; 16]),
    };
    let mut instance = activity_with_kind(12, TestActivityKind::Rental);
    instance.configuration = ActivityConfiguration::PropertyTax(config);
    let state = OpeningState {
        cash: money("500"),
        condition: AssetCondition::Available,
    };
    let mut frozen = input();
    frozen.period = YearMonth::new(2025, 4).unwrap();
    let ledger = ledger_for_cash(state.cash);
    let mut validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    validated
        .validate_activities_before_sampling(std::slice::from_ref(&instance), &ledger, [])
        .unwrap();
    let slice = PropertyTaxSlice {
        instance: instance.clone(),
    };
    let output = propose_slice(
        &slice,
        &validated,
        &ActivitySample::new(&instance, frozen.period, ()),
    )
    .unwrap();
    assert_eq!(output.events.len(), 1);
    assert_eq!(output.events[0].key, "property_tax.payment");
    assert_eq!(output.transactions[0].key, "payment");
    let (result, posted) = coordinate_month(
        &validated,
        &ledger,
        vec![output],
        TrialId::from_bytes([1; 16]),
        BranchId::from_bytes([2; 16]),
        &TestIds,
    )
    .unwrap();
    assert_eq!(
        result.activities()[0].configuration,
        ActivityConfiguration::PropertyTax(config)
    );
    assert_eq!(result.samples()[0].value, ActivitySampleValue::PropertyTax);
    assert_eq!(result.proposal().events()[0].key, "property_tax.payment");
    assert_eq!(result.proposal().transactions()[0].key, "payment");
    assert_eq!(
        result.proposal().transactions()[0].source_event_id,
        Some(result.proposal().events()[0].id)
    );
    assert_eq!(
        result.cash_flow_contributions()[0].transaction_id,
        result.proposal().transactions()[0].id
    );
    assert_eq!(result.cash_flow_contributions()[0].amount, money("-100"));
    assert_eq!(posted.balance(config.cash_account).unwrap().1, money("400"));
}

#[test]
fn property_tax_annual_schedule_carries_anchor_across_calendar_years() {
    let mut instance = activity_with_kind(26, TestActivityKind::Rental);
    instance.configuration = ActivityConfiguration::PropertyTax(PropertyTaxConfig {
        amount: money("1200"),
        cadence: PropertyTaxCadence::Annual,
        first_due: YearMonth::new(2026, 12).unwrap(),
        cash_account: AccountId::from_bytes([4; 16]),
        expense_account: AccountId::from_bytes([5; 16]),
    });
    let state = OpeningState {
        cash: money("2000"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(state.cash);
    for (period, expected_due) in [
        (YearMonth::new(2027, 12).unwrap(), true),
        (YearMonth::new(2028, 1).unwrap(), false),
    ] {
        let mut frozen = input();
        frozen.period = period;
        let mut validated =
            crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
        validated
            .validate_activities_before_sampling(std::slice::from_ref(&instance), &ledger, [])
            .unwrap();
        let output = propose_slice(
            &PropertyTaxSlice {
                instance: instance.clone(),
            },
            &validated,
            &ActivitySample::new(&instance, period, ()),
        )
        .unwrap();
        assert_eq!(!output.events.is_empty(), expected_due);
        assert_eq!(!output.transactions.is_empty(), expected_due);
    }
}

#[test]
fn property_tax_cadences_use_the_first_due_month_as_the_schedule_anchor() {
    let anchor = YearMonth::new(2026, 12).unwrap();
    assert!(crate::validation::tax_due(
        anchor,
        anchor,
        PropertyTaxCadence::Monthly
    ));
    assert!(!crate::validation::tax_due(
        YearMonth::new(2026, 11).unwrap(),
        anchor,
        PropertyTaxCadence::Annual
    ));
    assert!(crate::validation::tax_due(
        YearMonth::new(2027, 1).unwrap(),
        anchor,
        PropertyTaxCadence::Monthly
    ));
    assert!(!crate::validation::tax_due(
        YearMonth::new(2027, 2).unwrap(),
        anchor,
        PropertyTaxCadence::Quarterly
    ));
    assert!(crate::validation::tax_due(
        YearMonth::new(2027, 3).unwrap(),
        anchor,
        PropertyTaxCadence::Quarterly
    ));
    assert!(!crate::validation::tax_due(
        YearMonth::new(2027, 11).unwrap(),
        anchor,
        PropertyTaxCadence::Annual
    ));
    assert!(crate::validation::tax_due(
        YearMonth::new(2027, 12).unwrap(),
        anchor,
        PropertyTaxCadence::Annual
    ));
}

#[test]
fn property_tax_non_due_zero_and_empty_months_are_identity() {
    let mut instance = activity_with_kind(13, TestActivityKind::Rental);
    instance.configuration = ActivityConfiguration::PropertyTax(PropertyTaxConfig {
        amount: money("0.004"),
        cadence: PropertyTaxCadence::Annual,
        first_due: YearMonth::new(2025, 12).unwrap(),
        cash_account: AccountId::from_bytes([4; 16]),
        expense_account: AccountId::from_bytes([5; 16]),
    });
    let state = OpeningState {
        cash: money("500"),
        condition: AssetCondition::Available,
    };
    let mut frozen = input();
    frozen.period = YearMonth::new(2026, 1).unwrap();
    let ledger = ledger_for_cash(state.cash);
    let mut validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    validated
        .validate_activities_before_sampling(std::slice::from_ref(&instance), &ledger, [])
        .unwrap();
    let output = propose_slice(
        &PropertyTaxSlice {
            instance: instance.clone(),
        },
        &validated,
        &ActivitySample::new(&instance, frozen.period, ()),
    )
    .unwrap();
    assert!(output.events.is_empty() && output.transactions.is_empty());
    let (non_due, still_open) = coordinate_month(
        &validated,
        &ledger,
        vec![output],
        TrialId::from_bytes([1; 16]),
        BranchId::from_bytes([2; 16]),
        &TestIds,
    )
    .unwrap();
    assert_eq!(non_due.samples()[0].value, ActivitySampleValue::PropertyTax);
    assert!(non_due.proposal().events().is_empty() && non_due.proposal().transactions().is_empty());
    assert_eq!(still_open, ledger);
    frozen.period = YearMonth::new(2025, 12).unwrap();
    let mut december_input =
        crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    december_input
        .validate_activities_before_sampling(std::slice::from_ref(&instance), &ledger, [])
        .unwrap();
    let output = propose_slice(
        &PropertyTaxSlice {
            instance: instance.clone(),
        },
        &december_input,
        &ActivitySample::new(&instance, frozen.period, ()),
    )
    .unwrap();
    assert!(output.events.is_empty() && output.transactions.is_empty());
    let (rounded_zero, unchanged) = coordinate_month(
        &december_input,
        &ledger,
        vec![output],
        TrialId::from_bytes([1; 16]),
        BranchId::from_bytes([2; 16]),
        &TestIds,
    )
    .unwrap();
    assert_eq!(
        rounded_zero.samples()[0].value,
        ActivitySampleValue::PropertyTax
    );
    assert!(
        rounded_zero.proposal().events().is_empty()
            && rounded_zero.cash_flow_contributions().is_empty()
    );
    assert_eq!(unchanged, ledger);
    let mut exact_zero = instance.clone();
    if let ActivityConfiguration::PropertyTax(config) = &mut exact_zero.configuration {
        config.amount = money("0");
    }
    let mut exact_zero_input =
        crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    exact_zero_input
        .validate_activities_before_sampling(std::slice::from_ref(&exact_zero), &ledger, [])
        .unwrap();
    let zero_output = propose_slice(
        &PropertyTaxSlice {
            instance: exact_zero.clone(),
        },
        &exact_zero_input,
        &ActivitySample::new(&exact_zero, frozen.period, ()),
    )
    .unwrap();
    assert!(zero_output.events.is_empty() && zero_output.transactions.is_empty());
    let empty_input = validated_input(&state, &input());
    let (empty_result, unchanged) = coordinate_month(
        &empty_input,
        &ledger_for_cash(state.cash),
        vec![],
        TrialId::from_bytes([1; 16]),
        BranchId::from_bytes([2; 16]),
        &TestIds,
    )
    .unwrap();
    assert!(empty_result.activities().is_empty());
    assert_eq!(unchanged, ledger_for_cash(state.cash));
}

#[test]
fn property_tax_composes_with_rent_on_shared_cash_in_any_order() {
    let state = OpeningState {
        cash: money("500"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let mut validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let rent = rental_activity(20);
    let mut repair = activity(25);
    repair.configuration = ActivityConfiguration::RepairCost(RepairCostConfig {
        source_event_id: EventId::from_bytes([25; 16]),
        source_instance_id: ActivityInstanceId::from_bytes([26; 16]),
        source_slice_id: SliceId::from_bytes([27; 16]),
        occurrence_period: frozen.period.checked_add_months(-1).unwrap(),
        delay_months: 1,
        effective_period: frozen.period,
        maximum: money("10"),
        expense_account: repair_expense_account(),
    });
    let mut tax = activity_with_kind(21, TestActivityKind::Rental);
    tax.configuration = ActivityConfiguration::PropertyTax(PropertyTaxConfig {
        amount: money("100"),
        cadence: PropertyTaxCadence::Quarterly,
        first_due: frozen.period,
        cash_account: frozen.cash_account,
        expense_account: repair_expense_account(),
    });
    validated
        .validate_activities_before_sampling(
            &[rent.clone(), repair.clone(), tax.clone()],
            &ledger,
            [],
        )
        .unwrap();
    let rent_output = propose_slice(
        &RentalSlice {
            instance: rent.clone(),
        },
        &validated,
        &ActivitySample::new(&rent, frozen.period, ()),
    )
    .unwrap();
    let tax_output = propose_slice(
        &PropertyTaxSlice {
            instance: tax.clone(),
        },
        &validated,
        &ActivitySample::new(&tax, frozen.period, ()),
    )
    .unwrap();
    let repair_output = propose_slice(
        &RepairCostSlice {
            instance: repair.clone(),
        },
        &validated,
        &ActivitySample::new(
            &repair,
            frozen.period,
            property_simulation_models::RepairCostSample(money("10")),
        )
        .with_purpose(Some("repair.cost".into())),
    )
    .unwrap();

    let args = (TrialId::from_bytes([1; 16]), BranchId::from_bytes([2; 16]));
    assert!(
        coordinate_month(
            &validated,
            &ledger,
            vec![tax_output.clone(), tax_output.clone()],
            args.0,
            args.1,
            &TestIds,
        )
        .is_err()
    );
    let (forward, forward_ledger) = coordinate_month(
        &validated,
        &ledger,
        vec![
            rent_output.clone(),
            tax_output.clone(),
            repair_output.clone(),
        ],
        args.0,
        args.1,
        &TestIds,
    )
    .unwrap();
    let (reverse, reverse_ledger) = coordinate_month(
        &validated,
        &ledger,
        vec![repair_output, tax_output, rent_output],
        args.0,
        args.1,
        &TestIds,
    )
    .unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(forward_ledger, reverse_ledger);
    assert_eq!(
        forward_ledger.balance(frozen.cash_account).unwrap().1,
        money("470")
    );
    assert_eq!(forward.cash_flow_totals().unwrap().operating, money("-30"));
    assert_eq!(forward.proposal().events().len(), 3);
    assert_eq!(forward.proposal().transactions().len(), 3);
}

#[test]
fn property_tax_invalid_currency_and_cash_account_are_rejected() {
    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    for config in [
        PropertyTaxConfig {
            amount: Money::parse("10", Currency::from_code("USD").unwrap()).unwrap(),
            cadence: PropertyTaxCadence::Monthly,
            first_due: frozen.period,
            cash_account: frozen.cash_account,
            expense_account: repair_expense_account(),
        },
        PropertyTaxConfig {
            amount: money("10"),
            cadence: PropertyTaxCadence::Monthly,
            first_due: frozen.period,
            cash_account: frozen.rent_account,
            expense_account: repair_expense_account(),
        },
    ] {
        let mut instance = activity_with_kind(22, TestActivityKind::Rental);
        instance.configuration = ActivityConfiguration::PropertyTax(config);
        assert!(
            propose_slice(
                &PropertyTaxSlice {
                    instance: instance.clone()
                },
                &validated,
                &ActivitySample::new(&instance, frozen.period, ())
            )
            .is_err()
        );
    }
}

#[test]
fn property_tax_rejects_a_contra_expense_account() {
    use property_simulation_accounting::{
        Account, AccountCategory, AccountCode, BalanceSide, Ledger, OpeningBalance, OpeningBalances,
    };
    let mut chart = chart();
    let contra_expense = AccountId::from_bytes([7; 16]);
    chart
        .insert(
            Account::new(
                contra_expense,
                AccountCode::new(5001).unwrap(),
                AccountCategory::Expense,
                true,
                "Contra expense",
            )
            .unwrap(),
        )
        .unwrap();
    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let opening = OpeningBalances::new(
        &chart,
        [
            OpeningBalance::new(input().cash_account, BalanceSide::Debit, money("100")).unwrap(),
            OpeningBalance::new(
                AccountId::from_bytes([6; 16]),
                BalanceSide::Credit,
                money("100"),
            )
            .unwrap(),
        ],
        Some(state.cash),
    )
    .unwrap();
    let ledger = Ledger::new(chart, opening).unwrap();
    let frozen = input();
    let validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let mut instance = activity_with_kind(27, TestActivityKind::Rental);
    instance.configuration = ActivityConfiguration::PropertyTax(PropertyTaxConfig {
        amount: money("10"),
        cadence: PropertyTaxCadence::Monthly,
        first_due: frozen.period,
        cash_account: frozen.cash_account,
        expense_account: contra_expense,
    });
    assert!(
        propose_slice(
            &PropertyTaxSlice {
                instance: instance.clone()
            },
            &validated,
            &ActivitySample::new(&instance, frozen.period, ())
        )
        .is_err()
    );
}

#[test]
fn property_tax_due_amount_is_in_the_operating_support_bound() {
    let mut first = activity_with_kind(23, TestActivityKind::Rental);
    let mut second = activity_with_kind(24, TestActivityKind::Rental);
    let large =
        Money::from_parts(i128::MAX / 4 * 3, 2, Currency::from_code("CAD").unwrap()).unwrap();
    let config = |amount| PropertyTaxConfig {
        amount,
        cadence: PropertyTaxCadence::Quarterly,
        first_due: input().period,
        cash_account: input().cash_account,
        expense_account: repair_expense_account(),
    };
    first.configuration = ActivityConfiguration::PropertyTax(config(large));
    second.configuration = ActivityConfiguration::PropertyTax(config(large));
    assert!(
        crate::validation::validate_cashflow_support(&input(), &[first.clone(), second.clone()])
            .is_err()
    );
    let mut non_due = input();
    non_due.period = YearMonth::new(2025, 2).unwrap();
    assert!(crate::validation::validate_cashflow_support(&non_due, &[first, second]).is_ok());
}

fn utility_activity(id: u8, maximum_usage: u64, rate: &str) -> ActivityInstance {
    let mut instance = activity_with_kind(id, TestActivityKind::Rental);
    instance.configuration = ActivityConfiguration::Utility(UtilityConfig {
        usage_unit: "kWh".into(),
        maximum_usage,
        rate_per_unit: money(rate),
        cash_account: input().cash_account,
        expense_account: AccountId::from_bytes([9; 16]),
    });
    instance
}

#[test]
fn utility_slice_posts_exact_usage_charge_and_retains_sample_provenance() {
    let state = OpeningState {
        cash: money("500"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let mut validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let instance = utility_activity(61, 100, "0.125");
    validated
        .validate_activities_before_sampling(std::slice::from_ref(&instance), &ledger, [])
        .unwrap();
    let slice = UtilitySlice { instance };
    let sample = ActivitySample::new(&slice.instance, frozen.period, 3_u64)
        .with_purpose(Some(UTILITY_SAMPLE_PURPOSE.into()));
    let output = propose_slice(&slice, &validated, &sample).unwrap();
    assert_eq!(output.transactions.len(), 1);
    assert_eq!(output.transactions[0].key, "usage-charge");
    assert_eq!(
        output.transactions[0].source_event_key.as_deref(),
        Some("utility.usage_charged")
    );
    assert_eq!(output.events[0].key, "utility.usage_charged");
    assert_eq!(output.transactions[0].entries[0].amount(), money("0.38"));
    assert_eq!(output.transactions[0].entries[1].amount(), money("0.38"));

    let (result, committed) = coordinate_month(
        &validated,
        &ledger,
        vec![output],
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        identity(),
    )
    .unwrap();
    assert_eq!(result.samples()[0].value, ActivitySampleValue::Utility(3));
    assert_eq!(result.cash_flow_contributions().len(), 1);
    assert_eq!(result.cash_flow_contributions()[0].amount, money("-0.38"));
    assert_eq!(
        committed.balance(input().cash_account).unwrap().1,
        money("499.62")
    );
}

#[test]
fn utility_zero_and_rounded_zero_have_no_economic_outputs() {
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let mut validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let instances = [
        utility_activity(62, 100, "2.50"),
        utility_activity(63, 100, "0.001"),
    ];
    validated
        .validate_activities_before_sampling(&instances, &ledger, [])
        .unwrap();
    for (instance, usage) in instances.into_iter().zip([0, 3]) {
        let slice = UtilitySlice { instance };
        let sample = ActivitySample::new(&slice.instance, frozen.period, usage)
            .with_purpose(Some(crate::activity::UTILITY_SAMPLE_PURPOSE.into()));
        let output = propose_slice(&slice, &validated, &sample).unwrap();
        assert!(output.events.is_empty());
        assert!(output.transactions.is_empty());
    }
}

#[test]
fn utility_sample_and_configuration_boundaries_are_validated() {
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let mut validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let slice = UtilitySlice {
        instance: utility_activity(64, 10, "1"),
    };
    assert!(
        propose_slice(
            &slice,
            &validated,
            &ActivitySample::new(&slice.instance, frozen.period, 0)
        )
        .is_err()
    );
    validated
        .validate_activities_before_sampling(std::slice::from_ref(&slice.instance), &ledger, [])
        .unwrap();
    assert!(
        propose_slice(
            &slice,
            &validated,
            &ActivitySample::new(&slice.instance, frozen.period, 1)
        )
        .is_err()
    );
    assert!(
        propose_slice(
            &slice,
            &validated,
            &ActivitySample::new(&slice.instance, frozen.period, 11)
        )
        .is_err()
    );
    assert!(
        propose_slice(
            &slice,
            &validated,
            &ActivitySample::new(&slice.instance, frozen.period, 1)
                .with_purpose(Some("other".into()))
        )
        .is_err()
    );
    let mut unrepresentable = utility_activity(65, 2, "1");
    if let ActivityConfiguration::Utility(config) = &mut unrepresentable.configuration {
        config.rate_per_unit =
            Money::from_parts(i128::MAX, 0, config.rate_per_unit.currency()).unwrap();
    }
    let invalid = UtilitySlice {
        instance: unrepresentable,
    };
    assert!(
        crate::validation::validate_activity_configuration(&invalid.instance, &frozen, &ledger)
            .is_err()
    );
    assert!(crate::validation::validate_cashflow_support(&frozen, &[invalid.instance]).is_err());

    let mut large_a = utility_activity(70, 1, "1");
    let mut large_b = utility_activity(71, 1, "1");
    let large_rate =
        Money::from_parts(i128::MAX / 2 + 1, 0, Currency::from_code("CAD").unwrap()).unwrap();
    for activity in [&mut large_a, &mut large_b] {
        if let ActivityConfiguration::Utility(config) = &mut activity.configuration {
            config.rate_per_unit = large_rate;
        }
    }
    assert!(crate::validation::validate_activity_configuration(&large_a, &frozen, &ledger).is_ok());
    assert!(crate::validation::validate_activity_configuration(&large_b, &frozen, &ledger).is_ok());
    assert!(crate::validation::validate_cashflow_support(&frozen, &[large_a, large_b]).is_err());
}

#[test]
fn utility_sampling_uses_required_semantic_identity_and_checks_trial() {
    let frozen = input();
    let state = OpeningState {
        cash: money("0"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(state.cash);
    let instance = utility_activity(66, 100, "1");
    let wide_instance = utility_activity(73, u64::MAX, "1");
    let other_instance = utility_activity(72, u64::MAX, "1");
    let all_activities = [
        instance.clone(),
        wide_instance.clone(),
        other_instance.clone(),
    ];
    let mut validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let streams = property_simulation_randomness::RunKeyFactory::new(
        property_simulation_randomness::RootKey::from_seed([11; 32]),
    )
    .run_streams(TrialId::from_bytes([6; 16]));
    let mut random = semantic_random(
        &streams,
        TrialId::from_bytes([6; 16]),
        &instance,
        frozen.period,
        property_simulation_randomness::RandomCategory::OccupancyOperations,
        UTILITY_SAMPLE_PURPOSE,
    );
    assert!(matches!(
        sample_utility(
            &validated,
            &instance,
            frozen.period,
            &UtilityUsageDistribution { maximum: 100 },
            &mut UtilityUsageSampler,
            &mut random,
        ),
        Err(UtilitySamplingError::ActivitySetNotPreflighted)
    ));
    validated
        .validate_activities_before_sampling(&all_activities, &ledger, [])
        .unwrap();
    let distribution = UtilityUsageDistribution { maximum: 100 };
    let mut random = semantic_random(
        &streams,
        TrialId::from_bytes([6; 16]),
        &instance,
        frozen.period,
        property_simulation_randomness::RandomCategory::OccupancyOperations,
        UTILITY_SAMPLE_PURPOSE,
    );
    let sample = sample_utility(
        &validated,
        &instance,
        input().period,
        &distribution,
        &mut UtilityUsageSampler,
        &mut random,
    )
    .unwrap();
    assert!(sample.value <= 100);
    assert_eq!(sample.purpose.as_deref(), Some(UTILITY_SAMPLE_PURPOSE));
    let february = FrozenInput {
        period: YearMonth::new(2025, 2).unwrap(),
        ..frozen
    };
    let mut validated_february =
        crate::validation::validate_slice_input(&february, &ledger, &state).unwrap();
    validated_february
        .validate_activities_before_sampling(&all_activities, &ledger, [])
        .unwrap();
    let unrestricted = UtilityUsageDistribution { maximum: u64::MAX };
    let draw = |validated: &ValidatedInput, instance: &ActivityInstance, period| {
        let mut random = semantic_random(
            &streams,
            TrialId::from_bytes([6; 16]),
            instance,
            period,
            property_simulation_randomness::RandomCategory::OccupancyOperations,
            UTILITY_SAMPLE_PURPOSE,
        );
        sample_utility(
            validated,
            instance,
            period,
            &unrestricted,
            &mut UtilityUsageSampler,
            &mut random,
        )
        .unwrap()
        .value
    };
    assert_eq!(
        draw(&validated, &wide_instance, frozen.period),
        draw(&validated, &wide_instance, frozen.period)
    );
    assert_ne!(
        draw(&validated, &wide_instance, frozen.period),
        draw(&validated, &other_instance, frozen.period)
    );
    assert_ne!(
        draw(&validated, &wide_instance, frozen.period),
        draw(
            &validated_february,
            &wide_instance,
            YearMonth::new(2025, 2).unwrap()
        )
    );
    let foreign_identity = property_simulation_randomness::SemanticIdentity {
        trial_id: TrialId::from_bytes([7; 16]),
        category: property_simulation_randomness::RandomCategory::OccupancyOperations,
        slice_id: instance.slice_id,
        activity_id: instance.id,
        period: frozen.period,
        purpose: UTILITY_SAMPLE_PURPOSE.into(),
    };
    assert!(matches!(
        streams.stream(&foreign_identity),
        Err(property_simulation_randomness::RandomnessError::TrialIdentityMismatch)
    ));
}

#[test]
fn utility_and_rental_composition_is_permutation_invariant() {
    let state = OpeningState {
        cash: money("500"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let mut validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let utility_a = UtilitySlice {
        instance: utility_activity(67, 20, "2"),
    };
    let utility_b = UtilitySlice {
        instance: utility_activity(68, 20, "3"),
    };
    let rental = RentalSlice {
        instance: rental_activity(69),
    };
    validated
        .validate_activities_before_sampling(
            &[
                utility_a.instance.clone(),
                utility_b.instance.clone(),
                rental.instance.clone(),
            ],
            &ledger,
            [],
        )
        .unwrap();
    let ua = ActivitySample::new(&utility_a.instance, frozen.period, 2_u64)
        .with_purpose(Some(UTILITY_SAMPLE_PURPOSE.into()));
    let ub = ActivitySample::new(&utility_b.instance, frozen.period, 3_u64)
        .with_purpose(Some(UTILITY_SAMPLE_PURPOSE.into()));
    let rent = ActivitySample::new(&rental.instance, frozen.period, ());
    let out_ua = propose_slice(&utility_a, &validated, &ua).unwrap();
    let out_ub = propose_slice(&utility_b, &validated, &ub).unwrap();
    let out_rent = propose_slice(&rental, &validated, &rent).unwrap();

    let left = coordinate_month(
        &validated,
        &ledger,
        vec![out_ua.clone(), out_ub.clone(), out_rent.clone()],
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        identity(),
    )
    .unwrap();
    let right = coordinate_month(
        &validated,
        &ledger,
        vec![out_rent, out_ub, out_ua],
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        identity(),
    )
    .unwrap();
    assert_eq!(left, right);
}

#[test]
fn coordinator_enforces_output_provenance_and_posts_month_atomically() {
    use crate::{RentalSlice, coordinate_month, propose_slice};
    let mut frozen = input();
    frozen.rent = money("80");
    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let ledger = ledger_for_cash(state.cash);
    let validated = validated_input(&state, &frozen);
    let rental = rental_activity(11);
    let second_rental = rental_activity(12);
    let rental_output = propose_slice(
        &RentalSlice {
            instance: rental.clone(),
        },
        &validated,
        &sample(&rental, ()),
    )
    .unwrap();
    let second_rental_output = propose_slice(
        &RentalSlice {
            instance: second_rental.clone(),
        },
        &validated,
        &sample(&second_rental, ()),
    )
    .unwrap();

    let mut changed_frozen = frozen;
    changed_frozen.rent = money("90");
    let changed_input = validated_input(&state, &changed_frozen);
    assert!(
        coordinate_month(
            &changed_input,
            &ledger,
            vec![second_rental_output.clone(), rental_output.clone()],
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            &TestIds,
        )
        .is_err()
    );
    assert_eq!(ledger.transactions().count(), 0);
    let forward_outputs = vec![second_rental_output.clone(), rental_output.clone()];
    let (result, closing_ledger) = coordinate_month(
        &validated,
        &ledger,
        forward_outputs.clone(),
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        &TestIds,
    )
    .unwrap();
    assert_eq!(result.closing_state().cash, money("260"));
    assert_eq!(result.closing_state().condition, AssetCondition::Available);
    assert_eq!(
        closing_ledger.balance(frozen.cash_account).unwrap().1,
        money("260")
    );
    assert_eq!(closing_ledger.transactions().count(), 2);
    assert_eq!(result.cash_flow_contributions().len(), 2);
    assert_eq!(result.cash_flow_totals().unwrap().operating, money("160"));
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
    assert_eq!(amounts, vec![money("80"), money("80")]);
    for contribution in result.cash_flow_contributions() {
        assert!(closing_ledger.transactions().any(|transaction| {
            transaction.id() == contribution.transaction_id
                && transaction.source().is_some_and(|source| {
                    source.instance_id == contribution.instance_id
                        && source.slice_id == contribution.slice_id
                        && source.output_key == contribution.output_key
                })
        }));
    }
    assert_eq!(ledger.balance(frozen.cash_account).unwrap().1, money("100"));
    let (permuted_result, permuted_ledger) = coordinate_month(
        &validated,
        &ledger,
        forward_outputs.into_iter().rev().collect(),
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        &TestIds,
    )
    .unwrap();
    assert_eq!(permuted_result, result);
    assert_eq!(permuted_ledger, closing_ledger);
}

#[test]
fn coordinator_rejects_unknown_effect_and_invalid_journal_without_partial_commit() {
    use crate::{
        EffectProposal, RentalSlice, SliceProposal, TransactionProposal, coordinate_month,
    };
    use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};
    use property_simulation_domain::ResourceKey;

    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::NeedsRepair,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let instance = rental_activity(21);
    let slice = RentalSlice {
        instance: instance.clone(),
    };
    let mut resources = slice.resources(&frozen);
    resources
        .writes
        .insert(ResourceKey::for_account(repair_expense_account()));
    let unknown_target = ResourceKey::new("property.unknown").unwrap();
    resources.writes.insert(unknown_target.clone());
    let sample = ActivitySample::new(&instance, frozen.period, ActivitySampleValue::Rental);
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
                LedgerEntry::new(repair_expense_account(), BalanceSide::Debit, money("2")),
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
fn coordinator_rejects_sample_from_unknown_activity_without_mutating_opening_ledger() {
    use crate::{RentalSlice, SliceOutput, coordinate_month};

    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::Available,
    };
    let frozen = input();
    let ledger = ledger_for_cash(state.cash);
    let validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let instance = rental_activity(41);
    let slice = RentalSlice {
        instance: instance.clone(),
    };
    let wrong_source = ActivitySample::new(
        &rental_activity(42),
        frozen.period,
        ActivitySampleValue::Rental,
    );
    let output = SliceOutput {
        proposal: Default::default(),
        instance: instance.clone(),
        period: frozen.period,
        opening_state: state.clone(),
        frozen_input: frozen,
        resources: slice.resources(&input()),
        sample: wrong_source.clone(),
    };

    assert!(matches!(
        coordinate_month(
            &validated,
            &ledger,
            vec![output],
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            &TestIds,
        ),
        Err(SimulationError::InvalidProposal(
            "sample is missing, duplicated, or has an invalid source"
        ))
    ));
    assert_eq!(ledger.transactions().count(), 0);
    assert_eq!(
        ledger.balance(input().cash_account).unwrap().1,
        money("100")
    );
}

#[test]
fn slice_transition_errors_are_fatal_with_activity_and_month_diagnostics() {
    struct BrokenSlice(ActivityInstance);
    impl SimulationSlice for BrokenSlice {
        type Sample = ();
        fn instance(&self) -> &ActivityInstance {
            &self.0
        }
        fn resources(&self, input: &FrozenInput) -> SliceResources {
            RentalSlice {
                instance: self.0.clone(),
            }
            .resources(input)
        }
        fn sample_value(&self, _sample: &()) -> ActivitySampleValue {
            ActivitySampleValue::Rental
        }
        fn propose_validated(
            &self,
            _: &OpeningState,
            _: &FrozenInput,
            _: &ValidatedActivitySample<'_, ()>,
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
    let validated = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let slice = BrokenSlice(rental_activity(29));
    let sample = ActivitySample::new(&slice.0, frozen.period, ());
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
fn active_cashflow_support_bound_rejects_unrepresentable_monthly_absolute_sum() {
    let mut frozen = input();
    frozen.rent = Money::from_parts(i128::MAX, 0, frozen.rent.currency()).unwrap();
    let active = vec![rental_activity(41), rental_activity(42)];
    assert!(crate::validation::validate_cashflow_support(&frozen, &active).is_err());
    assert!(crate::validation::validate_cashflow_support(&frozen, &[rental_activity(43)]).is_ok());
}

#[test]
fn active_cashflow_support_includes_context_bound_contributions() {
    let frozen = input();
    let currency = frozen.rent.currency();
    let maximum = Money::from_parts(i128::MAX, 0, currency).unwrap();
    let loan = ActivityInstance {
        id: ActivityInstanceId::from_bytes([61; 16]),
        slice_id: SliceId::from_bytes([62; 16]),
        configuration: ActivityConfiguration::Loan(crate::LoanConfig::default()),
    };
    assert!(
        crate::validation::validate_cashflow_support_with_additional(
            &frozen,
            &[loan],
            [
                (
                    property_simulation_accounting::CashFlowCategory::Operating,
                    maximum,
                ),
                (
                    property_simulation_accounting::CashFlowCategory::Operating,
                    Money::new(1.into(), currency),
                ),
            ],
        )
        .is_err()
    );
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

    let errors =
        crate::validation::validate_slice_input(&frozen, &ledger_for_cash(money("0")), &state)
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

    let errors =
        crate::validation::validate_slice_input(&input(), &usd_ledger, &state).unwrap_err();

    assert!(errors.0.iter().any(|issue| issue.field == "rent"));
}

#[test]
fn input_validation_rejects_cash_state_that_differs_from_ledger_balance() {
    let state = OpeningState {
        cash: money("10"),
        condition: AssetCondition::Available,
    };
    let errors =
        crate::validation::validate_slice_input(&input(), &ledger_for_cash(money("0")), &state)
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
    frozen.rent_account = repair_expense_account();
    let errors =
        crate::validation::validate_slice_input(&frozen, &ledger_for_cash(state.cash), &state)
            .unwrap_err();

    assert!(errors.0.iter().any(|issue| issue.field == "cash_account"));
    assert!(errors.0.iter().any(|issue| issue.field == "rent_account"));
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
        period: None,
        events: Vec::new(),
        effects: Vec::new(),
        transactions: vec![ProposedTransaction {
            id: LedgerTransactionId::from_bytes([77; 16]),
            instance_id: activity(1).id,
            slice_id: activity(1).slice_id,
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
    assert!(
        crate::validation::validate_slice_input(&frozen, &ledger_for_cash(state.cash), &state)
            .is_ok()
    );

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
fn output_id_allocator_uses_the_typed_output_identity_tuple() {
    let factory = identity();
    let period = YearMonth::new(2025, 1).unwrap();
    let instance = activity(4).id;
    let transaction_id = factory.transaction_id(period, instance, "rent");
    assert_eq!(
        transaction_id,
        factory.transaction_id(period, instance, "rent")
    );
    assert_ne!(
        transaction_id,
        factory.transaction_id(period, instance, "repair")
    );
    assert_ne!(
        transaction_id,
        factory.transaction_id(period.checked_add_months(1).unwrap(), instance, "rent")
    );
    assert_ne!(
        transaction_id,
        factory.transaction_id(period, activity(5).id, "rent")
    );
    let event_id = factory.event_id(period, instance, "rent");
    let effect_id = factory.effect_id(period, instance, "rent");
    assert_ne!(transaction_id.as_bytes(), event_id.as_bytes());
    assert_ne!(event_id.as_bytes(), effect_id.as_bytes());
    assert_eq!(
        factory
            .activity_instance_id(period, instance, "repair_cost")
            .as_bytes()
            .len(),
        16
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
    assert_eq!(forward.transactions()[0].period, input().period);
    assert_eq!(
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
            identity(),
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
fn proposal_composition_is_permutation_and_grouping_invariant() {
    let period = input().period;
    let proposal = |activity_id: u8, key: &'static str| {
        identify_proposal_for(
            SliceProposal {
                events: vec![EventProposal {
                    period,
                    key: key.into(),
                }],
                ..Default::default()
            },
            identity(),
            &activity(activity_id),
            period,
        )
        .unwrap()
    };
    let a = proposal(1, "a");
    let b = proposal(2, "b");
    let c = proposal(3, "c");

    let canonical = merge_proposals([a.clone(), b.clone(), c.clone()]).unwrap();
    assert_eq!(
        merge_proposals([c.clone(), a.clone(), b.clone()]).unwrap(),
        canonical
    );
    let left_grouped =
        merge_proposals([merge_proposals([a.clone(), b.clone()]).unwrap(), c.clone()]).unwrap();
    let right_grouped = merge_proposals([a, merge_proposals([b, c]).unwrap()]).unwrap();
    assert_eq!(left_grouped, canonical);
    assert_eq!(right_grouped, canonical);
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
    let first_rental = rental_activity(1);
    let sample = ActivitySample {
        instance_id: first_rental.id,
        slice_id: first_rental.slice_id,
        period,
        purpose: None,
        value: ActivitySampleValue::Rental,
    };
    let result = TestMonthlyResult::new(
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        period,
        OpeningState {
            cash: money("0"),
            condition: AssetCondition::Available,
        },
        input(),
        vec![first_rental.clone()],
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
    assert_eq!(result.samples()[0].value, ActivitySampleValue::Rental);
    let foreign_sample =
        ActivitySample::new(&rental_activity(2), period, ActivitySampleValue::Rental);
    assert!(
        TestMonthlyResult::new(
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            period,
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available,
            },
            input(),
            vec![first_rental],
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
    let wrong_kind_sample_for_rental =
        ActivitySample::new(&rental_instance, period, ActivitySampleValue::Utility(0));
    assert!(
        TestMonthlyResult::new(
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            period,
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available,
            },
            input(),
            vec![rental_instance],
            vec![wrong_kind_sample_for_rental],
            OpeningState {
                cash: money("0"),
                condition: AssetCondition::Available,
            },
            IdentifiedProposal::default(),
        )
        .is_err()
    );
    let wrong_branch_proposal =
        identify_proposal_for(SliceProposal::default(), identity(), &activity(1), period).unwrap();
    assert!(
        TestMonthlyResult::new(
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
        TestMonthlyResult::new(
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
        TestMonthlyResult::new(
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
            "cannot merge outputs from different periods"
        ))
    );
}

#[test]
fn composition_allows_independent_read_write_overlap_and_rejects_undeclared_effect() {
    let key = property_simulation_domain::ResourceKey::new("property.condition").unwrap();
    let a = SliceResources {
        instance_id: ActivityInstanceId::from_bytes([1; 16]),
        slice_id: SliceId::from_bytes([1; 16]),
        reads: Default::default(),
        writes: [key.clone()].into_iter().collect(),
    };
    let b = SliceResources {
        instance_id: ActivityInstanceId::from_bytes([2; 16]),
        slice_id: SliceId::from_bytes([2; 16]),
        writes: Default::default(),
        reads: [key.clone()].into_iter().collect(),
    };
    let same_writer = SliceResources {
        instance_id: ActivityInstanceId::from_bytes([3; 16]),
        slice_id: SliceId::from_bytes([3; 16]),
        reads: Default::default(),
        writes: [key.clone()].into_iter().collect(),
    };
    assert!(validate_composition(&[a.clone(), same_writer], &[]).is_ok());
    assert!(validate_composition(&[a.clone(), b], &[]).is_ok());
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
        reads: Default::default(),
        writes: Default::default(),
    };
    assert!(validate_composition(&[undeclared], std::slice::from_ref(&proposal)).is_err());
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
            .contains(&property_simulation_domain::ResourceKey::for_account(
                period_input.cash_account
            ))
    );
    assert!(
        declaration
            .writes
            .contains(&property_simulation_domain::ResourceKey::for_account(
                period_input.rent_account
            ))
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
        .remove(&property_simulation_domain::ResourceKey::for_account(
            period_input.cash_account,
        ));
    assert!(validate_composition(&[incomplete], &[proposal]).is_err());
}

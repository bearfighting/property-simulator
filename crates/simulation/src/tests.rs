use super::*;
use property_simulation_accounting::ChartOfAccounts;
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, Currency, EffectId, EventId, LedgerTransactionId,
    Money, SliceId, TrialId, YearMonth,
};
use property_simulation_models::{
    RandomSource, RepairCostDistribution, RepairDistribution, RepairSample, RepairSampler,
    SamplingError, UtilityUsageDistribution, UtilityUsageSampler,
};

use crate::proposals::identify_proposal_for;

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

    fn derive_payment(
        &self,
        loan: property_simulation_models::FixedRateLoan,
    ) -> Result<Money, property_simulation_models::LoanModelError> {
        loan.derive_payment()
    }
}
struct TestIds;
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
impl OutputIdFactory for TestIds {
    fn event_id(&self, p: YearMonth, i: ActivityInstanceId, k: &str) -> EventId {
        EventId::from_bytes(output_bytes(1, p, i, k))
    }
    fn effect_id(&self, p: YearMonth, i: ActivityInstanceId, k: &str) -> EffectId {
        EffectId::from_bytes(output_bytes(2, p, i, k))
    }
    fn transaction_id(&self, p: YearMonth, i: ActivityInstanceId, k: &str) -> LedgerTransactionId {
        LedgerTransactionId::from_bytes(output_bytes(3, p, i, k))
    }
}
fn identity() -> &'static TestIds {
    static IDS: TestIds = TestIds;
    &IDS
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

struct PermissiveRepairSampler;
impl property_simulation_models::Sampler<RepairDistribution> for PermissiveRepairSampler {
    fn sample<R: RandomSource>(
        &mut self,
        _distribution: &RepairDistribution,
        _random: &mut R,
    ) -> Result<RepairSample, SamplingError<R::Error>> {
        Ok(RepairSample::NoRepair)
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
    assert_eq!(sample.period, YearMonth::new(2025, 1).unwrap());
    assert_eq!(sample.value, RepairSample::NoRepair);
    let wrong_purpose = sample.clone().with_purpose(Some("repair.cost".into()));
    assert!(
        propose_slice(
            &crate::RepairSlice {
                repair_validation: &crate::REPAIR_COST_RULES,
                instance: instance.clone()
            },
            &validated_input(
                &OpeningState {
                    cash: money("0"),
                    condition: AssetCondition::NeedsRepair
                },
                &input(),
            ),
            &wrong_purpose,
        )
        .is_err()
    );

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

#[test]
fn repair_sampling_rejects_invalid_distribution_before_custom_sampler() {
    let instance = activity(4);
    let distribution = RepairDistribution {
        occurrence_numerator: 1,
        occurrence_denominator: 0,
        cost: RepairCostDistribution {
            maximum: money("1.00"),
        },
    };
    let mut sampler = PermissiveRepairSampler;
    let mut random = FixedWords(vec![]);

    assert_eq!(
        sample_repair(
            &instance,
            YearMonth::new(2025, 1).unwrap(),
            &distribution,
            &mut sampler,
            &mut random,
        ),
        Err(SamplingError::InvalidDistribution)
    );
}

#[test]
fn repair_sampling_fixed_seed_smoke_check_matches_distribution_support() {
    use property_simulation_randomness::{RootKey, RunKeyFactory};
    let instance = activity(45);
    let period = input().period;
    let distribution = RepairDistribution {
        occurrence_numerator: 1,
        occurrence_denominator: 2,
        cost: RepairCostDistribution {
            maximum: money("1.00"),
        },
    };
    let mut occurrences = 0;
    let mut total_cents = 0_i128;
    const TRIALS: i128 = 512;
    for seed in 0_u16..TRIALS as u16 {
        let mut root = [0; 32];
        root[..2].copy_from_slice(&seed.to_le_bytes());
        let trial = TrialId::from_bytes([seed as u8; 16]);
        let streams = RunKeyFactory::new(RootKey::from_seed(root)).run_streams(trial);
        let mut random = semantic_random(
            &streams,
            trial,
            &instance,
            period,
            property_simulation_randomness::RandomCategory::Property,
            REPAIR_SAMPLE_PURPOSE,
        );
        let sample = sample_repair(
            &instance,
            period,
            &distribution,
            &mut RepairSampler,
            &mut random,
        )
        .unwrap();
        if let RepairSample::Repair { cost } = sample.value {
            occurrences += 1;
            assert!(cost.checked_cmp(money("0")).unwrap().is_ge());
            assert!(cost.checked_cmp(money("1.00")).unwrap().is_le());
            total_cents += cost.coefficient();
        }
    }
    assert!(
        (220..=292).contains(&occurrences),
        "occurrences={occurrences}"
    );
    let mean_cents = total_cents / i128::from(occurrences);
    assert!((40..=60).contains(&mean_cents), "mean cents={mean_cents}");
}

#[test]
fn semantic_repair_sample_flows_through_slice_coordinator_and_ledger() {
    use crate::{RepairSlice, coordinate_month, propose_slice, sample_repair};
    use property_simulation_randomness::{RootKey, RunKeyFactory};

    let instance = activity(44);
    let period = input().period;
    let trial = TrialId::from_bytes([61; 16]);
    let streams = RunKeyFactory::new(RootKey::from_seed([8; 32])).run_streams(trial);
    let distribution = RepairDistribution {
        occurrence_numerator: 1,
        occurrence_denominator: 1,
        cost: RepairCostDistribution {
            maximum: money("50"),
        },
    };
    let mut random = semantic_random(
        &streams,
        trial,
        &instance,
        period,
        property_simulation_randomness::RandomCategory::Property,
        REPAIR_SAMPLE_PURPOSE,
    );
    let sampled = sample_repair(
        &instance,
        period,
        &distribution,
        &mut RepairSampler,
        &mut random,
    )
    .unwrap();
    assert_eq!(sampled.purpose.as_deref(), Some("repair.sample"));
    assert_eq!(sampled, {
        let mut random = semantic_random(
            &streams,
            trial,
            &instance,
            period,
            property_simulation_randomness::RandomCategory::Property,
            REPAIR_SAMPLE_PURPOSE,
        );
        sample_repair(
            &instance,
            period,
            &distribution,
            &mut RepairSampler,
            &mut random,
        )
        .unwrap()
    });

    let state = OpeningState {
        cash: money("100"),
        condition: AssetCondition::NeedsRepair,
    };
    let ledger = ledger_for_cash(state.cash);
    let validated = validated_input(&state, &input());
    let output = propose_slice(
        &RepairSlice {
            repair_validation: &crate::REPAIR_COST_RULES,
            instance: instance.clone(),
        },
        &validated,
        &sampled,
    )
    .unwrap();
    let (result, posted) = coordinate_month(
        &validated,
        &ledger,
        vec![output],
        trial,
        BranchId::from_bytes([62; 16]),
        &TestIds,
    )
    .unwrap();
    assert_eq!(
        result.samples()[0].purpose.as_deref(),
        Some("repair.sample")
    );
    assert_eq!(result.cash_flow_contributions().len(), 1);
    assert_eq!(
        result.cash_flow_totals().unwrap().operating,
        result.closing_state().cash.checked_sub(state.cash).unwrap()
    );
    assert_eq!(posted.transactions().count(), 1);
    let projection = posted
        .project(period, period, &[input().cash_account])
        .unwrap();
    assert_eq!(
        projection.cash_flow_statement.totals.operating,
        result.cash_flow_totals().unwrap().operating
    );
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
            purpose: REPAIR_SAMPLE_PURPOSE.into(),
        }),
        Err(RandomnessError::TrialIdentityMismatch)
    ));
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
        configuration: match sample_kind {
            ActivitySampleKind::Rental => ActivityConfiguration::Rental,
            ActivitySampleKind::Repair => ActivityConfiguration::Repair,
            ActivitySampleKind::Loan => ActivityConfiguration::Loan,
            ActivitySampleKind::PropertyTax => {
                panic!("property tax uses its explicit configuration")
            }
            ActivitySampleKind::Utility => panic!("utility uses its explicit configuration"),
        },
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
        &crate::REPAIR_COST_RULES,
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
    ActivitySample::new(instance, input().period, value)
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
    let mut instance = activity_with_kind(12, ActivitySampleKind::Rental);
    instance.configuration = ActivityConfiguration::PropertyTax(config);
    let state = OpeningState {
        cash: money("500"),
        condition: AssetCondition::Available,
    };
    let mut frozen = input();
    frozen.period = YearMonth::new(2025, 4).unwrap();
    let ledger = ledger_for_cash(state.cash);
    let mut validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
    let mut instance = activity_with_kind(26, ActivitySampleKind::Rental);
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
        let mut validated = validate_input(
            &frozen,
            &RepairCostDistribution {
                maximum: frozen.repair_cost_limit,
            },
            &crate::REPAIR_COST_RULES,
            &ledger,
            &state,
        )
        .unwrap();
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
    let mut instance = activity_with_kind(13, ActivitySampleKind::Rental);
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
    let mut validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
    let mut december_input = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
    let mut exact_zero_input = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
    let mut validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
    let rent = rental_activity(20);
    let repair = activity(25);
    let mut tax = activity_with_kind(21, ActivitySampleKind::Rental);
    tax.configuration = ActivityConfiguration::PropertyTax(PropertyTaxConfig {
        amount: money("100"),
        cadence: PropertyTaxCadence::Quarterly,
        first_due: frozen.period,
        cash_account: frozen.cash_account,
        expense_account: frozen.repair_expense_account,
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
        &RepairSlice {
            repair_validation: &crate::REPAIR_COST_RULES,
            instance: repair.clone(),
        },
        &validated,
        &ActivitySample::new(
            &repair,
            frozen.period,
            RepairSample::Repair { cost: money("10") },
        ),
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
    let validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
    for config in [
        PropertyTaxConfig {
            amount: Money::parse("10", Currency::from_code("USD").unwrap()).unwrap(),
            cadence: PropertyTaxCadence::Monthly,
            first_due: frozen.period,
            cash_account: frozen.cash_account,
            expense_account: frozen.repair_expense_account,
        },
        PropertyTaxConfig {
            amount: money("10"),
            cadence: PropertyTaxCadence::Monthly,
            first_due: frozen.period,
            cash_account: frozen.rent_account,
            expense_account: frozen.repair_expense_account,
        },
    ] {
        let mut instance = activity_with_kind(22, ActivitySampleKind::Rental);
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
    let validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
    let mut instance = activity_with_kind(27, ActivitySampleKind::Rental);
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
    let mut first = activity_with_kind(23, ActivitySampleKind::Rental);
    let mut second = activity_with_kind(24, ActivitySampleKind::Rental);
    let large =
        Money::from_parts(i128::MAX / 4 * 3, 2, Currency::from_code("CAD").unwrap()).unwrap();
    let config = |amount| PropertyTaxConfig {
        amount,
        cadence: PropertyTaxCadence::Quarterly,
        first_due: input().period,
        cash_account: input().cash_account,
        expense_account: input().repair_expense_account,
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
    let mut instance = activity_with_kind(id, ActivitySampleKind::Rental);
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
    let mut validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
    let mut validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
    let instances = [
        utility_activity(62, 100, "2.50"),
        utility_activity(63, 100, "0.001"),
    ];
    validated
        .validate_activities_before_sampling(&instances, &ledger, [])
        .unwrap();
    for (instance, usage) in instances.into_iter().zip([0, 3]) {
        let slice = UtilitySlice { instance };
        let sample = ActivitySample::new(&slice.instance, frozen.period, usage);
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
    let mut validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
    let distribution_for_repairs = RepairCostDistribution {
        maximum: frozen.repair_cost_limit,
    };
    let instance = utility_activity(66, 100, "1");
    let wide_instance = utility_activity(73, u64::MAX, "1");
    let other_instance = utility_activity(72, u64::MAX, "1");
    let all_activities = [
        instance.clone(),
        wide_instance.clone(),
        other_instance.clone(),
    ];
    let mut validated = validate_input(
        &frozen,
        &distribution_for_repairs,
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
    let mut validated_february = validate_input(
        &february,
        &distribution_for_repairs,
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
    let mut validated = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
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
        &crate::REPAIR_COST_RULES,
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
            repair_validation: &crate::REPAIR_COST_RULES,
            instance: repair.clone(),
        },
        &validated,
        &sample(&repair, RepairSample::Repair { cost: money("10") }),
    )
    .unwrap();

    let mut changed_frozen = frozen;
    changed_frozen.rent = money("90");
    let changed_input = validate_input(
        &changed_frozen,
        &RepairCostDistribution {
            maximum: changed_frozen.repair_cost_limit,
        },
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
    assert!(
        coordinate_month(
            &changed_input,
            &ledger,
            vec![repair_output.clone(), rental_output.clone()],
            TrialId::from_bytes([6; 16]),
            BranchId::from_bytes([7; 16]),
            &TestIds,
        )
        .is_err()
    );
    assert_eq!(ledger.transactions().count(), 0);
    assert_eq!(ledger.transactions().count(), 0);
    let forward_outputs = vec![repair_output.clone(), rental_output.clone()];
    let (result, closing_ledger) = coordinate_month(
        &validated,
        &ledger,
        forward_outputs.clone(),
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
fn independent_repair_instances_superpose_in_any_order() {
    use crate::{RepairSlice, coordinate_month, propose_slice};

    let frozen = input();
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
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
    let first = activity(31);
    let second = activity(32);
    let first_value = RepairSample::Repair { cost: money("10") };
    let second_value = RepairSample::Repair { cost: money("20") };
    let first_output = propose_slice(
        &RepairSlice {
            repair_validation: &crate::REPAIR_COST_RULES,
            instance: first.clone(),
        },
        &validated,
        &sample(&first, first_value),
    )
    .unwrap();
    let second_output = propose_slice(
        &RepairSlice {
            repair_validation: &crate::REPAIR_COST_RULES,
            instance: second.clone(),
        },
        &validated,
        &sample(&second, second_value),
    )
    .unwrap();

    let (forward_result, forward_ledger) = coordinate_month(
        &validated,
        &ledger,
        vec![first_output.clone(), second_output.clone()],
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        identity(),
    )
    .unwrap();
    let (reverse_result, reverse_ledger) = coordinate_month(
        &validated,
        &ledger,
        vec![second_output, first_output],
        TrialId::from_bytes([6; 16]),
        BranchId::from_bytes([7; 16]),
        identity(),
    )
    .unwrap();

    assert_eq!(forward_result, reverse_result);
    assert_eq!(forward_ledger, reverse_ledger);
    assert_eq!(forward_result.closing_state().cash, money("70"));
    assert_eq!(forward_result.proposal().effects().len(), 2);
    assert_eq!(forward_result.proposal().transactions().len(), 2);
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
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
    let instance = activity(21);
    let slice = RepairSlice {
        repair_validation: &crate::REPAIR_COST_RULES,
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
    use crate::{RepairSlice, SliceOutput, coordinate_month};

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
        &crate::REPAIR_COST_RULES,
        &ledger,
        &state,
    )
    .unwrap();
    let instance = activity(41);
    let slice = RepairSlice {
        repair_validation: &crate::REPAIR_COST_RULES,
        instance: instance.clone(),
    };
    let wrong_source = ActivitySample::new(
        &activity(42),
        frozen.period,
        ActivitySampleValue::Repair(RepairSample::NoRepair),
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
        type Sample = RepairSample;
        const SAMPLE_KIND: ActivitySampleKind = ActivitySampleKind::Repair;
        fn instance(&self) -> &ActivityInstance {
            &self.0
        }
        fn resources(&self, input: &FrozenInput) -> SliceResources {
            RepairSlice {
                repair_validation: &crate::REPAIR_COST_RULES,
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
        &crate::REPAIR_COST_RULES,
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
    let validated = validate_input(
        &input(),
        &support,
        &crate::REPAIR_COST_RULES,
        &ledger_for_cash(state.cash),
        &state,
    )
    .unwrap();
    assert_eq!(validated.opening_state(), &state);
    assert_eq!(validated.frozen_input(), &input());
    assert_eq!(validated.repair_distribution(), &support);
    assert!(
        validate_input(
            &input(),
            &RepairCostDistribution {
                maximum: money("49"),
            },
            &crate::REPAIR_COST_RULES,
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
        &crate::REPAIR_COST_RULES,
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
        &crate::REPAIR_COST_RULES,
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
fn active_cashflow_support_includes_context_bound_contributions() {
    let frozen = input();
    let currency = frozen.rent.currency();
    let maximum = Money::from_parts(i128::MAX, 0, currency).unwrap();
    let loan = ActivityInstance {
        id: ActivityInstanceId::from_bytes([61; 16]),
        slice_id: SliceId::from_bytes([62; 16]),
        configuration: ActivityConfiguration::Loan,
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

    let errors = validate_input(
        &frozen,
        &RepairCostDistribution {
            maximum: money("50"),
        },
        &crate::REPAIR_COST_RULES,
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
        &crate::REPAIR_COST_RULES,
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
        &crate::REPAIR_COST_RULES,
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
        &crate::REPAIR_COST_RULES,
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
    frozen.repair_cost_limit = money("50.005");
    let support = RepairCostDistribution {
        maximum: frozen.repair_cost_limit,
    };
    assert!(
        validate_input(
            &frozen,
            &support,
            &crate::REPAIR_COST_RULES,
            &ledger_for_cash(state.cash),
            &state
        )
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

    let repair = RepairSlice {
        repair_validation: &crate::REPAIR_COST_RULES,
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
        repair_validation: &crate::REPAIR_COST_RULES,
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
        repair_validation: &crate::REPAIR_COST_RULES,
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
        repair_validation: &crate::REPAIR_COST_RULES,
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
    let factory = identity();
    let period = YearMonth::new(2025, 1).unwrap();
    let instance = activity(4).id;
    let id = factory.transaction_id(period, instance, "rent");
    assert_eq!(id, factory.transaction_id(period, instance, "rent"));
    assert_ne!(id, factory.transaction_id(period, instance, "repair"));
    assert_ne!(
        id,
        factory.transaction_id(period.checked_add_months(1).unwrap(), instance, "rent")
    );
    assert_ne!(id, factory.transaction_id(period, activity(5).id, "rent"));
    assert_ne!(
        factory.event_id(period, instance, "rent").as_bytes(),
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
    let sample = ActivitySample {
        instance_id: activity(1).id,
        slice_id: activity(1).slice_id,
        period,
        purpose: None,
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
    let wrong_branch_proposal =
        identify_proposal_for(SliceProposal::default(), identity(), &activity(1), period).unwrap();
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

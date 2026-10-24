use std::num::NonZeroU32;

use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_money::{Currency, Money};
use property_simulation_month::Month;
use property_simulation_random_engine::RandomInput;
use property_simulation_slice::{RepairCostSlice, RepairOccurrenceSlice, StochasticSlice};
use property_simulation_slice_catalogue::{
    Effect, Event, RepairCostParameters, RepairCostSliceInput, RepairOccurrenceSliceInput,
    SliceKind, SliceResult, SliceSetting,
};

struct Words(Vec<u64>);

impl RandomInput for Words {
    fn next_u64(&mut self) -> u64 {
        self.0.remove(0)
    }
}

fn cad() -> Currency {
    Currency::from_code("CAD").unwrap()
}

fn money(value: &str) -> Money {
    Money::parse(value, cad()).unwrap()
}

fn account(value: u32) -> AccountCode {
    AccountCode::new(value).unwrap()
}

fn bounded_word(value: u64, upper_bound: u64) -> u64 {
    let threshold = upper_bound.wrapping_neg() % upper_bound;
    let first_accepted = threshold + (upper_bound - threshold % upper_bound) % upper_bound;
    first_accepted + value
}

fn result(input: &mut Words, age_months: u64) -> SliceResult {
    RepairOccurrenceSlice::new(RepairOccurrenceSliceInput {
        month: Month::new(age_months),
        months_since_repair_cost: age_months,
    })
    .simulate(input)
    .unwrap()
}

#[test]
fn repair_occurrence_uses_the_documented_monthly_probability_curve() {
    for (age, numerator) in [(0, 25), (1, 27), (60, 87), (u64::MAX, 149)] {
        let occurred = result(&mut Words(vec![bounded_word(0, 10_000)]), age);
        assert_eq!(occurred.kind(), SliceKind::RepairOccurrence);
        assert_eq!(occurred.events(), &[Event::RepairOccurred]);

        let not_occurred = result(&mut Words(vec![bounded_word(numerator, 10_000)]), age);
        assert!(not_occurred.events().is_empty());
    }
}

#[test]
fn repair_cost_samples_inclusive_minor_unit_range_and_settles_zero_once() {
    let parameters = RepairCostParameters::new(money("1.23"), NonZeroU32::new(2).unwrap()).unwrap();
    let input = RepairCostSliceInput {
        month: Month::new(2),
        occurrence_month: Month::new(0),
        cash_account: account(100),
        expense_account: account(500),
    };

    let zero = RepairCostSlice::new(parameters, input)
        .simulate(&mut Words(vec![bounded_word(0, 124)]))
        .unwrap();
    assert_eq!(
        zero.events(),
        &[Event::RepairCostSettled { amount: money("0") }]
    );
    assert_eq!(
        zero.effects(),
        &[Effect::RepairCostBaselineUpdated {
            month: Month::new(2)
        }]
    );
    assert!(zero.journal_entries().is_empty());

    let maximum = RepairCostSlice::new(parameters, input)
        .simulate(&mut Words(vec![bounded_word(123, 124)]))
        .unwrap();
    assert_eq!(
        maximum.events(),
        &[Event::RepairCostSettled {
            amount: money("1.23")
        }]
    );
    let entry = &maximum.journal_entries()[0];
    assert_eq!(entry.month(), Month::new(2));
    assert_eq!(entry.lines()[0].side(), DebitCredit::Debit);
    assert_eq!(entry.lines()[0].account(), account(500));
    assert_eq!(entry.lines()[1].side(), DebitCredit::Credit);
    assert_eq!(entry.lines()[1].account(), account(100));
}

#[test]
fn repair_cost_rejects_a_settlement_month_that_does_not_match_its_delay() {
    let parameters = RepairCostParameters::new(money("10"), NonZeroU32::new(2).unwrap()).unwrap();
    let slice = RepairCostSlice::new(
        parameters,
        RepairCostSliceInput {
            month: Month::new(1),
            occurrence_month: Month::new(0),
            cash_account: account(100),
            expense_account: account(500),
        },
    );

    assert!(slice.simulate(&mut Words(vec![])).is_err());
}

#[test]
fn repair_occurrence_setting_is_enabled_without_slice_specific_parameters() {
    assert_eq!(
        SliceSetting::RepairOccurrence { enabled: true }.kind(),
        SliceKind::RepairOccurrence
    );
}

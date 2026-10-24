use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_money::{Currency, Money};
use property_simulation_month::Month;
use property_simulation_slice::{DeterministicSlice, FixedExpenseSlice, RentalSlice};
use property_simulation_slice_catalogue::{
    Event, FixedExpenseMethod, FixedExpenseParameters, FixedExpenseSliceInput, RentalParameters,
    RentalSliceInput, SliceKind,
};
use rust_decimal::Decimal;

fn code(value: u32) -> AccountCode {
    AccountCode::new(value).unwrap()
}

fn money(value: &str) -> Money {
    Money::parse(value, Currency::from_code("CAD").unwrap()).unwrap()
}

#[test]
fn rental_rounds_half_up_and_posts_income_against_cash() {
    let slice = RentalSlice::new(
        RentalParameters::new(money("10.005")).unwrap(),
        RentalSliceInput {
            month: Month::new(2),
            cash_account: code(100),
            rental_income_account: code(400),
        },
    );

    let result = slice.calculate().unwrap();

    assert_eq!(result.kind(), SliceKind::Rental);
    assert_eq!(
        result.events(),
        &[Event::RentCollected {
            amount: money("10.01")
        }]
    );
    let entry = &result.journal_entries()[0];
    assert_eq!(entry.month(), Month::new(2));
    assert_eq!(entry.lines()[0].side(), DebitCredit::Debit);
    assert_eq!(entry.lines()[0].account(), code(100));
    assert_eq!(entry.lines()[1].side(), DebitCredit::Credit);
    assert_eq!(entry.lines()[1].account(), code(400));
}

#[test]
fn fixed_expense_supports_monthly_amount_and_annual_property_rate() {
    let monthly = FixedExpenseSlice::new(
        FixedExpenseParameters::new(FixedExpenseMethod::MonthlyAmount(money("10.005"))).unwrap(),
        FixedExpenseSliceInput {
            month: Month::new(0),
            property_value: money("300000"),
            cash_account: code(100),
            expense_account: code(500),
        },
    );
    let annual_rate = FixedExpenseSlice::new(
        FixedExpenseParameters::new(FixedExpenseMethod::AnnualPropertyValueRate(Decimal::new(
            12, 3,
        )))
        .unwrap(),
        FixedExpenseSliceInput {
            month: Month::new(0),
            property_value: money("300000"),
            cash_account: code(100),
            expense_account: code(500),
        },
    );

    let monthly_result = monthly.calculate().unwrap();
    let annual_result = annual_rate.calculate().unwrap();

    assert_eq!(
        monthly_result.events(),
        &[Event::FixedExpensePaid {
            amount: money("10.01")
        }]
    );
    assert_eq!(
        annual_result.events(),
        &[Event::FixedExpensePaid {
            amount: money("300")
        }]
    );
    let entry = &annual_result.journal_entries()[0];
    assert_eq!(entry.lines()[0].side(), DebitCredit::Debit);
    assert_eq!(entry.lines()[0].account(), code(500));
    assert_eq!(entry.lines()[1].side(), DebitCredit::Credit);
    assert_eq!(entry.lines()[1].account(), code(100));
}

use std::num::NonZeroU32;

use property_simulation_domain_calculations::{calculate_installment, calculate_scheduled_payment};
use property_simulation_money::{Currency, Money};
use rust_decimal::Decimal;

fn money(value: &str) -> Money {
    Money::parse(value, Currency::from_code("CAD").unwrap()).unwrap()
}

#[test]
fn one_month_loan_vector_pays_interest_then_clears_principal() {
    let principal = money("100");
    let term = NonZeroU32::new(1).unwrap();
    let annual_rate = Decimal::new(12, 2);

    let scheduled = calculate_scheduled_payment(principal, annual_rate, term).unwrap();
    let installment = calculate_installment(principal, annual_rate, scheduled, term).unwrap();

    assert_eq!(scheduled, money("101.00"));
    assert_eq!(installment.payment(), money("101.00"));
    assert_eq!(installment.interest(), money("1.00"));
    assert_eq!(installment.principal(), money("100.00"));
    assert_eq!(installment.closing_balance(), money("0"));
}

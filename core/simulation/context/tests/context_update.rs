use std::num::NonZeroU32;

use property_simulation_context::{
    Context, InitialValues, SimulationName, StopCondition, UserSettings,
};
use property_simulation_money::{Currency, Money};
use property_simulation_random_engine::RandomRootKey;
use property_simulation_slice_catalogue::{LoanParameters, SliceSettings};
use rust_decimal::Decimal;

fn settings(currency: Currency, property_value: &str, stop_after: u32) -> UserSettings {
    UserSettings::new(
        InitialValues::new(
            currency,
            Money::parse(property_value, currency).unwrap(),
            Money::parse("100", currency).unwrap(),
            LoanParameters::new(Decimal::ZERO, NonZeroU32::new(360).unwrap()).unwrap(),
        )
        .unwrap(),
        vec![StopCondition::AfterMonths(
            NonZeroU32::new(stop_after).unwrap(),
        )],
        SliceSettings::default(),
    )
}

#[test]
fn latest_settings_replace_user_settings_but_preserve_initial_values_and_root() {
    let currency = Currency::from_code("CAD").unwrap();
    let initial = settings(currency, "300000", 12);
    let root = RandomRootKey::from_bytes([7; 32]);
    let context = Context::new(
        SimulationName::new("Household").unwrap(),
        root.clone(),
        initial.clone(),
    );
    let latest = settings(currency, "300000", 24);

    let updated = context.with_latest_settings(latest.clone()).unwrap();

    assert!(context.latest_settings().is_none());
    assert_eq!(context.effective_settings(), &initial);
    assert_eq!(updated.initial_settings(), &initial);
    assert_eq!(updated.latest_settings(), Some(&latest));
    assert_eq!(updated.effective_settings(), &latest);
    assert_eq!(updated.root_key().as_bytes(), root.as_bytes());
    assert_eq!(updated.simulation_name().as_str(), "Household");
}

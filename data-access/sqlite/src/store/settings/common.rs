use property_simulation_money::{Currency, Money};

use crate::SqliteDataAccessError;

pub(super) fn optional_payload(
    enabled: Option<bool>,
    configured: bool,
    name: &'static str,
) -> Result<(), SqliteDataAccessError> {
    match (enabled, configured) {
        (None, false) | (Some(_), true) => Ok(()),
        _ => Err(SqliteDataAccessError::InvalidData(name)),
    }
}

pub(super) fn bind_money(money: Money) -> (String, i64) {
    (money.coefficient().to_string(), i64::from(money.scale()))
}

pub(super) fn validate_currency(
    money: Money,
    expected: Currency,
) -> Result<(), SqliteDataAccessError> {
    if money.currency() == expected {
        Ok(())
    } else {
        Err(SqliteDataAccessError::from(
            property_simulation_money::MoneyError::CurrencyMismatch,
        ))
    }
}

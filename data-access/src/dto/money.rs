use property_simulation_money::{Currency, Money};

use crate::DtoError;

/// Exact fixed-point representation used for amounts in the Data Access contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoneyDto {
    pub coefficient: i128,
    pub scale: u8,
    pub currency: Currency,
}

impl MoneyDto {
    pub fn from_domain(money: Money) -> Self {
        Self {
            coefficient: money.coefficient(),
            scale: money.scale(),
            currency: money.currency(),
        }
    }

    pub fn to_domain(&self, currency: Currency) -> Result<Money, DtoError> {
        if self.currency != currency {
            return Err(DtoError::Money(
                property_simulation_money::MoneyError::CurrencyMismatch,
            ));
        }
        Ok(Money::from_parts(self.coefficient, self.scale, currency)?)
    }
}

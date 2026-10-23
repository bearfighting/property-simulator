use crate::MoneyError;
use iso_currency::Currency as IsoCurrency;
use std::fmt;

/// ISO 4217 currency metadata used by the domain. Formatting is deliberately not localized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Currency(IsoCurrency);

impl Currency {
    pub fn from_code(code: &str) -> Result<Self, MoneyError> {
        IsoCurrency::from_code(code)
            .map(Self)
            .ok_or_else(|| MoneyError::InvalidCurrency(code.to_owned()))
    }

    pub fn code(self) -> &'static str {
        self.0.code()
    }

    pub fn minor_unit_digits(self) -> u32 {
        self.0.exponent().unwrap_or(0) as u32
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

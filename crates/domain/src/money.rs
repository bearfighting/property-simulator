use crate::DomainError;
use iso_currency::Currency as IsoCurrency;
use rust_decimal::Decimal;
use std::fmt;

/// ISO 4217 currency metadata used by the domain. Formatting is deliberately not localized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Currency(IsoCurrency);

impl Currency {
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        IsoCurrency::from_code(code)
            .map(Self)
            .ok_or_else(|| DomainError::InvalidCurrency(code.to_owned()))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingPolicy {
    HalfUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    pub const fn new(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }
    pub const fn amount(self) -> Decimal {
        self.amount
    }
    pub const fn currency(self) -> Currency {
        self.currency
    }

    pub fn checked_add(self, other: Self) -> Result<Self, DomainError> {
        self.ensure_currency(other)?;
        self.amount
            .checked_add(other.amount)
            .map(|amount| Self::new(amount, self.currency))
            .ok_or(DomainError::ArithmeticOverflow)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, DomainError> {
        self.ensure_currency(other)?;
        self.amount
            .checked_sub(other.amount)
            .map(|amount| Self::new(amount, self.currency))
            .ok_or(DomainError::ArithmeticOverflow)
    }

    pub fn checked_cmp(self, other: Self) -> Result<std::cmp::Ordering, DomainError> {
        self.ensure_currency(other)?;
        self.amount
            .partial_cmp(&other.amount)
            .ok_or(DomainError::ArithmeticOverflow)
    }

    pub fn rounded_for_posting(self, _policy: RoundingPolicy) -> Result<Self, DomainError> {
        if self.amount.is_sign_negative() {
            return Err(DomainError::NegativePostingAmount);
        }
        let amount = self.amount.round_dp_with_strategy(
            self.currency.minor_unit_digits(),
            rust_decimal::RoundingStrategy::MidpointAwayFromZero,
        );
        Ok(Self::new(amount, self.currency))
    }

    fn ensure_currency(self, other: Self) -> Result<(), DomainError> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(DomainError::CurrencyMismatch)
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount.normalize(), self.currency.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn currency_safe_arithmetic_and_posting_rounding() {
        let cad = Currency::from_code("CAD").unwrap();
        let jpy = Currency::from_code("JPY").unwrap();
        let amount = Money::new(Decimal::from_str("0.005").unwrap(), cad);
        assert_eq!(
            amount
                .rounded_for_posting(RoundingPolicy::HalfUp)
                .unwrap()
                .amount(),
            Decimal::from_str("0.01").unwrap()
        );
        assert_eq!(
            Money::new(Decimal::from_str("0.0049").unwrap(), cad)
                .rounded_for_posting(RoundingPolicy::HalfUp)
                .unwrap()
                .amount(),
            Decimal::ZERO
        );
        assert_eq!(
            Money::new(Decimal::from_str("1.5").unwrap(), jpy)
                .rounded_for_posting(RoundingPolicy::HalfUp)
                .unwrap()
                .amount(),
            Decimal::from(2)
        );
        assert_eq!(
            amount
                .checked_add(Money::new(Decimal::ONE, cad))
                .unwrap()
                .amount(),
            Decimal::from_str("1.005").unwrap()
        );
        assert_eq!(
            amount.checked_add(Money::new(Decimal::ONE, jpy)),
            Err(DomainError::CurrencyMismatch)
        );
        assert!(
            Money::new(Decimal::NEGATIVE_ONE, cad)
                .checked_sub(Money::new(Decimal::ONE, cad))
                .is_ok()
        );
        assert_eq!(
            Money::new(Decimal::NEGATIVE_ONE, cad).rounded_for_posting(RoundingPolicy::HalfUp),
            Err(DomainError::NegativePostingAmount)
        );
        assert_eq!(
            Money::new(Decimal::MAX, cad).checked_add(Money::new(Decimal::ONE, cad)),
            Err(DomainError::ArithmeticOverflow)
        );
    }
}

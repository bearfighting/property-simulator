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

/// Fixed-point amount backed by a checked signed 128-bit coefficient.
/// Scale is capped at 28 decimal places; public arithmetic is checked and never wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Money {
    coefficient: i128,
    scale: u8,
    currency: Currency,
}

impl Money {
    pub const MAX_SCALE: u8 = 28;

    /// Constructor for decimal literals and existing call sites. Decimal's maximum scale fits.
    pub fn new(amount: Decimal, currency: Currency) -> Self {
        let (coefficient, scale) = normalize(amount.mantissa(), amount.scale() as u8);
        Self {
            coefficient,
            scale,
            currency,
        }
    }

    pub fn from_parts(
        coefficient: i128,
        scale: u8,
        currency: Currency,
    ) -> Result<Self, DomainError> {
        if scale > Self::MAX_SCALE {
            return Err(DomainError::InvalidMoneyScale(scale));
        }
        let (coefficient, scale) = normalize(coefficient, scale);
        Ok(Self {
            coefficient,
            scale,
            currency,
        })
    }

    pub fn parse(value: &str, currency: Currency) -> Result<Self, DomainError> {
        let (negative, unsigned) = value
            .strip_prefix('-')
            .map_or((false, value), |v| (true, v));
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        if fraction.len() > Self::MAX_SCALE as usize {
            return Err(DomainError::InvalidMoneyScale(
                fraction.len().min(u8::MAX as usize) as u8,
            ));
        }
        if whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || (unsigned.contains('.') && fraction.is_empty())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(DomainError::InvalidMoneyAmount);
        }
        let mut digits = format!("{whole}{fraction}");
        let mut scale = fraction.len() as u8;
        while scale > 0 && digits.ends_with('0') {
            digits.pop();
            scale -= 1;
        }
        if digits.is_empty() {
            digits.push('0');
        }
        let magnitude = digits
            .parse::<u128>()
            .map_err(|_| DomainError::MoneyOutOfRange)?;
        let coefficient = if negative {
            if magnitude == (i128::MAX as u128) + 1 {
                i128::MIN
            } else if magnitude <= i128::MAX as u128 {
                -(magnitude as i128)
            } else {
                return Err(DomainError::MoneyOutOfRange);
            }
        } else {
            i128::try_from(magnitude).map_err(|_| DomainError::MoneyOutOfRange)?
        };
        Self::from_parts(coefficient, scale, currency)
    }

    pub const fn coefficient(self) -> i128 {
        self.coefficient
    }
    pub const fn scale(self) -> u8 {
        self.scale
    }
    pub const fn currency(self) -> Currency {
        self.currency
    }
    pub const fn is_zero(self) -> bool {
        self.coefficient == 0
    }
    pub const fn is_negative(self) -> bool {
        self.coefficient < 0
    }

    pub fn checked_neg(self) -> Result<Self, DomainError> {
        Ok(Self {
            coefficient: self
                .coefficient
                .checked_neg()
                .ok_or(DomainError::ArithmeticOverflow)?,
            ..self
        })
    }
    pub fn abs(self) -> Result<Self, DomainError> {
        if self.is_negative() {
            self.checked_neg()
        } else {
            Ok(self)
        }
    }
    pub fn checked_add(self, other: Self) -> Result<Self, DomainError> {
        self.ensure_currency(other)?;
        let scale = self.scale.max(other.scale);
        let left = scale_coeff(self.coefficient, self.scale, scale)?;
        let right = scale_coeff(other.coefficient, other.scale, scale)?;
        Self::from_parts(
            left.checked_add(right)
                .ok_or(DomainError::ArithmeticOverflow)?,
            scale,
            self.currency,
        )
    }
    pub fn checked_sub(self, other: Self) -> Result<Self, DomainError> {
        self.ensure_currency(other)?;
        let scale = self.scale.max(other.scale);
        let left = scale_coeff(self.coefficient, self.scale, scale)?;
        let right = scale_coeff(other.coefficient, other.scale, scale)?;
        Self::from_parts(
            left.checked_sub(right)
                .ok_or(DomainError::ArithmeticOverflow)?,
            scale,
            self.currency,
        )
    }
    pub fn checked_cmp(self, other: Self) -> Result<std::cmp::Ordering, DomainError> {
        self.ensure_currency(other)?;
        let self_sign = self.coefficient.signum();
        let other_sign = other.coefficient.signum();
        if self_sign != other_sign {
            return Ok(self_sign.cmp(&other_sign));
        }
        if self_sign == 0 {
            return Ok(std::cmp::Ordering::Equal);
        }
        let magnitude_order = compare_magnitude(
            self.coefficient.unsigned_abs(),
            self.scale,
            other.coefficient.unsigned_abs(),
            other.scale,
        );
        Ok(if self_sign < 0 {
            magnitude_order.reverse()
        } else {
            magnitude_order
        })
    }

    pub fn rounded_for_posting(self, _policy: RoundingPolicy) -> Result<Self, DomainError> {
        if self.is_negative() {
            return Err(DomainError::NegativePostingAmount);
        }
        let target = self
            .currency
            .minor_unit_digits()
            .min(u32::from(Self::MAX_SCALE)) as u8;
        if self.scale <= target {
            return Ok(self);
        }
        let divisor = pow10(self.scale - target)?;
        let quotient = self.coefficient / divisor;
        let remainder = self.coefficient % divisor;
        let rounded = if remainder
            .checked_mul(2)
            .ok_or(DomainError::ArithmeticOverflow)?
            >= divisor
        {
            quotient
                .checked_add(1)
                .ok_or(DomainError::ArithmeticOverflow)?
        } else {
            quotient
        };
        Self::from_parts(rounded, target, self.currency)
    }

    fn ensure_currency(self, other: Self) -> Result<(), DomainError> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(DomainError::CurrencyMismatch)
        }
    }
}

fn normalize(mut coefficient: i128, mut scale: u8) -> (i128, u8) {
    while scale > 0 && coefficient % 10 == 0 {
        coefficient /= 10;
        scale -= 1;
    }
    (coefficient, scale)
}
fn pow10(power: u8) -> Result<i128, DomainError> {
    10_i128
        .checked_pow(power.into())
        .ok_or(DomainError::ArithmeticOverflow)
}
fn scale_coeff(value: i128, from: u8, to: u8) -> Result<i128, DomainError> {
    value
        .checked_mul(pow10(to - from)?)
        .ok_or(DomainError::ArithmeticOverflow)
}

fn compare_magnitude(
    left: u128,
    left_scale: u8,
    right: u128,
    right_scale: u8,
) -> std::cmp::Ordering {
    let left_digits = left.to_string();
    let right_digits = right.to_string();
    let left_order = left_digits.len() as i16 - i16::from(left_scale);
    let right_order = right_digits.len() as i16 - i16::from(right_scale);
    match left_order.cmp(&right_order) {
        std::cmp::Ordering::Equal => {
            let width = left_digits.len().max(right_digits.len());
            let left_aligned = format!("{left_digits:0<width$}");
            let right_aligned = format!("{right_digits:0<width$}");
            left_aligned.cmp(&right_aligned)
        }
        order => order,
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale == 0 {
            return write!(f, "{} {}", self.coefficient, self.currency);
        }
        let negative = self.coefficient < 0;
        let digits = self.coefficient.unsigned_abs().to_string();
        let scale = self.scale as usize;
        let padded = if digits.len() <= scale {
            format!("{}{}", "0".repeat(scale + 1 - digits.len()), digits)
        } else {
            digits
        };
        let split = padded.len() - scale;
        write!(
            f,
            "{}{}.{} {}",
            if negative { "-" } else { "" },
            &padded[..split],
            &padded[split..],
            self.currency
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_fixed_point_arithmetic_rounding_and_large_values() {
        let cad = Currency::from_code("CAD").unwrap();
        let jpy = Currency::from_code("JPY").unwrap();
        let half_cent = Money::parse("0.005", cad).unwrap();
        assert_eq!(
            half_cent
                .rounded_for_posting(RoundingPolicy::HalfUp)
                .unwrap(),
            Money::parse("0.01", cad).unwrap()
        );
        assert_eq!(
            Money::parse("0.0049", cad)
                .unwrap()
                .rounded_for_posting(RoundingPolicy::HalfUp)
                .unwrap(),
            Money::from_parts(0, 0, cad).unwrap()
        );
        assert_eq!(
            Money::parse("1.5", jpy)
                .unwrap()
                .rounded_for_posting(RoundingPolicy::HalfUp)
                .unwrap(),
            Money::parse("2", jpy).unwrap()
        );
        assert_eq!(
            half_cent
                .checked_add(Money::parse("1", cad).unwrap())
                .unwrap(),
            Money::parse("1.005", cad).unwrap()
        );
        assert_eq!(
            half_cent.checked_add(Money::parse("1", jpy).unwrap()),
            Err(DomainError::CurrencyMismatch)
        );
        let huge = Money::parse("170141183460469231731687303715884105727", cad).unwrap();
        assert!(huge.checked_add(Money::parse("1", cad).unwrap()).is_err());
        assert!(Money::parse("10000000000000000000000000000000000000", cad).is_ok());
        let minimum = Money::from_parts(i128::MIN, 0, cad).unwrap();
        assert_eq!(
            minimum.checked_sub(minimum).unwrap(),
            Money::from_parts(0, 0, cad).unwrap()
        );
        assert_eq!(
            Money::parse("0.00000000000000000000000000001", cad),
            Err(DomainError::InvalidMoneyScale(29))
        );
        assert_eq!(
            Money::parse("1.", cad),
            Err(DomainError::InvalidMoneyAmount)
        );
        assert_eq!(
            Money::new(Decimal::new(1_000, 3), cad),
            Money::parse("1", cad).unwrap()
        );
        assert_eq!(
            Money::new(Decimal::new(1_000, 3), cad)
                .rounded_for_posting(RoundingPolicy::HalfUp)
                .unwrap(),
            Money::parse("1", cad).unwrap()
        );
        assert_eq!(
            Money::parse("-170141183460469231731687303715884105728", cad).unwrap(),
            minimum
        );
        assert_eq!(
            Money::parse("170141183460469231731687303715884105727.0", cad).unwrap(),
            huge
        );
        assert_eq!(
            Money::parse("-170141183460469231731687303715884105728.0", cad).unwrap(),
            minimum
        );
        assert_eq!(
            Money::parse("170141183460469231731687303715884105728", cad),
            Err(DomainError::MoneyOutOfRange)
        );
        assert_eq!(
            Money::parse("-170141183460469231731687303715884105729", cad),
            Err(DomainError::MoneyOutOfRange)
        );
        assert_eq!(
            huge.checked_cmp(Money::from_parts(i128::MAX, 1, cad).unwrap()),
            Ok(std::cmp::Ordering::Greater)
        );
    }
}

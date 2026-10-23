use crate::{
    Currency, MoneyError,
    utils::{compare_magnitude, normalize, pow10, scale_coeff},
};
use rust_decimal::Decimal;
use std::fmt;

/// Fixed-point amount backed by a checked signed 128-bit coefficient.
/// Scale is capped at 28 decimal places; public arithmetic is checked and never wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Money {
    coefficient: i128,
    scale: u8,
    currency: Currency,
}

// Construction and parsing keep the internal coefficient and scale valid.
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
    ) -> Result<Self, MoneyError> {
        if scale > Self::MAX_SCALE {
            return Err(MoneyError::InvalidMoneyScale(scale));
        }
        let (coefficient, scale) = normalize(coefficient, scale);
        Ok(Self {
            coefficient,
            scale,
            currency,
        })
    }

    pub fn parse(value: &str, currency: Currency) -> Result<Self, MoneyError> {
        let (negative, unsigned) = value
            .strip_prefix('-')
            .map_or((false, value), |v| (true, v));
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        if fraction.len() > Self::MAX_SCALE as usize {
            return Err(MoneyError::InvalidMoneyScale(
                fraction.len().min(u8::MAX as usize) as u8,
            ));
        }
        if whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || (unsigned.contains('.') && fraction.is_empty())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(MoneyError::InvalidMoneyAmount);
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
            .map_err(|_| MoneyError::MoneyOutOfRange)?;
        let coefficient = if negative {
            if magnitude == (i128::MAX as u128) + 1 {
                i128::MIN
            } else if magnitude <= i128::MAX as u128 {
                -(magnitude as i128)
            } else {
                return Err(MoneyError::MoneyOutOfRange);
            }
        } else {
            i128::try_from(magnitude).map_err(|_| MoneyError::MoneyOutOfRange)?
        };
        Self::from_parts(coefficient, scale, currency)
    }
}

// Read-only access to the normalized amount and its currency.
impl Money {
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
}

// Unary sign operations and the shared currency compatibility check.
impl Money {
    pub fn checked_neg(self) -> Result<Self, MoneyError> {
        Ok(Self {
            coefficient: self
                .coefficient
                .checked_neg()
                .ok_or(MoneyError::ArithmeticOverflow)?,
            ..self
        })
    }
    fn ensure_currency(self, other: Self) -> Result<(), MoneyError> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch)
        }
    }
}

// Checked arithmetic and comparison between amounts of the same currency.
impl Money {
    pub fn abs(self) -> Result<Self, MoneyError> {
        if self.is_negative() {
            self.checked_neg()
        } else {
            Ok(self)
        }
    }
    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.ensure_currency(other)?;
        let scale = self.scale.max(other.scale);
        let left = scale_coeff(self.coefficient, self.scale, scale)?;
        let right = scale_coeff(other.coefficient, other.scale, scale)?;
        Self::from_parts(
            left.checked_add(right)
                .ok_or(MoneyError::ArithmeticOverflow)?,
            scale,
            self.currency,
        )
    }
    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.ensure_currency(other)?;
        let scale = self.scale.max(other.scale);
        let left = scale_coeff(self.coefficient, self.scale, scale)?;
        let right = scale_coeff(other.coefficient, other.scale, scale)?;
        Self::from_parts(
            left.checked_sub(right)
                .ok_or(MoneyError::ArithmeticOverflow)?,
            scale,
            self.currency,
        )
    }

    /// Multiply a non-negative amount by an integer quantity without changing
    /// its decimal scale or currency.
    pub fn checked_mul_u64(self, multiplier: u64) -> Result<Self, MoneyError> {
        if self.is_negative() {
            return Err(MoneyError::NegativePostingAmount);
        }
        if self.is_zero() || multiplier == 0 {
            return Self::from_parts(0, 0, self.currency);
        }

        // Cancel factors from the numerator against the decimal denominator
        // before multiplying. This keeps exact products representable when
        // the unreduced coefficient product alone would overflow.
        let mut coefficient = self.coefficient;
        let mut multiplier = multiplier;
        let mut scale = self.scale;
        while scale > 0 {
            let has_two = coefficient % 2 == 0 || multiplier % 2 == 0;
            let has_five = coefficient % 5 == 0 || multiplier % 5 == 0;
            if !has_two || !has_five {
                break;
            }
            if coefficient % 2 == 0 {
                coefficient /= 2;
            } else {
                multiplier /= 2;
            }
            if coefficient % 5 == 0 {
                coefficient /= 5;
            } else {
                multiplier /= 5;
            }
            scale -= 1;
        }

        let product = coefficient
            .checked_mul(i128::from(multiplier))
            .ok_or(MoneyError::ArithmeticOverflow)?;
        Self::from_parts(product, scale, self.currency)
    }
    pub fn checked_cmp(self, other: Self) -> Result<std::cmp::Ordering, MoneyError> {
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
}

// Apply the currency's minor-unit rounding rule before posting.
impl Money {
    /// Round a non-negative amount half-up to its currency's minor unit.
    pub fn rounded_for_posting(self) -> Result<Self, MoneyError> {
        if self.is_negative() {
            return Err(MoneyError::NegativePostingAmount);
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
            .ok_or(MoneyError::ArithmeticOverflow)?
            >= divisor
        {
            quotient
                .checked_add(1)
                .ok_or(MoneyError::ArithmeticOverflow)?
        } else {
            quotient
        };
        Self::from_parts(rounded, target, self.currency)
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

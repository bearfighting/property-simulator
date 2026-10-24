use std::{fmt, num::NonZeroU32};

use property_simulation_money::{Money, MoneyError};
use rust_decimal::{Decimal, MathematicalOps};

/// Pure amounts calculated for one fixed-rate loan installment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoanInstallment {
    payment: Money,
    interest: Money,
    principal: Money,
    closing_balance: Money,
}

impl LoanInstallment {
    pub const fn payment(self) -> Money {
        self.payment
    }

    pub const fn interest(self) -> Money {
        self.interest
    }

    pub const fn principal(self) -> Money {
        self.principal
    }

    pub const fn closing_balance(self) -> Money {
        self.closing_balance
    }
}

/// Calculate the fixed scheduled payment from opening principal and loan terms.
pub fn calculate_scheduled_payment(
    initial_principal: Money,
    annual_interest_rate: Decimal,
    remaining_term_months: NonZeroU32,
) -> Result<Money, LoanCalculationError> {
    let monthly_rate = monthly_interest_rate(annual_interest_rate)?;
    let principal = decimal_from_money(initial_principal)?;
    let term = remaining_term_months.get();

    let payment = if monthly_rate.is_zero() {
        principal
            .checked_div(Decimal::from(term))
            .ok_or(LoanCalculationError::ArithmeticOverflow)?
    } else {
        let one_plus_rate = Decimal::ONE
            .checked_add(monthly_rate)
            .ok_or(LoanCalculationError::ArithmeticOverflow)?;
        let discount_factor = Decimal::ONE
            .checked_div(one_plus_rate)
            .and_then(|discount| discount.checked_powu(u64::from(term)))
            .ok_or(LoanCalculationError::ArithmeticOverflow)?;
        let denominator = Decimal::ONE
            .checked_sub(discount_factor)
            .ok_or(LoanCalculationError::ArithmeticOverflow)?;
        principal
            .checked_mul(monthly_rate)
            .and_then(|numerator| numerator.checked_div(denominator))
            .ok_or(LoanCalculationError::ArithmeticOverflow)?
    };

    Ok(Money::new(payment, initial_principal.currency()).rounded_for_posting()?)
}

/// Calculate interest, principal, payment, and closing balance for one month.
///
/// This function performs decimal arithmetic and posting rounding only. The
/// owning Slice validates that the result is valid for its active loan state.
pub fn calculate_installment(
    opening_balance: Money,
    annual_interest_rate: Decimal,
    scheduled_payment: Money,
    remaining_term_months: NonZeroU32,
) -> Result<LoanInstallment, LoanCalculationError> {
    let monthly_rate = monthly_interest_rate(annual_interest_rate)?;
    let interest_decimal = decimal_from_money(opening_balance)?
        .checked_mul(monthly_rate)
        .ok_or(LoanCalculationError::ArithmeticOverflow)?;
    let interest =
        Money::new(interest_decimal, opening_balance.currency()).rounded_for_posting()?;
    let payment = if remaining_term_months.get() == 1 {
        opening_balance.checked_add(interest)?
    } else {
        scheduled_payment
    };
    let principal = payment.checked_sub(interest)?;
    let closing_balance = opening_balance.checked_sub(principal)?;

    Ok(LoanInstallment {
        payment,
        interest,
        principal,
        closing_balance,
    })
}

/// Failure caused by invalid numeric input or failed arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoanCalculationError {
    ArithmeticOverflow,
    NegativeAnnualInterestRate,
    Money(MoneyError),
}

impl From<MoneyError> for LoanCalculationError {
    fn from(error: MoneyError) -> Self {
        Self::Money(error)
    }
}

impl fmt::Display for LoanCalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => f.write_str("loan calculation exceeds decimal precision"),
            Self::NegativeAnnualInterestRate => {
                f.write_str("annual interest rate cannot be negative")
            }
            Self::Money(error) => write!(f, "invalid loan amount: {error}"),
        }
    }
}

impl std::error::Error for LoanCalculationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Money(error) => Some(error),
            Self::ArithmeticOverflow | Self::NegativeAnnualInterestRate => None,
        }
    }
}

fn monthly_interest_rate(annual_interest_rate: Decimal) -> Result<Decimal, LoanCalculationError> {
    if annual_interest_rate < Decimal::ZERO {
        return Err(LoanCalculationError::NegativeAnnualInterestRate);
    }

    annual_interest_rate
        .checked_div(Decimal::from(12))
        .ok_or(LoanCalculationError::ArithmeticOverflow)
}

fn decimal_from_money(amount: Money) -> Result<Decimal, LoanCalculationError> {
    Decimal::try_from_i128_with_scale(amount.coefficient(), u32::from(amount.scale()))
        .map_err(|_| LoanCalculationError::ArithmeticOverflow)
}

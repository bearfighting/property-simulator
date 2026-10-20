use property_simulation_domain::{DomainError, Money, RoundingPolicy};
use rust_decimal::Decimal;

/// Inputs and continuation data for a fixed-rate, fully amortizing loan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRateLoan {
    pub opening_balance: Money,
    pub remaining_months: u32,
    pub annual_rate: Decimal,
    /// Rounded monthly payment carried forward until terms change or payoff.
    pub scheduled_payment: Option<Money>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoanMonth {
    pub interest: Money,
    pub payment: Money,
    pub principal: Money,
    pub closing_balance: Money,
    pub remaining_months: u32,
    pub scheduled_payment: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoanModelError {
    NegativeRate,
    NonPositiveBalance,
    NonPositiveTerm,
    Arithmetic,
    InvalidScheduledPayment,
    DerivedPaymentRoundsToZero,
    InvalidPrincipalPrecision,
}

impl From<DomainError> for LoanModelError {
    fn from(_: DomainError) -> Self {
        Self::Arithmetic
    }
}

impl FixedRateLoan {
    pub fn validate(self) -> Result<(), LoanModelError> {
        if self.annual_rate < Decimal::ZERO {
            return Err(LoanModelError::NegativeRate);
        }
        if self.remaining_months == 0 {
            return Err(LoanModelError::NonPositiveTerm);
        }
        if self.opening_balance.is_zero() || self.opening_balance.is_negative() {
            return Err(LoanModelError::NonPositiveBalance);
        }
        if self
            .opening_balance
            .rounded_for_posting(RoundingPolicy::HalfUp)?
            != self.opening_balance
        {
            return Err(LoanModelError::InvalidPrincipalPrecision);
        }
        Ok(())
    }

    pub fn derive_payment(self) -> Result<Money, LoanModelError> {
        self.validate()?;
        if let Some(payment) = self.scheduled_payment {
            let interest = self.monthly_interest()?;
            if payment.currency() != self.opening_balance.currency()
                || payment.is_negative()
                || payment.is_zero()
                || payment.rounded_for_posting(RoundingPolicy::HalfUp)? != payment
                || payment.checked_cmp(interest)? == std::cmp::Ordering::Less
            {
                return Err(LoanModelError::InvalidScheduledPayment);
            }
            return Ok(payment);
        }
        let principal = decimal(self.opening_balance)?;
        let monthly_rate = self
            .annual_rate
            .checked_div(Decimal::from(12))
            .ok_or(LoanModelError::Arithmetic)?;
        let raw = if monthly_rate.is_zero() {
            principal
                .checked_div(Decimal::from(self.remaining_months))
                .ok_or(LoanModelError::Arithmetic)?
        } else {
            let growth = Decimal::ONE
                .checked_add(monthly_rate)
                .ok_or(LoanModelError::Arithmetic)?;
            let power = checked_pow(growth, self.remaining_months)?;
            let denominator = Decimal::ONE
                .checked_sub(
                    Decimal::ONE
                        .checked_div(power)
                        .ok_or(LoanModelError::Arithmetic)?,
                )
                .ok_or(LoanModelError::Arithmetic)?;
            principal
                .checked_mul(monthly_rate)
                .and_then(|v| v.checked_div(denominator))
                .ok_or(LoanModelError::Arithmetic)?
        };
        let payment = money(raw, self.opening_balance.currency())?
            .rounded_for_posting(RoundingPolicy::HalfUp)
            .map_err(LoanModelError::from)?;
        if payment.is_zero() {
            return Err(LoanModelError::DerivedPaymentRoundsToZero);
        }
        Ok(payment)
    }

    pub fn calculate_month(self) -> Result<LoanMonth, LoanModelError> {
        self.validate()?;
        let scheduled = self.derive_payment()?;
        let interest = self.monthly_interest()?;
        let remaining_due = self.opening_balance.checked_add(interest)?;
        let payment = if self.remaining_months == 1
            || scheduled.checked_cmp(remaining_due)? == std::cmp::Ordering::Greater
        {
            remaining_due
        } else {
            scheduled
        };
        let principal = payment.checked_sub(interest)?;
        let closing_balance = self.opening_balance.checked_sub(principal)?;
        Ok(LoanMonth {
            interest,
            payment,
            principal,
            closing_balance,
            remaining_months: if closing_balance.is_zero() {
                0
            } else {
                self.remaining_months - 1
            },
            scheduled_payment: scheduled,
        })
    }

    fn monthly_interest(self) -> Result<Money, LoanModelError> {
        let monthly_rate = self
            .annual_rate
            .checked_div(Decimal::from(12))
            .ok_or(LoanModelError::Arithmetic)?;
        money(
            decimal(self.opening_balance)?
                .checked_mul(monthly_rate)
                .ok_or(LoanModelError::Arithmetic)?,
            self.opening_balance.currency(),
        )?
        .rounded_for_posting(RoundingPolicy::HalfUp)
        .map_err(Into::into)
    }
}

fn checked_pow(mut base: Decimal, mut exponent: u32) -> Result<Decimal, LoanModelError> {
    let mut result = Decimal::ONE;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = result.checked_mul(base).ok_or(LoanModelError::Arithmetic)?;
        }
        exponent >>= 1;
        if exponent > 0 {
            base = base.checked_mul(base).ok_or(LoanModelError::Arithmetic)?;
        }
    }
    Ok(result)
}

/// `None` represents a settled or absent loan and creates no monthly activity.
pub fn calculate_optional_loan_month(
    loan: Option<FixedRateLoan>,
) -> Result<Option<LoanMonth>, LoanModelError> {
    loan.map(FixedRateLoan::calculate_month).transpose()
}

fn decimal(value: Money) -> Result<Decimal, LoanModelError> {
    Decimal::try_from_i128_with_scale(value.coefficient(), u32::from(value.scale()))
        .map_err(|_| LoanModelError::Arithmetic)
}
fn money(
    value: Decimal,
    currency: property_simulation_domain::Currency,
) -> Result<Money, LoanModelError> {
    Ok(Money::new(value, currency))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn loan(balance: &str, rate: &str, months: u32) -> FixedRateLoan {
        let currency = property_simulation_domain::Currency::from_code("CAD").unwrap();
        FixedRateLoan {
            opening_balance: Money::parse(balance, currency).unwrap(),
            remaining_months: months,
            annual_rate: Decimal::from_str_exact(rate).unwrap(),
            scheduled_payment: None,
        }
    }
    #[test]
    fn implements_mortgage_vectors_and_continuation() {
        let first = loan("10000", "0.12", 12).calculate_month().unwrap();
        assert_eq!(
            first.scheduled_payment,
            Money::parse("888.49", first.payment.currency()).unwrap()
        );
        assert_eq!(
            first.interest,
            Money::parse("100", first.payment.currency()).unwrap()
        );
        assert_eq!(
            first.principal,
            Money::parse("788.49", first.payment.currency()).unwrap()
        );
        let second = FixedRateLoan {
            opening_balance: first.closing_balance,
            remaining_months: 11,
            annual_rate: Decimal::new(12, 2),
            scheduled_payment: Some(first.scheduled_payment),
        }
        .calculate_month()
        .unwrap();
        assert_eq!(
            second.interest,
            Money::parse("92.12", first.payment.currency()).unwrap()
        );
        let final_month = FixedRateLoan {
            opening_balance: Money::parse("879.67", first.payment.currency()).unwrap(),
            remaining_months: 1,
            annual_rate: Decimal::new(12, 2),
            scheduled_payment: Some(first.scheduled_payment),
        }
        .calculate_month()
        .unwrap();
        assert_eq!(
            final_month.payment,
            Money::parse("888.47", first.payment.currency()).unwrap()
        );
        assert!(final_month.closing_balance.is_zero());
    }
    #[test]
    fn rejects_invalid_active_inputs_and_handles_zero_rate() {
        assert_eq!(
            loan("1000", "0", 10).calculate_month().unwrap().payment,
            Money::parse("100", loan("1000", "0", 10).opening_balance.currency()).unwrap()
        );
        assert_eq!(
            loan("1000", "-0.01", 12).validate(),
            Err(LoanModelError::NegativeRate)
        );
        assert_eq!(
            loan("1000", "0.12", 0).validate(),
            Err(LoanModelError::NonPositiveTerm)
        );
        assert_eq!(
            loan("0", "0.12", 12).validate(),
            Err(LoanModelError::NonPositiveBalance)
        );
    }

    #[test]
    fn implements_final_payment_and_half_up_rounding_boundaries() {
        let currency = property_simulation_domain::Currency::from_code("CAD").unwrap();
        let payoff = loan("100", "0.12", 1).calculate_month().unwrap();
        assert_eq!(payoff.interest, Money::parse("1", currency).unwrap());
        assert_eq!(payoff.payment, Money::parse("101", currency).unwrap());
        assert!(payoff.closing_balance.is_zero());
        let midpoint = loan("0.50", "0.12", 1).calculate_month().unwrap();
        assert_eq!(midpoint.interest, Money::parse("0.01", currency).unwrap());
        assert_eq!(midpoint.payment, Money::parse("0.51", currency).unwrap());
        let sub_cent = loan("0.01", "0.06", 1).calculate_month().unwrap();
        assert_eq!(sub_cent.interest, Money::parse("0", currency).unwrap());
        assert_eq!(sub_cent.payment, Money::parse("0.01", currency).unwrap());
    }

    #[test]
    fn validates_carried_payment_and_absent_loan_continuation() {
        let active = loan("100", "0.12", 12);
        let too_small = FixedRateLoan {
            scheduled_payment: Some(
                Money::parse("0.99", active.opening_balance.currency()).unwrap(),
            ),
            ..active
        };
        assert_eq!(
            too_small.calculate_month(),
            Err(LoanModelError::InvalidScheduledPayment)
        );
        let other_currency = Money::parse(
            "10",
            property_simulation_domain::Currency::from_code("USD").unwrap(),
        )
        .unwrap();
        let wrong_currency = FixedRateLoan {
            scheduled_payment: Some(other_currency),
            ..active
        };
        assert_eq!(
            wrong_currency.calculate_month(),
            Err(LoanModelError::InvalidScheduledPayment)
        );
        assert_eq!(calculate_optional_loan_month(None), Ok(None));
        let paid_off = loan("100", "0.12", 1).calculate_month().unwrap();
        assert!(paid_off.closing_balance.is_zero());
        assert_eq!(calculate_optional_loan_month(None), Ok(None));
    }

    #[test]
    fn rejects_subminor_unit_balances_and_carried_payments() {
        let invalid_balance = loan("1.001", "0.12", 12);
        assert_eq!(
            invalid_balance.validate(),
            Err(LoanModelError::InvalidPrincipalPrecision)
        );
        let base = loan("100", "0.12", 12);
        let invalid_payment = FixedRateLoan {
            scheduled_payment: Some(
                Money::parse("10.001", base.opening_balance.currency()).unwrap(),
            ),
            ..base
        };
        assert_eq!(
            invalid_payment.derive_payment(),
            Err(LoanModelError::InvalidScheduledPayment)
        );
    }

    #[test]
    fn rejects_a_derived_payment_that_rounds_to_zero() {
        let schedule = loan("0.01", "0", 1000);
        assert_eq!(
            schedule.derive_payment(),
            Err(LoanModelError::DerivedPaymentRoundsToZero)
        );
        assert_eq!(
            schedule.calculate_month(),
            Err(LoanModelError::DerivedPaymentRoundsToZero)
        );
    }

    #[test]
    fn amortization_power_uses_logarithmic_steps_for_long_terms() {
        let schedule = loan("250000", "0.065", 360);
        assert_eq!(
            schedule.derive_payment().unwrap(),
            Money::parse("1580.17", schedule.opening_balance.currency()).unwrap()
        );
    }
}

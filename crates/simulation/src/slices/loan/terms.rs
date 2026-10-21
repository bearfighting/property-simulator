use super::LoanTermsUpdate;
use crate::SimulationError;
use property_simulation_accounting::LedgerTransaction;
use property_simulation_domain::YearMonth;
use property_simulation_models::{FixedRateLoan, LoanModelError};

/// Loan mathematics supplied by the caller that connects this input transformation
/// to the loan model implementation.
pub trait LoanCalculation {
    fn validate(&self, loan: FixedRateLoan) -> Result<(), LoanModelError>;
    fn derive_payment(
        &self,
        loan: FixedRateLoan,
    ) -> Result<property_simulation_domain::Money, LoanModelError>;
}

pub(in crate::slices) fn map_model_error(error: LoanModelError) -> SimulationError {
    match error {
        LoanModelError::Arithmetic => {
            SimulationError::InvalidInput("loan calculation exceeds supported decimal precision")
        }
        LoanModelError::NegativeRate => {
            SimulationError::InvalidInput("loan annual rate cannot be negative")
        }
        LoanModelError::NonPositiveBalance => {
            SimulationError::InvalidInput("active loan principal must be positive")
        }
        LoanModelError::NonPositiveTerm => {
            SimulationError::InvalidInput("active loan remaining term must be positive")
        }
        LoanModelError::InvalidScheduledPayment => {
            SimulationError::InvalidInput("carried loan payment is invalid for its schedule")
        }
        LoanModelError::DerivedPaymentRoundsToZero => SimulationError::InvalidInput(
            "derived loan payment rounds to zero at currency minor-unit precision",
        ),
        LoanModelError::InvalidPrincipalPrecision => {
            SimulationError::InvalidInput("loan principal exceeds currency minor-unit precision")
        }
    }
}

/// Validates a month-start terms update and checks any principal delta against the
/// caller-supplied balanced financing transaction.
pub fn apply_loan_terms_update(
    current: FixedRateLoan,
    effective_period: YearMonth,
    opening_balance: property_simulation_domain::Money,
    remaining_months: u32,
    annual_rate: rust_decimal::Decimal,
    principal_transaction: Option<LedgerTransaction>,
    calculation: &impl LoanCalculation,
) -> Result<LoanTermsUpdate, SimulationError> {
    calculation.validate(current).map_err(map_model_error)?;
    calculation
        .derive_payment(current)
        .map_err(map_model_error)?;
    if opening_balance.currency() != current.opening_balance.currency()
        || opening_balance.is_negative()
    {
        return Err(SimulationError::InvalidInput(
            "loan balance update must be non-negative and use the loan currency",
        ));
    }
    let updated = if opening_balance.is_zero() {
        if remaining_months == 0 && annual_rate >= rust_decimal::Decimal::ZERO {
            None
        } else {
            return Err(SimulationError::InvalidInput(
                "a settled loan update must have zero remaining term and non-negative rate",
            ));
        }
    } else {
        let active = FixedRateLoan {
            opening_balance,
            remaining_months,
            annual_rate,
            scheduled_payment: None,
        };
        calculation.validate(active).map_err(map_model_error)?;
        calculation
            .derive_payment(active)
            .map_err(map_model_error)?;
        Some(active)
    };
    let principal_change = opening_balance.checked_sub(current.opening_balance)?;
    if principal_change.is_zero() != principal_transaction.is_none() {
        return Err(SimulationError::InvalidInput(
            "a principal balance change requires its balanced ledger transaction",
        ));
    }
    if let Some(transaction) = &principal_transaction {
        if transaction.period() != effective_period {
            return Err(SimulationError::InvalidInput(
                "principal update transaction must use the effective month",
            ));
        }
    }
    Ok(LoanTermsUpdate {
        expected_current: current,
        updated,
        effective_period,
        principal_change,
        principal_transaction,
    })
}

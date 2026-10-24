use std::fmt;

use property_simulation_account::DebitCredit;
use property_simulation_domain_calculations::{
    LoanCalculationError, calculate_installment, calculate_scheduled_payment,
};
use property_simulation_journal_entry::{JournalEntry, JournalEntryError, JournalLine};
use property_simulation_money::{Money, MoneyError};
use property_simulation_slice_catalogue::{
    Effect, Event, LoanParameters, LoanSliceInput, SliceKind, SliceResult, SliceResultError,
};

use crate::DeterministicSlice;

/// Calculates fixed-rate amortizing loan payments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanSlice {
    parameters: LoanParameters,
    scheduled_payment: Money,
    input: LoanSliceInput,
}

impl LoanSlice {
    /// Construct a one-period loan behavior from fixed terms and resolved inputs.
    pub fn new(parameters: LoanParameters, input: LoanSliceInput) -> Result<Self, LoanSliceError> {
        let LoanSliceInput {
            initial_principal,
            opening_balance,
            remaining_term_months,
            ..
        } = input;

        if initial_principal.is_negative() {
            return Err(LoanSliceError::NegativeInitialPrincipal);
        }
        if initial_principal.is_zero() {
            return Err(LoanSliceError::OpeningBalanceWithoutActiveLoan);
        }
        if opening_balance.is_negative() {
            return Err(LoanSliceError::NegativeOpeningBalance);
        }
        if opening_balance.is_zero() {
            return Err(LoanSliceError::OpeningBalanceWithoutActiveLoan);
        }
        if remaining_term_months > parameters.remaining_term_months() {
            return Err(LoanSliceError::RemainingTermExceedsOriginal);
        }
        if opening_balance.checked_cmp(initial_principal)?.is_gt() {
            return Err(LoanSliceError::OpeningBalanceExceedsOriginal);
        }

        let scheduled_payment = calculate_scheduled_payment(
            initial_principal,
            parameters.annual_interest_rate(),
            parameters.remaining_term_months(),
        )?;
        if scheduled_payment.is_zero() {
            return Err(LoanSliceError::PaymentRoundsToZero);
        }

        Ok(Self {
            parameters,
            scheduled_payment,
            input,
        })
    }
}

impl DeterministicSlice for LoanSlice {
    type Error = LoanSliceError;

    fn calculate(&self) -> Result<SliceResult, Self::Error> {
        (|| -> Result<SliceResult, LoanSliceError> {
            let installment = calculate_installment(
                self.input.opening_balance,
                self.parameters.annual_interest_rate(),
                self.scheduled_payment,
                self.input.remaining_term_months,
            )?;
            let payment = installment.payment();
            let interest = installment.interest();
            let principal = installment.principal();
            let closing_balance = installment.closing_balance();

            if principal.is_zero() || principal.is_negative() {
                return Err(LoanSliceError::PaymentDoesNotReducePrincipal);
            }
            if principal.checked_cmp(self.input.opening_balance)?.is_gt() {
                return Err(LoanSliceError::PaymentExceedsOpeningBalance);
            }

            let mut lines = Vec::with_capacity(3);
            lines.push(JournalLine::new(
                self.input.loan_liability_account,
                DebitCredit::Debit,
                principal,
            )?);
            if !interest.is_zero() {
                lines.push(JournalLine::new(
                    self.input.interest_expense_account,
                    DebitCredit::Debit,
                    interest,
                )?);
            }
            lines.push(JournalLine::new(
                self.input.cash_account,
                DebitCredit::Credit,
                payment,
            )?);

            let entry = JournalEntry::new(self.input.month, lines)?;
            let event = Event::LoanPayment {
                principal,
                interest,
                closing_balance,
            };
            let remaining_months = if closing_balance.is_zero() {
                0
            } else {
                self.input.remaining_term_months.get() - 1
            };
            let effect = Effect::LoanRemainingTermUpdated { remaining_months };

            Ok(SliceResult::new(
                SliceKind::Loan,
                vec![event],
                vec![effect],
                vec![entry],
            )?)
        })()
    }
}

/// Failure while constructing or evaluating a loan payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoanSliceError {
    NegativeInitialPrincipal,
    NegativeOpeningBalance,
    OpeningBalanceWithoutActiveLoan,
    OpeningBalanceExceedsOriginal,
    RemainingTermExceedsOriginal,
    PaymentRoundsToZero,
    PaymentDoesNotReducePrincipal,
    PaymentExceedsOpeningBalance,
    Calculation(LoanCalculationError),
    SliceResult(SliceResultError),
    Money(MoneyError),
    JournalEntry(JournalEntryError),
}

impl From<MoneyError> for LoanSliceError {
    fn from(error: MoneyError) -> Self {
        Self::Money(error)
    }
}

impl From<JournalEntryError> for LoanSliceError {
    fn from(error: JournalEntryError) -> Self {
        Self::JournalEntry(error)
    }
}

impl From<LoanCalculationError> for LoanSliceError {
    fn from(error: LoanCalculationError) -> Self {
        Self::Calculation(error)
    }
}

impl From<SliceResultError> for LoanSliceError {
    fn from(error: SliceResultError) -> Self {
        Self::SliceResult(error)
    }
}

impl fmt::Display for LoanSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeInitialPrincipal => f.write_str("initial principal cannot be negative"),
            Self::NegativeOpeningBalance => f.write_str("opening loan balance cannot be negative"),
            Self::OpeningBalanceWithoutActiveLoan => {
                f.write_str("loan calculation requires an active loan balance")
            }
            Self::OpeningBalanceExceedsOriginal => {
                f.write_str("opening loan balance exceeds its initial principal")
            }
            Self::RemainingTermExceedsOriginal => {
                f.write_str("remaining loan term exceeds its original term")
            }
            Self::PaymentRoundsToZero => f.write_str("scheduled loan payment rounds to zero"),
            Self::PaymentDoesNotReducePrincipal => {
                f.write_str("scheduled payment does not reduce loan principal")
            }
            Self::PaymentExceedsOpeningBalance => {
                f.write_str("scheduled payment exceeds the opening principal")
            }
            Self::Calculation(error) => write!(f, "loan calculation failed: {error}"),
            Self::SliceResult(error) => write!(f, "invalid loan Slice result: {error}"),
            Self::Money(error) => write!(f, "invalid loan amount: {error}"),
            Self::JournalEntry(error) => write!(f, "invalid loan journal entry: {error}"),
        }
    }
}

impl std::error::Error for LoanSliceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Money(error) => Some(error),
            Self::JournalEntry(error) => Some(error),
            Self::Calculation(error) => Some(error),
            Self::SliceResult(error) => Some(error),
            _ => None,
        }
    }
}

use super::{LoanCalculation, LoanSlice, LoanTermsUpdate, map_model_error};
use crate::SimulationError;
#[cfg(test)]
use property_simulation_accounting::Ledger;
use property_simulation_accounting::{AccountCategory, BalanceSide, CashFlowCategory, LedgerEntry};
#[cfg(test)]
use property_simulation_domain::AccountId;
use property_simulation_domain::Money;

#[cfg(test)]
use crate::validation::AccountFacts;
use crate::validation::LedgerRead;

/// Validate the configured loan accounts and active loan against the caller's ledger view.
/// The caller supplies the model implementation so loan rules remain independently injectable.
pub fn validate_loan_input(
    slice: &LoanSlice,
    loan: Option<property_simulation_models::FixedRateLoan>,
    ledger: &impl LedgerRead,
    calculation: &impl LoanCalculation,
) -> Result<(), SimulationError> {
    validate_account_roles(slice, ledger)?;
    if let Some(loan) = loan {
        if loan.opening_balance.currency() != ledger.currency() {
            return Err(SimulationError::InvalidInput(
                "loan balance and ledger must use the same currency",
            ));
        }
        let expected_liability = loan.opening_balance.checked_neg()?;
        if ledger.signed_balance(slice.liability_account)? != expected_liability {
            return Err(SimulationError::InvalidInput(
                "loan opening balance must match its liability account balance",
            ));
        }
        calculation.validate(loan).map_err(map_model_error)?;
        calculation.derive_payment(loan).map_err(map_model_error)?;
        calculation.calculate_month(loan).map_err(map_model_error)?;
    } else if !ledger.signed_balance(slice.liability_account)?.is_zero() {
        return Err(SimulationError::InvalidInput(
            "an absent or settled loan must have a zero liability account balance",
        ));
    }
    Ok(())
}

/// Validate a prepared terms update against the receiving loan Slice and opening ledger.
/// This checks the account references that cannot be verified by the account-agnostic
/// `apply_loan_terms_update` constructor.
pub fn validate_loan_terms_update(
    slice: &LoanSlice,
    update: &LoanTermsUpdate,
    period: property_simulation_domain::YearMonth,
    ledger: &impl LedgerRead,
    calculation: &impl LoanCalculation,
) -> Result<(), SimulationError> {
    if update.effective_period != period {
        return Err(SimulationError::InvalidInput(
            "loan terms update must use the effective month",
        ));
    }
    validate_loan_input(slice, Some(update.expected_current), ledger, calculation)?;

    let updated_balance = match update.updated {
        Some(updated) => {
            if updated.opening_balance.currency() != ledger.currency() {
                return Err(SimulationError::InvalidInput(
                    "updated loan balance and ledger must use the same currency",
                ));
            }
            calculation.validate(updated).map_err(map_model_error)?;
            calculation
                .derive_payment(updated)
                .map_err(map_model_error)?;
            calculation
                .calculate_month(updated)
                .map_err(map_model_error)?;
            updated.opening_balance
        }
        None => Money::new(0.into(), ledger.currency()),
    };
    if updated_balance.checked_sub(update.expected_current.opening_balance)?
        != update.principal_change
    {
        return Err(SimulationError::InvalidInput(
            "loan terms update principal delta does not match its updated balance",
        ));
    }

    if update.principal_change.is_zero() {
        if update.principal_transaction.is_some() {
            return Err(SimulationError::InvalidInput(
                "a zero principal change must not have a financing transaction",
            ));
        }
        return Ok(());
    }

    let transaction =
        update
            .principal_transaction
            .as_ref()
            .ok_or(SimulationError::InvalidInput(
                "a principal balance change requires its balanced ledger transaction",
            ))?;
    let amount = update.principal_change.abs()?;
    let entries = if update.principal_change.is_negative() {
        [
            LedgerEntry::new(slice.liability_account, BalanceSide::Debit, amount),
            LedgerEntry::new(slice.cash_account, BalanceSide::Credit, amount)
                .with_cash_flow_category(CashFlowCategory::Financing),
        ]
    } else {
        [
            LedgerEntry::new(slice.cash_account, BalanceSide::Debit, amount)
                .with_cash_flow_category(CashFlowCategory::Financing),
            LedgerEntry::new(slice.liability_account, BalanceSide::Credit, amount),
        ]
    };
    if transaction.period() != period
        || transaction.entries().len() != entries.len()
        || entries
            .iter()
            .any(|expected| !transaction.entries().contains(expected))
    {
        return Err(SimulationError::InvalidInput(
            "principal update transaction does not match its loan accounts and balance delta",
        ));
    }
    Ok(())
}

fn validate_account_roles(
    slice: &LoanSlice,
    ledger: &impl LedgerRead,
) -> Result<(), SimulationError> {
    let liability =
        ledger
            .account(slice.liability_account)
            .ok_or(SimulationError::InvalidInput(
                "loan liability account does not exist",
            ))?;
    if liability.category != AccountCategory::Liability || liability.contra {
        return Err(SimulationError::InvalidInput(
            "each loan must use a non-contra liability account",
        ));
    }
    let interest =
        ledger
            .account(slice.interest_expense_account)
            .ok_or(SimulationError::InvalidInput(
                "interest expense account does not exist",
            ))?;
    if interest.category != AccountCategory::Expense || interest.contra {
        return Err(SimulationError::InvalidInput(
            "loan interest must use a non-contra expense account",
        ));
    }
    let cash = ledger
        .account(slice.cash_account)
        .ok_or(SimulationError::InvalidInput(
            "loan cash account does not exist",
        ))?;
    if cash.category != AccountCategory::Asset || !cash.cash_position || cash.contra {
        return Err(SimulationError::InvalidInput(
            "loan payments must use a non-contra cash-position asset account",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(in crate::slices) fn validate_loan_accounts(
    slice: &LoanSlice,
    ledger: &Ledger,
) -> Result<(), SimulationError> {
    struct View<'a>(&'a Ledger);
    impl LedgerRead for View<'_> {
        fn currency(&self) -> property_simulation_domain::Currency {
            self.0.currency()
        }
        fn account(&self, id: AccountId) -> Option<AccountFacts> {
            self.0.chart().get(id).map(|account| AccountFacts {
                category: account.category(),
                cash_position: account.is_cash_position(),
                contra: account.is_contra(),
            })
        }
        fn signed_balance(
            &self,
            id: AccountId,
        ) -> Result<Money, property_simulation_accounting::AccountError> {
            let (side, amount) = self.0.balance(id)?;
            match side {
                BalanceSide::Debit => Ok(amount),
                BalanceSide::Credit => amount.checked_neg().map_err(Into::into),
            }
        }
    }
    validate_account_roles(slice, &View(ledger))
}

#[cfg(test)]
pub(in crate::slices) fn cash_classified_entry(
    account: AccountId,
    side: BalanceSide,
    amount: Money,
    cash_account: AccountId,
) -> LedgerEntry {
    let entry = LedgerEntry::new(account, side, amount);
    if account == cash_account {
        entry.with_cash_flow_category(CashFlowCategory::Financing)
    } else {
        entry
    }
}

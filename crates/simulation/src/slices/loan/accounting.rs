use super::LoanSlice;
use crate::SimulationError;
use property_simulation_accounting::{
    AccountCategory, BalanceSide, CashFlowCategory, Ledger, LedgerEntry,
};
use property_simulation_domain::{AccountId, Money};

pub(in crate::slices) fn validate_loan_accounts(
    slice: &LoanSlice,
    ledger: &Ledger,
) -> Result<(), SimulationError> {
    let chart = ledger.chart();
    let liability = chart
        .get(slice.liability_account)
        .ok_or(SimulationError::InvalidInput(
            "loan liability account does not exist",
        ))?;
    if liability.category() != AccountCategory::Liability || liability.is_contra() {
        return Err(SimulationError::InvalidInput(
            "each loan must use a non-contra liability account",
        ));
    }
    let interest =
        chart
            .get(slice.interest_expense_account)
            .ok_or(SimulationError::InvalidInput(
                "interest expense account does not exist",
            ))?;
    if interest.category() != AccountCategory::Expense {
        return Err(SimulationError::InvalidInput(
            "loan interest must use an expense account",
        ));
    }
    let cash = chart
        .get(slice.cash_account)
        .ok_or(SimulationError::InvalidInput(
            "loan cash account does not exist",
        ))?;
    if cash.category() != AccountCategory::Asset || !cash.is_cash_position() || cash.is_contra() {
        return Err(SimulationError::InvalidInput(
            "loan payments must use a cash-position asset account",
        ));
    }
    Ok(())
}

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

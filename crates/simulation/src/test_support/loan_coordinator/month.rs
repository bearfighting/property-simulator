use super::super::loan::{
    LoanCoordinatedMonth, LoanEvent, LoanMonthCommand, LoanSlice, LoanStateEffect,
    cash_classified_entry, map_model_error, validate_loan_accounts,
};
use crate::{CashFlowContribution, FatalSimulationFailure, FatalSimulationPhase, SimulationError};
use property_simulation_accounting::{
    BalanceSide, CashFlowCategory, LedgerTransaction, LedgerTransactionSource,
};
use property_simulation_domain::Money;

/// Coordinate loan postings and continuation state as an atomic monthly unit.
/// `None` is a no-op, so a run can continue after the final payment.
pub fn coordinate_loan_month(
    slice: &LoanSlice,
    command: LoanMonthCommand<'_>,
) -> Result<LoanCoordinatedMonth, SimulationError> {
    coordinate_loan_month_inner(slice, command, true)
}

pub(super) fn prepare_loan_month(
    slice: &LoanSlice,
    command: LoanMonthCommand<'_>,
) -> Result<LoanCoordinatedMonth, SimulationError> {
    coordinate_loan_month_inner(slice, command, false)
}

fn coordinate_loan_month_inner(
    slice: &LoanSlice,
    command: LoanMonthCommand<'_>,
    commit: bool,
) -> Result<LoanCoordinatedMonth, SimulationError> {
    let LoanMonthCommand {
        period,
        loan,
        update,
        opening_ledger,
        trial_id,
        branch_id,
        ids,
    } = command;
    validate_loan_accounts(slice, opening_ledger)?;
    let Some(loan) = loan else {
        if update.is_some() {
            return Err(SimulationError::InvalidInput(
                "loan terms update requires an active loan",
            ));
        }
        let (side, balance) = opening_ledger.balance(slice.liability_account)?;
        if side != BalanceSide::Credit || !balance.is_zero() {
            return Err(SimulationError::InvalidInput(
                "an inactive loan must reconcile to a zero liability balance",
            ));
        }
        return Ok(LoanCoordinatedMonth {
            instance_id: slice.instance_id,
            slice_id: slice.slice_id,
            cash_account: slice.cash_account,
            liability_account: slice.liability_account,
            period,
            closing_loan: None,
            month: None,
            ledger: opening_ledger.clone(),
            cash_flow_contributions: Vec::new(),
            events: Vec::new(),
            transactions: Vec::new(),
            state_effect: None,
        });
    };
    if loan.opening_balance.currency() != opening_ledger.currency() {
        return Err(SimulationError::InvalidInput(
            "loan and ledger currencies must match",
        ));
    }
    let (side, balance) = opening_ledger.balance(slice.liability_account)?;
    if side != BalanceSide::Credit || balance != loan.opening_balance {
        return Err(SimulationError::InvalidInput(
            "loan opening balance must match its independent ledger liability",
        ));
    }
    let opening_loan = loan;
    let mut transactions = Vec::new();
    let mut cash_flow_contributions = Vec::new();
    let mut events = Vec::new();
    let loan = if let Some(update) = update {
        if update.effective_period() != period || update.expected_loan() != loan {
            return Err(SimulationError::InvalidInput(
                "loan terms update must match the active loan and effective month",
            ));
        }
        let amount = update.principal_change().abs()?;
        let event_id = ids.event_id(period, slice.instance_id, "loan_terms_update");
        events.push(LoanEvent {
            id: event_id,
            instance_id: slice.instance_id,
            slice_id: slice.slice_id,
            period,
            key: "loan_terms_update".into(),
            previous_balance: Some(loan.opening_balance),
            updated_balance: Some(update.updated_loan().map_or(
                Money::from_parts(0, 0, loan.opening_balance.currency())?,
                |updated| updated.opening_balance,
            )),
            previous_remaining_months: Some(loan.remaining_months),
            updated_remaining_months: update
                .updated_loan()
                .map(|updated| updated.remaining_months),
            previous_annual_rate: Some(loan.annual_rate),
            updated_annual_rate: update.updated_loan().map(|updated| updated.annual_rate),
        });
        if !amount.is_zero() {
            let (debit_account, credit_account) = if update.principal_change().is_negative() {
                (slice.liability_account, slice.cash_account)
            } else {
                (slice.cash_account, slice.liability_account)
            };
            let supplied = update
                .principal_transaction()
                .ok_or(SimulationError::InvalidInput(
                    "a principal balance change requires its balanced ledger transaction",
                ))?;
            let expected_entries = [
                cash_classified_entry(
                    debit_account,
                    BalanceSide::Debit,
                    amount,
                    slice.cash_account,
                ),
                cash_classified_entry(
                    credit_account,
                    BalanceSide::Credit,
                    amount,
                    slice.cash_account,
                ),
            ];
            if supplied.entries().len() != expected_entries.len()
                || expected_entries
                    .iter()
                    .any(|expected| !supplied.entries().contains(expected))
            {
                return Err(SimulationError::InvalidInput(
                    "principal update transaction does not match the loan balance delta",
                ));
            }
            let tx_id = supplied.id();
            let transaction = LedgerTransaction::new(tx_id, period, supplied.entries().to_vec())
                .with_source(LedgerTransactionSource {
                    instance_id: slice.instance_id,
                    slice_id: slice.slice_id,
                    output_key: "loan_balance_update".into(),
                    source_event_id: Some(event_id),
                });
            opening_ledger.validate_transaction(&transaction)?;
            cash_flow_contributions.push(CashFlowContribution {
                trial_id,
                branch_id,
                period,
                instance_id: slice.instance_id,
                slice_id: slice.slice_id,
                output_key: "loan_balance_update".into(),
                transaction_id: tx_id,
                category: CashFlowCategory::Financing,
                amount: update.principal_change(),
            });
            transactions.push(transaction);
        }
        if let Some(updated) = update.updated_loan() {
            updated
        } else {
            let mut ledger = opening_ledger.clone();
            if commit {
                ledger.post_batch(transactions.clone()).map_err(|source| {
                    SimulationError::Fatal(FatalSimulationFailure::with_run_identity(
                        FatalSimulationPhase::LedgerCommit,
                        period,
                        Some(trial_id),
                        Some(branch_id),
                        Some(slice.instance_id),
                        source.into(),
                    ))
                })?;
            }
            if commit {
                let (closing_side, closing_balance) =
                    ledger.balance(slice.liability_account).map_err(|source| {
                        SimulationError::Fatal(FatalSimulationFailure::with_run_identity(
                            FatalSimulationPhase::ClosingStateDerivation,
                            period,
                            Some(trial_id),
                            Some(branch_id),
                            Some(slice.instance_id),
                            source.into(),
                        ))
                    })?;
                if closing_side != BalanceSide::Credit || !closing_balance.is_zero() {
                    return Err(SimulationError::Fatal(
                        FatalSimulationFailure::with_run_identity(
                            FatalSimulationPhase::ClosingStateDerivation,
                            period,
                            Some(trial_id),
                            Some(branch_id),
                            Some(slice.instance_id),
                            SimulationError::InvalidProposal(
                                "settled loan update does not reconcile to zero liability",
                            ),
                        ),
                    ));
                }
            } else if loan
                .opening_balance
                .checked_add(update.principal_change())?
                != Money::from_parts(0, 0, loan.opening_balance.currency())?
            {
                return Err(SimulationError::InvalidInput(
                    "settled loan update does not reconcile to zero liability",
                ));
            }
            return Ok(LoanCoordinatedMonth {
                instance_id: slice.instance_id,
                slice_id: slice.slice_id,
                cash_account: slice.cash_account,
                liability_account: slice.liability_account,
                period,
                closing_loan: None,
                month: None,
                ledger,
                cash_flow_contributions,
                events,
                transactions,
                state_effect: Some(LoanStateEffect {
                    id: ids.effect_id(period, slice.instance_id, "loan_state"),
                    instance_id: slice.instance_id,
                    slice_id: slice.slice_id,
                    period,
                    previous: Some(opening_loan),
                    updated: None,
                    source_event_id: event_id,
                }),
            });
        }
    } else {
        loan
    };
    let month = loan.calculate_month().map_err(map_model_error)?;
    let output = slice.propose_month(period, loan, month)?;
    let transaction_id = ids.transaction_id(period, slice.instance_id, "loan_payment");
    let payment_event_id = ids.event_id(period, slice.instance_id, "loan_payment");
    let transaction = LedgerTransaction::new(
        transaction_id,
        period,
        output.proposal.transactions[0].entries.clone(),
    )
    .with_source(LedgerTransactionSource {
        instance_id: slice.instance_id,
        slice_id: slice.slice_id,
        output_key: "loan_payment".into(),
        source_event_id: Some(payment_event_id),
    });
    events.push(LoanEvent {
        id: payment_event_id,
        instance_id: slice.instance_id,
        slice_id: slice.slice_id,
        period,
        key: "loan_payment".into(),
        previous_balance: Some(loan.opening_balance),
        updated_balance: Some(month.closing_balance),
        previous_remaining_months: Some(loan.remaining_months),
        updated_remaining_months: Some(month.remaining_months),
        previous_annual_rate: Some(loan.annual_rate),
        updated_annual_rate: Some(loan.annual_rate),
    });
    if events
        .iter()
        .map(|event| event.id)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != events.len()
    {
        return Err(SimulationError::DuplicateOutputId);
    }
    opening_ledger.validate_transaction(&transaction)?;
    transactions.push(transaction);
    let mut ledger = opening_ledger.clone();
    if commit {
        ledger.post_batch(transactions.clone()).map_err(|source| {
            SimulationError::Fatal(FatalSimulationFailure::with_run_identity(
                FatalSimulationPhase::LedgerCommit,
                period,
                Some(trial_id),
                Some(branch_id),
                Some(slice.instance_id),
                source.into(),
            ))
        })?;
    }
    if commit {
        let (closing_side, closing_balance) =
            ledger.balance(slice.liability_account).map_err(|source| {
                SimulationError::Fatal(FatalSimulationFailure::with_run_identity(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    Some(trial_id),
                    Some(branch_id),
                    Some(slice.instance_id),
                    source.into(),
                ))
            })?;
        if closing_side != BalanceSide::Credit || closing_balance != month.closing_balance {
            return Err(SimulationError::Fatal(
                FatalSimulationFailure::with_run_identity(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    Some(trial_id),
                    Some(branch_id),
                    Some(slice.instance_id),
                    SimulationError::InvalidProposal(
                        "loan closing balance does not match committed liability",
                    ),
                ),
            ));
        }
    }
    cash_flow_contributions.extend([
        CashFlowContribution {
            trial_id,
            branch_id,
            period,
            instance_id: slice.instance_id,
            slice_id: slice.slice_id,
            output_key: "loan_payment_interest".into(),
            transaction_id,
            category: CashFlowCategory::Operating,
            amount: month.interest.checked_neg()?,
        },
        CashFlowContribution {
            trial_id,
            branch_id,
            period,
            instance_id: slice.instance_id,
            slice_id: slice.slice_id,
            output_key: "loan_payment_principal".into(),
            transaction_id,
            category: CashFlowCategory::Financing,
            amount: month.principal.checked_neg()?,
        },
    ]);
    let closing_loan = output.state_effect.updated;
    Ok(LoanCoordinatedMonth {
        instance_id: slice.instance_id,
        slice_id: slice.slice_id,
        cash_account: slice.cash_account,
        liability_account: slice.liability_account,
        period,
        closing_loan,
        month: Some(month),
        ledger,
        cash_flow_contributions,
        events,
        transactions,
        state_effect: Some(LoanStateEffect {
            id: ids.effect_id(period, slice.instance_id, "loan_state"),
            instance_id: slice.instance_id,
            slice_id: slice.slice_id,
            period,
            previous: Some(opening_loan),
            updated: closing_loan,
            source_event_id: payment_event_id,
        }),
    })
}

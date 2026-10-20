use super::loan::{
    LoanCoordinatedMonth, LoanEvent, LoanMonthCommand, LoanMonthInput, LoanSlice, LoanStateEffect,
    cash_classified_entry, map_model_error, validate_loan_accounts,
};
use crate::{
    CashFlowContribution, FatalSimulationFailure, FatalSimulationPhase, MonthlyResult,
    OutputIdFactory, SimulationError, SliceOutput, ValidatedInput, activity::ActivitySample,
    coordinator::coordinate_month_allow_empty_with_transactions,
};
use property_simulation_accounting::{
    BalanceSide, CashFlowCategory, LedgerTransaction, LedgerTransactionSource,
};
use property_simulation_domain::Money;

/// Compose ordinary Slice outputs and loan instances into one monthly result and ledger.
/// Loan continuation states, events, postings, and cash-flow contributions are retained
/// in the same result that a caller saves as its month snapshot.
#[allow(clippy::too_many_arguments)]
pub fn coordinate_month_with_loans(
    input: &ValidatedInput,
    opening_ledger: &property_simulation_accounting::Ledger,
    outputs: Vec<SliceOutput>,
    samples: Vec<ActivitySample<crate::ActivitySampleValue>>,
    trial_id: property_simulation_domain::TrialId,
    branch_id: property_simulation_domain::BranchId,
    ids: &dyn OutputIdFactory,
    loans: Vec<LoanMonthInput>,
) -> Result<(MonthlyResult, property_simulation_accounting::Ledger), SimulationError> {
    if outputs.is_empty() && loans.is_empty() {
        return Err(SimulationError::InvalidProposal(
            "monthly coordination requires at least one active output",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut liability_accounts = std::collections::BTreeSet::new();
    let mut loan_outputs = Vec::with_capacity(loans.len());
    let mut loan_transactions = Vec::new();
    for loan_input in loans {
        if !seen.insert(loan_input.slice.instance_id)
            || !liability_accounts.insert(loan_input.slice.liability_account)
        {
            return Err(SimulationError::InvalidProposal(
                "monthly loan activity IDs and liability accounts must be unique",
            ));
        }
        let output = prepare_loan_month(
            &loan_input.slice,
            LoanMonthCommand {
                period: input.frozen_input().period,
                loan: loan_input.loan,
                update: loan_input.update,
                opening_ledger,
                trial_id,
                branch_id,
                ids,
            },
        )?;
        loan_transactions.extend(output.transactions.iter().cloned());
        loan_outputs.push(output);
    }
    let (mut result, ledger) = coordinate_month_allow_empty_with_transactions(
        input,
        opening_ledger,
        outputs,
        samples,
        trial_id,
        branch_id,
        ids,
        &loan_transactions,
    )?;
    for mut output in loan_outputs {
        let (side, balance) = ledger.balance(output.liability_account).map_err(|source| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                output.period,
                Some(trial_id),
                Some(branch_id),
                Some(output.instance_id),
                source.into(),
            ))
        })?;
        let expected = output.closing_loan.map_or_else(
            || Money::from_parts(0, 0, ledger.currency()),
            |loan| Ok(loan.opening_balance),
        )?;
        if side != BalanceSide::Credit || balance != expected {
            return Err(SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                output.period,
                Some(trial_id),
                Some(branch_id),
                Some(output.instance_id),
                SimulationError::InvalidProposal(
                    "loan closing balance does not reconcile to its liability after the monthly batch",
                ),
            )));
        }
        output.ledger = ledger.clone();
        let instance_id = output.instance_id;
        result = result.attach_loan_month(output).map_err(|source| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::MonthlyResultConstruction,
                input.frozen_input().period,
                Some(trial_id),
                Some(branch_id),
                Some(instance_id),
                source,
            ))
        })?;
    }
    // Direct callers may not go through SimulationRun's loan support preflight.
    // Refuse to return a committed-looking month whose category totals cannot fit Money.
    result.cash_flow_totals()?;
    Ok((result, ledger))
}

/// Coordinate loan postings and continuation state as an atomic monthly unit.
/// `None` is a no-op, so a run can continue after the final payment.
pub fn coordinate_loan_month(
    slice: &LoanSlice,
    command: LoanMonthCommand<'_>,
) -> Result<LoanCoordinatedMonth, SimulationError> {
    coordinate_loan_month_inner(slice, command, true)
}

fn prepare_loan_month(
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
            model_version: slice.model_version,
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
        if update.effective_period != period || update.expected_current != loan {
            return Err(SimulationError::InvalidInput(
                "loan terms update must match the active loan and effective month",
            ));
        }
        let amount = update.principal_change.abs()?;
        let event_id = ids.event_id(
            trial_id,
            branch_id,
            period,
            slice.instance_id,
            "loan_terms_update",
        );
        events.push(LoanEvent {
            id: event_id,
            instance_id: slice.instance_id,
            slice_id: slice.slice_id,
            model_version: slice.model_version,
            period,
            key: "loan_terms_update".into(),
            previous_balance: Some(loan.opening_balance),
            updated_balance: Some(update.updated.map_or(
                Money::from_parts(0, 0, loan.opening_balance.currency())?,
                |updated| updated.opening_balance,
            )),
            previous_remaining_months: Some(loan.remaining_months),
            updated_remaining_months: update.updated.map(|updated| updated.remaining_months),
            previous_annual_rate: Some(loan.annual_rate),
            updated_annual_rate: update.updated.map(|updated| updated.annual_rate),
        });
        if !amount.is_zero() {
            let (debit_account, credit_account) = if update.principal_change.is_negative() {
                (slice.liability_account, slice.cash_account)
            } else {
                (slice.cash_account, slice.liability_account)
            };
            let supplied =
                update
                    .principal_transaction
                    .as_ref()
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
                    model_version: slice.model_version,
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
                model_version: slice.model_version,
                output_key: "loan_balance_update".into(),
                transaction_id: tx_id,
                category: CashFlowCategory::Financing,
                amount: update.principal_change,
            });
            transactions.push(transaction);
        }
        if let Some(updated) = update.updated {
            updated
        } else {
            let mut ledger = opening_ledger.clone();
            if commit {
                ledger.post_batch(transactions.clone()).map_err(|source| {
                    SimulationError::Fatal(FatalSimulationFailure::new(
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
                        SimulationError::Fatal(FatalSimulationFailure::new(
                            FatalSimulationPhase::ClosingStateDerivation,
                            period,
                            Some(trial_id),
                            Some(branch_id),
                            Some(slice.instance_id),
                            source.into(),
                        ))
                    })?;
                if closing_side != BalanceSide::Credit || !closing_balance.is_zero() {
                    return Err(SimulationError::Fatal(FatalSimulationFailure::new(
                        FatalSimulationPhase::ClosingStateDerivation,
                        period,
                        Some(trial_id),
                        Some(branch_id),
                        Some(slice.instance_id),
                        SimulationError::InvalidProposal(
                            "settled loan update does not reconcile to zero liability",
                        ),
                    )));
                }
            } else if loan.opening_balance.checked_add(update.principal_change)?
                != Money::from_parts(0, 0, loan.opening_balance.currency())?
            {
                return Err(SimulationError::InvalidInput(
                    "settled loan update does not reconcile to zero liability",
                ));
            }
            return Ok(LoanCoordinatedMonth {
                instance_id: slice.instance_id,
                slice_id: slice.slice_id,
                model_version: slice.model_version,
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
                    id: ids.effect_id(trial_id, branch_id, period, slice.instance_id, "loan_state"),
                    instance_id: slice.instance_id,
                    slice_id: slice.slice_id,
                    model_version: slice.model_version,
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
    let transaction_id = ids.transaction_id(
        trial_id,
        branch_id,
        period,
        slice.instance_id,
        "loan_payment",
    );
    let payment_event_id = ids.event_id(
        trial_id,
        branch_id,
        period,
        slice.instance_id,
        "loan_payment",
    );
    let transaction = LedgerTransaction::new(
        transaction_id,
        period,
        output.proposal.transactions[0].entries.clone(),
    )
    .with_source(LedgerTransactionSource {
        instance_id: slice.instance_id,
        slice_id: slice.slice_id,
        model_version: slice.model_version,
        output_key: "loan_payment".into(),
        source_event_id: Some(payment_event_id),
    });
    events.push(LoanEvent {
        id: payment_event_id,
        instance_id: slice.instance_id,
        slice_id: slice.slice_id,
        model_version: slice.model_version,
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
            SimulationError::Fatal(FatalSimulationFailure::new(
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
                SimulationError::Fatal(FatalSimulationFailure::new(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    Some(trial_id),
                    Some(branch_id),
                    Some(slice.instance_id),
                    source.into(),
                ))
            })?;
        if closing_side != BalanceSide::Credit || closing_balance != month.closing_balance {
            return Err(SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                period,
                Some(trial_id),
                Some(branch_id),
                Some(slice.instance_id),
                SimulationError::InvalidProposal(
                    "loan closing balance does not match committed liability",
                ),
            )));
        }
    }
    cash_flow_contributions.extend([
        CashFlowContribution {
            trial_id,
            branch_id,
            period,
            instance_id: slice.instance_id,
            slice_id: slice.slice_id,
            model_version: slice.model_version,
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
            model_version: slice.model_version,
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
        model_version: slice.model_version,
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
            id: ids.effect_id(trial_id, branch_id, period, slice.instance_id, "loan_state"),
            instance_id: slice.instance_id,
            slice_id: slice.slice_id,
            model_version: slice.model_version,
            period,
            previous: Some(opening_loan),
            updated: closing_loan,
            source_event_id: payment_event_id,
        }),
    })
}

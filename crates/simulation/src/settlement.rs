use std::collections::{BTreeMap, BTreeSet};

use property_simulation_accounting::{LedgerTransaction, LedgerTransactionSource};
use property_simulation_domain::Money;

use crate::proposals::identify_proposal_for;
use crate::validation::LedgerRead;
use crate::{
    FatalSimulationFailure, FatalSimulationPhase, IdentifiedProposal, LoanCalculation,
    LoanContinuation, LoanEvent, LoanSlice, LoanStateEffect, MonthlyCashFlowContribution,
    MonthlyInput, MonthlyResult, MonthlySliceAdapter, OutputIdFactory, RepairCostValidation,
    SimulationError,
};

/// Calculate and settle one complete month against a private Ledger candidate.
/// Loan configurations must correspond one-to-one with the active loan activities.
pub fn calculate_monthly_result(
    input: &MonthlyInput<'_>,
    ledger_read: &impl LedgerRead,
    repair_validation: &dyn RepairCostValidation,
    adapters: &[&dyn MonthlySliceAdapter],
    loan_slices: &[LoanSlice],
    loan_calculation: &impl LoanCalculation,
    ids: &dyn OutputIdFactory,
) -> Result<MonthlyResult, SimulationError> {
    let period = input.frozen_input.period;
    if ledger_read.currency() != input.opening_ledger.currency() {
        return Err(SimulationError::InvalidInput(
            "Ledger reader currency does not match opening Ledger",
        ));
    }
    // Complete the correctable monthly and adapter preflight before invoking
    // loan mathematics or any Slice transition implementation.
    let prepared =
        crate::monthly::prepare_monthly_slices(input, ledger_read, repair_validation, adapters)?;
    let loan_activities = input
        .activities
        .iter()
        .filter(|a| a.configuration.sample_kind() == crate::ActivitySampleKind::Loan)
        .map(|a| (a.id, a))
        .collect::<BTreeMap<_, _>>();
    let continuations = input
        .loan_continuations
        .iter()
        .map(|c| (c.instance_id, c))
        .collect::<BTreeMap<_, _>>();
    let slices = loan_slices
        .iter()
        .map(|s| (s.instance_id, s))
        .collect::<BTreeMap<_, _>>();
    if loan_activities.len() != continuations.len()
        || loan_activities.len() != slices.len()
        || continuations.len() != input.loan_continuations.len()
        || slices.len() != loan_slices.len()
        || loan_activities
            .keys()
            .any(|id| !continuations.contains_key(id) || !slices.contains_key(id))
    {
        return Err(SimulationError::InvalidInput(
            "each loan activity requires one matching LoanSlice and continuation",
        ));
    }
    let liability_accounts = loan_slices
        .iter()
        .map(|slice| slice.liability_account)
        .collect::<BTreeSet<_>>();
    if liability_accounts.len() != loan_slices.len() {
        return Err(SimulationError::InvalidInput(
            "each loan activity must use its own liability account",
        ));
    }
    for (instance_id, activity) in &loan_activities {
        let slice = slices[instance_id];
        let continuation = continuations[instance_id];
        if slice.slice_id != activity.slice_id || continuation.slice_id != activity.slice_id {
            return Err(SimulationError::InvalidInput(
                "loan configuration and continuation identity mismatch",
            ));
        }
        crate::validate_loan_input(slice, continuation.loan, ledger_read, loan_calculation)?;
    }

    let mut proposal = crate::monthly::compose_prepared_monthly_slices(prepared, ids)?;

    let mut loan_events = Vec::new();
    let mut loan_effects = Vec::new();
    let mut closing_loans = Vec::new();
    let mut loan_proposals = Vec::new();
    for (instance_id, activity) in loan_activities {
        let slice = slices[&instance_id];
        let continuation = continuations[&instance_id];
        let mut events = Vec::new();
        let mut effects = Vec::new();
        let proposal_for_loan = match continuation.loan {
            Some(loan) => {
                let calculation = loan_calculation
                    .calculate_month(loan)
                    .map_err(crate::slices::map_model_error)?;
                let output = slice.propose_month(period, loan, calculation, loan_calculation)?;
                let identified = identify_proposal_for(output.proposal, ids, activity, period)?;
                for event in &identified.events {
                    events.push(LoanEvent {
                        id: event.id,
                        instance_id,
                        slice_id: activity.slice_id,
                        period,
                        key: event.key.clone(),
                        previous_balance: Some(loan.opening_balance),
                        updated_balance: Some(calculation.closing_balance),
                        previous_remaining_months: Some(loan.remaining_months),
                        updated_remaining_months: Some(calculation.remaining_months),
                        previous_annual_rate: Some(loan.annual_rate),
                        updated_annual_rate: (!calculation.closing_balance.is_zero())
                            .then_some(loan.annual_rate),
                    });
                }
                let updated = output.state_effect.updated;
                for event in &events {
                    effects.push(LoanStateEffect {
                        id: ids.effect_id(period, instance_id, "loan_payment"),
                        instance_id,
                        slice_id: activity.slice_id,
                        period,
                        previous: Some(loan),
                        updated,
                        source_event_id: event.id,
                    });
                }
                closing_loans.push(LoanContinuation {
                    instance_id,
                    slice_id: activity.slice_id,
                    loan: updated,
                });
                identified
            }
            None => {
                closing_loans.push(continuation.clone());
                IdentifiedProposal::default()
            }
        };
        loan_events.extend(events);
        loan_effects.extend(effects);
        loan_proposals.push(proposal_for_loan);
    }
    proposal = crate::merge_proposals(std::iter::once(proposal).chain(loan_proposals))?;
    let loan_event_ids = loan_events
        .iter()
        .map(|event| event.id)
        .collect::<BTreeSet<_>>();
    proposal
        .events
        .retain(|event| !loan_event_ids.contains(&event.id));
    let mut events = proposal
        .events()
        .iter()
        .map(|e| (e.id, (e.instance_id, e.slice_id)))
        .collect::<BTreeMap<_, _>>();
    events.extend(
        loan_events
            .iter()
            .map(|event| (event.id, (event.instance_id, event.slice_id))),
    );
    let mut transactions = Vec::new();
    for p in proposal.transactions() {
        transactions.push(
            LedgerTransaction::new(p.id, p.period, p.entries.clone()).with_source(
                LedgerTransactionSource {
                    instance_id: p.instance_id,
                    slice_id: p.slice_id,
                    output_key: p.key.clone(),
                    source_event_id: p.source_event_id,
                },
            ),
        );
    }
    let mut candidate = input.opening_ledger.clone();
    candidate.post_batch(transactions.clone()).map_err(|e| {
        SimulationError::Fatal(FatalSimulationFailure::new(
            FatalSimulationPhase::LedgerCommit,
            period,
            None,
            e.into(),
        ))
    })?;
    for continuation in &closing_loans {
        let slice = slices[&continuation.instance_id];
        let expected_balance = continuation
            .loan
            .map(|loan| loan.opening_balance.checked_neg())
            .transpose()
            .map_err(|error| {
                SimulationError::Fatal(FatalSimulationFailure::new(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    Some(continuation.instance_id),
                    error.into(),
                ))
            })?
            .unwrap_or_else(|| Money::new(0.into(), candidate.currency()));
        let actual_balance =
            signed_balance(&candidate, slice.liability_account).map_err(|error| {
                SimulationError::Fatal(FatalSimulationFailure::new(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    Some(continuation.instance_id),
                    error,
                ))
            })?;
        if actual_balance != expected_balance {
            return Err(SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                period,
                Some(continuation.instance_id),
                SimulationError::InvalidProposal(
                    "closing loan continuation does not match its Ledger liability balance",
                ),
            )));
        }
    }
    let transaction_ids = transactions
        .iter()
        .map(LedgerTransaction::id)
        .collect::<BTreeSet<_>>();
    let cash_flows = candidate
        .cash_flow_contributions(period, period)
        .map_err(|e| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                period,
                None,
                e.into(),
            ))
        })?
        .into_iter()
        .filter(|c| transaction_ids.contains(&c.transaction_id))
        .map(|c| {
            let source = c.source.ok_or_else(|| {
                SimulationError::Fatal(FatalSimulationFailure::new(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    None,
                    SimulationError::InvalidProposal(
                        "posted monthly transaction is missing source attribution",
                    ),
                ))
            })?;
            Ok(MonthlyCashFlowContribution {
                period: c.period,
                instance_id: source.instance_id,
                slice_id: source.slice_id,
                output_key: source.output_key,
                transaction_id: c.transaction_id,
                category: c.category,
                amount: c.amount,
            })
        })
        .collect::<Result<Vec<_>, SimulationError>>()?;
    let mut closing_state = input.opening_state.clone();
    closing_state.cash =
        signed_balance(&candidate, input.frozen_input.cash_account).map_err(|error| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                period,
                None,
                error,
            ))
        })?;
    for effect in proposal.effects() {
        if effect.target.as_str() != "property.condition" {
            return Err(SimulationError::InvalidProposal(
                "monthly result cannot apply an unknown state effect",
            ));
        }
        closing_state.condition = effect.condition;
    }
    for effect in &loan_effects {
        if !loan_events.iter().any(|e| e.id == effect.source_event_id) {
            return Err(SimulationError::InvalidProposal(
                "loan effect has no source event",
            ));
        }
    }
    // Ensure transaction/event source references survived merging before constructing the success value.
    if proposal.transactions().iter().any(|t| {
        t.source_event_id
            .is_some_and(|id| events.get(&id) != Some(&(t.instance_id, t.slice_id)))
    }) {
        return Err(SimulationError::InvalidProposal(
            "transaction source event is not in the monthly proposal",
        ));
    }
    MonthlyResult::from_settled_parts(
        period,
        input.opening_state.clone(),
        *input.frozen_input,
        input.activities.to_vec(),
        input.samples.to_vec(),
        closing_state,
        proposal,
        transactions,
        cash_flows,
        input.loan_continuations.to_vec(),
        closing_loans,
        loan_events,
        loan_effects,
    )
    .map_err(|e| {
        SimulationError::Fatal(FatalSimulationFailure::new(
            FatalSimulationPhase::MonthlyResultConstruction,
            period,
            None,
            e,
        ))
    })
}

fn signed_balance(
    ledger: &property_simulation_accounting::Ledger,
    account: property_simulation_domain::AccountId,
) -> Result<Money, SimulationError> {
    let (side, amount) = ledger.balance(account)?;
    match side {
        property_simulation_accounting::BalanceSide::Debit => Ok(amount),
        property_simulation_accounting::BalanceSide::Credit => {
            amount.checked_neg().map_err(SimulationError::from)
        }
    }
}

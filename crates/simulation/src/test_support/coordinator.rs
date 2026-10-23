use property_simulation_accounting::{BalanceSide, Ledger};
use property_simulation_domain::{BranchId, TrialId};

use crate::activity::{ActivitySample, ActivitySampleValue, SliceOutput};
use crate::composition::validate_composition;
use crate::error::{FatalSimulationFailure, FatalSimulationPhase, SimulationError};
use crate::proposals::{identify_proposal, merge_proposals};
use crate::types::{AssetCondition, OutputIdAllocator, ProposedEffect, TestMonthlyResult};
use crate::validation::{
    ValidatedInput, validate_proposal_transactions, validate_transaction_batch,
};

/// Coordinates already evaluated Slice outputs into an atomic monthly result.
/// Returns a new ledger; the caller's opening ledger is never modified.
pub fn coordinate_month(
    input: &ValidatedInput,
    opening_ledger: &Ledger,
    outputs: Vec<SliceOutput>,
    trial_id: TrialId,
    branch_id: BranchId,
    ids: &dyn OutputIdAllocator,
) -> Result<(TestMonthlyResult, Ledger), SimulationError> {
    coordinate_month_inner(
        input,
        opening_ledger,
        outputs,
        trial_id,
        branch_id,
        ids,
        &[],
    )
}

#[allow(clippy::too_many_arguments)] // Mirrors the single-month boundary and adds its unified batch.
pub(crate) fn coordinate_month_allow_empty_with_transactions(
    input: &ValidatedInput,
    opening_ledger: &Ledger,
    outputs: Vec<SliceOutput>,
    trial_id: TrialId,
    branch_id: BranchId,
    ids: &dyn OutputIdAllocator,
    additional_transactions: &[property_simulation_accounting::LedgerTransaction],
) -> Result<(TestMonthlyResult, Ledger), SimulationError> {
    coordinate_month_inner(
        input,
        opening_ledger,
        outputs,
        trial_id,
        branch_id,
        ids,
        additional_transactions,
    )
}

#[allow(clippy::too_many_arguments)]
fn coordinate_month_inner(
    input: &ValidatedInput,
    opening_ledger: &Ledger,
    outputs: Vec<SliceOutput>,
    trial_id: TrialId,
    branch_id: BranchId,
    ids: &dyn OutputIdAllocator,
    additional_transactions: &[property_simulation_accounting::LedgerTransaction],
) -> Result<(TestMonthlyResult, Ledger), SimulationError> {
    if outputs
        .iter()
        .any(|output| output.instance.configuration.is_loan())
    {
        return Err(SimulationError::InvalidProposal(
            "loan activities must be coordinated through coordinate_month_with_loans",
        ));
    }
    let period = input.frozen_input().period;
    // Identify first, then validate declarations and provenance.
    let mut declarations = Vec::with_capacity(outputs.len());
    let mut activities = Vec::with_capacity(outputs.len());
    let mut identified = Vec::with_capacity(outputs.len());
    let mut samples = Vec::with_capacity(outputs.len());
    for output in outputs {
        if output.period != period
            || output.frozen_input != *input.frozen_input()
            || output.opening_state != *input.opening_state()
        {
            return Err(SimulationError::InvalidProposal(
                "Slice output was evaluated from different opening state or frozen input",
            ));
        }
        activities.push(output.instance.clone());
        declarations.push(output.resources.clone());
        samples.push(output.sample.clone());
        identified.push(identify_proposal(output, ids)?);
    }
    crate::validation::validate_cashflow_support(input.frozen_input(), &activities)?;
    validate_sample_set(&activities, &samples, period)?;
    validate_composition(&declarations, &identified)?;
    let proposal = merge_proposals(identified)?;
    let condition = reduce_condition(input.opening_state().condition, proposal.effects())?;
    let mut transactions = proposal
        .transactions()
        .iter()
        .cloned()
        .map(|transaction| transaction.into_ledger_transaction())
        .collect::<Vec<_>>();
    transactions.extend_from_slice(additional_transactions);
    if additional_transactions.is_empty() {
        validate_proposal_transactions(opening_ledger, &proposal)
            .map_err(SimulationError::InvalidTransactions)?;
    } else {
        validate_transaction_batch(opening_ledger, &transactions)
            .map_err(SimulationError::InvalidTransactions)?;
    }
    let mut ledger = opening_ledger.clone();
    ledger.post_batch(transactions).map_err(|source| {
        SimulationError::Fatal(FatalSimulationFailure::with_run_identity(
            FatalSimulationPhase::LedgerCommit,
            period,
            Some(trial_id),
            Some(branch_id),
            None,
            source.into(),
        ))
    })?;
    let closing_cash =
        signed_balance(&ledger, input.frozen_input().cash_account).map_err(|source| {
            SimulationError::Fatal(FatalSimulationFailure::with_run_identity(
                FatalSimulationPhase::ClosingStateDerivation,
                period,
                Some(trial_id),
                Some(branch_id),
                None,
                source,
            ))
        })?;
    let closing_state = crate::types::OpeningState {
        cash: closing_cash,
        condition,
    };
    let result = TestMonthlyResult::new(
        trial_id,
        branch_id,
        period,
        input.opening_state().clone(),
        *input.frozen_input(),
        activities,
        samples,
        closing_state,
        proposal,
    )
    .map_err(|source| {
        SimulationError::Fatal(FatalSimulationFailure::with_run_identity(
            FatalSimulationPhase::MonthlyResultConstruction,
            period,
            Some(trial_id),
            Some(branch_id),
            None,
            source,
        ))
    })?;
    Ok((result, ledger))
}

fn validate_sample_set(
    activities: &[crate::types::ActivityInstance],
    samples: &[ActivitySample<ActivitySampleValue>],
    period: property_simulation_domain::YearMonth,
) -> Result<(), SimulationError> {
    use std::collections::BTreeSet;
    let ids: BTreeSet<_> = activities.iter().map(|activity| activity.id).collect();
    let mut seen = BTreeSet::new();
    if samples.len() != activities.len() {
        return Err(SimulationError::InvalidProposal(
            "each active output requires exactly one sample",
        ));
    }
    for sample in samples {
        if sample.period != period
            || !ids.contains(&sample.instance_id)
            || !seen.insert(sample.instance_id)
        {
            return Err(SimulationError::InvalidProposal(
                "sample is missing, duplicated, or has an invalid source",
            ));
        }
        let activity = activities
            .iter()
            .find(|activity| activity.id == sample.instance_id)
            .unwrap();
        if sample.slice_id != activity.slice_id
            || !sample.value.matches_configuration(&activity.configuration)
        {
            return Err(SimulationError::InvalidProposal(
                "sample provenance or kind does not match activity",
            ));
        }
    }
    Ok(())
}

fn reduce_condition(
    opening: AssetCondition,
    effects: &[ProposedEffect],
) -> Result<AssetCondition, SimulationError> {
    let mut condition = opening;
    for effect in effects {
        if effect.target.as_str() != "property.condition" {
            return Err(SimulationError::InvalidProposal(
                "unsupported Effect target",
            ));
        }
        condition = effect.condition;
    }
    Ok(condition)
}

fn signed_balance(
    ledger: &Ledger,
    account: property_simulation_domain::AccountId,
) -> Result<property_simulation_domain::Money, SimulationError> {
    let (side, amount) = ledger.balance(account)?;
    Ok(if side == BalanceSide::Debit {
        amount
    } else {
        amount.checked_neg()?
    })
}

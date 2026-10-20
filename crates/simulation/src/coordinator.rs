use property_simulation_accounting::{BalanceSide, Ledger};
use property_simulation_domain::{BranchId, TrialId};

use crate::activity::{ActivitySample, ActivitySampleValue, SliceOutput};
use crate::composition::validate_composition;
use crate::error::{FatalSimulationFailure, FatalSimulationPhase, SimulationError};
use crate::proposals::{OutputIdentity, identify_proposal, merge_proposals};
use crate::types::{AssetCondition, MonthlyResult, OutputIdFactory, ProposedEffect};
use crate::validation::{ValidatedInput, validate_proposal_transactions};

/// Coordinates already evaluated Slice outputs into an atomic monthly result.
/// Returns a new ledger; the caller's opening ledger is never modified.
pub fn coordinate_month(
    input: &ValidatedInput,
    opening_ledger: &Ledger,
    outputs: Vec<SliceOutput>,
    samples: Vec<ActivitySample<ActivitySampleValue>>,
    trial_id: TrialId,
    branch_id: BranchId,
    ids: &dyn OutputIdFactory,
) -> Result<(MonthlyResult, Ledger), SimulationError> {
    if !input.matches_opening_ledger(opening_ledger) {
        return Err(SimulationError::InvalidInput(
            "opening ledger differs from the ledger used to validate monthly input",
        ));
    }
    if outputs.is_empty() {
        return Err(SimulationError::InvalidProposal(
            "monthly coordination requires at least one active output",
        ));
    }
    let period = input.frozen_input().period;
    let identity = OutputIdentity {
        trial: trial_id,
        branch: branch_id,
        factory: ids,
    };
    // Identify first, then validate declarations and provenance.
    let mut declarations = Vec::with_capacity(outputs.len());
    let mut activities = Vec::with_capacity(outputs.len());
    let mut identified = Vec::with_capacity(outputs.len());
    let mut evaluated_samples = Vec::with_capacity(outputs.len());
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
        evaluated_samples.push(output.sample.clone());
        identified.push(identify_proposal(output, identity)?);
    }
    validate_sample_set(&activities, &samples, period)?;
    for sample in &samples {
        if !evaluated_samples
            .iter()
            .any(|evaluated| evaluated == sample)
        {
            return Err(SimulationError::InvalidProposal(
                "result sample differs from the value used to evaluate its Slice",
            ));
        }
    }
    validate_composition(&declarations, &identified)?;
    let proposal = merge_proposals(identified)?;
    let condition = reduce_condition(input.opening_state().condition, proposal.effects())?;
    validate_proposal_transactions(opening_ledger, &proposal)
        .map_err(SimulationError::InvalidTransactions)?;

    let mut ledger = opening_ledger.clone();
    let transactions = proposal
        .transactions()
        .iter()
        .cloned()
        .map(|transaction| transaction.into_ledger_transaction())
        .collect::<Vec<_>>();
    ledger.post_batch(transactions).map_err(|source| {
        SimulationError::Fatal(FatalSimulationFailure::new(
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
            SimulationError::Fatal(FatalSimulationFailure::new(
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
    let result = MonthlyResult::new(
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
        SimulationError::Fatal(FatalSimulationFailure::new(
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
            || sample.version != activity.version
            || !matches!(
                (&sample.value, activity.sample_kind),
                (
                    ActivitySampleValue::Rental,
                    crate::types::ActivitySampleKind::Rental
                ) | (
                    ActivitySampleValue::Repair(_),
                    crate::types::ActivitySampleKind::Repair
                )
            )
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

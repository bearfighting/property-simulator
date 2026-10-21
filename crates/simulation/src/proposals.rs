use std::collections::{BTreeMap, BTreeSet};

use property_simulation_domain::{BranchId, TrialId, YearMonth};

use crate::activity::SliceOutput;
use crate::error::SimulationError;
use crate::types::{
    ActivityInstance, IdentifiedProposal, OutputIdFactory, ProposedEffect, ProposedEvent,
    ProposedTransaction, SliceProposal,
};

/// Caller supplied identity context used to create stable output IDs.
#[derive(Clone, Copy)]
pub struct OutputIdentity<'a> {
    pub trial: TrialId,
    pub branch: BranchId,
    pub factory: &'a dyn OutputIdFactory,
}

/// Assign IDs centrally. The instance is supplied by the coordinator, not by
/// proposal data, and every draft must belong to the frozen input period.
pub fn identify_proposal(
    output: SliceOutput,
    identity: OutputIdentity<'_>,
) -> Result<IdentifiedProposal, SimulationError> {
    identify_proposal_for(output.proposal, identity, &output.instance, output.period)
}

pub(crate) fn identify_proposal_for(
    proposal: SliceProposal,
    identity: OutputIdentity<'_>,
    instance: &ActivityInstance,
    period: YearMonth,
) -> Result<IdentifiedProposal, SimulationError> {
    if proposal.events.iter().any(|draft| draft.period != period)
        || proposal.effects.iter().any(|draft| draft.period != period)
        || proposal
            .transactions
            .iter()
            .any(|draft| draft.period != period)
    {
        return Err(SimulationError::InvalidProposal(
            "proposal period does not match frozen period",
        ));
    }
    let events: Vec<ProposedEvent> = proposal
        .events
        .into_iter()
        .map(|event| ProposedEvent {
            id: identity.factory.event_id(
                identity.trial,
                identity.branch,
                period,
                instance.id,
                &event.key,
            ),
            instance_id: instance.id,
            slice_id: instance.slice_id,
            period,
            key: event.key,
        })
        .collect();
    let event_ids_by_key = events
        .iter()
        .map(|event| (event.key.clone(), event.id))
        .collect::<BTreeMap<_, _>>();
    let effects = proposal
        .effects
        .into_iter()
        .map(|effect| {
            let source_event_id = match effect.source_event_key.as_ref() {
                Some(key) => Some(*event_ids_by_key.get(key).ok_or(
                    SimulationError::InvalidProposal("effect references missing event key"),
                )?),
                None => None,
            };
            Ok(ProposedEffect {
                id: identity.factory.effect_id(
                    identity.trial,
                    identity.branch,
                    period,
                    instance.id,
                    &effect.key,
                ),
                instance_id: instance.id,
                slice_id: instance.slice_id,
                period,
                target: effect.target,
                key: effect.key,
                source_event_id,
                condition: effect.condition,
            })
        })
        .collect::<Result<Vec<_>, SimulationError>>()?;
    let transactions = proposal
        .transactions
        .into_iter()
        .map(|transaction| {
            let source_event_id = match transaction.source_event_key.as_ref() {
                Some(key) => Some(*event_ids_by_key.get(key).ok_or(
                    SimulationError::InvalidProposal("transaction references missing event key"),
                )?),
                None => None,
            };
            Ok(ProposedTransaction {
                id: identity.factory.transaction_id(
                    identity.trial,
                    identity.branch,
                    period,
                    instance.id,
                    &transaction.key,
                ),
                instance_id: instance.id,
                slice_id: instance.slice_id,
                period,
                key: transaction.key,
                source_event_id,
                entries: transaction.entries,
            })
        })
        .collect::<Result<Vec<_>, SimulationError>>()?;
    Ok(IdentifiedProposal {
        identity: Some((identity.trial, identity.branch, period)),
        events,
        effects,
        transactions,
    })
}

/// Merge compatible proposals. On that domain, merge is associative and
/// commutative with an empty proposal as identity. Duplicate IDs and
/// incompatible state writes are rejected; cash postings remain separate
/// traceable transactions and are summed by Ledger.
pub fn merge_proposals(
    proposals: impl IntoIterator<Item = IdentifiedProposal>,
) -> Result<IdentifiedProposal, SimulationError> {
    let mut merged = IdentifiedProposal::default();
    let mut event_ids = BTreeSet::new();
    let mut effect_ids = BTreeSet::new();
    let mut transaction_ids = BTreeSet::new();
    let mut effect_keys = BTreeMap::new();
    let mut identity = None;
    for mut p in proposals {
        if let Some(proposal_identity) = p.identity {
            if identity.is_some_and(|existing| existing != proposal_identity) {
                return Err(SimulationError::InvalidProposal(
                    "cannot merge outputs from different trials, branches, or periods",
                ));
            }
            identity = Some(proposal_identity);
        }
        for t in &p.transactions {
            if !transaction_ids.insert(t.id) {
                return Err(SimulationError::DuplicateOutputId);
            }
        }
        for e in &p.events {
            if !event_ids.insert(e.id) {
                return Err(SimulationError::DuplicateOutputId);
            }
        }
        for e in &p.effects {
            if !effect_ids.insert(e.id) {
                return Err(SimulationError::DuplicateOutputId);
            }
        }
        for e in &p.effects {
            if effect_keys
                .insert((e.period, e.target.clone()), e.condition)
                .is_some_and(|old| old != e.condition)
            {
                return Err(SimulationError::InvalidProposal(
                    "conflicting state effects",
                ));
            }
        }
        merged.events.append(&mut p.events);
        merged.effects.append(&mut p.effects);
        merged.transactions.append(&mut p.transactions);
    }
    merged.events.sort_by_key(|e| e.id);
    merged.effects.sort_by_key(|e| e.id);
    merged.transactions.sort_by_key(|t| t.id);
    merged.identity = identity;
    Ok(merged)
}

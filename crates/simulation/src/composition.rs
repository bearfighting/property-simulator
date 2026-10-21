use std::collections::{BTreeMap, BTreeSet};

use property_simulation_domain::{ActivityInstanceId, ResourceKey, SliceId};

use crate::{error::SimulationError, types::IdentifiedProposal};

/// Resource declarations attached to one active instance. Slice instances
/// evaluate independently against the same opening state; no dependency API
/// exists between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceResources {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub reads: BTreeSet<ResourceKey>,
    pub writes: BTreeSet<ResourceKey>,
}

/// Check unique instances and declared output writes. Every instance observes
/// the same immutable opening state, so read/write overlap does not create an
/// ordering relation. Incompatible writes are rejected by proposal merging.
pub fn validate_composition(
    resources: &[SliceResources],
    proposals: &[IdentifiedProposal],
) -> Result<(), SimulationError> {
    let by_id: BTreeMap<_, _> = resources.iter().map(|r| (r.instance_id, r)).collect();
    if by_id.len() != resources.len() {
        return Err(SimulationError::InvalidInput(
            "duplicate activity instance in composition",
        ));
    }
    for proposal in proposals {
        for (instance_id, slice_id) in proposal
            .events()
            .iter()
            .map(|output| (output.instance_id, output.slice_id))
            .chain(
                proposal
                    .effects()
                    .iter()
                    .map(|output| (output.instance_id, output.slice_id)),
            )
            .chain(
                proposal
                    .transactions()
                    .iter()
                    .map(|output| (output.instance_id, output.slice_id)),
            )
        {
            if by_id
                .get(&instance_id)
                .is_none_or(|declaration| declaration.slice_id != slice_id)
            {
                return Err(SimulationError::InvalidProposal(
                    "proposal provenance does not match resource declaration",
                ));
            }
        }
        for effect in proposal.effects() {
            let declaration = by_id[&effect.instance_id];
            if !declaration.writes.contains(&effect.target) {
                return Err(SimulationError::InvalidProposal(
                    "effect writes undeclared resource",
                ));
            }
        }
        for transaction in proposal.transactions() {
            let declaration = &by_id[&transaction.instance_id];
            if transaction.entries.iter().any(|entry| {
                !declaration
                    .writes
                    .contains(&ResourceKey::for_account(entry.account_id()))
            }) {
                return Err(SimulationError::InvalidProposal(
                    "transaction writes undeclared account resource",
                ));
            }
        }
    }
    Ok(())
}

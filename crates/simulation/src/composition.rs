use std::collections::{BTreeMap, BTreeSet};

use property_simulation_domain::{
    AccountId, ActivityInstanceId, ModelVersionId, ResourceKey, SliceId,
};

use crate::{error::SimulationError, types::IdentifiedProposal};

/// Resource declarations attached to one active instance. Slice instances
/// evaluate independently against the same opening state; no dependency API
/// exists between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceResources {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub version: ModelVersionId,
    pub reads: BTreeSet<ResourceKey>,
    pub writes: BTreeSet<ResourceKey>,
}

/// Canonical resource key for an account changed by a proposed journal entry.
pub fn account_resource_key(account: AccountId) -> ResourceKey {
    let mut key = String::from("account:");
    for byte in account.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut key, "{byte:02x}").expect("writing to String cannot fail");
    }
    ResourceKey::new(key).expect("account resource key is non-empty")
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
        for (instance_id, slice_id, version) in proposal
            .events()
            .iter()
            .map(|output| (output.instance_id, output.slice_id, output.version))
            .chain(
                proposal
                    .effects()
                    .iter()
                    .map(|output| (output.instance_id, output.slice_id, output.version)),
            )
            .chain(
                proposal
                    .transactions()
                    .iter()
                    .map(|output| (output.instance_id, output.slice_id, output.version)),
            )
        {
            if by_id.get(&instance_id).is_none_or(|declaration| {
                declaration.slice_id != slice_id || declaration.version != version
            }) {
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
                    .contains(&account_resource_key(entry.account_id()))
            }) {
                return Err(SimulationError::InvalidProposal(
                    "transaction writes undeclared account resource",
                ));
            }
        }
    }
    Ok(())
}

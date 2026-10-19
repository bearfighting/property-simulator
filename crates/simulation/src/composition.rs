use std::collections::{BTreeMap, BTreeSet};

use property_simulation_domain::{
    AccountId, ActivityInstanceId, ModelVersionId, ResourceKey, SliceId,
};

use crate::{error::SimulationError, types::IdentifiedProposal};

/// Resource and dependency declarations attached to one active instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceResources {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub version: ModelVersionId,
    pub reads: BTreeSet<ResourceKey>,
    pub writes: BTreeSet<ResourceKey>,
    pub dependencies: BTreeSet<ActivityInstanceId>,
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

/// Deterministic topological execution stages. Each stage is sorted by SliceId,
/// then instance ID; members of a stage can be evaluated independently.
pub fn plan_slice_stages(
    resources: &[SliceResources],
) -> Result<Vec<Vec<ActivityInstanceId>>, SimulationError> {
    let by_id: BTreeMap<_, _> = resources.iter().map(|r| (r.instance_id, r)).collect();
    if by_id.len() != resources.len() {
        return Err(SimulationError::InvalidInput(
            "duplicate activity instance in composition",
        ));
    }
    for resource in resources {
        if resource
            .dependencies
            .iter()
            .any(|dep| !by_id.contains_key(dep))
        {
            return Err(SimulationError::InvalidInput(
                "slice dependency references missing activity",
            ));
        }
    }
    let mut remaining: BTreeSet<_> = by_id.keys().copied().collect();
    let mut completed = BTreeSet::new();
    let mut stages = Vec::new();
    while !remaining.is_empty() {
        let mut ready: Vec<_> = remaining
            .iter()
            .copied()
            .filter(|id| by_id[id].dependencies.is_subset(&completed))
            .collect();
        if ready.is_empty() {
            return Err(SimulationError::InvalidInput(
                "slice dependency graph contains a cycle",
            ));
        }
        ready.sort_by_key(|id| (by_id[id].slice_id, *id));
        for id in &ready {
            remaining.remove(id);
            completed.insert(*id);
        }
        stages.push(ready);
    }
    Ok(stages)
}

/// Check declared effect writes and ensure shared read/write resources are
/// ordered through an explicit dependency path.
pub fn validate_composition(
    resources: &[SliceResources],
    proposals: &[IdentifiedProposal],
) -> Result<(), SimulationError> {
    plan_slice_stages(resources)?;
    let by_id: BTreeMap<_, _> = resources.iter().map(|r| (r.instance_id, r)).collect();
    let mut ancestors: BTreeMap<_, BTreeSet<_>> = resources
        .iter()
        .map(|r| (r.instance_id, r.dependencies.clone()))
        .collect();
    // Compute transitive dependency closure. Merely landing in different
    // topological stages does not imply an ordering between two instances.
    loop {
        let mut changed = false;
        for resource in resources {
            let inherited: BTreeSet<_> = ancestors[&resource.instance_id]
                .iter()
                .flat_map(|dependency| ancestors[dependency].iter().copied())
                .collect();
            let reachable = ancestors.get_mut(&resource.instance_id).unwrap();
            let old_len = reachable.len();
            reachable.extend(inherited);
            changed |= reachable.len() != old_len;
        }
        if !changed {
            break;
        }
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
    for (i, a) in resources.iter().enumerate() {
        for b in resources.iter().skip(i + 1) {
            // Write/write proposals have an explicit reducer: identical state
            // values are idempotent; differing values are rejected by merge_proposals.
            let conflict = a.writes.iter().any(|r| b.reads.contains(r))
                || b.writes.iter().any(|r| a.reads.contains(r));
            let explicitly_ordered = ancestors[&a.instance_id].contains(&b.instance_id)
                || ancestors[&b.instance_id].contains(&a.instance_id);
            if conflict && !explicitly_ordered {
                return Err(SimulationError::InvalidInput(
                    "conflicting resource access lacks an explicit dependency",
                ));
            }
        }
    }
    Ok(())
}

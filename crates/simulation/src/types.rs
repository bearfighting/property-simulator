use std::collections::BTreeMap;

use property_simulation_accounting::{LedgerEntry, LedgerTransaction, LedgerTransactionSource};
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, EffectId, EventId, LedgerTransactionId,
    ModelVersionId, Money, ResourceKey, SliceId, TrialId, YearMonth,
};
pub use property_simulation_models::RepairSample;

use crate::activity::{ActivitySample, ActivitySampleValue};
use crate::error::SimulationError;
use crate::proposals::merge_proposals;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetCondition {
    Available,
    NeedsRepair,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningState {
    pub cash: Money,
    pub condition: AssetCondition,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrozenInput {
    pub period: YearMonth,
    pub rent: Money,
    pub rent_account: AccountId,
    pub cash_account: AccountId,
    pub repair_expense_account: AccountId,
    pub repair_cost_limit: Money,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityInstance {
    pub id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub version: ModelVersionId,
    pub sample_kind: ActivitySampleKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivitySampleKind {
    Rental,
    Repair,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventProposal {
    pub period: YearMonth,
    pub key: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectProposal {
    pub period: YearMonth,
    /// Stable resource key identifying the state this effect writes.
    pub target: ResourceKey,
    pub key: String,
    /// Stable event key within this activity instance, if this effect was caused by an event.
    pub source_event_key: Option<String>,
    pub condition: AssetCondition,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionProposal {
    pub key: String,
    pub period: YearMonth,
    pub entries: Vec<LedgerEntry>,
    /// Stable event key within this activity instance, if this posting is caused by an event.
    pub source_event_key: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SliceProposal {
    pub events: Vec<EventProposal>,
    pub effects: Vec<EffectProposal>,
    pub transactions: Vec<TransactionProposal>,
}

/// A proposal after IDs have been assigned by the coordination boundary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdentifiedProposal {
    pub(crate) identity: Option<(TrialId, BranchId, YearMonth)>,
    pub(crate) events: Vec<ProposedEvent>,
    pub(crate) effects: Vec<ProposedEffect>,
    pub(crate) transactions: Vec<ProposedTransaction>,
}
impl IdentifiedProposal {
    pub fn identity(&self) -> Option<(TrialId, BranchId, YearMonth)> {
        self.identity
    }
    pub fn events(&self) -> &[ProposedEvent] {
        &self.events
    }
    pub fn effects(&self) -> &[ProposedEffect] {
        &self.effects
    }
    pub fn transactions(&self) -> &[ProposedTransaction] {
        &self.transactions
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedTransaction {
    pub id: LedgerTransactionId,
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub version: ModelVersionId,
    pub period: YearMonth,
    pub key: String,
    pub source_event_id: Option<EventId>,
    pub entries: Vec<LedgerEntry>,
}
impl ProposedTransaction {
    pub fn into_ledger_transaction(self) -> LedgerTransaction {
        LedgerTransaction::new(self.id, self.period, self.entries).with_source(
            LedgerTransactionSource {
                instance_id: self.instance_id,
                slice_id: self.slice_id,
                model_version: self.version,
                output_key: self.key,
                source_event_id: self.source_event_id,
            },
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedEvent {
    pub id: EventId,
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub version: ModelVersionId,
    pub period: YearMonth,
    pub key: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedEffect {
    pub id: EffectId,
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub version: ModelVersionId,
    pub period: YearMonth,
    pub target: ResourceKey,
    pub key: String,
    pub source_event_id: Option<EventId>,
    pub condition: AssetCondition,
}

/// The caller's implementation must be deterministic and collision-resistant
/// over the supplied identity tuple. Duplicate IDs in a monthly result are rejected.
pub trait OutputIdFactory {
    fn event_id(
        &self,
        trial: TrialId,
        branch: BranchId,
        period: YearMonth,
        instance: ActivityInstanceId,
        key: &str,
    ) -> EventId;
    fn effect_id(
        &self,
        trial: TrialId,
        branch: BranchId,
        period: YearMonth,
        instance: ActivityInstanceId,
        key: &str,
    ) -> EffectId;
    fn transaction_id(
        &self,
        trial: TrialId,
        branch: BranchId,
        period: YearMonth,
        instance: ActivityInstanceId,
        key: &str,
    ) -> LedgerTransactionId;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyResult {
    trial_id: TrialId,
    branch_id: BranchId,
    period: YearMonth,
    opening_state: OpeningState,
    frozen_input: FrozenInput,
    activities: Vec<ActivityInstance>,
    samples: Vec<ActivitySample<ActivitySampleValue>>,
    closing_state: OpeningState,
    proposal: IdentifiedProposal,
}
impl MonthlyResult {
    // These fields form one validated period result; keeping them in one constructor
    // makes it impossible to omit the identity or any of the transition inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        trial_id: TrialId,
        branch_id: BranchId,
        period: YearMonth,
        opening_state: OpeningState,
        frozen_input: FrozenInput,
        activities: Vec<ActivityInstance>,
        samples: Vec<ActivitySample<ActivitySampleValue>>,
        closing_state: OpeningState,
        proposal: IdentifiedProposal,
    ) -> Result<Self, SimulationError> {
        // A result is itself a commit boundary: validate uniqueness and canonical
        // ordering even if the caller did not explicitly merge the proposal.
        let proposal = merge_proposals([proposal])?;
        let mut activity_by_id = BTreeMap::new();
        for activity in &activities {
            if activity_by_id.insert(activity.id, activity).is_some() {
                return Err(SimulationError::InvalidProposal(
                    "monthly result contains duplicate activity instance IDs",
                ));
            }
        }
        let source_matches = |instance_id, slice_id, version| {
            activity_by_id.get(&instance_id).is_some_and(|activity| {
                activity.slice_id == slice_id && activity.version == version
            })
        };
        let sample_kind_matches = |sample: &ActivitySample<ActivitySampleValue>| {
            activity_by_id
                .get(&sample.instance_id)
                .is_some_and(|activity| {
                    matches!(
                        (&sample.value, activity.sample_kind),
                        (ActivitySampleValue::Rental, ActivitySampleKind::Rental)
                            | (ActivitySampleValue::Repair(_), ActivitySampleKind::Repair)
                    )
                })
        };
        if frozen_input.period != period
            || samples.iter().any(|sample| sample.period != period)
            || samples
                .iter()
                .any(|sample| !source_matches(sample.instance_id, sample.slice_id, sample.version))
            || samples.iter().any(|sample| !sample_kind_matches(sample))
            || proposal.events.iter().any(|output| output.period != period)
            || proposal
                .events
                .iter()
                .any(|output| !source_matches(output.instance_id, output.slice_id, output.version))
            || proposal
                .effects
                .iter()
                .any(|output| output.period != period)
            || proposal
                .effects
                .iter()
                .any(|output| !source_matches(output.instance_id, output.slice_id, output.version))
            || proposal
                .transactions
                .iter()
                .any(|output| output.period != period)
            || proposal
                .transactions
                .iter()
                .any(|output| !source_matches(output.instance_id, output.slice_id, output.version))
            || proposal
                .identity
                .is_some_and(|identity| identity != (trial_id, branch_id, period))
        {
            return Err(SimulationError::InvalidProposal(
                "monthly result identity or period does not match its contents",
            ));
        }
        Ok(Self {
            trial_id,
            branch_id,
            period,
            opening_state,
            frozen_input,
            activities,
            samples,
            closing_state,
            proposal,
        })
    }
    pub const fn trial_id(&self) -> TrialId {
        self.trial_id
    }
    pub const fn branch_id(&self) -> BranchId {
        self.branch_id
    }
    pub const fn period(&self) -> YearMonth {
        self.period
    }
    pub fn opening_state(&self) -> &OpeningState {
        &self.opening_state
    }
    pub fn frozen_input(&self) -> &FrozenInput {
        &self.frozen_input
    }
    pub fn activities(&self) -> &[ActivityInstance] {
        &self.activities
    }
    pub fn samples(&self) -> &[ActivitySample<ActivitySampleValue>] {
        &self.samples
    }
    pub fn closing_state(&self) -> &OpeningState {
        &self.closing_state
    }
    pub fn proposal(&self) -> &IdentifiedProposal {
        &self.proposal
    }
}

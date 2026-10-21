use std::collections::BTreeMap;

use property_simulation_accounting::{
    CashFlowCategory, LedgerEntry, LedgerTransaction, LedgerTransactionSource,
};
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, EffectId, EventId, LedgerTransactionId,
    ModelVersionId, Money, ResourceKey, SliceId, TrialId, YearMonth,
};
pub use property_simulation_models::RepairSample;

use crate::activity::{ActivitySample, ActivitySampleValue};
use crate::error::SimulationError;
use crate::proposals::merge_proposals;
use crate::slices::{LoanContinuation, LoanCoordinatedMonth, LoanEvent, LoanStateEffect};

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
    pub configuration: ActivityConfiguration,
}
/// Per-instance model configuration. The variant is the activity/sample kind,
/// so it cannot disagree with the configuration carried into snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityConfiguration {
    Rental,
    Repair,
    Loan,
    PropertyTax(PropertyTaxConfig),
    Utility(UtilityConfig),
}
impl ActivityConfiguration {
    pub const fn sample_kind(&self) -> ActivitySampleKind {
        match self {
            Self::Rental => ActivitySampleKind::Rental,
            Self::Repair => ActivitySampleKind::Repair,
            Self::Loan => ActivitySampleKind::Loan,
            Self::PropertyTax(_) => ActivitySampleKind::PropertyTax,
            Self::Utility(_) => ActivitySampleKind::Utility,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyTaxCadence {
    Monthly,
    Quarterly,
    Annual,
}
impl PropertyTaxCadence {
    pub const fn months(self) -> i32 {
        match self {
            Self::Monthly => 1,
            Self::Quarterly => 3,
            Self::Annual => 12,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyTaxConfig {
    pub amount: Money,
    pub cadence: PropertyTaxCadence,
    pub first_due: YearMonth,
    pub cash_account: AccountId,
    pub expense_account: AccountId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilityConfig {
    pub usage_unit: String,
    pub maximum_usage: u64,
    pub rate_per_unit: Money,
    pub cash_account: AccountId,
    pub expense_account: AccountId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivitySampleKind {
    Rental,
    Repair,
    Loan,
    PropertyTax,
    Utility,
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
    cash_flow_contributions: Vec<CashFlowContribution>,
    input_postings: Vec<RunInputPosting>,
    input_cash_flow_contributions: Vec<InputCashFlowContribution>,
    input_updates: Vec<crate::RunUpdate>,
    input_effects: Vec<RunInputEffect>,
    loan_continuations: Vec<LoanContinuation>,
    loan_events: Vec<LoanEvent>,
    loan_effects: Vec<LoanStateEffect>,
}

/// A balanced month-start transaction supplied with a run update. These postings
/// are recorded separately from Slice outputs because they have no Slice identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInputPosting {
    pub source: String,
    pub transaction: LedgerTransaction,
}

/// State change caused by an economically accounted month-start adjustment.
/// Its identity is deterministically derived from the caller-supplied transaction ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInputEffect {
    pub id: EffectId,
    pub source: String,
    pub period: YearMonth,
    pub previous: OpeningState,
    pub updated: OpeningState,
    pub transaction_id: LedgerTransactionId,
}

/// Cash-flow impact of a month-start input posting, attributed to its update source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputCashFlowContribution {
    pub source: String,
    pub period: YearMonth,
    pub transaction_id: property_simulation_domain::LedgerTransactionId,
    pub category: CashFlowCategory,
    pub amount: Money,
}

/// Net signed cash impact for one committed transaction and cash-flow category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashFlowContribution {
    pub trial_id: TrialId,
    pub branch_id: BranchId,
    pub period: YearMonth,
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub output_key: String,
    pub transaction_id: LedgerTransactionId,
    pub category: CashFlowCategory,
    /// Cash debits are positive; cash credits are negative.
    pub amount: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashFlowContributionTotals {
    pub operating: Money,
    pub investing: Money,
    pub financing: Money,
}

impl CashFlowContributionTotals {
    pub fn zero(currency: property_simulation_domain::Currency) -> Self {
        let zero = Money::from_parts(0, 0, currency).expect("zero is representable");
        Self {
            operating: zero,
            investing: zero,
            financing: zero,
        }
    }
    /// Sum signed contributions exactly, checking the `Money` range only after cancellation.
    pub fn sum(
        amounts: impl IntoIterator<Item = (CashFlowCategory, Money)>,
        currency: property_simulation_domain::Currency,
    ) -> Result<Self, SimulationError> {
        let mut buckets = [Vec::new(), Vec::new(), Vec::new()];
        for (category, amount) in amounts {
            buckets[match category {
                CashFlowCategory::Operating => 0,
                CashFlowCategory::Investing => 1,
                CashFlowCategory::Financing => 2,
            }]
            .push(amount);
        }
        Ok(Self {
            operating: exact_signed_sum(buckets[0].iter().copied(), currency)?,
            investing: exact_signed_sum(buckets[1].iter().copied(), currency)?,
            financing: exact_signed_sum(buckets[2].iter().copied(), currency)?,
        })
    }
}
impl MonthlyResult {
    // These fields form one validated period result; keeping them in one constructor
    // makes it impossible to omit the identity or any of the transition inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
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
        let mut activities = activities;
        activities.sort_by_key(|activity| activity.id);
        let mut samples = samples;
        samples.sort_by_key(|sample| sample.instance_id);
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
                        (&sample.value, activity.configuration.sample_kind()),
                        (ActivitySampleValue::Rental, ActivitySampleKind::Rental)
                            | (ActivitySampleValue::Repair(_), ActivitySampleKind::Repair)
                            | (ActivitySampleValue::Loan(_), ActivitySampleKind::Loan)
                            | (
                                ActivitySampleValue::PropertyTax,
                                ActivitySampleKind::PropertyTax
                            )
                            | (ActivitySampleValue::Utility(_), ActivitySampleKind::Utility)
                    )
                })
        };
        if frozen_input.period != period
            || opening_state.cash.currency() != frozen_input.rent.currency()
            || closing_state.cash.currency() != opening_state.cash.currency()
            || samples.len() != activities.len()
            || samples
                .iter()
                .map(|sample| sample.instance_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != samples.len()
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
        let mut cash_flow_contributions = Vec::new();
        for transaction in &proposal.transactions {
            let mut grouped = BTreeMap::<CashFlowCategory, Vec<Money>>::new();
            for entry in &transaction.entries {
                if let Some(category) = entry.cash_flow_category() {
                    let signed = match entry.side() {
                        property_simulation_accounting::BalanceSide::Debit => entry.amount(),
                        property_simulation_accounting::BalanceSide::Credit => {
                            entry.amount().checked_neg()?
                        }
                    };
                    grouped.entry(category).or_default().push(signed);
                }
            }
            for (category, amounts) in grouped {
                let amount = sum_contribution_amounts(amounts, opening_state.cash.currency())?;
                let contribution = CashFlowContribution {
                    trial_id,
                    branch_id,
                    period,
                    instance_id: transaction.instance_id,
                    slice_id: transaction.slice_id,
                    model_version: transaction.version,
                    output_key: transaction.key.clone(),
                    transaction_id: transaction.id,
                    category,
                    amount,
                };
                cash_flow_contributions.push(contribution);
            }
        }
        cash_flow_contributions.sort_by_key(|c| (c.transaction_id, c.category));
        let mut actual_support = BTreeMap::<ActivityInstanceId, Money>::new();
        for contribution in &cash_flow_contributions {
            let amount = contribution.amount.abs()?;
            let total = actual_support
                .entry(contribution.instance_id)
                .or_insert(Money::from_parts(0, 0, opening_state.cash.currency())?);
            *total = total.checked_add(amount)?;
        }
        for activity in &activities {
            let bound = match activity.configuration.sample_kind() {
                ActivitySampleKind::Rental => frozen_input.rent,
                ActivitySampleKind::Repair => frozen_input.repair_cost_limit,
                ActivitySampleKind::Loan => continue,
                ActivitySampleKind::PropertyTax => match &activity.configuration {
                    ActivityConfiguration::PropertyTax(config)
                        if crate::validation::tax_due(period, config.first_due, config.cadence) =>
                    {
                        config.amount
                    }
                    _ => Money::from_parts(0, 0, opening_state.cash.currency())?,
                },
                ActivitySampleKind::Utility => match &activity.configuration {
                    ActivityConfiguration::Utility(config) => {
                        config.rate_per_unit.checked_mul_u64(config.maximum_usage)?
                    }
                    _ => Money::from_parts(0, 0, opening_state.cash.currency())?,
                },
            }
            .rounded_for_posting(property_simulation_domain::RoundingPolicy::HalfUp)?;
            if actual_support
                .get(&activity.id)
                .copied()
                .unwrap_or(Money::from_parts(0, 0, opening_state.cash.currency())?)
                .checked_cmp(bound)?
                == std::cmp::Ordering::Greater
            {
                return Err(SimulationError::InvalidProposal(
                    "activity cash-flow contributions exceed their validated support",
                ));
            }
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
            cash_flow_contributions,
            input_postings: Vec::new(),
            input_cash_flow_contributions: Vec::new(),
            input_updates: Vec::new(),
            input_effects: Vec::new(),
            loan_continuations: Vec::new(),
            loan_events: Vec::new(),
            loan_effects: Vec::new(),
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
    pub fn cash_flow_contributions(&self) -> &[CashFlowContribution] {
        &self.cash_flow_contributions
    }
    pub fn input_postings(&self) -> &[RunInputPosting] {
        &self.input_postings
    }
    pub fn input_cash_flow_contributions(&self) -> &[InputCashFlowContribution] {
        &self.input_cash_flow_contributions
    }
    pub fn input_updates(&self) -> &[crate::RunUpdate] {
        &self.input_updates
    }
    pub fn input_effects(&self) -> &[RunInputEffect] {
        &self.input_effects
    }

    pub(crate) fn with_input_updates(
        mut self,
        updates: Vec<crate::RunUpdate>,
        effects: Vec<RunInputEffect>,
    ) -> Result<Self, SimulationError> {
        if updates
            .iter()
            .any(|update| update.source.trim().is_empty() || update.effective_month != self.period)
        {
            return Err(SimulationError::InvalidProposal(
                "completed input updates must have a source and matching period",
            ));
        }
        let postings = updates
            .iter()
            .filter_map(|update| match &update.value {
                crate::RunUpdateValue::StateAdjustment { transaction, .. } => {
                    Some(RunInputPosting {
                        source: update.source.clone(),
                        transaction: transaction.clone(),
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut seen = std::collections::BTreeSet::new();
        let mut contributions = Vec::new();
        for posting in &postings {
            if posting.source.trim().is_empty()
                || posting.transaction.period() != self.period
                || !seen.insert(posting.transaction.id())
            {
                return Err(SimulationError::InvalidProposal(
                    "run input postings must have a unique ID, source, and matching period",
                ));
            }
            let mut grouped = BTreeMap::<CashFlowCategory, Vec<Money>>::new();
            for entry in posting.transaction.entries() {
                if let Some(category) = entry.cash_flow_category() {
                    let signed = match entry.side() {
                        property_simulation_accounting::BalanceSide::Debit => entry.amount(),
                        property_simulation_accounting::BalanceSide::Credit => {
                            entry.amount().checked_neg()?
                        }
                    };
                    grouped.entry(category).or_default().push(signed);
                }
            }
            for (category, amounts) in grouped {
                contributions.push(InputCashFlowContribution {
                    source: posting.source.clone(),
                    period: self.period,
                    transaction_id: posting.transaction.id(),
                    category,
                    amount: exact_signed_sum(amounts, self.opening_state.cash.currency())?,
                });
            }
        }
        let mut effect_ids = std::collections::BTreeSet::new();
        if effects.iter().any(|effect| {
            effect.period != self.period
                || effect.source.trim().is_empty()
                || !effect_ids.insert(effect.id)
                || self
                    .proposal
                    .effects
                    .iter()
                    .any(|existing| existing.id == effect.id)
                || !postings.iter().any(|posting| {
                    posting.source == effect.source
                        && posting.transaction.id() == effect.transaction_id
                })
        }) {
            return Err(SimulationError::InvalidProposal(
                "run input Effects must be unique and linked to a matching posting",
            ));
        }
        self.input_postings = postings;
        self.input_cash_flow_contributions = contributions;
        self.input_updates = updates;
        self.input_effects = effects;
        Ok(self)
    }
    pub fn loan_continuations(&self) -> &[LoanContinuation] {
        &self.loan_continuations
    }
    pub fn loan_events(&self) -> &[LoanEvent] {
        &self.loan_events
    }
    pub fn loan_effects(&self) -> &[LoanStateEffect] {
        &self.loan_effects
    }

    pub(crate) fn attach_loan_month(
        &self,
        output: LoanCoordinatedMonth,
    ) -> Result<Self, SimulationError> {
        if output.period != self.period
            || self
                .loan_continuations
                .iter()
                .any(|loan| loan.instance_id == output.instance_id)
        {
            return Err(SimulationError::InvalidProposal(
                "loan month period is invalid or loan instance is duplicated",
            ));
        }
        let mut proposal = self.proposal.clone();
        for event in &output.events {
            proposal.events.push(ProposedEvent {
                id: event.id,
                instance_id: event.instance_id,
                slice_id: event.slice_id,
                version: event.model_version,
                period: event.period,
                key: event.key.clone(),
            });
        }
        for transaction in &output.transactions {
            let source = transaction
                .source()
                .ok_or(SimulationError::InvalidProposal(
                    "loan transaction is missing source metadata",
                ))?;
            if source.instance_id != output.instance_id
                || source.slice_id != output.slice_id
                || source.model_version != output.model_version
                || transaction.period() != self.period
                || source
                    .source_event_id
                    .is_none_or(|event_id| !output.events.iter().any(|event| event.id == event_id))
            {
                return Err(SimulationError::InvalidProposal(
                    "loan transaction source does not match its event and activity",
                ));
            }
            proposal.transactions.push(ProposedTransaction {
                id: transaction.id(),
                instance_id: source.instance_id,
                slice_id: source.slice_id,
                version: source.model_version,
                period: transaction.period(),
                key: source.output_key.clone(),
                source_event_id: source.source_event_id,
                entries: transaction.entries().to_vec(),
            });
        }
        let mut activities = self.activities.clone();
        let loan_activity = ActivityInstance {
            id: output.instance_id,
            slice_id: output.slice_id,
            version: output.model_version,
            configuration: ActivityConfiguration::Loan,
        };
        if activities
            .iter()
            .any(|activity| activity.id == output.instance_id)
        {
            return Err(SimulationError::InvalidProposal(
                "loan instance conflicts with an existing monthly activity",
            ));
        }
        activities.push(loan_activity.clone());
        let mut samples = self.samples.clone();
        samples.push(crate::activity::ActivitySample::new(
            &loan_activity,
            self.period,
            ActivitySampleValue::Loan(output.closing_loan),
        ));
        // OpeningState has one canonical cash account from FrozenInput. A loan may
        // post through any declared cash-position account, so do not overwrite the
        // state from that particular loan account when composing multiple accounts.
        let (cash_side, cash_balance) = output.ledger.balance(self.frozen_input.cash_account)?;
        let cash = if cash_side == property_simulation_accounting::BalanceSide::Debit {
            cash_balance
        } else {
            cash_balance.checked_neg()?
        };
        let mut closing_state = self.closing_state.clone();
        closing_state.cash = cash;
        let mut result = Self::new(
            self.trial_id,
            self.branch_id,
            self.period,
            self.opening_state.clone(),
            self.frozen_input,
            activities,
            samples,
            closing_state,
            proposal,
        )?;
        result.loan_continuations = self.loan_continuations.clone();
        result.loan_continuations.push(LoanContinuation {
            instance_id: output.instance_id,
            slice_id: output.slice_id,
            model_version: output.model_version,
            loan: output.closing_loan,
        });
        result
            .loan_continuations
            .sort_by_key(|loan| loan.instance_id);
        result.loan_events = self.loan_events.clone();
        result.loan_events.extend(output.events.iter().cloned());
        result.loan_events.sort_by_key(|event| event.id);
        result.loan_effects = self.loan_effects.clone();
        if let Some(effect) = output.state_effect {
            if effect.period != self.period
                || effect.instance_id != output.instance_id
                || effect.slice_id != output.slice_id
                || effect.model_version != output.model_version
                || !output
                    .events
                    .iter()
                    .any(|event| event.id == effect.source_event_id)
            {
                return Err(SimulationError::InvalidProposal(
                    "loan Effect identity or source does not match its monthly output",
                ));
            }
            if result
                .proposal
                .effects
                .iter()
                .any(|existing| existing.id == effect.id)
                || result
                    .loan_effects
                    .iter()
                    .any(|existing| existing.id == effect.id)
            {
                return Err(SimulationError::DuplicateOutputId);
            }
            result.loan_effects.push(effect);
            result.loan_effects.sort_by_key(|effect| effect.id);
        }
        Ok(result)
    }
    pub fn cash_flow_totals(&self) -> Result<CashFlowContributionTotals, SimulationError> {
        CashFlowContributionTotals::sum(
            self.cash_flow_contributions
                .iter()
                .map(|c| (c.category, c.amount))
                // Month-start adjustments are part of the month's Ledger cash flow,
                // even though they deliberately have no Slice/activity attribution.
                // Keep their provenance separate while including them in the total.
                .chain(
                    self.input_cash_flow_contributions
                        .iter()
                        .map(|c| (c.category, c.amount)),
                ),
            self.closing_state.cash.currency(),
        )
    }
}

/// Sum signed fixed-point values without overflowing intermediate partial sums.
/// Conversion back to `Money` is checked only after exact cancellation is complete.
pub(crate) fn exact_signed_sum(
    values: impl IntoIterator<Item = Money>,
    currency: property_simulation_domain::Currency,
) -> Result<Money, SimulationError> {
    let values = values.into_iter().collect::<Vec<_>>();
    let scale = values.iter().map(|value| value.scale()).max().unwrap_or(0);
    let mut total = DecimalAccumulator::zero();
    total.scale = scale;
    for value in values {
        if value.currency() != currency {
            return Err(property_simulation_domain::DomainError::CurrencyMismatch.into());
        }
        let mut digits = value
            .coefficient()
            .unsigned_abs()
            .to_string()
            .bytes()
            .rev()
            .map(|digit| digit - b'0')
            .collect::<Vec<_>>();
        digits.splice(
            0..0,
            std::iter::repeat_n(0, usize::from(scale - value.scale())),
        );
        total.add(value.is_negative(), digits);
    }
    let mut digits = total.digits;
    while total.scale > 0 && digits.len() > 1 && digits.first() == Some(&0) {
        digits.remove(0);
        // The accumulator is aligned to `scale`; removing low zeros reduces it.
        total.scale -= 1;
    }
    let magnitude = digits
        .iter()
        .rev()
        .map(|d| char::from(b'0' + *d))
        .collect::<String>();
    let magnitude = magnitude
        .parse::<u128>()
        .map_err(|_| property_simulation_domain::DomainError::ArithmeticOverflow)?;
    let coefficient = if total.negative {
        if magnitude == (i128::MAX as u128) + 1 {
            i128::MIN
        } else if magnitude <= i128::MAX as u128 {
            -(magnitude as i128)
        } else {
            return Err(property_simulation_domain::DomainError::ArithmeticOverflow.into());
        }
    } else {
        i128::try_from(magnitude)
            .map_err(|_| property_simulation_domain::DomainError::ArithmeticOverflow)?
    };
    Ok(Money::from_parts(coefficient, total.scale, currency)?)
}

struct DecimalAccumulator {
    negative: bool,
    digits: Vec<u8>,
    scale: u8,
}
impl DecimalAccumulator {
    fn zero() -> Self {
        Self {
            negative: false,
            digits: vec![0],
            scale: 0,
        }
    }
    fn add(&mut self, negative: bool, mut value: Vec<u8>) {
        while value.len() > 1 && value.last() == Some(&0) {
            value.pop();
        }
        if value == [0] {
            return;
        }
        if self.digits == [0] {
            self.negative = negative;
            self.digits = value;
            return;
        }
        if self.negative == negative {
            decimal_add(&mut self.digits, &value);
            return;
        }
        match decimal_compare(&self.digits, &value) {
            std::cmp::Ordering::Equal => {
                self.negative = false;
                self.digits = vec![0];
            }
            std::cmp::Ordering::Greater => decimal_subtract(&mut self.digits, &value),
            std::cmp::Ordering::Less => {
                decimal_subtract(&mut value, &self.digits);
                self.digits = value;
                self.negative = negative;
            }
        }
    }
}
fn decimal_compare(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.iter().rev().cmp(b.iter().rev()))
}
fn decimal_add(a: &mut Vec<u8>, b: &[u8]) {
    let mut carry = 0;
    for i in 0..a.len().max(b.len()) {
        if i == a.len() {
            a.push(0);
        }
        let sum = a[i] + b.get(i).copied().unwrap_or(0) + carry;
        a[i] = sum % 10;
        carry = sum / 10;
    }
    if carry != 0 {
        a.push(carry);
    }
}
fn decimal_subtract(a: &mut Vec<u8>, b: &[u8]) {
    let mut borrow = 0_i8;
    for (i, digit) in a.iter_mut().enumerate() {
        let difference = i8::try_from(*digit).unwrap()
            - i8::try_from(b.get(i).copied().unwrap_or(0)).unwrap()
            - borrow;
        if difference < 0 {
            *digit = u8::try_from(difference + 10).unwrap();
            borrow = 1;
        } else {
            *digit = u8::try_from(difference).unwrap();
            borrow = 0;
        }
    }
    while a.len() > 1 && a.last() == Some(&0) {
        a.pop();
    }
}

fn sum_contribution_amounts(
    amounts: Vec<Money>,
    currency: property_simulation_domain::Currency,
) -> Result<Money, SimulationError> {
    exact_signed_sum(amounts, currency)
}

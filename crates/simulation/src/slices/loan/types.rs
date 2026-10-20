use crate::{CashFlowContribution, OutputIdFactory, SliceProposal};
use property_simulation_accounting::{Ledger, LedgerTransaction};
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, EffectId, EventId, ModelVersionId, Money, SliceId,
    TrialId, YearMonth,
};
use property_simulation_models::{FixedRateLoan, LoanMonth};

/// Fixed-rate loan activity with an independently tracked liability account.
#[derive(Debug, Clone)]
pub struct LoanSlice {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub liability_account: AccountId,
    pub interest_expense_account: AccountId,
    pub cash_account: AccountId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanMonthOutput {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub period: YearMonth,
    pub result: LoanMonth,
    pub state_effect: LoanStateEffectProposal,
    pub proposal: SliceProposal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoanStateEffectProposal {
    pub updated: Option<FixedRateLoan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanCoordinatedMonth {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub cash_account: AccountId,
    pub liability_account: AccountId,
    pub period: YearMonth,
    pub closing_loan: Option<FixedRateLoan>,
    pub month: Option<LoanMonth>,
    pub ledger: Ledger,
    pub cash_flow_contributions: Vec<CashFlowContribution>,
    pub events: Vec<LoanEvent>,
    pub transactions: Vec<LedgerTransaction>,
    pub state_effect: Option<LoanStateEffect>,
}

#[derive(Debug, Clone)]
pub struct LoanMonthInput {
    pub slice: LoanSlice,
    pub loan: Option<FixedRateLoan>,
    pub update: Option<LoanTermsUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanContinuation {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub loan: Option<FixedRateLoan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanStateEffect {
    pub id: EffectId,
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub period: YearMonth,
    pub previous: Option<FixedRateLoan>,
    pub updated: Option<FixedRateLoan>,
    pub source_event_id: EventId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanEvent {
    pub id: EventId,
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub period: YearMonth,
    pub key: String,
    pub previous_balance: Option<Money>,
    pub updated_balance: Option<Money>,
    pub previous_remaining_months: Option<u32>,
    pub updated_remaining_months: Option<u32>,
    pub previous_annual_rate: Option<rust_decimal::Decimal>,
    pub updated_annual_rate: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanTermsUpdate {
    pub(super) expected_current: FixedRateLoan,
    pub(super) updated: Option<FixedRateLoan>,
    pub(super) effective_period: YearMonth,
    pub(super) principal_change: Money,
    pub(super) principal_transaction: Option<LedgerTransaction>,
}

pub struct LoanMonthCommand<'a> {
    pub period: YearMonth,
    pub loan: Option<FixedRateLoan>,
    pub update: Option<LoanTermsUpdate>,
    pub opening_ledger: &'a Ledger,
    pub trial_id: TrialId,
    pub branch_id: BranchId,
    pub ids: &'a dyn OutputIdFactory,
}

impl LoanTermsUpdate {
    pub const fn expected_loan(&self) -> FixedRateLoan {
        self.expected_current
    }
    pub const fn effective_period(&self) -> YearMonth {
        self.effective_period
    }
    pub const fn updated_loan(&self) -> Option<FixedRateLoan> {
        self.updated
    }
    pub const fn principal_change(&self) -> Money {
        self.principal_change
    }
    pub fn principal_transaction(&self) -> Option<&LedgerTransaction> {
        self.principal_transaction.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanRunUpdate {
    pub instance_id: ActivityInstanceId,
    pub update: LoanTermsUpdate,
}

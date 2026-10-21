use property_simulation_domain::{
    AccountId, ActivityInstanceId, EventId, LedgerTransactionId, Money, SliceId, YearMonth,
};

use crate::{BalanceSide, CashFlowCategory};

/// One debit or credit in a proposed ledger transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerEntry {
    pub(crate) account_id: AccountId,
    pub(crate) side: BalanceSide,
    pub(crate) amount: Money,
    pub(crate) cash_flow_category: Option<CashFlowCategory>,
}

impl LedgerEntry {
    pub const fn new(account_id: AccountId, side: BalanceSide, amount: Money) -> Self {
        Self {
            account_id,
            side,
            amount,
            cash_flow_category: None,
        }
    }
    pub const fn account_id(self) -> AccountId {
        self.account_id
    }
    pub const fn side(self) -> BalanceSide {
        self.side
    }
    pub const fn amount(self) -> Money {
        self.amount
    }
    pub const fn cash_flow_category(self) -> Option<CashFlowCategory> {
        self.cash_flow_category
    }
    pub const fn with_cash_flow_category(mut self, category: CashFlowCategory) -> Self {
        self.cash_flow_category = Some(category);
        self
    }
}

/// A caller-identified transaction proposed for a specific inclusive month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerTransactionSource {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub output_key: String,
    pub source_event_id: Option<EventId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerTransaction {
    pub(crate) id: LedgerTransactionId,
    pub(crate) period: YearMonth,
    pub(crate) entries: Vec<LedgerEntry>,
    pub(crate) source: Option<LedgerTransactionSource>,
}

impl LedgerTransaction {
    pub fn new(
        id: LedgerTransactionId,
        period: YearMonth,
        entries: impl IntoIterator<Item = LedgerEntry>,
    ) -> Self {
        Self {
            id,
            period,
            entries: entries.into_iter().collect(),
            source: None,
        }
    }
    pub fn with_source(mut self, source: LedgerTransactionSource) -> Self {
        self.source = Some(source);
        self
    }
    pub const fn id(&self) -> LedgerTransactionId {
        self.id
    }
    pub const fn period(&self) -> YearMonth {
        self.period
    }
    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }
    pub fn source(&self) -> Option<&LedgerTransactionSource> {
        self.source.as_ref()
    }
}

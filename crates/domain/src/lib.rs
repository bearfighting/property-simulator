//! Shared domain concepts and rules, independent of application and I/O layers.

mod error;
mod ids;
mod money;
mod time;

pub use error::DomainError;
pub use ids::{
    AccountId, ActivityInstanceId, BranchId, DecisionId, EffectId, EntityId, EventId,
    LedgerTransactionId, ResourceKey, RunId, ScenarioId, SliceId, SnapshotId, TrialId,
};
pub use money::{Currency, Money};
pub use time::{YearMonth, YearMonthRange};

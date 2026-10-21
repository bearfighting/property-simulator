//! Shared domain concepts and rules, independent of application and I/O layers.

mod error;
mod ids;
mod money;
mod time;

pub use error::DomainError;
pub use ids::{
    AccountId, ActivityInstanceId, BranchId, EffectId, EventId, LedgerTransactionId, ResourceKey,
    RunId, SliceId, SnapshotId, TrialId,
};
pub use money::{Currency, Money};
pub use time::{YearMonth, YearMonthRange};

//! Shared domain concepts and rules, independent of application and I/O layers.

mod error;
mod ids;
mod money;
mod time;

pub use error::DomainError;
pub use ids::{
    AccountId, BranchId, DecisionId, EntityId, EventId, LedgerTransactionId, ModelVersionId, RunId,
    ScenarioId, SliceId, TrialId,
};
pub use money::{Currency, Money, RoundingPolicy};
pub use time::{YearMonth, YearMonthRange};

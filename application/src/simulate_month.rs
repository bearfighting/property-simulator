//! Run one complete simulation month.

mod calculation;
mod execution;
mod ledger;
mod load;
mod persist;
mod prepare;
mod projection;
mod random;
mod repair_slices;
mod result;
mod settings;
mod standard_slices;

pub use execution::simulate_month;
pub use result::{
    CashFlowAccountChanges, CashFlowCategory, CashFlowCategoryChanges, CashFlowChanges,
    MonthSimulationError, MonthSimulationOutput,
};

//! Generate a simulation-wide report from persisted accounting facts.

pub(crate) mod cash_flow;
mod execution;
mod ledger;
mod projection;
mod result;

pub use execution::generate_report;
pub use result::{
    AccountAmount, CashFlowAccountSummary, CashFlowCategorySummary, CashFlowSummary,
    FinancialPositionReport, GenerateReportError, PeriodReport, SimulationReport,
};

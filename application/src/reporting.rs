//! Generate a simulation-wide report from persisted accounting facts.

pub(crate) mod cash_flow;
mod execution;
mod ledger;
mod projection;
mod result;
mod timeline;

pub use execution::generate_report;
pub use result::{
    AccountAmount, CashFlowAccountSummary, CashFlowCategorySummary, CashFlowSummary,
    FinancialPositionReport, GenerateReportError, PeriodReport, SimulationReport,
};
pub use timeline::{MonthTimelineItem, load_month_timeline};

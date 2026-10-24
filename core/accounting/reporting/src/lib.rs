//! Read-only projections and audit checks over ledger facts.

mod calculations;
mod error;
mod input;
mod reports;

pub use calculations::{audit, balance_sheet, cash_flow, income_statement, trial_balance};
pub use error::ReportingError;
pub use input::ReportingInput;
pub(crate) use reports::BalanceSheetTotals;
pub use reports::{
    AccountBalanceLine, AuditReport, BalanceSheet, BalanceSheetLine, CashFlowAccountClassification,
    CashFlowCategory, CashFlowCategoryTotal, CashFlowMovement, CashFlowReport,
    CashFlowSubcategoryTotal, IncomeStatement, IncomeStatementLine, TrialBalance,
};

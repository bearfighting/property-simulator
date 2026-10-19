//! Accounting definitions and validation built on shared domain identifiers.
//!
//! Account metadata and chart validation live here; posting and statement
//! projection are added in later milestones. This crate depends on `domain`
//! only for shared identifiers and values.

mod account;
mod account_code;
mod cash_flow;
mod chart;
mod error;
mod opening_balance;

pub use account::{Account, AccountCategory, BalanceSide};
pub use account_code::AccountCode;
pub use cash_flow::CashFlowCategory;
pub use chart::ChartOfAccounts;
pub use error::AccountError;
pub use opening_balance::{OpeningBalance, OpeningBalances, OpeningTotals};

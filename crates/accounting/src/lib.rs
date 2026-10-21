//! Value-based accounting rules built on shared domain identifiers and amounts.
//!
//! This crate defines account metadata, chart and opening-balance validation,
//! balanced transaction validation and posting, balance calculation, and
//! financial statement projections from a ledger. Callers own and pass ledger
//! values; this crate does not provide persistence or a resident ledger service.
//! Its only workspace dependency is `domain`.

mod account;
mod account_code;
mod cash_flow;
mod chart;
mod error;
mod ledger;
mod opening_balance;
mod projection;
mod transaction;

pub use account::{Account, AccountCategory, BalanceSide};
pub use account_code::AccountCode;
pub use cash_flow::CashFlowCategory;
pub use chart::ChartOfAccounts;
pub use error::AccountError;
pub use ledger::Ledger;
pub use opening_balance::{OpeningBalance, OpeningBalances, OpeningTotals};
pub use projection::{
    AccountBalance, BalanceSheet, CashFlowContribution, CashFlowStatement, CashFlowTotals,
    FinancialProjection, IncomeStatement, ValidatedProjection,
};
pub use transaction::{LedgerEntry, LedgerTransaction, LedgerTransactionSource};

use property_simulation_account::AccountCode;
use property_simulation_data_access::DataAccessError;
use property_simulation_money::Money;
use property_simulation_month::Month;
use property_simulation_reporting::ReportingError;
use std::{error::Error, fmt};

use crate::CashFlowCategory;

/// One account's amount in a derived financial statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountAmount {
    pub account_code: AccountCode,
    pub amount: Money,
}

/// Account totals grouped by financial-position section at the report cutoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinancialPositionReport {
    pub assets: Vec<AccountAmount>,
    pub liabilities: Vec<AccountAmount>,
    pub equity_accounts: Vec<AccountAmount>,
    pub total_assets: Money,
    pub total_liabilities: Money,
    pub equity_account_total: Money,
    pub cumulative_net_income: Money,
    pub total_equity: Money,
}

/// Signed cumulative cash movement for one main cash-flow category.
/// Inflows are zero or positive; outflows are zero or negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashFlowCategorySummary {
    pub category: CashFlowCategory,
    pub inflows: Money,
    pub outflows: Money,
}

/// Signed cumulative cash movement attributed to one account.
/// Inflows are zero or positive; outflows are zero or negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashFlowAccountSummary {
    pub account_code: AccountCode,
    pub category: CashFlowCategory,
    pub inflows: Money,
    pub outflows: Money,
}

/// Cash movements over all completed simulation months.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashFlowSummary {
    pub total_inflows: Money,
    pub total_outflows: Money,
    pub categories: [CashFlowCategorySummary; 3],
    pub accounts: Vec<CashFlowAccountSummary>,
}

/// Income, expenses, and cash movements over the completed simulation period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodReport {
    pub start_month: Month,
    pub end_month: Month,
    pub income: Vec<AccountAmount>,
    pub expenses: Vec<AccountAmount>,
    pub total_income: Money,
    pub total_expenses: Money,
    pub net_income: Money,
    pub cash_flow: CashFlowSummary,
}

/// Final derived report. Month 0 is a valid cutoff with no period report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationReport {
    pub simulation_name: String,
    pub as_of_month: Month,
    pub completed_months: u16,
    pub financial_position: FinancialPositionReport,
    pub period: Option<PeriodReport>,
}

/// Failure to load or derive a simulation-wide report.
#[derive(Debug)]
pub enum GenerateReportError {
    SimulationNotFound(String),
    DataAccess(DataAccessError),
    InvalidData(String),
    LedgerCalculation {
        operation: &'static str,
        message: String,
    },
    CoreReporting(ReportingError),
}

impl fmt::Display for GenerateReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimulationNotFound(simulation_id) => {
                write!(formatter, "simulation {simulation_id} does not exist")
            }
            Self::DataAccess(error) => write!(formatter, "{error}"),
            Self::InvalidData(message) => write!(formatter, "invalid simulation data: {message}"),
            Self::LedgerCalculation { operation, message } => {
                write!(formatter, "{operation} failed: {message}")
            }
            Self::CoreReporting(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for GenerateReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DataAccess(error) => Some(error),
            Self::CoreReporting(error) => Some(error),
            Self::SimulationNotFound(_) | Self::InvalidData(_) | Self::LedgerCalculation { .. } => {
                None
            }
        }
    }
}

impl From<DataAccessError> for GenerateReportError {
    fn from(error: DataAccessError) -> Self {
        Self::DataAccess(error)
    }
}

impl From<ReportingError> for GenerateReportError {
    fn from(error: ReportingError) -> Self {
        Self::CoreReporting(error)
    }
}

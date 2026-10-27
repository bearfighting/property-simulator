use property_simulation_data_access::DataAccessError;
use property_simulation_money::Money;
use property_simulation_month::Month;
use property_simulation_stop_condition::StopCondition;
use std::{error::Error, fmt};

/// Top-level cash-flow classification for amounts posted against cash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CashFlowCategory {
    Operating,
    Investing,
    Financing,
}

/// Positive and negative cash movements attributed to one category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashFlowCategoryChanges {
    pub category: CashFlowCategory,
    pub positive: Money,
    pub negative: Money,
}

/// Positive and negative cash movements attributed to one ledger account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashFlowAccountChanges {
    pub account_name: String,
    pub category: CashFlowCategory,
    pub positive: Money,
    pub negative: Money,
}

/// Separate totals for positive and negative cash changes during one month.
///
/// Consumers may rely on the signs and separation of these totals, but should
/// rely on the account-level movements when explaining individual amounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashFlowChanges {
    /// Sum of the positive cash movements; zero when there were none.
    pub positive: Money,
    /// Sum of the negative cash movements; zero when there were none.
    pub negative: Money,
    /// Category totals that add up to `positive` and `negative` respectively.
    pub categories: [CashFlowCategoryChanges; 3],
    /// Account totals that explain each category's movements.
    pub accounts: Vec<CashFlowAccountChanges>,
}

/// Results needed by the desktop after one complete month has been simulated.
///
/// It contains a single month's cash changes and satisfied automatic stop
/// conditions; lifecycle state remains the composition layer's responsibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthSimulationOutput {
    pub month: Month,
    pub cash_flow_changes: CashFlowChanges,
    pub satisfied_stop_conditions: Vec<StopCondition>,
}

/// Fatal error from one monthly simulation attempt.
#[derive(Debug)]
pub enum MonthSimulationError {
    DataAccess(DataAccessError),
    InvalidData(String),
    Calculation {
        operation: &'static str,
        message: String,
    },
}

impl MonthSimulationError {
    pub(super) fn calculation(operation: &'static str, error: impl fmt::Display) -> Self {
        Self::Calculation {
            operation,
            message: error.to_string(),
        }
    }

    pub(super) fn from_money(error: property_simulation_money::MoneyError) -> Self {
        Self::calculation("money arithmetic", error)
    }
}

impl fmt::Display for MonthSimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataAccess(error) => write!(formatter, "{error}"),
            Self::InvalidData(message) => write!(formatter, "invalid simulation data: {message}"),
            Self::Calculation { operation, message } => {
                write!(formatter, "{operation} failed: {message}")
            }
        }
    }
}

impl Error for MonthSimulationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DataAccess(error) => Some(error),
            Self::InvalidData(_) | Self::Calculation { .. } => None,
        }
    }
}

impl From<property_simulation_data_access::DataAccessError> for MonthSimulationError {
    fn from(error: property_simulation_data_access::DataAccessError) -> Self {
        Self::DataAccess(error)
    }
}

impl From<property_simulation_money::MoneyError> for MonthSimulationError {
    fn from(error: property_simulation_money::MoneyError) -> Self {
        Self::from_money(error)
    }
}

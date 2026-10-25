use std::{error::Error, fmt};

use property_simulation_account::AccountRole;
use property_simulation_data_access::DataAccessError;
use property_simulation_journal_entry::JournalEntryError;
use property_simulation_money::{Currency, Money, MoneyError};
use property_simulation_slice_catalogue::{
    FixedExpenseParameters, LoanParameters, RentalParameters, RepairCostParameters,
};
use property_simulation_stop_condition::StopCondition;

/// One configured Slice and its initial enabled state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredSlice<P> {
    pub enabled: bool,
    pub parameters: P,
}

/// Initial configuration for the Slice kinds persisted by the current schema.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InitialSliceSettings {
    pub loan: Option<ConfiguredSlice<LoanParameters>>,
    pub fixed_expense: Option<ConfiguredSlice<FixedExpenseParameters>>,
    pub rental: Option<ConfiguredSlice<RentalParameters>>,
    pub repair_occurrence_enabled: Option<bool>,
    pub repair_cost: Option<RepairCostParameters>,
}

/// A signed opening balance keyed by the stable semantic account role.
///
/// Positive values use the account's normal balance side; negative values use
/// its opposite side. The collection must form balanced opening facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningBalance {
    pub account_role: AccountRole,
    pub amount: Money,
}

/// All user-provided facts needed to initialize one simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializeSimulationRequest {
    /// Opaque simulation identifier assigned by the caller.
    pub simulation_id: String,
    pub name: String,
    pub currency: Currency,
    pub opening_balances: Vec<OpeningBalance>,
    pub slice_settings: InitialSliceSettings,
    pub stop_conditions: Vec<StopCondition>,
}

/// Failure while creating the initialization journal or persisting its facts.
#[derive(Debug)]
pub enum InitializeSimulationError {
    Entropy(String),
    MissingAccountRole(AccountRole),
    Money(MoneyError),
    JournalEntry(JournalEntryError),
    DataAccess(DataAccessError),
}

impl fmt::Display for InitializeSimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Entropy(error) => write!(f, "could not obtain simulation random root: {error}"),
            Self::MissingAccountRole(role) => {
                write!(
                    f,
                    "opening balance account role is not configured: {}",
                    role.as_str()
                )
            }
            Self::Money(error) => write!(f, "could not build opening balance: {error}"),
            Self::JournalEntry(error) => {
                write!(f, "could not build opening journal entry: {error}")
            }
            Self::DataAccess(error) => {
                write!(f, "could not persist simulation initialization: {error}")
            }
        }
    }
}

impl Error for InitializeSimulationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Entropy(_) | Self::MissingAccountRole(_) => None,
            Self::Money(error) => Some(error),
            Self::JournalEntry(error) => Some(error),
            Self::DataAccess(error) => Some(error),
        }
    }
}

impl From<MoneyError> for InitializeSimulationError {
    fn from(error: MoneyError) -> Self {
        Self::Money(error)
    }
}

impl From<JournalEntryError> for InitializeSimulationError {
    fn from(error: JournalEntryError) -> Self {
        Self::JournalEntry(error)
    }
}

impl From<DataAccessError> for InitializeSimulationError {
    fn from(error: DataAccessError) -> Self {
        Self::DataAccess(error)
    }
}

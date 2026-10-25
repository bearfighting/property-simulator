//! Application use cases that coordinate persisted facts and Core capabilities.

mod accounts;
mod initialize_simulation;

pub use accounts::initialization_account_catalogue;
pub use accounts::{
    CASH_ROLE, EQUITY_ROLE, INTEREST_EXPENSE_ROLE, MORTGAGE_ROLE, OPERATING_EXPENSE_ROLE,
    PROPERTY_ROLE, RENTAL_INCOME_ROLE, REPAIR_EXPENSE_ROLE,
};
pub use initialize_simulation::{
    ConfiguredSlice, InitialSliceSettings, InitializeSimulationError, InitializeSimulationRequest,
    OpeningBalance, initialize_simulation,
};

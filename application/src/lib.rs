//! Application use cases that coordinate persisted facts and Core capabilities.

mod accounts;
mod initialize_simulation;
mod reporting;
mod simulate_month;

pub use accounts::initialization_account_catalogue;
pub use accounts::{
    CASH_ROLE, EQUITY_ROLE, INTEREST_EXPENSE_ROLE, MORTGAGE_ROLE, OPERATING_EXPENSE_ROLE,
    PROPERTY_ROLE, RENTAL_INCOME_ROLE, REPAIR_EXPENSE_ROLE,
};
pub use initialize_simulation::{
    ConfiguredSlice, InitialSliceSettings, InitializeSimulationError, InitializeSimulationRequest,
    OpeningBalance, initialize_simulation,
};
pub use reporting::{
    AccountAmount, CashFlowAccountSummary, CashFlowCategorySummary, CashFlowSummary,
    FinancialPositionReport, GenerateReportError, PeriodReport, SimulationReport, generate_report,
};
pub use simulate_month::{
    CashFlowAccountChanges, CashFlowCategory, CashFlowCategoryChanges, CashFlowChanges,
    MonthSimulationError, MonthSimulationOutput, simulate_month,
};

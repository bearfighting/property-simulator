use property_simulation_month::Month;
use property_simulation_slice_catalogue::{
    FixedExpenseParameters, LoanParameters, RentalParameters, RepairCostParameters,
};

/// Versioned loan settings; absent `enabled` and parameters represent a removed setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanSettingsDto {
    pub simulation_id: String,
    pub effective_from_month: Month,
    pub enabled: Option<bool>,
    pub parameters: Option<LoanParameters>,
}

/// Versioned FixedExpense settings with a typed, mutually exclusive calculation method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedExpenseSettingsDto {
    pub simulation_id: String,
    pub effective_from_month: Month,
    pub enabled: Option<bool>,
    pub parameters: Option<FixedExpenseParameters>,
}

/// Versioned rental settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RentalSettingsDto {
    pub simulation_id: String,
    pub effective_from_month: Month,
    pub enabled: Option<bool>,
    pub parameters: Option<RentalParameters>,
}

/// Versioned repair occurrence enablement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairOccurrenceSettingsDto {
    pub simulation_id: String,
    pub effective_from_month: Month,
    pub enabled: Option<bool>,
}

/// Versioned repair cost parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairCostSettingsDto {
    pub simulation_id: String,
    pub effective_from_month: Month,
    pub parameters: Option<RepairCostParameters>,
}

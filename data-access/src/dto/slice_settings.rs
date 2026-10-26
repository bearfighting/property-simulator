use property_simulation_slice_catalogue::{
    FixedExpenseParameters, LoanParameters, RentalParameters, RepairCostParameters,
};

/// One simulation's loan settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanSettingsDto {
    pub simulation_id: String,
    pub enabled: Option<bool>,
    pub parameters: Option<LoanParameters>,
}

/// One simulation's fixed expense settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedExpenseSettingsDto {
    pub simulation_id: String,
    pub enabled: Option<bool>,
    pub parameters: Option<FixedExpenseParameters>,
}

/// One simulation's rental settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RentalSettingsDto {
    pub simulation_id: String,
    pub enabled: Option<bool>,
    pub parameters: Option<RentalParameters>,
}

/// One simulation's repair occurrence settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairOccurrenceSettingsDto {
    pub simulation_id: String,
    pub enabled: Option<bool>,
}

/// One simulation's repair cost settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairCostSettingsDto {
    pub simulation_id: String,
    pub parameters: Option<RepairCostParameters>,
}

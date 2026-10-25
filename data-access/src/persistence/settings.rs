use crate::{DataAccessError, dto::*};

/// Read and write the versioned settings facts represented by the schema.
#[allow(async_fn_in_trait)]
pub trait SettingsStore {
    async fn save_loan_settings(&self, settings: &LoanSettingsDto) -> Result<(), DataAccessError>;

    async fn load_loan_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<LoanSettingsDto>, DataAccessError>;

    async fn save_fixed_expense_settings(
        &self,
        settings: &FixedExpenseSettingsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_fixed_expense_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<FixedExpenseSettingsDto>, DataAccessError>;

    async fn save_rental_settings(
        &self,
        settings: &RentalSettingsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_rental_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RentalSettingsDto>, DataAccessError>;

    async fn save_repair_occurrence_settings(
        &self,
        settings: &RepairOccurrenceSettingsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_repair_occurrence_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairOccurrenceSettingsDto>, DataAccessError>;

    async fn save_repair_cost_settings(
        &self,
        settings: &RepairCostSettingsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_repair_cost_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairCostSettingsDto>, DataAccessError>;

    async fn save_stop_condition_set(
        &self,
        settings: &StopConditionSetDto,
    ) -> Result<(), DataAccessError>;

    async fn load_stop_condition_sets(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<StopConditionSetDto>, DataAccessError>;
}

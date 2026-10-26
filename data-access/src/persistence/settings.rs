use crate::{DataAccessError, dto::*};

/// Read and write each simulation's one-time settings facts.
#[allow(async_fn_in_trait)]
pub trait SettingsStore {
    async fn save_loan_settings(&self, settings: &LoanSettingsDto) -> Result<(), DataAccessError>;

    async fn load_loan_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Option<LoanSettingsDto>, DataAccessError>;

    async fn save_fixed_expense_settings(
        &self,
        settings: &FixedExpenseSettingsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_fixed_expense_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Option<FixedExpenseSettingsDto>, DataAccessError>;

    async fn save_rental_settings(
        &self,
        settings: &RentalSettingsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_rental_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Option<RentalSettingsDto>, DataAccessError>;

    async fn save_repair_occurrence_settings(
        &self,
        settings: &RepairOccurrenceSettingsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_repair_occurrence_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Option<RepairOccurrenceSettingsDto>, DataAccessError>;

    async fn save_repair_cost_settings(
        &self,
        settings: &RepairCostSettingsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_repair_cost_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Option<RepairCostSettingsDto>, DataAccessError>;

    async fn save_stop_conditions(
        &self,
        settings: &StopConditionsDto,
    ) -> Result<(), DataAccessError>;

    async fn load_stop_conditions(
        &self,
        simulation_id: &str,
    ) -> Result<StopConditionsDto, DataAccessError>;
}

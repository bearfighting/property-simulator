use property_simulation_data_access::{DataAccessError, SettingsStore, dto::*};

use crate::store::SqliteDataAccess;

impl SettingsStore for SqliteDataAccess {
    async fn save_loan_settings(&self, settings: &LoanSettingsDto) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_loan_settings(self, settings)
            .await
            .map_err(Into::into)
    }

    async fn load_loan_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<LoanSettingsDto>, DataAccessError> {
        SqliteDataAccess::load_loan_settings(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn save_fixed_expense_settings(
        &self,
        settings: &FixedExpenseSettingsDto,
    ) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_fixed_expense_settings(self, settings)
            .await
            .map_err(Into::into)
    }

    async fn load_fixed_expense_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<FixedExpenseSettingsDto>, DataAccessError> {
        SqliteDataAccess::load_fixed_expense_settings(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn save_rental_settings(
        &self,
        settings: &RentalSettingsDto,
    ) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_rental_settings(self, settings)
            .await
            .map_err(Into::into)
    }

    async fn load_rental_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RentalSettingsDto>, DataAccessError> {
        SqliteDataAccess::load_rental_settings(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn save_repair_occurrence_settings(
        &self,
        settings: &RepairOccurrenceSettingsDto,
    ) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_repair_occurrence_settings(self, settings)
            .await
            .map_err(Into::into)
    }

    async fn load_repair_occurrence_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairOccurrenceSettingsDto>, DataAccessError> {
        SqliteDataAccess::load_repair_occurrence_settings(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn save_repair_cost_settings(
        &self,
        settings: &RepairCostSettingsDto,
    ) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_repair_cost_settings(self, settings)
            .await
            .map_err(Into::into)
    }

    async fn load_repair_cost_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairCostSettingsDto>, DataAccessError> {
        SqliteDataAccess::load_repair_cost_settings(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn save_stop_condition_set(
        &self,
        settings: &StopConditionSetDto,
    ) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_stop_condition_set(self, settings)
            .await
            .map_err(Into::into)
    }

    async fn load_stop_condition_sets(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<StopConditionSetDto>, DataAccessError> {
        SqliteDataAccess::load_stop_condition_sets(self, simulation_id)
            .await
            .map_err(Into::into)
    }
}

use property_simulation_data_access::{DataAccessError, SimulationStore, dto::*};

use crate::store::SqliteDataAccess;

impl SimulationStore for SqliteDataAccess {
    async fn create_simulation(&self, simulation: &SimulationDto) -> Result<(), DataAccessError> {
        SqliteDataAccess::create_simulation(self, simulation)
            .await
            .map_err(Into::into)
    }

    async fn load_simulation(
        &self,
        simulation_id: &str,
    ) -> Result<Option<SimulationDto>, DataAccessError> {
        SqliteDataAccess::load_simulation(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn save_root_key(&self, root_key: &SimulationRootKeyDto) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_root_key(self, root_key)
            .await
            .map_err(Into::into)
    }

    async fn load_root_key(
        &self,
        simulation_id: &str,
    ) -> Result<Option<SimulationRootKeyDto>, DataAccessError> {
        SqliteDataAccess::load_root_key(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn save_account(&self, account: &SimulationAccountDto) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_account(self, account)
            .await
            .map_err(Into::into)
    }

    async fn load_accounts(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<SimulationAccountDto>, DataAccessError> {
        SqliteDataAccess::load_accounts(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn load_months(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<SimulationMonthDto>, DataAccessError> {
        SqliteDataAccess::load_months(self, simulation_id)
            .await
            .map_err(Into::into)
    }
}

use crate::{DataAccessError, dto::*};

/// Read and write simulation identity, root, account, and month index facts.
#[allow(async_fn_in_trait)]
pub trait SimulationStore {
    async fn create_simulation(&self, simulation: &SimulationDto) -> Result<(), DataAccessError>;

    async fn load_simulation(
        &self,
        simulation_id: &str,
    ) -> Result<Option<SimulationDto>, DataAccessError>;

    async fn save_root_key(&self, root_key: &SimulationRootKeyDto) -> Result<(), DataAccessError>;

    async fn load_root_key(
        &self,
        simulation_id: &str,
    ) -> Result<Option<SimulationRootKeyDto>, DataAccessError>;

    async fn save_account(&self, account: &SimulationAccountDto) -> Result<(), DataAccessError>;

    async fn load_accounts(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<SimulationAccountDto>, DataAccessError>;

    async fn load_months(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<SimulationMonthDto>, DataAccessError>;
}

use property_simulation_account::AccountCatalogue;
use property_simulation_data_access::DataAccess;

use super::{
    MonthSimulationError, MonthSimulationOutput, load::load_month_inputs, persist::persist_month,
    prepare::prepare_month,
};

/// Simulate, persist, and return one complete month.
///
/// The caller controls whether this work unit is currently allowed. This use
/// case does not read or change the desktop lifecycle state.
pub async fn simulate_month<D: DataAccess>(
    data_access: &D,
    account_catalogue: &AccountCatalogue,
    simulation_id: &str,
) -> Result<MonthSimulationOutput, MonthSimulationError> {
    let inputs = load_month_inputs(data_access, account_catalogue, simulation_id).await?;
    let prepared = prepare_month(&inputs)?;
    persist_month(data_access, simulation_id, prepared).await
}

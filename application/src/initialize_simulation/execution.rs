use property_simulation_account::AccountCatalogue;
use property_simulation_data_access::{
    DataAccess,
    dto::{
        JournalEntryDto, MonthlyEventsDto, SimulationAccountDto, SimulationDto,
        SimulationRootKeyDto, StopConditionsDto,
    },
};
use property_simulation_month::Month;
use property_simulation_random_engine::RandomRootKey;

use super::{
    opening_entry, settings,
    types::{InitializeSimulationError, InitializeSimulationRequest},
};

/// Initialize a simulation and persist its Month 0 opening journal.
///
/// The account catalogue is supplied by the caller as application-owned
/// accounting configuration; only its account codes are persisted. The random
/// root is obtained from the system here and persisted with the simulation.
pub async fn initialize_simulation<D: DataAccess>(
    data_access: &D,
    account_catalogue: &AccountCatalogue,
    request: InitializeSimulationRequest,
) -> Result<(), InitializeSimulationError> {
    let opening_entry = opening_entry::build(&request, account_catalogue)?;
    let mut root_bytes = [0; 32];
    getrandom::getrandom(&mut root_bytes)
        .map_err(|error| InitializeSimulationError::Entropy(error.to_string()))?;
    let root_key = RandomRootKey::from_bytes(root_bytes);
    let simulation_id = request.simulation_id;

    data_access
        .create_simulation(&SimulationDto {
            id: simulation_id.clone(),
            name: request.name,
            currency: request.currency,
        })
        .await?;
    data_access
        .save_root_key(&SimulationRootKeyDto::from_domain(
            simulation_id.clone(),
            &root_key,
        ))
        .await?;

    for account in account_catalogue.iter() {
        data_access
            .save_account(&SimulationAccountDto {
                simulation_id: simulation_id.clone(),
                account_code: account.code(),
            })
            .await?;
    }

    let opening_entry = JournalEntryDto::from_domain(simulation_id.clone(), 0, &opening_entry);
    data_access
        .save_accounting_month(
            &simulation_id,
            Month::initial(),
            &MonthlyEventsDto {
                simulation_id: simulation_id.clone(),
                month: Month::initial(),
                events: Vec::new(),
            },
            &[opening_entry],
            &[],
        )
        .await?;

    settings::save(data_access, &simulation_id, request.slice_settings).await?;
    data_access
        .save_stop_conditions(&StopConditionsDto {
            simulation_id,
            conditions: request.stop_conditions,
        })
        .await?;

    Ok(())
}

//! Bridge the GPUI callback to the Tokio-backed Application use case.

mod request;

use std::{rc::Rc, sync::Arc};

use property_simulation_account::AccountCatalogue;
use property_simulation_application::initialize_simulation;
use property_simulation_data_access_sqlite::SqliteDataAccess;
use property_simulation_gpui_ui::{InitializationFormData, InitializationHandler};
use property_simulation_lifecycle::WorkUnit;
use tokio::runtime::Runtime;

use super::session::SimulationSession;

pub(super) fn handler(
    runtime: Arc<Runtime>,
    data_access: SqliteDataAccess,
    account_catalogue: AccountCatalogue,
    session: SimulationSession,
) -> InitializationHandler {
    Rc::new(move |form| {
        let runtime = runtime.clone();
        let data_access = data_access.clone();
        let account_catalogue = account_catalogue.clone();
        let session = session.clone();

        Box::pin(async move {
            if session.allowed_work() != WorkUnit::InitialSettings {
                return Err("initial settings are not allowed in the current state".into());
            }

            let (result_sender, result_receiver) = tokio::sync::oneshot::channel();
            runtime.spawn(async move {
                let result = execute(&data_access, &account_catalogue, form).await;
                let _ = result_sender.send(result);
            });

            let result = result_receiver
                .await
                .unwrap_or_else(|error| Err(format!("initialization task stopped: {error}")));
            match result {
                Ok(simulation_id) => session
                    .initialize(simulation_id)
                    .ok_or_else(|| "initial settings are not allowed in the current state".into()),
                Err(error) => Err(error),
            }
        })
    })
}

async fn execute(
    data_access: &SqliteDataAccess,
    account_catalogue: &AccountCatalogue,
    form: InitializationFormData,
) -> Result<String, String> {
    let request = request::from_form(form)?;
    let simulation_id = request.simulation_id.clone();
    initialize_simulation(data_access, account_catalogue, request)
        .await
        .map_err(|error| error.to_string())?;
    Ok(simulation_id)
}

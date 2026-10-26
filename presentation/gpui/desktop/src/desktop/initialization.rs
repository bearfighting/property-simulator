//! Bridge the GPUI callback to the Tokio-backed Application use case.

mod request;

use std::{cell::Cell, rc::Rc, sync::Arc};

use property_simulation_account::AccountCatalogue;
use property_simulation_application::initialize_simulation;
use property_simulation_data_access_sqlite::SqliteDataAccess;
use property_simulation_gpui_ui::{InitializationFormData, InitializationHandler};
use property_simulation_lifecycle::{SimulationState, WorkUnit};
use tokio::runtime::Runtime;

pub(super) fn handler(
    runtime: Arc<Runtime>,
    data_access: SqliteDataAccess,
    account_catalogue: AccountCatalogue,
    lifecycle: Rc<Cell<SimulationState>>,
) -> InitializationHandler {
    Rc::new(move |form| {
        let runtime = runtime.clone();
        let data_access = data_access.clone();
        let account_catalogue = account_catalogue.clone();
        let lifecycle = lifecycle.clone();

        Box::pin(async move {
            let (result_sender, result_receiver) = tokio::sync::oneshot::channel();
            runtime.spawn(async move {
                let result = execute(&data_access, &account_catalogue, form).await;
                let _ = result_sender.send(result);
            });

            let result = result_receiver
                .await
                .unwrap_or_else(|error| Err(format!("initialization task stopped: {error}")));
            if result.is_ok() && lifecycle.get().allowed_work() == WorkUnit::InitialSettings {
                lifecycle.set(
                    lifecycle
                        .get()
                        .transition()
                        .expect("running follows initial settings"),
                );
            }
            result.map(|()| lifecycle.get())
        })
    })
}

async fn execute(
    data_access: &SqliteDataAccess,
    account_catalogue: &AccountCatalogue,
    form: InitializationFormData,
) -> Result<(), String> {
    let request = request::from_form(form)?;
    initialize_simulation(data_access, account_catalogue, request)
        .await
        .map_err(|error| error.to_string())
}

//! Desktop composition root: wire startup infrastructure, Application, and UI.

mod database;
mod initialization;

use std::sync::Arc;
use std::{cell::Cell, rc::Rc};

use property_simulation_application::initialization_account_catalogue;
use property_simulation_gpui_ui::run_initialization_ui;
use property_simulation_lifecycle::{SimulationState, new};

pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?,
    );
    let data_access = database::connect(&runtime)?;
    let account_catalogue = initialization_account_catalogue()?;
    let lifecycle = Rc::new(Cell::new(new()));
    let initial_state = SimulationState::Initial;
    let handler = initialization::handler(runtime, data_access, account_catalogue, lifecycle);

    run_initialization_ui(handler, initial_state);
    Ok(())
}

//! Desktop composition root: wire startup infrastructure, Application, and UI.

mod database;
mod initialization;
mod monthly_simulation;
mod reporting;
mod session;

use std::sync::Arc;

use property_simulation_application::initialization_account_catalogue;
use property_simulation_gpui_ui::run_simulation_ui;

use self::session::SimulationSession;

pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?,
    );
    let data_access = database::connect(&runtime)?;
    let account_catalogue = initialization_account_catalogue()?;
    let session = SimulationSession::new();
    let stop_handler = {
        let session = session.clone();
        std::rc::Rc::new(move || session.stop())
    };
    let reset_handler = {
        let session = session.clone();
        std::rc::Rc::new(move || session.reset())
    };
    let handler = initialization::handler(
        runtime.clone(),
        data_access.clone(),
        account_catalogue.clone(),
        session.clone(),
    );
    let report_handler = reporting::handler(
        runtime.clone(),
        data_access.clone(),
        account_catalogue.clone(),
        session.clone(),
    );
    let month_handler = monthly_simulation::handler(
        runtime.clone(),
        data_access.clone(),
        account_catalogue.clone(),
        session.clone(),
    );
    let timeline_handler = monthly_simulation::timeline_handler(
        runtime,
        data_access,
        account_catalogue,
        session.clone(),
    );

    run_simulation_ui(
        handler,
        month_handler,
        timeline_handler,
        report_handler,
        stop_handler,
        reset_handler,
    );
    Ok(())
}

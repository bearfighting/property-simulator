use gpui_kit::component::TitleBar;
use gpui_kit::{
    AppContext as _, WindowBounds, WindowOptions, application, assets, init, open_window, px, size,
};
use property_simulation_lifecycle::SimulationState;

use crate::{InitializationHandler, app::shell::AppShell};

/// Open the application window with a host-provided initialization handler.
pub fn run_initialization_ui(handler: InitializationHandler, initial_state: SimulationState) {
    application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1120.0), px(860.0)), cx)),
            window_min_size: Some(size(px(820.0), px(700.0))),
            ..TitleBar::window_options()
        };
        open_window(options, cx, {
            let handler = handler.clone();
            move |window, cx| {
                cx.new(|window_cx| AppShell::new(handler, initial_state, window, window_cx))
            }
        })
        .expect("failed to open the simulation window");
    });
}

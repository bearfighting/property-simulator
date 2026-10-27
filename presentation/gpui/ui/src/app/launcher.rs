use crate::{
    InitializationHandler, MonthlySimulationHandler, ReportingHandler, ResetSimulationHandler,
    StopSimulationHandler, app::shell::AppShell,
};
use gpui_kit::component::TitleBar;
use gpui_kit::{
    AppContext as _, WindowBounds, WindowOptions, application, assets, init, open_window, px, size,
};

/// Open the application window with host-provided simulation work handlers.
pub fn run_simulation_ui(
    handler: InitializationHandler,
    month_handler: MonthlySimulationHandler,
    report_handler: ReportingHandler,
    stop_handler: StopSimulationHandler,
    reset_handler: ResetSimulationHandler,
) {
    application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1120.0), px(860.0)), cx)),
            window_min_size: Some(size(px(820.0), px(700.0))),
            ..TitleBar::window_options()
        };
        open_window(options, cx, {
            let handler = handler.clone();
            let month_handler = month_handler.clone();
            let report_handler = report_handler.clone();
            let stop_handler = stop_handler.clone();
            let reset_handler = reset_handler.clone();
            move |window, cx| {
                cx.new(|window_cx| {
                    AppShell::new(
                        handler,
                        month_handler,
                        report_handler,
                        stop_handler,
                        reset_handler,
                        window,
                        window_cx,
                    )
                })
            }
        })
        .expect("failed to open the simulation window");
    });
}

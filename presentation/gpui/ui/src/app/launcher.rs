use crate::{
    InitializationHandler, MonthlySimulationHandler, MonthlyTimelineHandler, ReportingHandler,
    ResetSimulationHandler, StopSimulationHandler,
    app::shell::{AppShell, ShellHandlers},
};
use gpui_kit::component::TitleBar;
use gpui_kit::{
    AppContext as _, WindowBounds, WindowOptions, application, assets, init, open_window, px, size,
};
use std::sync::Arc;

/// Open the application window with host-provided simulation work handlers.
pub fn run_simulation_ui(
    handler: InitializationHandler,
    month_handler: MonthlySimulationHandler,
    timeline_handler: MonthlyTimelineHandler,
    report_handler: ReportingHandler,
    stop_handler: StopSimulationHandler,
    reset_handler: ResetSimulationHandler,
) {
    let icon = Arc::new(
        image::load_from_memory(include_bytes!(
            "../../../../../assets/app-icons/prop-orbit-512.png"
        ))
        .expect("embedded application icon must be a valid PNG")
        .to_rgba8(),
    );
    application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1120.0), px(860.0)), cx)),
            window_min_size: Some(size(px(820.0), px(700.0))),
            app_id: Some("com.proporbit.PropertySimulator".to_owned()),
            icon: Some(Arc::clone(&icon)),
            ..TitleBar::window_options()
        };
        open_window(options, cx, {
            let handler = handler.clone();
            let month_handler = month_handler.clone();
            let timeline_handler = timeline_handler.clone();
            let report_handler = report_handler.clone();
            let stop_handler = stop_handler.clone();
            let reset_handler = reset_handler.clone();
            move |window, cx| {
                cx.new(|window_cx| {
                    AppShell::new(
                        ShellHandlers {
                            initialization: handler,
                            month: month_handler,
                            timeline: timeline_handler,
                            report: report_handler,
                            stop: stop_handler,
                            reset: reset_handler,
                        },
                        window,
                        window_cx,
                    )
                })
            }
        })
        .expect("failed to open the simulation window");
    });
}

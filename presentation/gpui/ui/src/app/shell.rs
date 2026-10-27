use gpui_kit::component::{WindowExt as _, dialog::AlertDialog};
use gpui_kit::{AppContext as _, Context, Entity, IntoElement, Render, Window};
use property_simulation_lifecycle::SimulationState;

use crate::{
    InitializationHandler, MonthlySimulationHandler, MonthlySimulationViewData, ReportingHandler,
    ReportingViewData, ResetSimulationHandler, StopSimulationHandler, app::page::AppPage,
    pages::InitializationPage,
};

pub(crate) struct AppShell {
    page: AppPage,
    initialization: Entity<InitializationPage>,
    state: SimulationState,
    stop_handler: StopSimulationHandler,
    reset_handler: ResetSimulationHandler,
    month_handler: MonthlySimulationHandler,
    report_handler: ReportingHandler,
    monthly: MonthlyUiState,
    reporting: ReportingUiState,
    reporting_generation: u64,
}

#[derive(Default)]
struct MonthlyUiState {
    is_simulating: bool,
    result: Option<MonthlySimulationViewData>,
    error: Option<String>,
}

#[derive(Default)]
struct ReportingUiState {
    data: Option<ReportingViewData>,
    is_loading: bool,
    error: Option<String>,
}

impl AppShell {
    pub(super) fn new(
        handler: InitializationHandler,
        month_handler: MonthlySimulationHandler,
        report_handler: ReportingHandler,
        stop_handler: StopSimulationHandler,
        reset_handler: ResetSimulationHandler,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let shell = cx.entity().downgrade();
        let on_state_changed = {
            let shell = shell.clone();
            std::rc::Rc::new(
                move |state, cx: &mut gpui_kit::Context<InitializationPage>| {
                    let _ = shell.update(cx, |shell, cx| shell.initialization_succeeded(state, cx));
                },
            )
        };
        let on_continue = {
            let shell = shell.clone();
            std::rc::Rc::new(move |cx: &mut gpui_kit::Context<InitializationPage>| {
                let _ = shell.update(cx, |shell, cx| shell.navigate_to_simulation(cx));
            })
        };
        let initialization = cx
            .new(|cx| InitializationPage::new(handler, on_state_changed, on_continue, window, cx));
        Self {
            page: AppPage::Initialization,
            initialization,
            state: SimulationState::Initial,
            stop_handler,
            reset_handler,
            month_handler,
            report_handler,
            monthly: MonthlyUiState::default(),
            reporting: ReportingUiState::default(),
            reporting_generation: 0,
        }
    }

    pub(crate) fn initialization_succeeded(
        &mut self,
        state: SimulationState,
        cx: &mut Context<Self>,
    ) {
        self.state = state;
        cx.notify();
    }

    pub(super) fn navigate(&mut self, page: AppPage, cx: &mut Context<Self>) {
        self.page = page;
        if page == AppPage::Summary && self.state == SimulationState::Stopped {
            self.load_report(cx);
        } else {
            cx.notify();
        }
    }

    pub(crate) fn load_report(&mut self, cx: &mut Context<Self>) {
        if self.state != SimulationState::Stopped || self.reporting.is_loading {
            return;
        }
        if self.reporting.data.is_some() && self.reporting.error.is_none() {
            return;
        }
        self.reporting.is_loading = true;
        self.reporting.error = None;
        let generation = self.reporting_generation;
        cx.notify();

        let handler = self.report_handler.clone();
        cx.spawn(async move |shell, cx| {
            let response = handler().await;
            shell
                .update(cx, |this, cx| {
                    if this.reporting_generation != generation {
                        return;
                    }
                    this.reporting.is_loading = false;
                    match response {
                        Ok(data) => {
                            this.reporting.data = Some(data);
                            this.reporting.error = None;
                        }
                        Err(error) => this.reporting.error = Some(error),
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(crate) fn navigate_to_simulation(&mut self, cx: &mut Context<Self>) {
        self.navigate(AppPage::MonthlySimulation, cx);
    }

    pub(crate) fn simulate_month(&mut self, cx: &mut Context<Self>) {
        if self.state != SimulationState::Running || self.monthly.is_simulating {
            return;
        }
        self.monthly.is_simulating = true;
        self.monthly.error = None;
        cx.notify();

        let handler = self.month_handler.clone();
        cx.spawn(async move |shell, cx| {
            let response = handler().await;
            shell
                .update(cx, |this, cx| {
                    this.state = response.state;
                    this.monthly.is_simulating = false;
                    match response.result {
                        Ok(result) => {
                            this.monthly.result = Some(result);
                            this.monthly.error = None;
                        }
                        Err(error) => this.monthly.error = Some(error),
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn confirm_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let shell = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert: AlertDialog, _, _| {
            let shell = shell.clone();
            alert
                .title("End this simulation?")
                .description(
                    "No additional months will be simulated. You can still open Reporting to view the results.",
                )
                .confirm()
                .ok_text("End simulation")
                .cancel_text("Keep simulating")
                .on_ok(move |_, _, cx| {
                    shell
                        .update(cx, |shell, cx| shell.end_simulation(cx))
                        .unwrap_or(false)
                })
        });
    }

    pub(super) fn confirm_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state == SimulationState::Initial || self.monthly.is_simulating {
            return;
        }

        let shell = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert: AlertDialog, _, _| {
            let shell = shell.clone();
            alert
                .title("Reset this simulation?")
                .description(
                    "This run will be abandoned. You will not be able to continue it or view its results in the app for now. Its saved data will remain on this device.",
                )
                .confirm()
                .ok_text("Reset simulation")
                .cancel_text("Keep this simulation")
                .on_ok(move |_, window, cx| {
                    shell
                        .update(cx, |shell, cx| shell.reset_simulation(window, cx))
                        .unwrap_or(false)
                })
        });
    }

    fn reset_simulation(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.monthly.is_simulating {
            return false;
        }
        let Some(state) = (self.reset_handler)() else {
            return false;
        };

        self.state = state;
        self.page = AppPage::Initialization;
        self.monthly = MonthlyUiState::default();
        self.reporting_generation = self.reporting_generation.wrapping_add(1);
        self.reporting = ReportingUiState::default();
        self.initialization
            .update(cx, |page, cx| page.reset(window, cx));
        cx.notify();
        true
    }

    fn end_simulation(&mut self, cx: &mut Context<Self>) -> bool {
        if self.monthly.is_simulating {
            return false;
        }
        let Some(state) = (self.stop_handler)() else {
            return false;
        };
        self.state = state;
        if self.page == AppPage::Summary {
            self.reporting.data = None;
            self.load_report(cx);
        } else {
            cx.notify();
        }
        true
    }
}

impl Render for AppShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::view::render(
            super::view::ViewState {
                page: self.page,
                state: self.state,
                monthly: super::view::MonthlyViewState {
                    is_simulating: self.monthly.is_simulating,
                    result: self.monthly.result.clone(),
                    error: self.monthly.error.clone(),
                },
                reporting: super::view::ReportingViewState {
                    data: self.reporting.data.clone(),
                    is_loading: self.reporting.is_loading,
                    error: self.reporting.error.clone(),
                },
            },
            self.initialization.clone(),
            cx,
        )
    }
}

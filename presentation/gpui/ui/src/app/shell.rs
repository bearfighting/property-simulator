use gpui_kit::component::{WindowExt as _, dialog::AlertDialog};
use gpui_kit::{AppContext as _, Context, Entity, IntoElement, Render, Window};
use property_simulation_lifecycle::SimulationState;

use crate::{
    InitializationHandler, MONTHS_PER_TIMELINE_PAGE, MonthlySimulationHandler,
    MonthlySimulationViewData, MonthlyTimelineHandler, ReportingHandler, ReportingViewData,
    ResetSimulationHandler, StopSimulationHandler, TimelinePageDirection, app::page::AppPage,
    pages::InitializationPage,
};

pub(crate) struct AppShell {
    page: AppPage,
    initialization: Entity<InitializationPage>,
    state: SimulationState,
    stop_handler: StopSimulationHandler,
    reset_handler: ResetSimulationHandler,
    month_handler: MonthlySimulationHandler,
    timeline_handler: MonthlyTimelineHandler,
    report_handler: ReportingHandler,
    monthly: MonthlyUiState,
    timeline_generation: u64,
    timeline_page: usize,
    reporting: ReportingUiState,
    reporting_generation: u64,
}

#[derive(Default)]
struct MonthlyUiState {
    is_simulating: bool,
    is_loading_timeline: bool,
    timeline: Vec<MonthlySimulationViewData>,
    selected_month: Option<u16>,
    timeline_error: Option<String>,
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
        timeline_handler: MonthlyTimelineHandler,
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
            timeline_handler,
            report_handler,
            monthly: MonthlyUiState::default(),
            timeline_generation: 0,
            timeline_page: 0,
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
        self.load_timeline(cx);
    }

    pub(crate) fn navigate_to_reporting(&mut self, cx: &mut Context<Self>) {
        if self.state == SimulationState::Stopped {
            self.navigate(AppPage::Summary, cx);
        }
    }

    pub(crate) fn load_timeline(&mut self, cx: &mut Context<Self>) {
        if self.monthly.is_loading_timeline || self.state == SimulationState::Initial {
            return;
        }
        self.monthly.is_loading_timeline = true;
        self.monthly.timeline_error = None;
        let generation = self.timeline_generation;
        let handler = self.timeline_handler.clone();
        cx.notify();

        cx.spawn(async move |shell, cx| {
            let response = handler().await;
            shell
                .update(cx, |this, cx| {
                    if this.timeline_generation != generation {
                        return;
                    }
                    this.monthly.is_loading_timeline = false;
                    match response {
                        Ok(timeline) => {
                            let prior_timeline = &this.monthly.timeline;
                            let mut timeline = timeline;
                            for item in &mut timeline {
                                item.stop_reason = prior_timeline
                                    .iter()
                                    .find(|prior| prior.month_number == item.month_number)
                                    .and_then(|prior| prior.stop_reason.clone());
                            }
                            this.monthly.selected_month = this
                                .monthly
                                .selected_month
                                .filter(|selected| {
                                    timeline.iter().any(|item| item.month_number == *selected)
                                })
                                .or_else(|| timeline.last().map(|item| item.month_number));
                            this.monthly.timeline = timeline;
                            let page_count = this
                                .monthly
                                .timeline
                                .len()
                                .div_ceil(MONTHS_PER_TIMELINE_PAGE);
                            this.timeline_page =
                                this.timeline_page.min(page_count.saturating_sub(1));
                            this.monthly.timeline_error = None;
                        }
                        Err(error) => this.monthly.timeline_error = Some(error),
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(crate) fn select_month(&mut self, month_number: u16, cx: &mut Context<Self>) {
        if self
            .monthly
            .timeline
            .iter()
            .any(|item| item.month_number == month_number)
        {
            self.monthly.selected_month = Some(month_number);
            cx.notify();
        }
    }

    pub(crate) fn change_timeline_page(
        &mut self,
        direction: TimelinePageDirection,
        cx: &mut Context<Self>,
    ) {
        let page_count = self
            .monthly
            .timeline
            .len()
            .div_ceil(MONTHS_PER_TIMELINE_PAGE);
        self.timeline_page = match direction {
            TimelinePageDirection::Newer => self.timeline_page.saturating_sub(1),
            TimelinePageDirection::Older => self
                .timeline_page
                .saturating_add(1)
                .min(page_count.saturating_sub(1)),
        };
        let end = self
            .monthly
            .timeline
            .len()
            .saturating_sub(self.timeline_page * MONTHS_PER_TIMELINE_PAGE);
        if let Some(month) = end
            .checked_sub(1)
            .and_then(|index| self.monthly.timeline.get(index))
        {
            self.monthly.selected_month = Some(month.month_number);
        }
        cx.notify();
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
                            let month_number = result.month_number;
                            if let Some(existing) = this
                                .monthly
                                .timeline
                                .iter_mut()
                                .find(|item| item.month_number == month_number)
                            {
                                *existing = result;
                            } else {
                                this.monthly.timeline.push(result);
                            }
                            this.monthly.timeline.sort_by_key(|item| item.month_number);
                            this.monthly.selected_month = Some(month_number);
                            this.timeline_page = 0;
                            this.monthly.timeline_error = None;
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
        self.timeline_generation = self.timeline_generation.wrapping_add(1);
        self.timeline_page = 0;
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
                    is_loading_timeline: self.monthly.is_loading_timeline,
                    timeline: self.monthly.timeline.clone(),
                    selected_month: self.monthly.selected_month,
                    timeline_page: self.timeline_page,
                    timeline_error: self.monthly.timeline_error.clone(),
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
